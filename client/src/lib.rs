//! GhostHandDesk Client Library
//!
//! Bibliothèque de bureau à distance avec WebRTC

pub mod adaptive_bitrate;
pub mod audit;
pub mod clipboard;
pub mod config;
pub mod crypto;
pub mod error;
pub mod file_transfer;
pub mod input_control;
pub mod network;
pub mod protocol;
pub mod screen_capture;
pub mod storage;
pub mod streaming;
pub mod validation;
pub mod video_encoder;

// Ré-exporter les types principaux
pub use adaptive_bitrate::{AdaptiveBitrateController, AdaptiveBitrateConfig, AdaptiveBitrateStats};
pub use audit::{audit_log, audit_log_with_metadata, init_global_logger, AuditEvent, AuditLevel};
pub use config::Config;
pub use error::{GhostHandError, Result};
pub use network::{SessionManager, generate_device_id, load_or_generate_device_id};
pub use storage::{
    global_storage, init_global_storage, ConnectionHistory, KnownPeer, Storage, StorageStats,
};
pub use streaming::{Streamer, Receiver, InputHandler};
