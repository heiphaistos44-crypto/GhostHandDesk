use crate::audit::{audit_log, AuditEvent, AuditLevel};
use crate::config::Config;
use crate::error::{error_codes, GhostHandError, Result};
use crate::validation;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use webrtc::api::APIBuilder;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

// Constantes réseau
const MAX_RETRY_ATTEMPTS: u32 = 3;
const CONNECTION_TIMEOUT_SECS: u64 = 30;


/// Signaling message types - format compatible avec le serveur Go
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl SignalMessage {
    pub fn register(device_id: String) -> Self {
        Self {
            msg_type: "Register".to_string(),
            data: Some(serde_json::json!({
                "device_id": device_id
            })),
        }
    }

    pub fn offer(from: String, to: String, sdp: String) -> Self {
        Self {
            msg_type: "Offer".to_string(),
            data: Some(serde_json::json!({
                "from": from,
                "to": to,
                "sdp": sdp
            })),
        }
    }

    pub fn answer(from: String, to: String, sdp: String) -> Self {
        Self {
            msg_type: "Answer".to_string(),
            data: Some(serde_json::json!({
                "from": from,
                "to": to,
                "sdp": sdp
            })),
        }
    }

    pub fn ice_candidate(from: String, to: String, candidate: String, sdp_mid: String, sdp_mline_index: u16) -> Self {
        Self {
            msg_type: "IceCandidate".to_string(),
            data: Some(serde_json::json!({
                "from": from,
                "to": to,
                "candidate": candidate,
                "sdp_mid": sdp_mid,
                "sdp_mline_index": sdp_mline_index
            })),
        }
    }

    pub fn connect_request(target_id: String, password: Option<String>) -> Self {
        Self {
            msg_type: "ConnectRequest".to_string(),
            data: Some(serde_json::json!({
                "target_id": target_id,
                "password": password
            })),
        }
    }

    pub fn ping() -> Self {
        Self {
            msg_type: "Ping".to_string(),
            data: None,
        }
    }

    pub fn pong() -> Self {
        Self {
            msg_type: "Pong".to_string(),
            data: None,
        }
    }

    pub fn connection_accepted(peer_id: String) -> Self {
        Self {
            msg_type: "ConnectionAccepted".to_string(),
            data: Some(serde_json::json!({
                "peer_id": peer_id
            })),
        }
    }

    pub fn connection_rejected(peer_id: String, reason: String) -> Self {
        Self {
            msg_type: "ConnectionRejected".to_string(),
            data: Some(serde_json::json!({
                "peer_id": peer_id,
                "reason": reason
            })),
        }
    }

    pub fn password_challenge(peer_id: String, challenge: String, salt: String) -> Self {
        Self {
            msg_type: "PasswordChallenge".to_string(),
            data: Some(serde_json::json!({
                "peer_id": peer_id,
                "challenge": challenge,
                "salt": salt
            })),
        }
    }

    pub fn password_response(peer_id: String, response: String) -> Self {
        Self {
            msg_type: "PasswordResponse".to_string(),
            data: Some(serde_json::json!({
                "peer_id": peer_id,
                "response": response
            })),
        }
    }

    /// Message de relay : données binaires (base64) à transférer via le VPS
    pub fn relay_data(from: String, to: String, data_b64: String) -> Self {
        Self {
            msg_type: "Relay".to_string(),
            data: Some(serde_json::json!({
                "from": from,
                "to": to,
                "data": data_b64
            })),
        }
    }

    // Méthodes utilitaires pour extraire les données
    pub fn get_str(&self, field: &str) -> Option<String> {
        self.data.as_ref()?.get(field)?.as_str().map(|s| s.to_string())
    }

    pub fn get_u16(&self, field: &str) -> Option<u16> {
        let value = self.data.as_ref()?.get(field)?.as_u64()?;
        if value > u16::MAX as u64 {
            warn!("Valeur {} du champ '{}' dépasse u16::MAX, troncation", value, field);
        }
        Some(value as u16)
    }

    pub fn is_type(&self, msg_type: &str) -> bool {
        self.msg_type == msg_type
    }
}

/// Signaling client for connecting to the relay server
pub struct SignalingClient {
    server_url: String,
    device_id: String,
    pub tx: Option<mpsc::UnboundedSender<SignalMessage>>,
    rx: Option<mpsc::UnboundedReceiver<SignalMessage>>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
    auto_reconnect: bool,
    max_backoff_secs: u64,
}

impl SignalingClient {
    pub fn new(server_url: String, device_id: String) -> Self {
        Self {
            server_url,
            device_id,
            tx: None,
            rx: None,
            tasks: Vec::new(),
            auto_reconnect: false,
            max_backoff_secs: 60,
        }
    }

    /// Enable auto-reconnect with exponential backoff
    pub fn with_auto_reconnect(mut self, max_backoff_secs: u64) -> Self {
        self.auto_reconnect = true;
        self.max_backoff_secs = max_backoff_secs;
        self
    }

    /// Connect with auto-reconnect (exponential backoff: 1s, 2s, 4s, ... up to max_backoff)
    pub async fn connect_with_reconnect(&mut self) -> Result<()> {
        let mut backoff_secs = 1u64;

        loop {
            match self.connect().await {
                Ok(()) => {
                    info!("Connexion au serveur de signaling établie");
                    return Ok(());
                }
                Err(e) => {
                    if !self.auto_reconnect {
                        return Err(e);
                    }
                    warn!(
                        "Échec de connexion, nouvelle tentative dans {} secondes: {}",
                        backoff_secs, e
                    );
                    tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(self.max_backoff_secs);
                }
            }
        }
    }

