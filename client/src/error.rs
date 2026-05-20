use thiserror::Error;

/// Codes d'erreur standardisés pour le diagnostic
pub mod error_codes {
    // Erreurs réseau (1xxx)
    pub const NETWORK_CONNECTION_FAILED: &str = "E1001";
    pub const NETWORK_TIMEOUT: &str = "E1002";
    pub const NETWORK_DISCONNECTED: &str = "E1003";
    pub const NETWORK_INVALID_MESSAGE: &str = "E1004";

    // Erreurs WebRTC (2xxx)
    pub const WEBRTC_INIT_FAILED: &str = "E2001";
    pub const WEBRTC_OFFER_FAILED: &str = "E2002";
    pub const WEBRTC_ANSWER_FAILED: &str = "E2003";
    pub const WEBRTC_ICE_FAILED: &str = "E2004";
    pub const WEBRTC_DATA_CHANNEL_FAILED: &str = "E2005";

    // Erreurs capture d'écran (3xxx)
    pub const CAPTURE_INIT_FAILED: &str = "E3001";
    pub const CAPTURE_FRAME_FAILED: &str = "E3002";
    pub const CAPTURE_NO_DISPLAY: &str = "E3003";

    // Erreurs encodage vidéo (4xxx)
    pub const ENCODING_INIT_FAILED: &str = "E4001";
    pub const ENCODING_FRAME_FAILED: &str = "E4002";
    pub const ENCODING_FORMAT_UNSUPPORTED: &str = "E4003";

    // Erreurs contrôle d'input (5xxx)
    pub const INPUT_INIT_FAILED: &str = "E5001";
    pub const INPUT_SEND_FAILED: &str = "E5002";

    // Erreurs cryptographie (6xxx)
    pub const CRYPTO_KEY_GENERATION_FAILED: &str = "E6001";
    pub const CRYPTO_ENCRYPTION_FAILED: &str = "E6002";
    pub const CRYPTO_DECRYPTION_FAILED: &str = "E6003";
    pub const CRYPTO_KEY_EXCHANGE_FAILED: &str = "E6004";

    // Erreurs configuration (7xxx)
    pub const CONFIG_INVALID: &str = "E7001";
    pub const CONFIG_LOAD_FAILED: &str = "E7002";
}

#[derive(Error, Debug)]
pub enum GhostHandError {
    // Compatibilité: formes simples sans code (utilisent "E0000" par défaut)
    #[error("Erreur de capture d'écran: {0}")]
    ScreenCapture(String),

    #[error("Erreur d'encodage vidéo: {0}")]
    VideoEncoding(String),

    #[error("Erreur réseau: {0}")]
    Network(String),

    #[error("Erreur WebRTC: {0}")]
    WebRTC(String),

    #[error("Erreur de contrôle d'input: {0}")]
    InputControl(String),

    #[error("Erreur cryptographique: {0}")]
    Crypto(String),

    #[error("Erreur de configuration: {0}")]
    Config(String),

    #[error("Erreur de validation: {0}")]
    Validation(String),

    #[error("Rate limit atteint: {0}")]
    RateLimit(String),

    #[error("Erreur interne: {0}")]
    Internal(String),

    // Formes avec codes d'erreur standardisés
    #[error("[{code}] Capture d'écran: {message}")]
    ScreenCaptureWithCode { code: String, message: String },

    #[error("[{code}] Encodage vidéo: {message}")]
    VideoEncodingWithCode { code: String, message: String },

    #[error("[{code}] Réseau: {message}")]
    NetworkWithCode { code: String, message: String },

    #[error("[{code}] WebRTC: {message}")]
    WebRTCWithCode { code: String, message: String },

    #[error("[{code}] Contrôle d'input: {message}")]
    InputControlWithCode { code: String, message: String },

    #[error("[{code}] Cryptographie: {message}")]
    CryptoWithCode { code: String, message: String },

    #[error("[{code}] Configuration: {message}")]
    ConfigWithCode { code: String, message: String },

    // Erreurs système
    #[error("Erreur IO: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erreur de sérialisation: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Erreur inconnue: {0}")]
    Unknown(String),
}

// Méthodes helper pour créer des erreurs avec codes
impl GhostHandError {
    pub fn network_with_code(code: &str, message: impl Into<String>) -> Self {
        Self::NetworkWithCode {
            code: code.to_string(),
            message: message.into(),
        }
    }

    pub fn webrtc_with_code(code: &str, message: impl Into<String>) -> Self {
        Self::WebRTCWithCode {
            code: code.to_string(),
            message: message.into(),
        }
    }

    pub fn screen_capture_with_code(code: &str, message: impl Into<String>) -> Self {
        Self::ScreenCaptureWithCode {
            code: code.to_string(),
            message: message.into(),
        }
    }

    pub fn video_encoding_with_code(code: &str, message: impl Into<String>) -> Self {
        Self::VideoEncodingWithCode {
            code: code.to_string(),
            message: message.into(),
        }
    }

    pub fn input_control_with_code(code: &str, message: impl Into<String>) -> Self {
        Self::InputControlWithCode {
            code: code.to_string(),
            message: message.into(),
        }
    }

    pub fn crypto_with_code(code: &str, message: impl Into<String>) -> Self {
        Self::CryptoWithCode {
            code: code.to_string(),
            message: message.into(),
        }
    }

    pub fn config_with_code(code: &str, message: impl Into<String>) -> Self {
        Self::ConfigWithCode {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, GhostHandError>;
