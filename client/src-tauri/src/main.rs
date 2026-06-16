#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings_commands;
mod storage_commands;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Emitter, Manager, State, AppHandle};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tokio::sync::Mutex;
use ghost_hand_client::adaptive_bitrate::AdaptiveBitrateController;
use ghost_hand_client::audit::{audit_log, init_global_logger, AuditEvent, AuditLevel};
use ghost_hand_client::clipboard::ClipboardManager;
use ghost_hand_client::config::{Config, VideoCodec};
use ghost_hand_client::crypto::KeyExchange;
use ghost_hand_client::file_transfer::FileTransferManager;
use ghost_hand_client::network::{generate_device_id, SessionManager};
use ghost_hand_client::protocol::{ControlMessage, DisplayInfoProto};
use ghost_hand_client::storage::{global_storage, init_global_storage, ConnectionHistory};
use ghost_hand_client::streaming::{Streamer, Receiver, InputHandler};
use ghost_hand_client::screen_capture::{self, ScreenCapturer};
use ghost_hand_client::video_encoder::{self, VideoEncoder};
use base64::Engine;
use std::os::windows::process::CommandExt;
// Fonction de diagnostic - écrit dans un fichier log
fn diag_log(msg: &str) {
    use std::io::Write;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let line = format!("[{}] {}\n", timestamp, msg);
    eprintln!("{}", line.trim());
    let log_path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("diag.log")))
        .unwrap_or_else(|| std::path::PathBuf::from("diag.log"));
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true).append(true)
        .open(&log_path)
    {
        let _ = f.write_all(line.as_bytes());
    }
}

// Serveur de signalement embarqué dans le binaire
const EMBEDDED_SERVER: &[u8] = include_bytes!("../../../server/signaling-server.exe");

/// Liste des ports à essayer pour le serveur de signalement
const SERVER_PORTS: &[u16] = &[9000, 9001, 9002, 9003, 9004];

/// Vérifier si le port est déjà utilisé (un autre serveur tourne déjà)
fn is_port_in_use(port: u16) -> bool {
    std::net::TcpStream::connect(("127.0.0.1", port)).is_ok()
}

/// Écrire le port actif dans server_port.txt (à côté de l'exécutable)
/// Config::default() lira ce fichier pour construire le server_url
fn write_server_port(port: u16) {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let port_file = dir.join("server_port.txt");
            if let Err(e) = std::fs::write(&port_file, port.to_string()) {
                eprintln!("[SERVER] Impossible d'écrire server_port.txt: {}", e);
            } else {
                println!("[SERVER] Port {} écrit dans {}", port, port_file.display());
            }
        }
    }
}

/// Extraire le binaire du serveur embarqué (si nécessaire)
fn extract_server_binary() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    use std::io::Write;

    let server_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join(".ghd-server")))
        .unwrap_or_else(|| std::env::temp_dir().join("ghd-server"));

    if let Err(e) = std::fs::create_dir_all(&server_dir) {
        eprintln!("[SERVER] Impossible de créer le dossier serveur: {}", e);
        return None;
    }

    let server_path = server_dir.join("signaling-server.exe");

    let need_extract = if server_path.exists() {
        match std::fs::metadata(&server_path) {
            Ok(meta) => meta.len() != EMBEDDED_SERVER.len() as u64,
            Err(_) => true,
        }
    } else {
        true
    };

    if need_extract {
        println!("[SERVER] Extraction du serveur embarqué ({} bytes)...", EMBEDDED_SERVER.len());
        match std::fs::File::create(&server_path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(EMBEDDED_SERVER) {
                    eprintln!("[SERVER] Erreur d'écriture du serveur: {}", e);
                    return None;
                }
            }
            Err(e) => {
                eprintln!("[SERVER] Impossible de créer le fichier serveur: {}", e);
                return None;
            }
        }
    }

    Some((server_path, server_dir))
}

