// UI module - To be implemented with Tauri

// This module will contain:
// - Main window
// - Connection dialog
// - Settings panel
// - Status indicators
// - Remote desktop viewer

pub struct AppState {
    pub connected: bool,
    pub device_id: String,
    pub remote_device_id: Option<String>,
}

impl AppState {
    pub fn new(device_id: String) -> Self {
        Self {
            connected: false,
            device_id,
            remote_device_id: None,
        }
    }
}

// Tauri commands will be implemented here
// #[tauri::command]
// async fn connect_to_device(device_id: String, password: String) -> Result<(), String> {
//     // Implementation
// }
