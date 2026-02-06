//! GhostHandDesk Client Library
//!
//! Bibliothèque de bureau à distance avec WebRTC
#![allow(dead_code)] // Bibliothèque : les exports publics ne sont pas tous consommés par le binaire

pub mod audit;
pub mod config;
pub mod crypto;
pub mod error;
pub mod input_control;
pub mod network;
pub mod protocol;
pub mod screen_capture;
pub mod storage;
pub mod streaming;
pub mod ui;
pub mod video_encoder;

// Ré-exporter les types principaux
pub use audit::{audit_log, audit_log_with_metadata, init_global_logger, AuditEvent, AuditLevel};
pub use config::Config;
pub use error::{GhostHandError, Result};
pub use network::{SessionManager, generate_device_id};
pub use storage::{
    global_storage, init_global_storage, ConnectionHistory, KnownPeer, Storage, StorageStats,
};
pub use streaming::{Streamer, Receiver, InputHandler};