/// Extraire et lancer le serveur de signalement embarqué
/// Essaie les ports 9000-9004. Si un port est déjà pris, utilise le serveur existant.
fn start_embedded_server() -> Option<(std::process::Child, std::path::PathBuf)> {
    // 1. Vérifier si un serveur tourne déjà sur un des ports connus
    for &port in SERVER_PORTS {
        if is_port_in_use(port) {
            println!("[SERVER] Port {} déjà en écoute - serveur existant détecté, connexion au serveur existant", port);
            write_server_port(port);
            return None; // Pas besoin de lancer, on se connecte à l'existant
        }
    }

    // 2. Aucun serveur existant - extraire le binaire
    let (server_path, server_dir) = extract_server_binary()?;

    // 3. Essayer de lancer le serveur sur chaque port
    for &port in SERVER_PORTS {
        println!("[SERVER] Tentative de lancement sur le port {}...", port);
        match std::process::Command::new(&server_path)
            .env("REQUIRE_TLS", "false")
            .env("SERVER_HOST", format!(":{}", port))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
        {
            Ok(mut child) => {
                println!("[SERVER] Serveur démarré (PID: {}) sur port {}", child.id(), port);
                // Attendre que le serveur soit prêt
                std::thread::sleep(std::time::Duration::from_millis(500));

                // Vérifier que le serveur écoute bien
                if is_port_in_use(port) {
                    println!("[SERVER] Serveur confirmé actif sur port {}", port);
                    write_server_port(port);
                    return Some((child, server_dir));
                } else {
                    println!("[SERVER] Port {} pas encore prêt, attente supplémentaire...", port);
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    if is_port_in_use(port) {
                        println!("[SERVER] Serveur confirmé actif sur port {}", port);
                        write_server_port(port);
                        return Some((child, server_dir));
                    }
                    eprintln!("[SERVER] Serveur pas prêt sur port {}, kill et essai du port suivant", port);
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
            Err(e) => {
                eprintln!("[SERVER] Erreur de lancement sur port {}: {}", port, e);
            }
        }
    }

    eprintln!("[SERVER] Impossible de lancer le serveur sur aucun port!");
    None
}

/// Pair découvert sur le réseau local via UDP broadcast
#[derive(Clone, Serialize)]
struct DiscoveredPeer {
    device_id: String,
    ip: String,
    port: u16,
    last_seen: u64,
}

#[derive(Clone, Serialize)]
struct ConnectionRequest {
    from: String,
    timestamp: u64,
    expires_at: u64,
}

#[allow(dead_code)]
struct AppState {
    device_id: String,
    data_dir: std::path::PathBuf,
    session_manager: Arc<Mutex<Option<SessionManager>>>,
    config: Arc<Mutex<Config>>,
    pending_requests: Arc<Mutex<Vec<ConnectionRequest>>>,
    streamer_handle: Arc<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>>,
    discovered_peers: Arc<std::sync::Mutex<Vec<DiscoveredPeer>>>,
    clipboard_manager: Arc<std::sync::Mutex<ClipboardManager>>,
    file_transfer_manager: Arc<Mutex<FileTransferManager>>,
    active_capturer: Arc<Mutex<Option<Arc<Mutex<Box<dyn ScreenCapturer>>>>>>,
    active_encoder: Arc<Mutex<Option<Arc<Mutex<Box<dyn VideoEncoder>>>>>>,
    /// Clé de session E2E partagée (dérivée via X25519 ECDH lors du handshake)
    e2e_session_key: Arc<Mutex<Option<Vec<u8>>>>,
}

#[derive(Debug, Deserialize)]
struct MouseEvent {
    x: i32,
    y: i32,
    button: String,
    r#type: String,
    #[serde(default)]
    #[allow(dead_code)]
    delta: i32,
}

#[derive(Debug, Deserialize)]
struct KeyboardEvent {
    key: String,
    #[allow(dead_code)]
    code: String,
    r#type: String,
    #[allow(dead_code)]
    modifiers: KeyModifiers,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct KeyModifiers {
    ctrl: bool,
    shift: bool,
    alt: bool,
    meta: bool,
}

// Commandes Settings importées depuis settings_commands.rs
use settings_commands::{load_settings, save_settings};

// Commandes Storage importées depuis storage_commands.rs
use storage_commands::{
    get_connection_history, get_known_peers, get_favorite_peers,
    set_peer_favorite, get_storage_stats,
};

/// Démarrer la découverte LAN via UDP broadcast
fn start_lan_discovery(
    device_id: String,
    server_port: u16,
    discovered_peers: Arc<std::sync::Mutex<Vec<DiscoveredPeer>>>,
) {
    const DISCOVERY_PORT: u16 = 19876;
    const BROADCAST_INTERVAL_SECS: u64 = 3;

    // Thread d'envoi : broadcast UDP toutes les 3 secondes
    let device_id_send = device_id.clone();
    std::thread::spawn(move || {
        let socket = match std::net::UdpSocket::bind("0.0.0.0:0") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[DISCOVERY] Impossible de créer le socket d'envoi: {}", e);
                return;
            }
        };
        let _ = socket.set_broadcast(true);
        let msg = format!("GHD|{}|{}", device_id_send, server_port);
        loop {
            let _ = socket.send_to(msg.as_bytes(), format!("255.255.255.255:{}", DISCOVERY_PORT));
            std::thread::sleep(std::time::Duration::from_secs(BROADCAST_INTERVAL_SECS));
        }
    });

    // Thread de réception : écouter les broadcasts des autres instances
    let own_device_id = device_id;
    std::thread::spawn(move || {
        let socket = match std::net::UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[DISCOVERY] Impossible de bind le listener (port {}): {}", DISCOVERY_PORT, e);
                return;
            }
        };
        let mut buf = [0u8; 512];
        println!("[DISCOVERY] Écoute sur port {}", DISCOVERY_PORT);
        loop {
            match socket.recv_from(&mut buf) {
                Ok((len, src)) => {
                    let msg = String::from_utf8_lossy(&buf[..len]);
                    if let Some(rest) = msg.strip_prefix("GHD|") {
                        let parts: Vec<&str> = rest.split('|').collect();
                        if parts.len() >= 2 {
                            let peer_id = parts[0];
                            // Ignorer son propre broadcast
                            if peer_id == own_device_id {
                                continue;
                            }
                            // Rejeter les device_id invalides (injection LAN)
                            if peer_id.len() < 4 || peer_id.len() > 64
                                || !peer_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                            {
                                continue;
                            }
                            let peer_port: u16 = parts[1].parse().unwrap_or(9000);
                            let peer_ip = src.ip().to_string();
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();

                            if let Ok(mut peers) = discovered_peers.lock() {
                                // Nettoyer les peers plus vus depuis 15 secondes
                                peers.retain(|p| now - p.last_seen < 15);
                                // Mettre à jour ou ajouter
                                if let Some(existing) = peers.iter_mut().find(|p| p.device_id == peer_id) {
                                    existing.last_seen = now;
                                    existing.ip = peer_ip;
                                    existing.port = peer_port;
                                } else {
                                    println!("[DISCOVERY] Nouveau pair: {} @ {}:{}", peer_id, peer_ip, peer_port);
                                    peers.push(DiscoveredPeer {
                                        device_id: peer_id.to_string(),
                                        ip: peer_ip,
                                        port: peer_port,
                                        last_seen: now,
                                    });
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[DISCOVERY] Erreur réception: {}", e);
                    break;
                }
            }
        }
    });
}

/// Récupérer les pairs découverts sur le réseau local
#[tauri::command]
fn get_discovered_peers(state: State<AppState>) -> Vec<DiscoveredPeer> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Ok(mut peers) = state.discovered_peers.lock() {
        // Retirer les peers plus vus depuis 15 secondes
        peers.retain(|p| now - p.last_seen < 15);
        peers.clone()
    } else {
        Vec::new()
    }
}

/// Nettoyer les requêtes de connexion expirées
fn cleanup_old_requests(requests: &mut Vec<ConnectionRequest>) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let before_count = requests.len();
    requests.retain(|req| req.expires_at > now);
    let after_count = requests.len();

    if before_count > after_count {
        println!("[TAURI] {} requêtes expirées nettoyées", before_count - after_count);
    }
}

/// Récupérer le Device ID
#[tauri::command]
fn get_device_id(state: State<AppState>) -> String {
    state.device_id.clone()
}

/// Initialiser le SessionManager pour écouter les demandes entrantes
#[tauri::command]
async fn initialize_session(
    state: State<'_, AppState>,
) -> Result<(), String> {
    println!("[TAURI] Initialisation de la session");

    // Récupérer la config
    let config = state.config.lock().await.clone();
    let device_id = state.device_id.clone();

    // Créer session manager
    let mut session = SessionManager::new(config, device_id);

    // Initialiser signaling
    session
        .initialize()
        .await
        .map_err(|e| format!("Erreur d'initialisation: {}", e))?;

    println!("[TAURI] Signaling initialisé - Prêt à recevoir des demandes");

    // Stocker la session
    *state.session_manager.lock().await = Some(session);

    Ok(())
}

/// Récupérer les infos réseau (IP locale + port serveur)
#[tauri::command]
async fn get_network_info(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = state.config.lock().await;
    let server_url = config.server_url.clone();

    // Trouver l'IP locale
    let local_ip = get_local_ip().unwrap_or_else(|| "inconnu".to_string());

    // Extraire le port du server_url
    let port = server_url
        .split(':')
        .next_back()
        .and_then(|s| s.split('/').next())
        .unwrap_or("9000");

    Ok(serde_json::json!({
        "local_ip": local_ip,
        "port": port,
        "server_url": server_url,
    }))
}

/// Trouver l'IP locale de la machine
fn get_local_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    Some(addr.ip().to_string())
}

