//! GhostHandDesk Client Library
//!
//! Bibliothèque de bureau à distance avec WebRTC

pub mod config;
pub mod crypto;
pub mod error;
pub mod input_control;
pub mod network;
pub mod screen_capture;
pub mod streaming;
pub mod ui;
pub mod video_encoder;

// Ré-exporter les types principaux
pub use config::Config;
pub use error::{GhostHandError, Result};
pub use network::{SessionManager, generate_device_id};
