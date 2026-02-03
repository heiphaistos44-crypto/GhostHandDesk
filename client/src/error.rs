use thiserror::Error;

#[derive(Error, Debug)]
pub enum GhostHandError {
    #[error("Screen capture error: {0}")]
    ScreenCapture(String),

    #[error("Video encoding error: {0}")]
    VideoEncoding(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("WebRTC error: {0}")]
    WebRTC(String),

    #[error("Input control error: {0}")]
    InputControl(String),

    #[error("Cryptography error: {0}")]
    Crypto(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, GhostHandError>;