/// Se connecter à un appareil distant
#[tauri::command]
async fn connect_to_device(
    state: State<'_, AppState>,
    target_id: String,
    password: Option<String>,
) -> Result<(), String> {
    diag_log(&format!("connect_to_device: APPELÉ pour {}", target_id));

    let mut session_guard = state.session_manager.lock().await;
    diag_log("connect_to_device: lock acquis");

    // S'assurer qu'on a un SessionManager
    if session_guard.is_none() {
        return Err("SessionManager non initialisé. Appelez initialize_session() d'abord.".to_string());
    }

    let session = session_guard.as_mut().unwrap();

    // Connecter au device cible
    diag_log(&format!("connect_to_device: appel session.connect_to_device({})...", target_id));
    match session.connect_to_device(target_id.clone(), password).await {
        Ok(_) => {
            diag_log(&format!("connect_to_device: WebRTC établi avec {}", target_id));
        }
        Err(e) => {
            diag_log(&format!("connect_to_device: ERREUR: {}", e));
            return Err(format!("Erreur de connexion: {}", e));
        }
    }

    // Enregistrer dans l'historique
    if let Some(storage_mutex) = global_storage() {
        if let Ok(mut storage) = storage_mutex.lock() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let connection_id = format!("conn-{}-{}", timestamp, target_id);

            storage.add_connection_history(ConnectionHistory {
                id: connection_id,
                peer_id: target_id.clone(),
                timestamp,
                duration_secs: None, // Sera mis à jour à la déconnexion
                direction: "outgoing".to_string(),
                success: true,
                disconnect_reason: None,
            });

            // Mettre à jour le pair connu
            if let Some(peer) = storage.get_known_peer(&target_id) {
                let mut updated_peer = peer.clone();
                updated_peer.last_seen = timestamp;
                updated_peer.connection_count += 1;
                storage.upsert_known_peer(updated_peer);
            } else {
                storage.upsert_known_peer(ghost_hand_client::storage::KnownPeer {
                    peer_id: target_id.clone(),
                    display_name: None,
                    last_seen: timestamp,
                    favorite: false,
                    connection_count: 1,
                    notes: None,
                });
            }

            // Sauvegarder immédiatement
            if let Err(e) = storage.save() {
                eprintln!("⚠️  Erreur sauvegarde storage: {}", e);
            }
        }
    }

    Ok(())
}

/// Se déconnecter
#[tauri::command]
async fn disconnect(state: State<'_, AppState>) -> Result<(), String> {
    println!("[TAURI] Déconnexion demandée");

    // Supprimer la session
    *state.session_manager.lock().await = None;

    println!("[TAURI] Déconnecté");
    Ok(())
}

/// Envoyer un événement souris
#[tauri::command]
async fn send_mouse_event(
    state: State<'_, AppState>,
    event: MouseEvent,
) -> Result<(), String> {
    // Récupérer la session
    let session_guard = state.session_manager.lock().await;

    if let Some(session) = session_guard.as_ref() {
        if let Some(webrtc) = &session.webrtc {
            match event.r#type.as_str() {
                "move" => {
                    let msg = ControlMessage::MouseMove { x: event.x, y: event.y };
                    let bytes = msg.to_bytes().map_err(|e| format!("Erreur sérialisation: {}", e))?;
                    webrtc.send_data(&bytes).await.map_err(|e| format!("Erreur envoi: {}", e))?;
                },
                "down" | "up" => {
                    // FIX: Envoyer MouseMove AVANT MouseClick pour positionner le curseur
                    let move_msg = ControlMessage::MouseMove { x: event.x, y: event.y };
                    let move_bytes = move_msg.to_bytes().map_err(|e| format!("Erreur sérialisation move: {}", e))?;
                    webrtc.send_data(&move_bytes).await.map_err(|e| format!("Erreur envoi move: {}", e))?;

                    let click_msg = ControlMessage::MouseClick {
                        button: event.button.clone(),
                        pressed: event.r#type == "down",
                    };
                    let click_bytes = click_msg.to_bytes().map_err(|e| format!("Erreur sérialisation click: {}", e))?;
                    webrtc.send_data(&click_bytes).await.map_err(|e| format!("Erreur envoi click: {}", e))?;
                },
                "wheel" => {
                    let msg = ControlMessage::MouseScroll { delta: event.delta };
                    let bytes = msg.to_bytes().map_err(|e| format!("Erreur sérialisation: {}", e))?;
                    webrtc.send_data(&bytes).await.map_err(|e| format!("Erreur envoi: {}", e))?;
                },
                _ => return Err("Unknown mouse event type".to_string()),
            };

            Ok(())
        } else {
            Err("Pas de connexion WebRTC".to_string())
        }
    } else {
        Err("Non connecté".to_string())
    }
}

/// Envoyer un événement clavier
#[tauri::command]
async fn send_keyboard_event(
    state: State<'_, AppState>,
    event: KeyboardEvent,
) -> Result<(), String> {
    // Récupérer la session
    let session_guard = state.session_manager.lock().await;

    if let Some(session) = session_guard.as_ref() {
        if let Some(webrtc) = &session.webrtc {
            // Convertir en ControlMessage avec modifiers
            use ghost_hand_client::protocol::KeyModifiersProto;
            let msg = ControlMessage::KeyPress {
                key: event.key.clone(),
                pressed: event.r#type == "keydown",
                modifiers: Some(KeyModifiersProto {
                    ctrl: event.modifiers.ctrl,
                    shift: event.modifiers.shift,
                    alt: event.modifiers.alt,
                    meta: event.modifiers.meta,
                }),
            };

            // Envoyer via WebRTC
            let bytes = msg.to_bytes().map_err(|e| format!("Erreur sérialisation: {}", e))?;
            webrtc.send_data(&bytes)
                .await
                .map_err(|e| format!("Erreur envoi: {}", e))?;

            println!("[TAURI] Keyboard event envoyé: {} ({})", event.key, event.r#type);
            Ok(())
        } else {
            Err("Pas de connexion WebRTC".to_string())
        }
    } else {
        Err("Non connecté".to_string())
    }
}

/// Récupérer la configuration actuelle
#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    let config = state.config.lock().await.clone();
    Ok(config)
}

/// Mettre à jour la configuration
#[tauri::command]
async fn update_config(
    state: State<'_, AppState>,
    new_config: Config,
) -> Result<(), String> {
    *state.config.lock().await = new_config;
    println!("[TAURI] Configuration mise à jour");
    Ok(())
}

/// Changer l'URL du serveur de signalement et réinitialiser la session
#[tauri::command]
async fn update_server_url(
    state: State<'_, AppState>,
    server_url: String,
) -> Result<(), String> {
    diag_log(&format!("update_server_url: {} ", server_url));

    // 1. Mettre à jour la config
    {
        let mut config = state.config.lock().await;
        config.server_url = server_url.clone();
    }

    // 2. Détruire l'ancienne session (ferme le WebSocket)
    {
        *state.session_manager.lock().await = None;
    }

    // 3. Créer une nouvelle session avec la nouvelle URL
    let config = state.config.lock().await.clone();
    let device_id = state.device_id.clone();

    let mut session = SessionManager::new(config, device_id);
    session
        .initialize()
        .await
        .map_err(|e| format!("Impossible de se connecter au serveur {}: {}", server_url, e))?;

    *state.session_manager.lock().await = Some(session);

    diag_log(&format!("update_server_url: session réinitialisée sur {}", server_url));
    Ok(())
}

