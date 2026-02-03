// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Manager, State, Window};
use tokio::sync::Mutex;

// Import des modules du client
use ghost_hand_client::config::Config;
use ghost_hand_client::network::{generate_device_id, SessionManager};

// État global de l'application
struct AppState {
    device_id: String,
    session_manager: Arc<Mutex<Option<SessionManager>>>,
    config: Arc<Mutex<Config>>,
}

// Structures pour les événements
#[derive(Clone, Serialize)]
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
    button: String,
    r#type: String,
    #[serde(default)]
    delta: i32,
}

#[derive(Debug, Deserialize)]
struct KeyboardEvent {
    key: String,
    code: String,
    r#type: String,
    modifiers: KeyModifiers,
}

#[derive(Debug, Deserialize)]
struct KeyModifiers {
    ctrl: bool,
    shift: bool,
    alt: bool,
    meta: bool,
}

/// Récupérer le Device ID
#[tauri::command]
fn get_device_id(state: State<AppState>) -> String {
    state.device_id.clone()
}

/// Se connecter à un appareil distant
#[tauri::command]
async fn connect_to_device(
    state: State<'_, AppState>,
    target_id: String,
    password: Option<String>,
) -> Result<(), String> {
    println!("[TAURI] Connexion à {} demandée", target_id);

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

    println!("[TAURI] Signaling initialisé");

    // Connecter au device cible
    session
        .connect_to_device(target_id.clone(), password)
        .await
        .map_err(|e| format!("Erreur de connexion: {}", e))?;

    println!("[TAURI] Connexion WebRTC établie avec {}", target_id);

    // Stocker la session
    *state.session_manager.lock().await = Some(session);

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

    if let Some(_session) = session_guard.as_ref() {
        // TODO: Implémenter l'envoi via WebRTC data channel
        // Pour l'instant, juste logger
        println!(
            "[TAURI] Mouse event: {} at ({}, {})",
            event.r#type, event.x, event.y
        );
        Ok(())
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

    if let Some(_session) = session_guard.as_ref() {
        // TODO: Implémenter l'envoi via WebRTC data channel
        // Pour l'instant, juste logger
        println!("[TAURI] Keyboard event: {} ({})", event.key, event.r#type);
        Ok(())
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

fn main() {
    // Initialiser la configuration
    let config = Config::default();

    // Générer le Device ID
    let device_id = generate_device_id();

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
    };

    // Lancer l'application Tauri
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_device_id,
            connect_to_device,
            disconnect,
            send_mouse_event,
            send_keyboard_event,
            get_config,
            update_config,
        ])
        .setup(|app| {
            // Récupérer la fenêtre principale
            let window = app.get_webview_window("main").unwrap();

            // Définir le titre avec le Device ID
            window
                .set_title(&format!("GhostHandDesk - {}", device_id))
                .unwrap();

            println!("[TAURI] Application initialisée");
            println!("[TAURI] Interface disponible");

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