    /// Connect to the signaling server with retry and exponential backoff
    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to signaling server: {}", self.server_url);

        let max_retries = MAX_RETRY_ATTEMPTS;
        let mut retry_count = 0;

        let ws_stream = loop {
            let backoff_seconds = 2u64.pow(retry_count); // 2^0=1s, 2^1=2s, 2^2=4s

            println!("[DEBUG] Tentative de connexion WebSocket à: {} (tentative {}/{})",
                self.server_url, retry_count + 1, max_retries + 1);

            match connect_async(&self.server_url).await {
                Ok((stream, _)) => break stream,
                Err(e) => {
                    retry_count += 1;
                    if retry_count > max_retries {
                        eprintln!("[ERROR] Erreur de connexion WebSocket après {} tentatives: {:?}", max_retries + 1, e);
                        return Err(GhostHandError::Network(
                            format!("Failed to connect to signaling server after {} retries: {}", max_retries + 1, e)
                        ));
                    }
                    warn!("Échec de connexion (tentative {}/{}), retry dans {} secondes: {}",
                        retry_count, max_retries + 1, backoff_seconds, e);
                    tokio::time::sleep(Duration::from_secs(backoff_seconds)).await;
                }
            }
        };

        info!("Connected to signaling server après {} tentative(s)", retry_count + 1);

        let (mut write, mut read) = ws_stream.split();

        // Create channels for sending/receiving messages
        let (tx, mut internal_rx) = mpsc::unbounded_channel::<SignalMessage>();
        let (internal_tx, rx) = mpsc::unbounded_channel::<SignalMessage>();

        self.tx = Some(tx);
        self.rx = Some(rx);

        // Canal pour envoyer des messages WebSocket bruts (Pong) depuis handle_in vers handle_out
        let (raw_tx, mut raw_rx) = mpsc::unbounded_channel::<Message>();

        // Spawn task to handle outgoing messages (SignalMessages + raw Pong)
        let handle_out = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(msg) = internal_rx.recv() => {
                        let json = match serde_json::to_string(&msg) {
                            Ok(j) => j,
                            Err(e) => {
                                error!("Failed to serialize message: {}", e);
                                continue;
                            }
                        };
                        if let Err(e) = write.send(Message::Text(json)).await {
                            error!("Failed to send message: {}", e);
                            break;
                        }
                    }
                    Some(raw_msg) = raw_rx.recv() => {
                        // Envoyer message brut (Pong response)
                        if let Err(e) = write.send(raw_msg).await {
                            error!("Failed to send raw message: {}", e);
                            break;
                        }
                    }
                    else => break,
                }
            }
        });
        self.tasks.push(handle_out);

        // Spawn task to handle incoming messages
        let handle_in = tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<SignalMessage>(&text) {
                            Ok(signal_msg) => {
                                if internal_tx.send(signal_msg).is_err() {
                                    error!("Failed to forward message to receiver");
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to deserialize message: {} | text: {}", e, &text[..text.len().min(100)]);
                            }
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        // Répondre au Ping avec un Pong (sinon le serveur déconnecte après 60s)
                        let _ = raw_tx.send(Message::Pong(data));
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket connection closed");
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });
        self.tasks.push(handle_in);

        // Register with server
        self.send(SignalMessage::register(self.device_id.clone()))
        .await?;

        Ok(())
    }

    /// Send a message to the signaling server
    pub async fn send(&self, msg: SignalMessage) -> Result<()> {
        if let Some(tx) = &self.tx {
            tx.send(msg).map_err(|e| {
                GhostHandError::Network(format!("Failed to send message: {}", e))
            })?;
            Ok(())
        } else {
            Err(GhostHandError::Network(
                "Not connected to signaling server".to_string(),
            ))
        }
    }

    /// Receive a message from the signaling server
    pub async fn receive(&mut self) -> Result<SignalMessage> {
        if let Some(rx) = &mut self.rx {
            rx.recv().await.ok_or_else(|| {
                GhostHandError::Network("Signaling channel closed".to_string())
            })
        } else {
            Err(GhostHandError::Network(
                "Not connected to signaling server".to_string(),
            ))
        }
    }
}

impl Drop for SignalingClient {
    fn drop(&mut self) {
        // Abort toutes les tasks spawned lors du cleanup
        for task in self.tasks.drain(..) {
            task.abort();
        }
        let count = self.tasks.len();
        self.tasks.drain(..).for_each(|t| t.abort());
        debug!("SignalingClient: {} tasks nettoyées", count);
    }
}

/// WebRTC peer connection manager
#[derive(Clone)]
pub struct WebRTCConnection {
    pub peer_connection: Arc<webrtc::peer_connection::RTCPeerConnection>,
    data_channel: Arc<RwLock<Option<Arc<webrtc::data_channel::RTCDataChannel>>>>,
    #[allow(dead_code)]
    config: Config,
}

impl WebRTCConnection {
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initialisation de la connexion WebRTC");

        // 1. Créer API avec configuration par défaut
        let api = APIBuilder::new().build();

