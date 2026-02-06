// Prevents additional console window on Windows in release
// TEMPORAIREMENT DÉSACTIVÉ POUR DEBUG
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Emitter, Manager, State, AppHandle};
use tokio::sync::Mutex;

// Import des modules du client
use ghost_hand_client::audit::{audit_log, init_global_logger, AuditEvent, AuditLevel};
use ghost_hand_client::config::{Config, VideoCodec};
use ghost_hand_client::network::{generate_device_id, SessionManager};
use ghost_hand_client::protocol::ControlMessage;
use ghost_hand_client::storage::{global_storage, init_global_storage, ConnectionHistory};
use ghost_hand_client::streaming::{Streamer, Receiver, InputHandler};
use ghost_hand_client::screen_capture;
use ghost_hand_client::video_encoder;

// Structure pour une demande de connexion entrante
#[derive(Clone, Serialize)]
struct ConnectionRequest {
    from: String,
    timestamp: u64,
    expires_at: u64, // Timestamp d'expiration (timestamp + 60 secondes)
}

// État global de l'application
struct AppState {
    device_id: String,
    session_manager: Arc<Mutex<Option<SessionManager>>>,
    config: Arc<Mutex<Config>>,
    pending_requests: Arc<Mutex<Vec<ConnectionRequest>>>,
    streamer_handle: Arc<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>>,
}

// Note: Le serveur de signalement doit être lancé MANUELLEMENT en externe
// avec le script 1-SERVEUR.bat pour permettre plusieurs instances
// La fonction start_signaling_server() a été supprimée