/// Démarrer le streaming vidéo (côté contrôlé)
#[tauri::command]
async fn start_streaming(
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    diag_log("start_streaming: APPELÉ");

    let session_guard = state.session_manager.lock().await;

    if let Some(session) = session_guard.as_ref() {
        if let Some(webrtc) = &session.webrtc {
            // Vérifier l'état du data channel avant de démarrer
            let dc_test = webrtc.send_data(b"test").await;
            diag_log(&format!("start_streaming: data_channel test = {:?}", dc_test.is_ok()));
            if let Err(ref e) = dc_test {
                diag_log(&format!("start_streaming: data_channel ERREUR: {}", e));
                // Notifier l'UI via l'API d'événements typés Tauri (pas d'eval/injection JS possible)
                let _ = app_handle.emit(
                    "ghosthand-streaming-error",
                    format!("STREAMING: Data channel non prêt: {}", e),
                );
            }

            // --- E2E Key Exchange (X25519 ECDH) ---
            // Le contrôleur génère sa paire et envoie sa clé publique au viewer.
            // Le viewer répond via KeyExchangeAccept (géré dans start_receiving).
            // Le secret partagé est stocké dans e2e_session_key.
            let kex_private_key: Option<Vec<u8>> = {
                let kex = KeyExchange::new();
                match kex.generate_keypair() {
                    Ok((priv_key, pub_key)) => {
                        let init_msg = ControlMessage::KeyExchangeInit { public_key: pub_key };
                        match init_msg.to_bytes() {
                            Ok(bytes) => {
                                if let Err(e) = webrtc.send_data(&bytes).await {
                                    diag_log(&format!("KEx: erreur envoi KeyExchangeInit: {}", e));
                                    None
                                } else {
                                    diag_log("KEx: KeyExchangeInit envoyé au viewer");
                                    Some(priv_key)
                                }
                            }
                            Err(e) => { diag_log(&format!("KEx: erreur sérialisation: {}", e)); None }
                        }
                    }
                    Err(e) => { diag_log(&format!("KEx: erreur génération keypair: {}", e)); None }
                }
            };
            // Stocker la clé privée temporaire pour que start_input_handler puisse finaliser l'échange
            if let Some(ref priv_key) = kex_private_key {
                let mut key_guard = state.e2e_session_key.lock().await;
                // Tag spécial: on stocke la clé privée avec préfixe "PENDING:" jusqu'à réception de KeyExchangeAccept
                let mut tagged = b"PENDING:".to_vec();
                tagged.extend_from_slice(priv_key);
                *key_guard = Some(tagged);
            }
            diag_log("start_streaming: KEx initié (non bloquant)");

            // Créer capturer et encoder
            let capturer = screen_capture::create_capturer()
                .map_err(|e| { diag_log(&format!("Erreur capturer: {}", e)); format!("Erreur capturer: {}", e) })?;
            let encoder = video_encoder::create_encoder(
                VideoCodec::H264, 1920, 1080, 30, 4000
            ).map_err(|e| { diag_log(&format!("Erreur encoder: {}", e)); format!("Erreur encoder: {}", e) })?;

            diag_log("start_streaming: capturer + encoder OK");

            // Récupérer la fenêtre pour le callback local (preview)
            let preview_window = app_handle.get_webview_window("main");

            // Créer streamer avec callback local pour le preview sur le PC contrôlé
            // La session_key E2E sera disponible si le viewer a déjà répondu à KeyExchangeInit
            let initial_session_key: Option<Vec<u8>> = {
                let key_guard = state.e2e_session_key.lock().await;
                key_guard.as_ref().and_then(|k| {
                    if k.starts_with(b"PENDING:") { None } else { Some(k.clone()) }
                })
            };
            let mut streamer_builder = Streamer::new(
                capturer,
                encoder,
                webrtc.clone(),
                30,
            ).with_adaptive_bitrate(AdaptiveBitrateController::new());
            if let Some(key) = initial_session_key {
                diag_log("start_streaming: chiffrement E2E actif dès le début");
                streamer_builder = streamer_builder.with_session_key(key);
            }
            let mut streamer = streamer_builder;

            // Stocker le capturer partagé pour le switch de moniteur
            let shared_capturer = streamer.capturer();
            *state.active_capturer.lock().await = Some(shared_capturer.clone());

            // Stocker l'encodeur partagé pour le changement de résolution en live
            let shared_encoder = streamer.encoder();
            *state.active_encoder.lock().await = Some(shared_encoder.clone());

            // Envoyer la liste d'écrans au viewer distant
            {
                let cap = shared_capturer.lock().await;
                if let Ok(displays) = cap.get_displays() {
                    let display_infos: Vec<DisplayInfoProto> = displays.iter().map(|d| {
                        DisplayInfoProto {
                            id: d.id,
                            name: d.name.clone(),
                            width: d.width,
                            height: d.height,
                            is_primary: d.is_primary,
                        }
                    }).collect();
                    let msg = ControlMessage::DisplayListResponse { displays: display_infos };
                    if let Ok(bytes) = msg.to_bytes() {
                        let _ = webrtc.send_data(&bytes).await;
                        diag_log(&format!("start_streaming: display list envoyée ({} écrans)", displays.len()));
                    }
                }
            }

            if let Some(pw) = preview_window {
                streamer = streamer.with_local_callback(move |data, width, height, timestamp| {
                    let b64_data = base64::engine::general_purpose::STANDARD.encode(&data);
                    let _ = pw.emit(
                        "ghosthand-local-preview",
                        serde_json::json!({
                            "data": b64_data,
                            "width": width,
                            "height": height,
                            "timestamp": timestamp,
                        }),
                    );
                });
            }

            // Lancer dans un task local et stocker le handle
            let handle = tauri::async_runtime::spawn(async move {
                diag_log("streaming task: DÉMARRÉ");
                if let Err(e) = streamer.start().await {
                    diag_log(&format!("streaming task: ERREUR: {}", e));
                }
                diag_log("streaming task: TERMINÉ");
            });

            // Stocker le handle pour pouvoir arrêter le streaming plus tard
            drop(session_guard); // Libérer le lock avant de prendre streamer_handle
            *state.streamer_handle.lock().await = Some(handle);

            diag_log("start_streaming: handle stocké, OK");
            Ok(())
        } else {
            diag_log("start_streaming: ERREUR - Pas de connexion WebRTC");
            Err("Pas de connexion WebRTC".to_string())
        }
    } else {
        diag_log("start_streaming: ERREUR - Non connecté");
        Err("Non connecté".to_string())
    }
}

/// Arrêter le streaming vidéo
#[tauri::command]
async fn stop_streaming(state: State<'_, AppState>) -> Result<(), String> {
    println!("[TAURI] Arrêt du streaming vidéo");

    if let Some(handle) = state.streamer_handle.lock().await.take() {
        handle.abort();
        println!("[TAURI] Streaming arrêté");
        Ok(())
    } else {
        Err("Aucun streaming actif".to_string())
    }
}