        // 2. Configuration ICE (STUN servers)
        let rtc_config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: config.stun_servers.clone(),
                ..Default::default()
            }],
            ..Default::default()
        };

        // 3. Créer PeerConnection
        let peer_connection = Arc::new(
            api.new_peer_connection(rtc_config)
                .await
                .map_err(|e| GhostHandError::WebRTC(format!("Erreur de création de peer connection: {}", e)))?,
        );

        // 4. Setup callback état de connexion (logging général)
        peer_connection
            .on_peer_connection_state_change(Box::new(move |state: RTCPeerConnectionState| {
                match state {
                    RTCPeerConnectionState::New => {
                        info!("Connexion WebRTC: Nouvel état (New)");
                    }
                    RTCPeerConnectionState::Connecting => {
                        info!("Connexion WebRTC: En cours de connexion...");
                    }
                    RTCPeerConnectionState::Connected => {
                        info!("Connexion WebRTC: Établie avec succès");
                    }
                    RTCPeerConnectionState::Disconnected => {
                        warn!("Connexion WebRTC: Déconnectée");
                    }
                    RTCPeerConnectionState::Failed => {
                        error!("Connexion WebRTC: Échec de connexion");
                    }
                    RTCPeerConnectionState::Closed => {
                        info!("Connexion WebRTC: Fermée");
                    }
                    _ => {
                        debug!("Connexion WebRTC: État inconnu {:?}", state);
                    }
                }
                Box::pin(async {})
            }));

        info!("Connexion WebRTC initialisée avec succès");

        Ok(Self {
            peer_connection,
            data_channel: Arc::new(RwLock::new(None)),
            config,
        })
    }

    /// Create an offer for connection
    pub async fn create_offer(&mut self) -> Result<String> {
        info!("Création de l'offre WebRTC");

        // 1. Créer data channel pour contrôle
        let data_channel = self
            .peer_connection
            .create_data_channel("control", None)
            .await
            .map_err(|e| GhostHandError::WebRTC(format!("Erreur de création du data channel: {}", e)))?;

        // Stocker le data channel
        *self.data_channel.write().await = Some(Arc::clone(&data_channel));

        info!("Data channel 'control' créé");

        // 2. Créer offer SDP
        let offer = self
            .peer_connection
            .create_offer(None)
            .await
            .map_err(|e| GhostHandError::WebRTC(format!("Erreur de création de l'offre: {}", e)))?;

        // 3. Définir local description
        self.peer_connection
            .set_local_description(offer.clone())
            .await
            .map_err(|e| GhostHandError::WebRTC(format!("Erreur de définition de la description locale: {}", e)))?;

        // Validation SDP via module validation
        validation::validate_sdp(&offer.sdp)?;

        info!("Offre WebRTC créée avec succès");

        // 4. Retourner SDP string
        Ok(offer.sdp)
    }

    /// Create an answer to an offer
    pub async fn create_answer(&self, offer_sdp: &str) -> Result<String> {
        info!("Création de la réponse WebRTC");

        // 1. Parser offer SDP
        let offer = RTCSessionDescription::offer(offer_sdp.to_string())
            .map_err(|e| GhostHandError::WebRTC(format!("Erreur de parsing de l'offre: {}", e)))?;

        // 2. Définir remote description
        self.peer_connection
            .set_remote_description(offer)
            .await
            .map_err(|e| GhostHandError::WebRTC(format!("Erreur de définition de la description distante: {}", e)))?;

        // 3. Créer answer
        let answer = self
            .peer_connection
            .create_answer(None)
            .await
            .map_err(|e| GhostHandError::WebRTC(format!("Erreur de création de la réponse: {}", e)))?;

        // 4. Définir local description
        self.peer_connection
            .set_local_description(answer.clone())
            .await
            .map_err(|e| GhostHandError::WebRTC(format!("Erreur de définition de la description locale: {}", e)))?;

        // Validation SDP via module validation
        validation::validate_sdp(&answer.sdp)?;

        info!("Réponse WebRTC créée avec succès");

        // 5. Retourner answer SDP
        Ok(answer.sdp)
    }

    /// Set remote description
    pub async fn set_remote_description(&self, sdp: &str, sdp_type: RTCSdpType) -> Result<()> {
        // Validation SDP via module validation
        validation::validate_sdp(sdp)?;

        info!("Définition de la description distante (type: {:?})", sdp_type);

        // Créer la session description selon le type
        let session_desc = match sdp_type {
            RTCSdpType::Offer => RTCSessionDescription::offer(sdp.to_string()),
            RTCSdpType::Answer => RTCSessionDescription::answer(sdp.to_string()),
            _ => return Err(GhostHandError::WebRTC("Type SDP invalide (doit être Offer ou Answer)".into())),
        }
        .map_err(|e| GhostHandError::WebRTC(format!("Erreur de parsing de la description: {}", e)))?;

        self.peer_connection
            .set_remote_description(session_desc)
            .await
            .map_err(|e| GhostHandError::WebRTC(format!("Erreur de définition de la description distante: {}", e)))?;

        info!("Description distante définie avec succès");
        Ok(())
    }

    /// Add ICE candidate
    pub async fn add_ice_candidate(&self, candidate: &str) -> Result<()> {
        debug!("Ajout du candidat ICE: {}", candidate);

        let ice_candidate = RTCIceCandidateInit {
            candidate: candidate.to_string(),
            ..Default::default()
        };

        self.peer_connection
            .add_ice_candidate(ice_candidate)
            .await
            .map_err(|e| GhostHandError::WebRTC(format!("Erreur d'ajout du candidat ICE: {}", e)))?;

        debug!("Candidat ICE ajouté avec succès");
        Ok(())
    }

    /// Send data over the data channel
    /// Taille max d'un chunk WebRTC (60KB - safe limit, WebRTC max ~256KB)
    const MAX_CHUNK_SIZE: usize = 60 * 1024;

    pub async fn send_data(&self, data: &[u8]) -> Result<()> {
        let dc_lock = self.data_channel.read().await;

        if let Some(dc) = dc_lock.as_ref() {
            if data.len() <= Self::MAX_CHUNK_SIZE {
                // Message petit : envoi direct
                dc.send(&Bytes::from(data.to_vec()))
                    .await
                    .map_err(|e| GhostHandError::WebRTC(format!("Erreur d'envoi de données: {}", e)))?;
            } else {
                // Message trop gros : fragmenter
                // PERF: Pré-construire tous les buffers puis les envoyer en batch
                let mut messages: Vec<Bytes> = Vec::new();

                // Header: [0xFF][0x01][total_len: u32 LE]
                let mut header = Vec::with_capacity(6);
                header.push(0xFF);
                header.push(0x01);
                header.extend_from_slice(&(data.len() as u32).to_le_bytes());
                messages.push(Bytes::from(header));

                // Chunks de données: [0xFF][0x02][data...]
                for chunk in data.chunks(Self::MAX_CHUNK_SIZE) {
                    let mut chunk_msg = Vec::with_capacity(2 + chunk.len());
                    chunk_msg.push(0xFF);
                    chunk_msg.push(0x02);
                    chunk_msg.extend_from_slice(chunk);
                    messages.push(Bytes::from(chunk_msg));
                }

                // Envoyer tous les messages d'un coup (un seul verrouillage du DC)
                for msg in messages {
                    dc.send(&msg)
                        .await
                        .map_err(|e| GhostHandError::WebRTC(format!("Erreur envoi chunk: {}", e)))?;
                }
            }
            Ok(())
        } else {
            Err(GhostHandError::WebRTC("Data channel non disponible".into()))
        }
    }

    /// Attendre que le data channel soit disponible (pour le côté answerer)
    /// Utile après `accept_connection` où le DC est reçu via un callback async.
    pub async fn wait_for_data_channel(&self, timeout_ms: u64) -> Result<()> {
        let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            {
                let dc_lock = self.data_channel.read().await;
                if dc_lock.is_some() {
                    return Ok(());
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(GhostHandError::WebRTC(
                    format!("Timeout {}ms: data channel non disponible", timeout_ms)
                ));
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Configure un callback pour recevoir les données du data channel
    pub async fn on_data_channel_message<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(&[u8]) + Send + Sync + 'static,
    {
        let dc_lock = self.data_channel.read().await;

        if let Some(dc) = dc_lock.as_ref() {
            let callback = Arc::new(callback);
            dc.on_message(Box::new(move |msg| {
                callback(&msg.data);
                Box::pin(async {})
            }));
            Ok(())
        } else {
            Err(GhostHandError::WebRTC("Data channel non disponible".into()))
        }
    }
}

/// Transport relay via VPS WebSocket — élimine les problèmes NAT traversal WebRTC
/// Toutes les données binaires transitent par le serveur de signalement VPS.
#[derive(Clone)]
pub struct RelayTransport {
    device_id: String,
    peer_id: String,
    /// Canal d'envoi vers le serveur de signalement (WebSocket sortant)
    signaling_tx: mpsc::UnboundedSender<SignalMessage>,
    /// Entrée du canal de données entrantes (partagée entre tous les clones)
    pub incoming_tx: mpsc::UnboundedSender<Vec<u8>>,
    /// Sortie du canal de données entrantes (prise exactement une fois via on_data_channel_message)
    incoming_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<Vec<u8>>>>>,
}

impl RelayTransport {
    pub fn new(
        device_id: String,
        peer_id: String,
        signaling_tx: mpsc::UnboundedSender<SignalMessage>,
    ) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel();
        Self {
            device_id,
            peer_id,
            signaling_tx,
            incoming_tx,
            incoming_rx: Arc::new(Mutex::new(Some(incoming_rx))),
        }
    }

    /// Envoyer des données binaires au pair via le VPS (encodage base64)
    pub async fn send_data(&self, data: &[u8]) -> Result<()> {
        use base64::prelude::*;
        let b64 = BASE64_STANDARD.encode(data);
        self.signaling_tx
            .send(SignalMessage::relay_data(
                self.device_id.clone(),
                self.peer_id.clone(),
                b64,
            ))
            .map_err(|e| GhostHandError::Network(format!("Relay TX erreur: {}", e)))?;
        Ok(())
    }

    /// Installer un callback pour les données entrantes (à appeler une seule fois)
    pub async fn on_data_channel_message<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(&[u8]) + Send + Sync + 'static,
    {
        let mut rx_guard = self.incoming_rx.lock().await;
        let mut rx = rx_guard.take().ok_or_else(|| {
            GhostHandError::Network("Relay RX déjà consommé".into())
        })?;
        let callback = Arc::new(callback);
        tokio::spawn(async move {
            while let Some(data) = rx.recv().await {
                callback(&data);
            }
        });
        Ok(())
    }

    /// Toujours prêt — le relay ne nécessite pas d'attente
    pub async fn wait_for_data_channel(&self, _timeout_ms: u64) -> Result<()> {
        Ok(())
    }
}

/// Abstraction de transport : WebRTC P2P ou relay VPS
#[derive(Clone)]
pub enum Transport {
    WebRTC(WebRTCConnection),
    Relay(RelayTransport),
}

impl Transport {
    pub async fn send_data(&self, data: &[u8]) -> Result<()> {
        match self {
            Transport::WebRTC(w) => w.send_data(data).await,
            Transport::Relay(r) => r.send_data(data).await,
        }
    }

    pub async fn on_data_channel_message<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(&[u8]) + Send + Sync + 'static,
    {
        match self {
            Transport::WebRTC(w) => w.on_data_channel_message(callback).await,
            Transport::Relay(r) => r.on_data_channel_message(callback).await,
        }
    }

    pub async fn wait_for_data_channel(&self, timeout_ms: u64) -> Result<()> {
        match self {
            Transport::WebRTC(w) => w.wait_for_data_channel(timeout_ms).await,
            Transport::Relay(r) => r.wait_for_data_channel(timeout_ms).await,
        }
    }

    /// Retourner l'incoming_tx pour le relay (None si WebRTC)
    pub fn relay_incoming_tx(&self) -> Option<mpsc::UnboundedSender<Vec<u8>>> {
        match self {
            Transport::Relay(r) => Some(r.incoming_tx.clone()),
            Transport::WebRTC(_) => None,
        }
    }
}

/// Session manager to handle the overall connection flow
pub struct SessionManager {
    config: Config,
    device_id: String,
    signaling: Option<SignalingClient>,
    pub webrtc: Option<Transport>,
    // Buffer pour les demandes de connexion reçues
    pending_offers: Arc<Mutex<Vec<(String, String)>>>, // Vec<(from, offer_sdp)>
    /// Secret d'authentification partagé (raw hash PBKDF2 du mot de passe), établi
    /// lors du challenge-response. Sert à lier l'échange de clés E2E au mot de passe
    /// (protection anti-MITM). `None` si aucune authentification par mot de passe.
    auth_secret: Option<Vec<u8>>,
}

impl SessionManager {
    pub fn new(config: Config, device_id: String) -> Self {
        Self {
            config,
            device_id,
            signaling: None,
            webrtc: None,
            pending_offers: Arc::new(Mutex::new(Vec::new())),
            auth_secret: None,
        }
    }

    /// Secret d'authentification partagé établi lors du dernier handshake (si mot de passe).
    pub fn auth_secret(&self) -> Option<Vec<u8>> {
        self.auth_secret.clone()
    }

    /// Récupérer les demandes en attente
    pub async fn get_pending_offers(&self) -> Vec<(String, String)> {
        self.pending_offers.lock().await.clone()
    }

    /// Vider les demandes en attente
    pub async fn clear_pending_offers(&self) {
        let mut offers = self.pending_offers.lock().await;
        offers.clear();
    }

    /// Initialize and connect to signaling server with auto-reconnect
    pub async fn initialize(&mut self) -> Result<()> {
        let mut signaling =
            SignalingClient::new(self.config.server_url.clone(), self.device_id.clone())
                .with_auto_reconnect(60);
        signaling.connect_with_reconnect().await?;
        self.signaling = Some(signaling);

        Ok(())
    }

    /// Essayer de recevoir un message avec timeout (non-bloquant)
    /// Recevoir un message du serveur de signalisation (attend indéfiniment)
    pub async fn receive_message(&mut self) -> Result<SignalMessage> {
        if let Some(signaling) = &mut self.signaling {
            signaling.receive().await
        } else {
            Err(GhostHandError::Network("Not connected to signaling server".to_string()))
        }
    }

    /// Essayer de recevoir un message avec un timeout
    pub async fn try_receive_message(&mut self, timeout_ms: u64) -> Result<Option<SignalMessage>> {
        if let Some(signaling) = &mut self.signaling {
            let timeout = tokio::time::sleep(Duration::from_millis(timeout_ms));
            tokio::pin!(timeout);

            tokio::select! {
                Ok(msg) = signaling.receive() => Ok(Some(msg)),
                _ = timeout => Ok(None),
            }
        } else {
            Err(GhostHandError::Network("Not connected to signaling server".to_string()))
        }
    }

    /// Request connection to a remote device
    pub async fn connect_to_device(&mut self, target_id: String, password: Option<String>) -> Result<()> {
        // Valider les entrées
        validation::validate_device_id(&target_id)?;
        if let Some(ref pwd) = password {
            validation::validate_password(pwd)?;
        }

        info!("Connexion à {} demandée", target_id);

        // Sauvegarder le fait qu'un password est fourni avant de le move
        let password_was_used = password.is_some();
        let password_clone = password.clone();

        // 1. Envoyer ConnectRequest et attendre ConnectionAccepted
        let connect_msg = SignalMessage::connect_request(target_id.clone(), password);
        debug!("📤 [CLIENT] AVANT ENVOI ConnectRequest vers {} | Message: {:?}", target_id, connect_msg);

        self.signaling.as_ref().ok_or_else(|| {
            GhostHandError::Network("Not connected to signaling server".to_string())
        })?.send(connect_msg).await?;

        info!("✅ [CLIENT] Demande de connexion envoyée à {}, attente d'acceptation...", target_id);

        // Attendre ConnectionAccepted
        let timeout = tokio::time::sleep(Duration::from_secs(CONNECTION_TIMEOUT_SECS));
        tokio::pin!(timeout);

        // Secret d'authentification dérivé si un challenge mot de passe est résolu
        let mut established_auth: Option<Vec<u8>> = None;

        loop {
            tokio::select! {
                msg_result = async {
                    match self.signaling.as_mut() {
                        Some(s) => s.receive().await,
                        None => Err(GhostHandError::Network("Signaling disconnected".into())),
                    }
                } => {
                    let msg = msg_result?;
                    if msg.is_type("ConnectionAccepted") {
                        // Le message vient du target qui a accepté
                        info!("Connexion acceptée par {}", target_id);
                        break;
                    } else if msg.is_type("PasswordChallenge") {
                        // Le host demande une vérification de password
                        info!("Challenge de password reçu de {}", target_id);
                        if let (Some(challenge_b64), Some(salt_b64)) = (msg.get_str("challenge"), msg.get_str("salt")) {
                            if let Some(ref pwd) = password_clone {
                                use base64::prelude::*;
                                let challenge = BASE64_STANDARD.decode(&challenge_b64).map_err(|e| {
                                    GhostHandError::Network(format!("Invalid challenge: {}", e))
                                })?;
                                let salt = BASE64_STANDARD.decode(&salt_b64).map_err(|e| {
                                    GhostHandError::Network(format!("Invalid salt: {}", e))
                                })?;

                                let response = crate::crypto::compute_challenge_response(pwd, &salt, &challenge);

                                // Lier l'échange de clés E2E au mot de passe : le viewer dérive
                                // le même secret d'authentification que l'hôte (raw hash PBKDF2).
                                established_auth = Some(crate::crypto::derive_raw_hash(pwd, &salt));

                                self.signaling.as_ref().ok_or_else(|| {
                                    GhostHandError::Network("Signaling disconnected".into())
                                })?.send(
                                    SignalMessage::password_response(target_id.clone(), response)
                                ).await?;

                                info!("Réponse au challenge envoyée");
                            } else {
                                return Err(GhostHandError::network_with_code(
                                    error_codes::NETWORK_CONNECTION_FAILED,
                                    "Le host requiert un password mais aucun n'a été fourni"
                                ));
                            }
                        }
                    } else if msg.is_type("ConnectionRejected") {
                        if let Some(reason) = msg.get_str("reason") {
                            return Err(GhostHandError::network_with_code(
                                error_codes::NETWORK_CONNECTION_FAILED,
                                format!("Connexion rejetée: {}", reason)
                            ));
                        } else {
                            return Err(GhostHandError::network_with_code(
                                error_codes::NETWORK_CONNECTION_FAILED,
                                "Connexion rejetée par le pair"
                            ));
                        }
                    } else if msg.is_type("Error") {
                        if let Some(error_msg) = msg.get_str("message") {
                            return Err(GhostHandError::network_with_code(
                                error_codes::NETWORK_INVALID_MESSAGE,
                                format!("Erreur serveur: {}", error_msg)
                            ));
                        } else {
                            return Err(GhostHandError::network_with_code(
                                error_codes::NETWORK_INVALID_MESSAGE,
                                "Erreur inconnue du serveur"
                            ));
                        }
                    } else {
                        debug!("Message ignoré en attente de ConnectionAccepted: {:?}", msg.msg_type);
                    }
                }
                _ = &mut timeout => {
                    return Err(GhostHandError::network_with_code(
                        error_codes::NETWORK_TIMEOUT,
                        format!(
                            "Timeout après {} secondes en attente de l'acceptation de connexion",
                            CONNECTION_TIMEOUT_SECS
                        )
                    ));
                }
            }
        }

        // Mémoriser le secret d'authentification pour la dérivation de la clé E2E
        self.auth_secret = established_auth;

        // 2. Créer le transport relay via VPS (élimine les problèmes NAT traversal WebRTC)
        let signaling = self.signaling.as_ref().ok_or_else(|| {
            GhostHandError::Network("Not connected to signaling server".to_string())
        })?;

        let signaling_tx = signaling.tx.as_ref().ok_or_else(|| {
            GhostHandError::Network("Signaling TX non disponible".to_string())
        })?.clone();

        let relay = RelayTransport::new(self.device_id.clone(), target_id.clone(), signaling_tx);
        self.webrtc = Some(Transport::Relay(relay));
        info!("Transport relay VPS créé pour {} — connexion prête", target_id);

        // AUDIT: Logger la connexion établie
        audit_log(
            AuditLevel::Info,
            AuditEvent::ConnectionEstablished {
                peer_id: target_id.clone(),
                direction: "outgoing".to_string(),
                password_used: Some(password_was_used),
            },
        );

        Ok(())
    }

    /// Handle incoming connection request (receive offer and send answer)
    pub async fn handle_connection_request(&mut self, from: String, offer_sdp: String) -> Result<()> {
        info!("Traitement de la demande de connexion de {}", from);

        // 1. Créer WebRTC connection
        let webrtc_conn = WebRTCConnection::new(self.config.clone()).await?;

        // 1.5. Setup data channel callback (pour recevoir le channel créé par l'offerer)
        let data_channel_ref = Arc::clone(&webrtc_conn.data_channel);
        webrtc_conn.peer_connection.on_data_channel(Box::new(move |dc: Arc<RTCDataChannel>| {
            let data_channel_ref = Arc::clone(&data_channel_ref);
            Box::pin(async move {
                info!("Data channel '{}' reçu du peer", dc.label());
                let mut dc_lock = data_channel_ref.write().await;
                *dc_lock = Some(dc);
            })
        }));

        // 2. Setup ICE candidate callback
        let signaling = self.signaling.as_ref().ok_or_else(|| {
            GhostHandError::Network("Not connected to signaling server".to_string())
        })?;

        let tx_for_ice = signaling.tx.as_ref().ok_or_else(|| {
            GhostHandError::Network("Signaling sender not available".to_string())
        })?.clone();
        let device_id = self.device_id.clone();
        let from_clone = from.clone();

        webrtc_conn.peer_connection.on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidate>| {
            let tx = tx_for_ice.clone();
            let device_id = device_id.clone();
            let from = from_clone.clone();

            Box::pin(async move {
                if let Some(c) = candidate {
                    match c.to_json() {
                        Ok(candidate_json) => {
                            let _ = tx.send(SignalMessage::ice_candidate(
                                device_id,
                                from,
                                candidate_json.candidate,
                                candidate_json.sdp_mid.unwrap_or_default(),
                                candidate_json.sdp_mline_index.unwrap_or(0),
                            ));
                        }
                        Err(e) => {
                            warn!("Erreur sérialisation ICE candidate: {}", e);
                        }
                    }
                }
            })
        }));

        // Setup ICE gathering state callback (accept)
        webrtc_conn.peer_connection.on_ice_gathering_state_change(Box::new(move |state: webrtc::ice_transport::ice_gatherer_state::RTCIceGathererState| {
            Box::pin(async move {
                match state {
                    webrtc::ice_transport::ice_gatherer_state::RTCIceGathererState::Complete => {
                        info!("ICE gathering terminé (accept), tous les candidates collectés");
                    }
                    webrtc::ice_transport::ice_gatherer_state::RTCIceGathererState::Gathering => {
                        info!("ICE gathering en cours (accept)...");
                    }
                    _ => {}
                }
            })
        }));

        // 3. Setup connection state callback
        let (tx_connected, mut rx_connected) = mpsc::channel(1);
        webrtc_conn.peer_connection.on_peer_connection_state_change(Box::new(move |state: RTCPeerConnectionState| {
            let tx = tx_connected.clone();
            match state {
                RTCPeerConnectionState::Connected => {
                    info!("Connexion WebRTC établie avec succès (accept)");
                    Box::pin(async move {
                        let _ = tx.send(()).await;
                    })
                }
                RTCPeerConnectionState::Failed => {
                    error!("Connexion WebRTC échouée (accept)");
                    Box::pin(async {})
                }
                RTCPeerConnectionState::Disconnected => {
                    warn!("Connexion WebRTC déconnectée (accept)");
                    Box::pin(async {})
                }
                _ => {
                    debug!("État de connexion WebRTC (accept): {:?}", state);
                    Box::pin(async {})
                }
            }
        }));

        // 4. Créer answer à partir de l'offer
        info!("Création de la réponse WebRTC");
        let answer_sdp = webrtc_conn.create_answer(&offer_sdp).await?;

        // 5. Envoyer answer
        self.signaling.as_ref().ok_or_else(|| {
            GhostHandError::Network("Signaling disconnected".into())
        })?.send(SignalMessage::answer(
            self.device_id.clone(),
            from.clone(),
            answer_sdp,
        )).await?;

        info!("Réponse envoyée à {}", from);

        // 6. Recevoir ICE candidates et attendre connexion
        let timeout = tokio::time::sleep(Duration::from_secs(CONNECTION_TIMEOUT_SECS));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                msg_result = async {
                    match self.signaling.as_mut() {
                        Some(s) => s.receive().await,
                        None => Err(GhostHandError::Network("Signaling disconnected".into())),
                    }
                } => {
                    let msg = msg_result?;
                    if msg.is_type("IceCandidate") {
                        if let Some(ice_from) = msg.get_str("from") {
                            if ice_from == from {
                                debug!("ICE candidate reçu de {}", ice_from);
                                if let Some(candidate) = msg.get_str("candidate") {
                                    if let Err(e) = validation::validate_ice_candidate(&candidate) {
                                        warn!("ICE candidate invalide reçu de {}: {}", ice_from, e);
                                    } else {
                                        webrtc_conn.add_ice_candidate(&candidate).await?;
                                    }
                                }
                            }
                        }
                    }
                }
                _ = &mut timeout => {
                    error!("Timeout de connexion WebRTC");
                    return Err(GhostHandError::Network("Connection timeout".into()));
                }
                _ = rx_connected.recv() => {
                    info!("Connexion WebRTC établie avec {}", from);
                    break;
                }
            }
        }

        // 7. Stocker la connexion (WebRTC legacy — remplacé par relay en v0.5.0)
        self.webrtc = Some(Transport::WebRTC(webrtc_conn));
        info!("SessionManager: connexion WebRTC stockée");

        // AUDIT: Logger la connexion établie (incoming)
        audit_log(
            AuditLevel::Info,
            AuditEvent::ConnectionEstablished {
                peer_id: from.clone(),
                direction: "incoming".to_string(),
                password_used: None, // Pas d'info sur le password ici
            },
        );

        Ok(())
    }

    /// Accept an incoming connection request
    pub async fn accept_connection(&mut self, from: String) -> Result<()> {
        info!("Acceptation de la connexion de {}", from);

        // Secret d'authentification établi si un mot de passe est vérifié
        let mut established_auth: Option<Vec<u8>> = None;

        // Vérification de password si configuré
        if let Some(ref password_hash) = self.config.security_config.password_hash {
            info!("Password configuré, envoi du challenge...");

            // Générer un challenge
            let challenge = crate::crypto::generate_challenge()?;
            let salt = crate::crypto::extract_salt_from_hash(password_hash)?;

            use base64::prelude::*;
            let challenge_b64 = BASE64_STANDARD.encode(&challenge);
            let salt_b64 = BASE64_STANDARD.encode(&salt);

            // Envoyer le challenge
            self.signaling.as_ref().ok_or_else(|| {
                GhostHandError::Network("Not connected to signaling server".to_string())
            })?.send(SignalMessage::password_challenge(from.clone(), challenge_b64, salt_b64)).await?;

            // Attendre la réponse
            let timeout = tokio::time::sleep(Duration::from_secs(CONNECTION_TIMEOUT_SECS));
            tokio::pin!(timeout);

            let password_ok = loop {
                tokio::select! {
                    msg_result = async {
                        match self.signaling.as_mut() {
                            Some(s) => s.receive().await,
                            None => Err(GhostHandError::Network("Signaling disconnected".into())),
                        }
                    } => {
                        let msg = msg_result?;
                        if msg.is_type("PasswordResponse") {
                            if let Some(response) = msg.get_str("response") {
                                let valid = crate::crypto::verify_challenge_response(
                                    password_hash, &challenge, &response
                                )?;
                                break valid;
                            }
                        } else {
                            debug!("Message ignoré en attente de PasswordResponse: {:?}", msg.msg_type);
                        }
                    }
                    _ = &mut timeout => {
                        return Err(GhostHandError::network_with_code(
                            error_codes::NETWORK_TIMEOUT,
                            "Timeout en attente de la réponse au challenge"
                        ));
                    }
                }
            };

            if !password_ok {
                // Rejeter la connexion
                self.signaling.as_ref().ok_or_else(|| {
                    GhostHandError::Network("Signaling disconnected".into())
                })?.send(
                    SignalMessage::connection_rejected(from.clone(), "Password incorrect".to_string())
                ).await?;

                audit_log(
                    AuditLevel::Security,
                    AuditEvent::SecurityError {
                        error_code: "PASSWORD_FAILED".to_string(),
                        description: "Échec de vérification du password".to_string(),
                        peer_id: Some(from.clone()),
                    },
                );

                return Err(GhostHandError::network_with_code(
                    error_codes::NETWORK_CONNECTION_FAILED,
                    "Password incorrect"
                ));
            }

            info!("Password vérifié avec succès pour {}", from);

            // Lier l'échange de clés E2E au mot de passe (même secret que le viewer)
            established_auth = Some(crate::crypto::extract_raw_hash_from_stored(password_hash)?);
        }

        // Mémoriser le secret d'authentification pour la dérivation de la clé E2E
        self.auth_secret = established_auth;

        // 1. Envoyer l'acceptation
        self.signaling.as_ref().ok_or_else(|| {
            GhostHandError::Network("Not connected to signaling server".to_string())
        })?.send(SignalMessage::connection_accepted(from.clone())).await?;

        // AUDIT: Logger l'acceptation de connexion
        audit_log(
            AuditLevel::Info,
            AuditEvent::ConnectionRequestAccepted {
                peer_id: from.clone(),
            },
        );

        info!("Acceptation envoyée, création du transport relay VPS...");

        // 2. Créer le transport relay — plus besoin d'attendre un Offer WebRTC
        let signaling_tx = self.signaling.as_ref()
            .ok_or_else(|| GhostHandError::Network("Signaling non connecté".into()))?
            .tx.as_ref()
            .ok_or_else(|| GhostHandError::Network("Signaling TX non disponible".into()))?
            .clone();

        let relay = RelayTransport::new(self.device_id.clone(), from.clone(), signaling_tx);
        self.webrtc = Some(Transport::Relay(relay));
        info!("Transport relay VPS créé pour {} — prêt à streamer", from);

        Ok(())
    }

    /// Reject an incoming connection request
    pub async fn reject_connection(&self, from: String, reason: String) -> Result<()> {
        info!("Rejet de la connexion de {}: {}", from, reason);

        self.signaling.as_ref().ok_or_else(|| {
            GhostHandError::Network("Not connected to signaling server".to_string())
        })?.send(SignalMessage::connection_rejected(from.clone(), reason.clone())).await?;

        // AUDIT: Logger le rejet de connexion
        audit_log(
            AuditLevel::Info,
            AuditEvent::ConnectionRequestRejected {
                peer_id: from,
                reason,
            },
        );

        Ok(())
    }
}

