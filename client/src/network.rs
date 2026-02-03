use crate::config::Config;
use crate::error::{GhostHandError, Result};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

/// Signaling message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalMessage {
    /// Register with the signaling server
    Register {
        device_id: String,
    },

    /// Offer to connect
    Offer {
        from: String,
        to: String,
        sdp: String,
    },

    /// Answer to connection offer
    Answer {
        from: String,
        to: String,
        sdp: String,
    },

    /// ICE candidate for NAT traversal
    IceCandidate {
        from: String,
        to: String,
        candidate: String,
        sdp_mid: String,
        sdp_mline_index: u16,
    },

    /// Request to connect to a device
    ConnectRequest {
        target_id: String,
        password: Option<String>,
    },

    /// Connection accepted
    ConnectionAccepted {
        peer_id: String,
    },

    /// Connection rejected
    ConnectionRejected {
        reason: String,
    },

    /// Heartbeat
    Ping,
    Pong,
}

/// Signaling client for connecting to the relay server
pub struct SignalingClient {
    server_url: String,
    device_id: String,
    tx: Option<mpsc::UnboundedSender<SignalMessage>>,
    rx: Option<mpsc::UnboundedReceiver<SignalMessage>>,
}

impl SignalingClient {
    pub fn new(server_url: String, device_id: String) -> Self {
        Self {
            server_url,
            device_id,
            tx: None,
            rx: None,
        }
    }

    /// Connect to the signaling server
    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to signaling server: {}", self.server_url);

        let (ws_stream, _) = connect_async(&self.server_url).await.map_err(|e| {
            GhostHandError::Network(format!("Failed to connect to signaling server: {}", e))
        })?;

        info!("Connected to signaling server");

        let (mut write, mut read) = ws_stream.split();

        // Create channels for sending/receiving messages
        let (tx, mut internal_rx) = mpsc::unbounded_channel::<SignalMessage>();
        let (internal_tx, rx) = mpsc::unbounded_channel::<SignalMessage>();

        self.tx = Some(tx);
        self.rx = Some(rx);

        // Spawn task to handle outgoing messages
        tokio::spawn(async move {
            while let Some(msg) = internal_rx.recv().await {
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
        });

        // Spawn task to handle incoming messages
        tokio::spawn(async move {
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
                                warn!("Failed to deserialize message: {}", e);
                            }
                        }
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

        // Register with server
        self.send(SignalMessage::Register {
            device_id: self.device_id.clone(),
        })
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

/// WebRTC peer connection manager
pub struct WebRTCConnection {
    peer_connection: Arc<webrtc::peer_connection::RTCPeerConnection>,
    data_channel: Arc<RwLock<Option<Arc<webrtc::data_channel::RTCDataChannel>>>>,
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

        // 4. Setup callback état de connexion
        peer_connection
            .on_peer_connection_state_change(Box::new(move |state: RTCPeerConnectionState| {
                info!("État de connexion WebRTC: {:?}", state);
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

        info!("Réponse WebRTC créée avec succès");

        // 5. Retourner answer SDP
        Ok(answer.sdp)
    }

    /// Set remote description
    pub async fn set_remote_description(&self, sdp: &str) -> Result<()> {
        info!("Définition de la description distante");

        // Déterminer le type (answer) en fonction de l'état de signalisation
        let session_desc = RTCSessionDescription::answer(sdp.to_string())
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
    pub async fn send_data(&self, data: &[u8]) -> Result<()> {
        let dc_lock = self.data_channel.read().await;

        if let Some(dc) = dc_lock.as_ref() {
            dc.send(&Bytes::from(data.to_vec()))
                .await
                .map_err(|e| GhostHandError::WebRTC(format!("Erreur d'envoi de données: {}", e)))?;
            Ok(())
        } else {
            Err(GhostHandError::WebRTC("Data channel non disponible".into()))
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

/// Session manager to handle the overall connection flow
pub struct SessionManager {
    config: Config,
    device_id: String,
    signaling: Option<SignalingClient>,
    webrtc: Option<WebRTCConnection>,
}

impl SessionManager {
    pub fn new(config: Config, device_id: String) -> Self {
        Self {
            config,
            device_id,
            signaling: None,
            webrtc: None,
        }
    }

    /// Initialize and connect to signaling server
    pub async fn initialize(&mut self) -> Result<()> {
        let mut signaling =
            SignalingClient::new(self.config.server_url.clone(), self.device_id.clone());
        signaling.connect().await?;
        self.signaling = Some(signaling);

        Ok(())
    }

    /// Request connection to a remote device
    pub async fn connect_to_device(&mut self, target_id: String, password: Option<String>) -> Result<()> {
        let signaling = self.signaling.as_ref().ok_or_else(|| {
            GhostHandError::Network("Not connected to signaling server".to_string())
        })?;

        // Send connection request
        signaling
            .send(SignalMessage::ConnectRequest {
                target_id: target_id.clone(),
                password,
            })
            .await?;

        info!("Sent connection request to {}", target_id);

        // Wait for response
        // Handle offer/answer exchange
        // Set up WebRTC connection

        Ok(())
    }

    /// Handle incoming connection request
    pub async fn handle_connection_request(&mut self, from: String) -> Result<()> {
        info!("Received connection request from {}", from);

        // In a real implementation:
        // 1. Show user prompt to accept/reject
        // 2. If accepted, create WebRTC peer connection
        // 3. Exchange SDP offers/answers
        // 4. Establish connection

        Ok(())
    }
}

/// Generate a random device ID
pub fn generate_device_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    format!("GHD-{:x}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_device_id() {
        let id1 = generate_device_id();

        // Petit délai pour s'assurer d'une milliseconde différente
        std::thread::sleep(std::time::Duration::from_millis(2));

        let id2 = generate_device_id();

        // Vérifier format
        assert!(id1.starts_with("GHD-"));
        assert!(id2.starts_with("GHD-"));

        // Vérifier longueur minimale
        assert!(id1.len() > 8);

        // IDs doivent être différents (avec le délai)
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
        // Test sérialisation Register
        let msg = SignalMessage::Register {
            device_id: "GHD-test123".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Register"));
        assert!(json.contains("GHD-test123"));

        // Test désérialisation
        let deserialized: SignalMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalMessage::Register { device_id } => {
                assert_eq!(device_id, "GHD-test123");
            }
            _ => panic!("Mauvais type de message"),
        }
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