/// Démarrer la réception vidéo (côté contrôleur)
#[tauri::command]
async fn start_receiving(
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    diag_log("start_receiving: APPELÉ");

    let session_guard = state.session_manager.lock().await;

    if let Some(session) = session_guard.as_ref() {
        if let Some(webrtc) = &session.webrtc {
            // Récupérer la fenêtre
            let window = app_handle.get_webview_window("main")
                .ok_or("Fenêtre non trouvée")?;

            diag_log("start_receiving: fenêtre + webrtc OK");

            // Créer receiver (la clé E2E sera disponible si le contrôleur a déjà envoyé KeyExchangeInit)
            let initial_rx_key: Option<Vec<u8>> = {
                let key_guard = state.e2e_session_key.lock().await;
                key_guard.as_ref().and_then(|k| {
                    if k.starts_with(b"PENDING:") { None } else { Some(k.clone()) }
                })
            };
            let mut recv_builder = Receiver::new(webrtc.clone());
            if let Some(key) = initial_rx_key {
                diag_log("start_receiving: chiffrement E2E actif");
                recv_builder = recv_builder.with_session_key(key);
            }
            let receiver = Arc::new(recv_builder);

            // Référence partagée pour que le callback message puisse gérer KeyExchangeInit
            let webrtc_for_kex = webrtc.clone();
            let e2e_key_for_rx = state.e2e_session_key.clone();

            // Fenêtre pour les messages non-vidéo (display list, chat, clipboard)
            let msg_window = app_handle.get_webview_window("main");

            // Démarrer avec callbacks séparés pour vidéo et messages de contrôle
            let frame_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
            let frame_counter_clone = frame_counter.clone();
            receiver.start_with_message_handler(
                move |data, width, height, timestamp| {
                    let count = frame_counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if count < 3 {
                        diag_log(&format!("RECEIVER: Frame #{} reçue: {}x{} ({} bytes)", count, width, height, data.len()));
                    }
                    if count == 0 {
                        let _ = window.set_title("FRAME REÇUE!");
                    }
                    let b64_data = base64::engine::general_purpose::STANDARD.encode(&data);
                    if let Err(e) = window.emit(
                        "ghosthand-video-frame",
                        serde_json::json!({
                            "data": b64_data,
                            "width": width,
                            "height": height,
                            "timestamp": timestamp,
                        }),
                    ) {
                        diag_log(&format!("RECEIVER: emit erreur: {}", e));
                    }
                },
                move |msg| {
                    // Dispatcher les messages non-vidéo vers l'UI via CustomEvents
                    match msg {
                        ControlMessage::KeyExchangeInit { public_key: remote_pub } => {
                            // Viewer répond au KeyExchangeInit du contrôleur
                            let kex = KeyExchange::new();
                            let webrtc_kex = webrtc_for_kex.clone();
                            let key_store = e2e_key_for_rx.clone();
                            tauri::async_runtime::spawn(async move {
                                match kex.generate_keypair() {
                                    Ok((priv_key, pub_key)) => {
                                        match kex.derive_shared_secret(&priv_key, &remote_pub) {
                                            Ok(shared) => {
                                                let accept = ControlMessage::KeyExchangeAccept { public_key: pub_key };
                                                if let Ok(bytes) = accept.to_bytes() {
                                                    let _ = webrtc_kex.send_data(&bytes).await;
                                                }
                                                *key_store.lock().await = Some(shared);
                                                println!("[CRYPTO] Viewer: clé E2E dérivée — AES-256-GCM actif");
                                            }
                                            Err(e) => eprintln!("[CRYPTO] Viewer: erreur dérivation: {}", e),
                                        }
                                    }
                                    Err(e) => eprintln!("[CRYPTO] Viewer: erreur keypair: {}", e),
                                }
                            });
                        }
                        other => {
                            if let Some(ref w) = msg_window {
                                match &other {
                                    ControlMessage::DisplayListResponse { displays } => {
                                        let _ = w.emit("ghosthand-display-list", displays);
                                    }
                                    ControlMessage::ChatMessage { from, text, timestamp } => {
                                        let _ = w.emit(
                                            "ghosthand-chat-message",
                                            serde_json::json!({
                                                "from": from, "text": text, "timestamp": timestamp
                                            }),
                                        );
                                    }
                                    ControlMessage::ClipboardSync { content } => {
                                        let _ = w.emit(
                                            "ghosthand-clipboard-sync",
                                            serde_json::json!({ "content": content }),
                                        );
                                    }
                                    _ => println!("[RECEIVER] Message non géré: {:?}", other),
                                }
                            }
                        }
                    }
                },
            ).await
                .map_err(|e| {
                    diag_log(&format!("start_receiving: ERREUR receiver.start: {}", e));
                    format!("Erreur receiver: {}", e)
                })?;

            diag_log("start_receiving: OK");
            Ok(())
        } else {
            diag_log("start_receiving: ERREUR - Pas de connexion WebRTC");
            Err("Pas de connexion WebRTC".to_string())
        }
    } else {
        diag_log("start_receiving: ERREUR - Non connecté");
        Err("Non connecté".to_string())
    }
}

/// Démarrer le handler d'input (côté contrôlé)
/// Gère les messages input (souris, clavier) + SelectDisplay pour le switch de moniteur
#[tauri::command]
async fn start_input_handler(
    state: State<'_, AppState>,
) -> Result<(), String> {
    println!("[TAURI] Démarrage du handler d'input");

    let session_guard = state.session_manager.lock().await;

    if let Some(session) = session_guard.as_ref() {
        if let Some(webrtc) = &session.webrtc {
            // Utiliser la vraie résolution de l'écran pour le clamping des coordonnées
            let (res_w, res_h) = {
                let cap_opt = state.active_capturer.lock().await;
                if let Some(ref cap) = *cap_opt {
                    let cap_guard = cap.lock().await;
                    cap_guard.get_resolution()
                } else {
                    (1920, 1080)
                }
            };
            let handler = Arc::new(InputHandler::new_with_resolution(res_w as i32, res_h as i32)
                .map_err(|e| format!("Erreur création handler: {}", e))?);
            println!("[TAURI] InputHandler créé avec résolution {}x{}", res_w, res_h);

            // Setup manuel du data channel callback (au lieu de attach_to_webrtc)
            // pour pouvoir aussi gérer SelectDisplay
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

            webrtc.on_data_channel_message(move |data: &[u8]| {
                let _ = tx.send(data.to_vec());
            }).await.map_err(|e| format!("Erreur callback: {}", e))?;

            let handler_clone = handler.clone();
            let capturer_ref = state.active_capturer.clone();
            let encoder_ref = state.active_encoder.clone();
            let e2e_key_ref = state.e2e_session_key.clone();

            tokio::spawn(async move {
                while let Some(data) = rx.recv().await {
                    if let Ok(msg) = ControlMessage::from_bytes(&data) {
                        match msg {
                            ControlMessage::KeyExchangeAccept { public_key: remote_pub } => {
                                // Finaliser l'échange de clés E2E
                                let pending = {
                                    let guard = e2e_key_ref.lock().await;
                                    guard.as_ref().and_then(|k| {
                                        if k.starts_with(b"PENDING:") {
                                            Some(k[8..].to_vec()) // Extraire la clé privée
                                        } else { None }
                                    })
                                };
                                if let Some(priv_key) = pending {
                                    let kex = KeyExchange::new();
                                    match kex.derive_shared_secret(&priv_key, &remote_pub) {
                                        Ok(shared) => {
                                            *e2e_key_ref.lock().await = Some(shared.clone());
                                            println!("[CRYPTO] Clé E2E dérivée — chiffrement AES-256-GCM actif");
                                        }
                                        Err(e) => {
                                            eprintln!("[CRYPTO] Erreur dérivation secret: {}", e);
                                            *e2e_key_ref.lock().await = None;
                                        }
                                    }
                                }
                            }
                            ControlMessage::SelectDisplay { display_id } => {
                                // Switch de moniteur
                                let cap_opt = capturer_ref.lock().await;
                                if let Some(ref cap) = *cap_opt {
                                    let mut cap_guard = cap.lock().await;
                                    match cap_guard.select_display(display_id) {
                                        Ok(_) => println!("[INPUT] Moniteur switché → {}", display_id),
                                        Err(e) => println!("[INPUT] Erreur switch moniteur: {}", e),
                                    }
                                }
                            }
                            ControlMessage::SetResolution { width } => {
                                // Changer la résolution de streaming en live
                                let enc_opt = encoder_ref.lock().await;
                                if let Some(ref enc) = *enc_opt {
                                    let mut enc_guard = enc.lock().await;
                                    let target = if width == 0 { None } else { Some(width) };
                                    enc_guard.set_target_width(target);
                                    println!("[INPUT] Résolution changée → {:?}", target);
                                }
                            }
                            other => {
                                let _ = handler_clone.handle_message(other).await;
                            }
                        }
                    }
                }
            });

            println!("[TAURI] Input handler démarré (avec support multi-monitor)");
            Ok(())
        } else {
            Err("Pas de connexion WebRTC".to_string())
        }
    } else {
        Err("Non connecté".to_string())
    }
}

