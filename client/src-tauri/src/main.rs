#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod storage_commands;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Manager, State, AppHandle};
use tokio::sync::Mutex;
use ghost_hand_client::audit::{audit_log, init_global_logger, AuditEvent, AuditLevel};
use ghost_hand_client::config::{Config, VideoCodec};
use ghost_hand_client::network::{generate_device_id, SessionManager};
use ghost_hand_client::protocol::ControlMessage;
use ghost_hand_client::storage::{global_storage, init_global_storage, ConnectionHistory};
use ghost_hand_client::streaming::{Streamer, Receiver, InputHandler};
use ghost_hand_client::screen_capture;
use ghost_hand_client::video_encoder;
use base64::Engine;

// Fonction de diagnostic - écrit dans un fichier log
fn diag_log(msg: &str) {
    use std::io::Write;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let line = format!("[{}] {}\n", timestamp, msg);
    eprintln!("{}", line.trim());
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true).append(true)
        .open("C:\\Users\\Momo\\Documents\\GhostHandDesk\\diag.log")
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
            .env("DISABLE_ORIGIN_CHECK", "true")
            .env("SERVER_HOST", format!(":{}", port))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => {
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
                    eprintln!("[SERVER] Serveur pas prêt sur port {}, essai du port suivant", port);
                    // Le processus sera nettoyé par le système
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

#[derive(Clone, Serialize)]
struct ConnectionRequest {
    from: String,
    timestamp: u64,
    expires_at: u64,
}

struct AppState {
    device_id: String,
    session_manager: Arc<Mutex<Option<SessionManager>>>,
    config: Arc<Mutex<Config>>,
    pending_requests: Arc<Mutex<Vec<ConnectionRequest>>>,
    streamer_handle: Arc<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>>,
}

#[derive(Debug, Deserialize)]
struct MouseEvent {
    x: i32,
    y: i32,
    #[allow(dead_code)]
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

// Commandes Storage importées depuis storage_commands.rs
use storage_commands::{
    get_connection_history, get_known_peers, get_favorite_peers,
    set_peer_favorite, get_storage_stats,
};

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
        .last()
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
    session
        .connect_to_device(target_id.clone(), password)
        .await
        .map_err(|e| format!("Erreur de connexion: {}", e))?;

    diag_log(&format!("connect_to_device: WebRTC établi avec {}", target_id));

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
            // Convertir en ControlMessage
            let msg = match event.r#type.as_str() {
                "move" => ControlMessage::MouseMove {
                    x: event.x,
                    y: event.y
                },
                "down" | "up" => ControlMessage::MouseClick {
                    button: event.button.clone(),
                    pressed: event.r#type == "down",
                },
                "wheel" => ControlMessage::MouseScroll {
                    delta: event.delta
                },
                _ => return Err("Unknown mouse event type".to_string()),
            };

            // Envoyer via WebRTC
            let bytes = msg.to_bytes().map_err(|e| format!("Erreur sérialisation: {}", e))?;
            webrtc.send_data(&bytes)
                .await
                .map_err(|e| format!("Erreur envoi: {}", e))?;

            println!("[TAURI] Mouse event envoyé: {} at ({}, {})", event.r#type, event.x, event.y);
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
                // Notifier l'UI
                if let Some(w) = app_handle.get_webview_window("main") {
                    let _ = w.eval(&format!("alert('STREAMING: Data channel non prêt: {}');", e));
                }
            }

            // Créer capturer et encoder
            let capturer = screen_capture::create_capturer()
                .map_err(|e| { diag_log(&format!("Erreur capturer: {}", e)); format!("Erreur capturer: {}", e) })?;
            let encoder = video_encoder::create_encoder(
                VideoCodec::H264, 1920, 1080, 30, 4000
            ).map_err(|e| { diag_log(&format!("Erreur encoder: {}", e)); format!("Erreur encoder: {}", e) })?;

            diag_log("start_streaming: capturer + encoder OK");

            // Créer streamer
            let streamer = Streamer::new(
                capturer,
                encoder,
                webrtc.clone(),
                30,
            );

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

            // Créer receiver
            let receiver = Arc::new(Receiver::new(webrtc.clone()));

            // Démarrer avec callback pour émettre les frames à l'UI
            let frame_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
            let frame_counter_clone = frame_counter.clone();
            receiver.start(move |data, width, height, timestamp| {
                let count = frame_counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if count < 3 {
                    diag_log(&format!("RECEIVER: Frame #{} reçue: {}x{} ({} bytes)", count, width, height, data.len()));
                }
                if count == 0 {
                    // Première frame - alerte pour confirmer
                    let _ = window.eval("document.title = 'FRAME REÇUE!';");
                }
                let b64_data = base64::engine::general_purpose::STANDARD.encode(&data);
                let js_code = format!(
                    "window.dispatchEvent(new CustomEvent('ghosthand-video-frame', {{ detail: {{ data: '{}', width: {}, height: {}, timestamp: {} }} }}));",
                    b64_data, width, height, timestamp
                );
                if let Err(e) = window.eval(&js_code) {
                    diag_log(&format!("RECEIVER: eval erreur: {}", e));
                }
            }).await
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
#[tauri::command]
async fn start_input_handler(
    state: State<'_, AppState>,
) -> Result<(), String> {
    println!("[TAURI] Démarrage du handler d'input");

    let session_guard = state.session_manager.lock().await;

    if let Some(session) = session_guard.as_ref() {
        if let Some(webrtc) = &session.webrtc {
            // Créer input handler
            let handler = Arc::new(InputHandler::new()
                .map_err(|e| format!("Erreur création handler: {}", e))?);

            // Attacher au WebRTC
            handler.attach_to_webrtc(Arc::new(Mutex::new(webrtc.clone()))).await
                .map_err(|e| format!("Erreur attachement handler: {}", e))?;

            println!("[TAURI] Input handler démarré");
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
                            cleanup_old_requests(&mut *requests_guard);
                            requests_guard.push(request.clone());
                            drop(requests_guard); // Libérer explicitement le lock

                            // Envoyer les données à l'UI via window.eval() + DOM CustomEvent
                            // Note: Tauri v2 window.emit() + listen() ne fonctionne pas,
                            // donc on utilise window.eval() qui est confirmé fonctionnel.
                            let js_code = format!(
                                "window.dispatchEvent(new CustomEvent('ghosthand-connect-request', {{ detail: {{ from: '{}', timestamp: {} }} }}));",
                                from, now
                            );
                            if let Err(e) = window.eval(&js_code) {
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

fn main() {
    diag_log("=== APPLICATION DÉMARRÉE ===");

    // Lancer le serveur de signalement embarqué
    let server_process: Arc<std::sync::Mutex<Option<(std::process::Child, std::path::PathBuf)>>> =
        Arc::new(std::sync::Mutex::new(start_embedded_server()));

    // Initialiser la configuration
    let config = Config::default();

    // Générer le Device ID
    let device_id = generate_device_id();

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
    let data_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("data")))
        .unwrap_or_else(|| std::path::PathBuf::from("./data"));

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

    // Créer l'état global
    let app_state = AppState {
        device_id: device_id.clone(),
        session_manager: Arc::new(Mutex::new(None)),
        config: Arc::new(Mutex::new(config)),
        pending_requests: Arc::new(Mutex::new(Vec::new())),
        streamer_handle: Arc::new(Mutex::new(None)),
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
            start_streaming,
            stop_streaming,
            start_receiving,
            start_input_handler,
            accept_connection,
            reject_connection,
            get_pending_requests,
            start_listening_for_requests,
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
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                println!("[APP] Fenêtre fermée, nettoyage en cours...");
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
