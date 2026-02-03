mod config;
mod crypto;
mod error;
mod input_control;
mod network;
mod screen_capture;
mod streaming;
mod ui;
mod video_encoder;

use config::Config;
use error::Result;
use screen_capture::create_capturer;
use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("GhostHandDesk Client v{}", env!("CARGO_PKG_VERSION"));
    info!("Starting remote desktop client...");

    // Load or create configuration
    let config = Config::default();

    // Initialize components
    info!("Initializing screen capture...");
    let mut capturer = create_capturer()?;

    let displays = capturer.get_displays()?;
    info!("Available displays:");
    for disp in &displays {
        info!(
            "  - Display {}: {} ({}x{}) at ({}, {}) {}",
            disp.id,
            disp.name,
            disp.width,
            disp.height,
            disp.x,
            disp.y,
            if disp.is_primary {
                "[PRIMARY]"
            } else {
                ""
            }
        );
    }

    // Test capture
    info!("Capturing test frame...");
    let frame = capturer.capture()?;
    info!(
        "Captured frame: {}x{}, {} bytes",
        frame.width,
        frame.height,
        frame.data.len()
    );

    // Initialize video encoder
    info!("Initializing video encoder...");
    let mut encoder = video_encoder::create_encoder(
        config.video_config.codec.clone(),
        frame.width,
        frame.height,
        config.video_config.framerate,
        config.video_config.bitrate,
    )?;

    // Test encoding
    info!("Encoding test frame...");
    let encoded = encoder.encode(&frame).await?;
    info!(
        "Encoded frame: {} bytes ({}x reduction)",
        encoded.data.len(),
        frame.data.len() / encoded.data.len().max(1)
    );

    // Initialize input controller
    info!("Initializing input controller...");
    let _input_controller = input_control::InputController::new()?;
    info!("Input controller ready");

    // Initialize crypto manager
    info!("Initializing cryptography...");
    let crypto = crypto::CryptoManager::new();
    let test_key = crypto.generate_key()?;
    info!("Crypto manager ready (key size: {} bytes)", test_key.len());

    // Initialize network
    info!("Initializing network...");
    let device_id = network::generate_device_id();
    info!("Device ID: {}", device_id);

    let _session = network::SessionManager::new(config.clone(), device_id.clone());

    // Note: Actual connection would happen here
    // session.initialize().await?;

    info!("==============================================");
    info!("GhostHandDesk Client initialized successfully");
    info!("==============================================");
    info!("Device ID: {}", device_id);
    info!("Status: Ready (not connected)");
    info!("");
    info!("Next steps:");
    info!("  1. Implement Tauri UI");
    info!("  2. Connect to signaling server");
    info!("  3. Establish WebRTC connection");
    info!("  4. Start streaming");

    // Keep running
    // In a real application, this would start the UI event loop
    // For now, just demonstrate the core functionality

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_initialization() {
        // Test that all modules can be initialized
        let config = Config::default();
        assert!(!config.server_url.is_empty());

        let capturer = create_capturer();
        assert!(capturer.is_ok());

        let crypto = crypto::CryptoManager::new();
        let key = crypto.generate_key();
        assert!(key.is_ok());
    }
}