// Structures pour les événements
#[derive(Clone, Serialize)]
#[allow(dead_code)]
struct VideoFramePayload {
    data: Vec<u8>,
    width: u32,
    height: u32,
    timestamp: u64,
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

// ============================================================================
// Commandes Storage
// ============================================================================

/// Obtenir l'historique des connexions
#[tauri::command]
fn get_connection_history(limit: Option<usize>) -> Result<Vec<ConnectionHistory>, String> {
    if let Some(storage_mutex) = global_storage() {
        if let Ok(storage) = storage_mutex.lock() {
            let history = storage.get_connection_history(limit);
            Ok(history.into_iter().cloned().collect())
        } else {
            Err("Impossible de verrouiller le storage".to_string())
        }
    } else {
        Err("Storage non initialisé".to_string())
    }
}

/// Obtenir les pairs connus
#[tauri::command]
fn get_known_peers() -> Result<Vec<ghost_hand_client::storage::KnownPeer>, String> {
    if let Some(storage_mutex) = global_storage() {
        if let Ok(storage) = storage_mutex.lock() {
            let peers = storage.get_all_known_peers();
            Ok(peers.into_iter().cloned().collect())
        } else {
            Err("Impossible de verrouiller le storage".to_string())
        }
    } else {
        Err("Storage non initialisé".to_string())
    }
}

/// Obtenir les pairs favoris
#[tauri::command]
fn get_favorite_peers() -> Result<Vec<ghost_hand_client::storage::KnownPeer>, String> {
    if let Some(storage_mutex) = global_storage() {
        if let Ok(storage) = storage_mutex.lock() {
            let peers = storage.get_favorite_peers();
            Ok(peers.into_iter().cloned().collect())
        } else {
            Err("Impossible de verrouiller le storage".to_string())
        }
    } else {
        Err("Storage non initialisé".to_string())
    }
}

/// Marquer un pair comme favori
#[tauri::command]
fn set_peer_favorite(peer_id: String, favorite: bool) -> Result<(), String> {
    if let Some(storage_mutex) = global_storage() {
        if let Ok(mut storage) = storage_mutex.lock() {
            if storage.set_peer_favorite(&peer_id, favorite) {
                storage.save().map_err(|e| format!("Erreur sauvegarde: {}", e))?;
                Ok(())
            } else {
                Err(format!("Pair {} introuvable", peer_id))
            }
        } else {
            Err("Impossible de verrouiller le storage".to_string())
        }
    } else {
        Err("Storage non initialisé".to_string())
    }
}

/// Obtenir les statistiques du storage
#[tauri::command]
async fn get_storage_stats() -> Result<ghost_hand_client::storage::StorageStats, String> {
    if let Some(storage_mutex) = global_storage() {
        if let Ok(storage) = storage_mutex.lock() {
            Ok(storage.get_stats())
        } else {
            Err("Impossible de verrouiller le storage".to_string())
        }
    } else {
        Err("Storage non initialisé".to_string())
    }
}

// ============================================================================

/// Nettoyer les requêtes de connexion expirées
fn cleanup_old_requests(requests: &mut Vec<ConnectionRequest>) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
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

/// Se connecter à un appareil distant
#[tauri::command]
async fn connect_to_device(
    state: State<'_, AppState>,
    target_id: String,
    password: Option<String>,
) -> Result<(), String> {
    println!("[TAURI] Connexion à {} demandée", target_id);

    let mut session_guard = state.session_manager.lock().await;

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

    println!("[TAURI] Connexion WebRTC établie avec {}", target_id);

    // Enregistrer dans l'historique
    if let Some(storage_mutex) = global_storage() {
        if let Ok(mut storage) = storage_mutex.lock() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
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
            // Convertir en ControlMessage
            let msg = ControlMessage::KeyPress {
                key: event.key.clone(),
                pressed: event.r#type == "keydown",
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

/// Démarrer le streaming vidéo (côté contrôlé)
#[tauri::command]
async fn start_streaming(
    state: State<'_, AppState>,
) -> Result<(), String> {
    println!("[TAURI] Démarrage du streaming vidéo");

    let session_guard = state.session_manager.lock().await;

    if let Some(session) = session_guard.as_ref() {
        if let Some(webrtc) = &session.webrtc {
            // Créer capturer et encoder
            let capturer = screen_capture::create_capturer()
                .map_err(|e| format!("Erreur capturer: {}", e))?;
            let encoder = video_encoder::create_encoder(
                VideoCodec::H264, 1920, 1080, 30, 4000
            ).map_err(|e| format!("Erreur encoder: {}", e))?;

            // Créer streamer
            let streamer = Streamer::new(
                capturer,
                encoder,
                webrtc.clone(),
                30,
            );

            // Lancer dans un task local et stocker le handle
            let handle = tauri::async_runtime::spawn(async move {
                if let Err(e) = streamer.start().await {
                    eprintln!("[STREAMING] Erreur: {}", e);
                }
            });

            // Stocker le handle pour pouvoir arrêter le streaming plus tard
            drop(session_guard); // Libérer le lock avant de prendre streamer_handle
            *state.streamer_handle.lock().await = Some(handle);

            println!("[TAURI] Streaming démarré");
            Ok(())
        } else {
            Err("Pas de connexion WebRTC".to_string())
        }
    } else {
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
    println!("[TAURI] Démarrage de la réception vidéo");

    let session_guard = state.session_manager.lock().await;

    if let Some(session) = session_guard.as_ref() {
        if let Some(webrtc) = &session.webrtc {
            // Récupérer la fenêtre
            let window = app_handle.get_webview_window("main")
                .ok_or("Fenêtre non trouvée")?;

            // Créer receiver
            let receiver = Arc::new(Receiver::new(webrtc.clone()));

            // Démarrer avec callback pour émettre les frames à l'UI
            receiver.start(move |data, width, height, timestamp| {
                // Émettre l'événement à l'UI
                if let Err(e) = window.emit("video-frame", VideoFramePayload {
                    data,
                    width,
                    height,
                    timestamp,
                }) {
                    eprintln!("[TAURI] Impossible d'émettre video-frame: {}", e);
                }
            }).await
                .map_err(|e| format!("Erreur receiver: {}", e))?;

            println!("[TAURI] Réception démarrée");
            Ok(())
        } else {
            Err("Pas de connexion WebRTC".to_string())
        }
    } else {
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
    println!("[TAURI] Acceptation de la connexion de {}", from);

    let mut session_guard = state.session_manager.lock().await;

    if let Some(session) = session_guard.as_mut() {
        session.accept_connection(from.clone()).await
            .map_err(|e| format!("Erreur acceptation: {}", e))?;

        // Retirer la demande de la liste des demandes en attente
        let mut requests = state.pending_requests.lock().await;
        requests.retain(|r| r.from != from);

        println!("[TAURI] Connexion acceptée de {}", from);

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
    tauri::async_runtime::spawn(async move {
        loop {
            // Recevoir un message (bloque jusqu'à réception)
            // Note: le lock est maintenu pendant l'attente, ce qui est acceptable
            // car cette task est la seule à accéder à la session pendant cette phase
            let mut session_guard = session_manager.lock().await;

            if let Some(session) = session_guard.as_mut() {
                match session.receive_message().await {
                    Ok(msg) => {
                        if msg.is_type("ConnectRequest") {
                            if let Some(from) = msg.get_str("from") {
                                println!("[LISTENER] Demande de connexion reçue de {}", from);

                                // Ajouter à la liste des demandes en attente
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
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

                                // Émettre un event vers l'UI
                                if let Err(e) = window.emit("connection-request", request) {
                                    eprintln!("[TAURI] Impossible d'émettre connection-request: {}", e);
                                }
                            }
                        }
                        // Les autres messages (Offer, Answer, ICE) sont gérés directement
                        // par les méthodes de SessionManager (accept_connection, connect_to_device, etc.)
                    }
                    Err(e) => {
                        eprintln!("[LISTENER] Erreur de réception: {}", e);
                        break;
                    }
                }
            } else {
                println!("[LISTENER] Session fermée, arrêt de l'écoute");
                break;
            }
            // Le lock est automatiquement libéré ici à la fin du scope
            // Pas besoin de délai car receive_message() bloque déjà
        }
    });

    println!("[TAURI] Écoute démarrée en arrière-plan");
    Ok(())
}

fn main() {
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

    // Cloner le device_id pour la closure setup
    let device_id_for_title = device_id.clone();

    // Lancer l'application Tauri
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_device_id,
            initialize_session,
            connect_to_device,
            disconnect,
            send_mouse_event,
            send_keyboard_event,
            get_config,
            update_config,
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
            // NE PAS démarrer le serveur automatiquement
            // Il doit être lancé MANUELLEMENT en externe pour permettre plusieurs instances
            // Le serveur doit être lancé avec 1-SERVEUR.bat qui configure le port 9000

            // Récupérer la fenêtre principale
            let window = app.get_webview_window("main").unwrap();

            // Définir le titre avec le Device ID
            window
                .set_title(&format!("GhostHandDesk - {}", device_id_for_title))
                .unwrap();

            println!("[TAURI] Application initialisée");
            println!("[TAURI] Interface disponible");

            Ok(())
        })
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                println!("[APP] Fenêtre fermée - Nettoyage des processus...");

                // Tuer tous les processus du serveur
                #[cfg(target_os = "windows")]
                {
                    use std::process::Command;

                    // Tuer le serveur de signalement
                    let _ = Command::new("taskkill")
                        .args(&["/F", "/IM", "signaling-server.exe"])
                        .output();

                    println!("[APP] Processus serveur tués");
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