/// Accepter une demande de connexion
#[tauri::command]
async fn accept_connection(
    state: State<'_, AppState>,
    from: String,
) -> Result<(), String> {
    diag_log(&format!("accept_connection: APPELÉ pour {}", from));

    let mut session_guard = state.session_manager.lock().await;
    diag_log("accept_connection: lock acquis");

    if let Some(session) = session_guard.as_mut() {
        session.accept_connection(from.clone()).await
            .map_err(|e| format!("Erreur acceptation: {}", e))?;

        // Retirer la demande de la liste des demandes en attente
        let mut requests = state.pending_requests.lock().await;
        requests.retain(|r| r.from != from);

        diag_log(&format!("accept_connection: WebRTC accepté de {}", from));

        // Auto-démarrer le streaming et l'input handler
        if let Some(session) = session_guard.as_ref() {
            if let Some(_webrtc) = &session.webrtc {
                println!("[TAURI] Démarrage automatique du streaming...");
                // Note: Le streaming sera démarré par une commande séparée
            }
        }

        Ok(())
    } else {
        Err("Non connecté au serveur de signalement".to_string())
    }
}

/// Rejeter une demande de connexion
#[tauri::command]
async fn reject_connection(
    state: State<'_, AppState>,
    from: String,
    reason: String,
) -> Result<(), String> {
    println!("[TAURI] Rejet de la connexion de {}: {}", from, reason);

    let session_guard = state.session_manager.lock().await;

    if let Some(session) = session_guard.as_ref() {
        session.reject_connection(from.clone(), reason).await
            .map_err(|e| format!("Erreur rejet: {}", e))?;

        // Retirer la demande de la liste des demandes en attente
        let mut requests = state.pending_requests.lock().await;
        requests.retain(|r| r.from != from);

        println!("[TAURI] Connexion rejetée de {}", from);
        Ok(())
    } else {
        Err("Non connecté au serveur de signalement".to_string())
    }
}