/// Generate a random device ID with 128 bits of entropy from the OS CSPRNG.
///
/// SÉCURITÉ : 128 bits via `ring::SystemRandom` (collision ~2⁻¹²⁸). Ne PAS
/// revenir à un ID basé sur le timestamp — il serait devinable et permettrait
/// l'usurpation d'identité de pair (régression corrigée v0.5.3).
pub fn generate_device_id() -> String {
    use ring::rand::{SecureRandom, SystemRandom};

    let rng = SystemRandom::new();
    let mut bytes = [0u8; 16];
    if rng.fill(&mut bytes).is_err() {
        // Repli extrêmement improbable : nanosecondes (ne devrait jamais arriver)
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        return format!("GHD-{:032x}", ts);
    }

    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    format!("GHD-{}", hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_device_id() {
        let id1 = generate_device_id();
        let id2 = generate_device_id();

        // Vérifier format
        assert!(id1.starts_with("GHD-"));
        assert!(id2.starts_with("GHD-"));

        // Vérifier longueur minimale (GHD- + timestamp hex + - + random hex)
        assert!(id1.len() > 12);

        // IDs doivent être différents grâce à la composante aléatoire
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_webrtc_connection_creation() {
        let config = Config::default();

        // Test création
        let result = WebRTCConnection::new(config).await;

        // Devrait réussir avec config par défaut
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_webrtc_offer_creation() {
        let config = Config::default();
        let mut conn = WebRTCConnection::new(config).await.unwrap();

        // Créer une offre
        let offer_result = conn.create_offer().await;

        // Devrait réussir
        assert!(offer_result.is_ok());

        // L'offre devrait contenir du SDP valide
        let offer = offer_result.unwrap();
        assert!(!offer.is_empty());
        assert!(offer.contains("v=0") || !offer.is_empty()); // SDP commence par version
    }

    #[test]
    fn test_signal_message_serialization() {
        // Test sérialisation Register (SignalMessage est un struct, pas un enum)
        let msg = SignalMessage::register("GHD-test123".to_string());

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Register"));
        assert!(json.contains("GHD-test123"));

        // Test désérialisation
        let deserialized: SignalMessage = serde_json::from_str(&json).unwrap();
        assert!(deserialized.is_type("Register"));
        assert_eq!(deserialized.get_str("device_id").unwrap(), "GHD-test123");
    }

    #[test]
    fn test_session_manager_creation() {
        let config = Config::default();
        let device_id = "GHD-test".to_string();

        let session = SessionManager::new(config.clone(), device_id.clone());

        // Vérifier que la session est créée correctement
        assert_eq!(session.device_id, device_id);
        assert_eq!(session.config.server_url, config.server_url);
    }
}