/// Récupérer les demandes de connexion en attente
#[tauri::command]
async fn get_pending_requests(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectionRequest>, String> {
    // Nettoyer les requêtes expirées avant de retourner la liste
    let mut requests = state.pending_requests.lock().await;
    cleanup_old_requests(&mut requests);
    Ok(requests.clone())
}

/// Démarrer l'écoute des demandes de connexion entrantes
#[tauri::command]
async fn start_listening_for_requests(
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    println!("[TAURI] Démarrage de l'écoute des demandes de connexion");

    // Cloner les références nécessaires
    let session_manager = state.session_manager.clone();
    let pending_requests = state.pending_requests.clone();
    let window = app_handle.get_webview_window("main")
        .ok_or("Fenêtre principale non trouvée")?;

    // Démarrer la boucle d'écoute en arrière-plan
    // IMPORTANT: On utilise try_receive_message avec timeout court au lieu de
    // receive_message (bloquant) pour éviter un DEADLOCK.
    // receive_message() garde le lock session_manager pendant l'attente indéfinie,
    // ce qui bloque accept_connection/connect_to_device qui ont besoin du même lock.
    // Avec try_receive_message(500ms), le lock est relâché entre chaque itération.
    tauri::async_runtime::spawn(async move {
        loop {
            // Recevoir le message avec timeout court pour libérer le lock régulièrement
            let msg_result = {
                let mut session_guard = session_manager.lock().await;
                if let Some(session) = session_guard.as_mut() {
                    session.try_receive_message(500).await
                } else {
                    println!("[LISTENER] Session fermée, arrêt de l'écoute");
                    return; // Sortir si session fermée
                }
            }; // Lock libéré ici - accept_connection/connect_to_device peuvent acquérir le lock

            // Traiter le message après avoir libéré le lock
            match msg_result {
                Ok(Some(msg)) => {
                    if msg.is_type("ConnectRequest") {
                        if let Some(from) = msg.get_str("from") {
                            println!("[LISTENER] Demande de connexion de {}", from);

                            // Ajouter à la liste des demandes en attente
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();

                            let request = ConnectionRequest {
                                from: from.clone(),
                                timestamp: now,
                                expires_at: now + 60, // Expire après 60 secondes
                            };

                            // Nettoyer les vieilles requêtes avant d'ajouter la nouvelle
                            let mut requests_guard = pending_requests.lock().await;
                            cleanup_old_requests(&mut requests_guard);
                            requests_guard.push(request.clone());
                            drop(requests_guard); // Libérer explicitement le lock

                            // Envoyer les données à l'UI via l'API d'événements typés Tauri
                            if let Err(e) = window.emit(
                                "ghosthand-connect-request",
                                serde_json::json!({ "from": from, "timestamp": now }),
                            ) {
                                eprintln!("[LISTENER] Erreur envoi event UI: {}", e);
                            }
                        }
                    }
                    // Les autres messages (Offer, Answer, ICE) sont gérés directement
                    // par les méthodes de SessionManager (accept_connection, connect_to_device, etc.)
                }
                Ok(None) => {
                    // Timeout - pas de message, on reboucle (le lock a été relâché)
                    continue;
                }
                Err(e) => {
                    eprintln!("[LISTENER] Erreur de réception: {}", e);
                    break;
                }
            }
        }
    });

    println!("[TAURI] Écoute démarrée en arrière-plan");
    Ok(())
}

/// Synchroniser le presse-papiers : lire le clipboard local et l'envoyer au peer distant
#[tauri::command]
async fn sync_clipboard(state: State<'_, AppState>) -> Result<String, String> {
    let content = {
        let cm = state.clipboard_manager.lock().map_err(|e| format!("Lock erreur: {}", e))?;
        cm.get_clipboard().map_err(|e| format!("Erreur clipboard: {}", e))?
    };

    // Envoyer via WebRTC si connecté
    let session_guard = state.session_manager.lock().await;
    if let Some(session) = session_guard.as_ref() {
        if let Some(webrtc) = &session.webrtc {
            let msg = ControlMessage::ClipboardSync { content: content.clone() };
            let bytes = msg.to_bytes().map_err(|e| format!("Erreur sérialisation: {}", e))?;
            webrtc.send_data(&bytes).await.map_err(|e| format!("Erreur envoi: {}", e))?;
        }
    }

    Ok(content)
}

/// Récupérer le contenu du presse-papiers
#[tauri::command]
fn get_clipboard(state: State<AppState>) -> Result<String, String> {
    let cm = state.clipboard_manager.lock().map_err(|e| format!("Lock erreur: {}", e))?;
    cm.get_clipboard().map_err(|e| format!("Erreur: {}", e))
}

/// Définir le contenu du presse-papiers
#[tauri::command]
fn set_clipboard(state: State<AppState>, content: String) -> Result<(), String> {
    let cm = state.clipboard_manager.lock().map_err(|e| format!("Lock erreur: {}", e))?;
    cm.set_clipboard(&content).map_err(|e| format!("Erreur: {}", e))
}

/// Envoyer un message de chat
#[tauri::command]
async fn send_chat_message(
    state: State<'_, AppState>,
    text: String,
) -> Result<(), String> {
    let session_guard = state.session_manager.lock().await;
    if let Some(session) = session_guard.as_ref() {
        if let Some(webrtc) = &session.webrtc {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let msg = ControlMessage::ChatMessage {
                from: state.device_id.clone(),
                text,
                timestamp,
            };
            let bytes = msg.to_bytes().map_err(|e| format!("Erreur sérialisation: {}", e))?;
            webrtc.send_data(&bytes).await.map_err(|e| format!("Erreur envoi: {}", e))?;
            Ok(())
        } else {
            Err("Pas de connexion WebRTC".to_string())
        }
    } else {
        Err("Non connecté".to_string())
    }
}

/// Demander au PC contrôlé de changer d'écran (côté viewer)
#[tauri::command]
async fn change_display(
    state: State<'_, AppState>,
    display_id: u32,
) -> Result<(), String> {
    let session_guard = state.session_manager.lock().await;
    if let Some(session) = session_guard.as_ref() {
        if let Some(webrtc) = &session.webrtc {
            let msg = ControlMessage::SelectDisplay { display_id };
            let bytes = msg.to_bytes().map_err(|e| format!("Erreur sérialisation: {}", e))?;
            webrtc.send_data(&bytes).await.map_err(|e| format!("Erreur envoi: {}", e))?;
            println!("[TAURI] SelectDisplay envoyé: {}", display_id);
            Ok(())
        } else {
            Err("Pas de connexion WebRTC".to_string())
        }
    } else {
        Err("Non connecté".to_string())
    }
}

/// Changer la résolution de streaming (côté viewer → envoie au PC contrôlé)
#[tauri::command]
async fn change_resolution(
    state: State<'_, AppState>,
    width: u32,
) -> Result<(), String> {
    let session_guard = state.session_manager.lock().await;
    if let Some(session) = session_guard.as_ref() {
        if let Some(webrtc) = &session.webrtc {
            let msg = ControlMessage::SetResolution { width };
            let bytes = msg.to_bytes().map_err(|e| format!("Erreur sérialisation: {}", e))?;
            webrtc.send_data(&bytes).await.map_err(|e| format!("Erreur envoi: {}", e))?;
            println!("[TAURI] SetResolution envoyé: {}", width);
            Ok(())
        } else {
            Err("Pas de connexion WebRTC".to_string())
        }
    } else {
        Err("Non connecté".to_string())
    }
}

/// Récupérer la liste des écrans disponibles (locaux)
#[tauri::command]
fn get_displays() -> Result<Vec<serde_json::Value>, String> {
    let monitors = xcap::Monitor::all().map_err(|e| format!("Erreur moniteurs: {}", e))?;
    let displays: Vec<serde_json::Value> = monitors.iter().enumerate().map(|(i, m)| {
        serde_json::json!({
            "id": i,
            "name": m.name(),
            "width": m.width(),
            "height": m.height(),
            "x": m.x(),
            "y": m.y(),
            "is_primary": m.is_primary(),
        })
    }).collect();
    Ok(displays)
}

/// Envoyer un fichier au peer distant
#[tauri::command]
async fn send_file(
    state: State<'_, AppState>,
    file_path: String,
) -> Result<(), String> {
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err("Fichier non trouvé".to_string());
    }

    let (id, name, size, chunks) = ghost_hand_client::file_transfer::FileTransferManager::prepare_send(path)
        .map_err(|e| format!("Erreur préparation: {}", e))?;

    let session_guard = state.session_manager.lock().await;
    if let Some(session) = session_guard.as_ref() {
        if let Some(webrtc) = &session.webrtc {
            // Envoyer FileTransferStart
            let start_msg = ControlMessage::FileTransferStart {
                id: id.clone(), name, size,
            };
            let bytes = start_msg.to_bytes().map_err(|e| format!("Erreur: {}", e))?;
            webrtc.send_data(&bytes).await.map_err(|e| format!("Erreur envoi: {}", e))?;

            // Envoyer les chunks
            let mut offset = 0u64;
            for chunk in chunks {
                let chunk_len = chunk.len() as u64;
                let chunk_msg = ControlMessage::FileTransferChunk {
                    id: id.clone(), data: chunk, offset,
                };
                let bytes = chunk_msg.to_bytes().map_err(|e| format!("Erreur: {}", e))?;
                webrtc.send_data(&bytes).await.map_err(|e| format!("Erreur envoi: {}", e))?;
                offset += chunk_len;
            }

            // Envoyer FileTransferComplete
            let complete_msg = ControlMessage::FileTransferComplete { id };
            let bytes = complete_msg.to_bytes().map_err(|e| format!("Erreur: {}", e))?;
            webrtc.send_data(&bytes).await.map_err(|e| format!("Erreur envoi: {}", e))?;

            Ok(())
        } else {
            Err("Pas de connexion WebRTC".to_string())
        }
    } else {
        Err("Non connecté".to_string())
    }
}

fn main() {
    diag_log("=== APPLICATION DÉMARRÉE ===");

    // Calculer data_dir en premier (utilisé pour Device ID + storage + settings)
    let data_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("data")))
        .unwrap_or_else(|| std::path::PathBuf::from("./data"));
    let _ = std::fs::create_dir_all(&data_dir);

    // Charger le Device ID persisté (ou le générer et le sauvegarder)
    let device_id = {
        let id_file = data_dir.join("device_id.txt");
        match std::fs::read_to_string(&id_file) {
            Ok(id) if !id.trim().is_empty() => id.trim().to_string(),
            _ => {
                let new_id = generate_device_id();
                let _ = std::fs::write(&id_file, &new_id);
                new_id
            }
        }
    };

    // Initialiser la configuration (charger depuis settings.json si présent)
    let config = {
        let saved = settings_commands::load_settings_from_disk(&data_dir);
        let mut cfg = Config::default();
        saved.apply_to_config(&mut cfg);
        cfg
    };

    // Lancer le serveur local uniquement si l'URL pointe sur localhost (mode LAN/auto-hébergé)
    // Par défaut (URL VPS), aucun serveur local n'est lancé — comportement RustDesk/AnyDesk
    let server_process: Arc<std::sync::Mutex<Option<(std::process::Child, std::path::PathBuf)>>> = {
        let is_local = config.server_url.contains("localhost") || config.server_url.contains("127.0.0.1");
        Arc::new(std::sync::Mutex::new(if is_local {
            diag_log("[SERVER] Mode local — lancement du serveur embarqué...");
            start_embedded_server()
        } else {
            diag_log(&format!("[SERVER] Mode VPS — connexion directe vers {}", config.server_url));
            None
        }))
    };

    // Initialiser le logger d'audit
    let log_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("logs")))
        .unwrap_or_else(|| std::path::PathBuf::from("./logs"));

    if let Err(e) = init_global_logger(&log_dir, device_id.clone()) {
        eprintln!("⚠️  Erreur initialisation audit logger: {}", e);
    } else {
        println!("✅ Audit logger initialisé: {}", log_dir.display());

        // Logger le démarrage de l'application
        audit_log(
            AuditLevel::Info,
            AuditEvent::ConfigurationChange {
                setting: "application_start".to_string(),
                old_value: "stopped".to_string(),
                new_value: "running".to_string(),
            },
        );
    }

    // Initialiser le storage persistant
    if let Err(e) = init_global_storage(&data_dir) {
        eprintln!("⚠️  Erreur initialisation storage: {}", e);
    } else {
        println!("✅ Storage persistant initialisé: {}", data_dir.display());

        // Afficher des statistiques
        if let Some(storage_mutex) = global_storage() {
            if let Ok(storage) = storage_mutex.lock() {
                let stats = storage.get_stats();
                println!("   📊 Connexions historiques: {}", stats.total_connections);
                println!("   👥 Pairs connus: {} ({} favoris)", stats.known_peers, stats.favorite_peers);
            }
        }
    }

    println!("==============================================");
    println!("🚀 GhostHandDesk v{}", env!("CARGO_PKG_VERSION"));
    println!("==============================================");
    println!("📱 Device ID: {}", device_id);
    println!("🌐 Serveur: {}", config.server_url);
    println!("==============================================");

    // Extraire le port du serveur pour la discovery
    let server_port: u16 = config.server_url
        .split(':')
        .next_back()
        .and_then(|s| s.split('/').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(9000);

    // Créer le store de peers découverts
    let discovered_peers: Arc<std::sync::Mutex<Vec<DiscoveredPeer>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));

    // Lancer la découverte LAN
    start_lan_discovery(device_id.clone(), server_port, discovered_peers.clone());

    // Créer l'état global
    let app_state = AppState {
        device_id: device_id.clone(),
        data_dir: data_dir.clone(),
        session_manager: Arc::new(Mutex::new(None)),
        config: Arc::new(Mutex::new(config)),
        pending_requests: Arc::new(Mutex::new(Vec::new())),
        streamer_handle: Arc::new(Mutex::new(None)),
        discovered_peers,
        clipboard_manager: Arc::new(std::sync::Mutex::new(ClipboardManager::new())),
        file_transfer_manager: Arc::new(Mutex::new(FileTransferManager::new())),
        active_capturer: Arc::new(Mutex::new(None)),
        active_encoder: Arc::new(Mutex::new(None)),
        e2e_session_key: Arc::new(Mutex::new(None)),
    };

    // Cloner pour les closures
    let device_id_for_title = device_id.clone();
    let server_for_setup = Arc::clone(&server_process);

    // Lancer l'application Tauri
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_device_id,
            get_network_info,
            initialize_session,
            connect_to_device,
            disconnect,
            send_mouse_event,
            send_keyboard_event,
            get_config,
            update_config,
            update_server_url,
            get_discovered_peers,
            start_streaming,
            stop_streaming,
            start_receiving,
            start_input_handler,
            accept_connection,
            reject_connection,
            get_pending_requests,
            start_listening_for_requests,
            // Clipboard
            sync_clipboard,
            get_clipboard,
            set_clipboard,
            // Chat
            send_chat_message,
            // Multi-monitor
            get_displays,
            change_display,
            // Resolution
            change_resolution,
            // File transfer
            send_file,
            // Settings commands
            load_settings,
            save_settings,
            // Storage commands
            get_connection_history,
            get_known_peers,
            get_favorite_peers,
            set_peer_favorite,
            get_storage_stats,
        ])
        .setup(move |app| {
            // Récupérer la fenêtre principale
            let window = match app.get_webview_window("main") {
                Some(w) => w,
                None => {
                    eprintln!("[TAURI] Fenêtre principale non trouvée");
                    return Ok(());
                }
            };

            // Définir le titre avec le Device ID
            if let Err(e) = window.set_title(&format!("GhostHandDesk - {}", device_id_for_title)) {
                eprintln!("[TAURI] Impossible de définir le titre: {}", e);
            }

            // Configurer le System Tray
            let device_id_label = device_id_for_title.clone();
            let open_item = MenuItemBuilder::with_id("open", "Ouvrir GhostHandDesk").build(app)?;
            let device_item = MenuItemBuilder::with_id("device_id", format!("ID: {}", device_id_label))
                .enabled(false)
                .build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quitter").build(app)?;

            let tray_menu = MenuBuilder::new(app)
                .item(&open_item)
                .separator()
                .item(&device_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let _tray = TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip(format!("GhostHandDesk - {}", device_id_for_title))
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "open" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        "quit" => {
                            std::process::exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Afficher le PID du serveur
            if let Ok(guard) = server_for_setup.lock() {
                if let Some((ref child, _)) = *guard {
                    println!("[TAURI] Serveur embarqué actif (PID: {})", child.id());
                }
            }

            println!("[TAURI] Application initialisée");
            println!("[TAURI] Interface disponible");

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    // Minimiser dans le tray au lieu de fermer
                    window.hide().unwrap_or_default();
                    api.prevent_close();
                    println!("[APP] Fenêtre minimisée dans le tray");
                }
                tauri::WindowEvent::Destroyed => {
                    println!("[APP] Fenêtre détruite, nettoyage en cours...");
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    // Nettoyage : tuer le serveur embarqué à la fermeture de l'app
    // take() pour extraire la valeur et dropper le guard avant la fin du scope
    let server_data = server_process.lock().ok().and_then(|mut guard| guard.take());
    if let Some((mut child, server_dir)) = server_data {
        println!("[SERVER] Arrêt du serveur (PID: {})...", child.id());
        let _ = child.kill();
        let _ = child.wait();
        // Nettoyer le dossier d'extraction
        let _ = std::fs::remove_dir_all(&server_dir);
        println!("[SERVER] Serveur arrêté et nettoyé");
    }
}
