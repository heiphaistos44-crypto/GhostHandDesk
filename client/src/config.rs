use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Signaling server URL
    pub server_url: String,

    /// STUN server configuration
    pub stun_servers: Vec<String>,

    /// TURN server configuration (optional)
    pub turn_servers: Vec<TurnServer>,

    /// Video encoding settings
    pub video_config: VideoConfig,

    /// Network settings
    pub network_config: NetworkConfig,

    /// Security settings
    pub security_config: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnServer {
    pub url: String,
    pub username: String,
    pub credential: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    /// Target framerate (fps)
    pub framerate: u32,

    /// Video codec (H264, H265, VP8, VP9)
    pub codec: VideoCodec,

    /// Bitrate in kbps
    pub bitrate: u32,

    /// Enable hardware acceleration
    pub hardware_acceleration: bool,

    /// Capture resolution (None = native)
    pub resolution: Option<(u32, u32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    H265,
    VP8,
    VP9,
    AV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Maximum packet size for WebRTC data channel
    pub max_packet_size: usize,

    /// Connection timeout in seconds
    pub connection_timeout: u64,

    /// Enable IPv6
    pub enable_ipv6: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable end-to-end encryption
    pub e2e_encryption: bool,

    /// Require authentication
    pub require_auth: bool,

    /// Path to certificate (for custom CA)
    pub cert_path: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        // Lire le port depuis la variable d'environnement (priorité 1) ou le fichier (priorité 2)
        let port = std::env::var("GHD_SERVER_PORT")
            .ok()
            .or_else(|| {
                // Lire depuis UN SEUL emplacement standardisé : à côté de l'exécutable
                std::env::current_exe()
                    .ok()
                    .and_then(|exe_path| {
                        let port_file = exe_path.parent()?.join("server_port.txt");
                        match std::fs::read_to_string(&port_file) {
                            Ok(content) => Some(content.trim().to_string()),
                            Err(e) => {
                                tracing::warn!("Impossible de lire {}: {}", port_file.display(), e);
                                None
                            }
                        }
                    })
            })
            .unwrap_or_else(|| {
                tracing::warn!("Port non trouvé (variable GHD_SERVER_PORT ou server_port.txt), utilisation du port par défaut 9000");
                "9000".to_string()
            });

        let server_url = format!("ws://localhost:{}/ws", port);

        Self {
            server_url,
            stun_servers: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ],
            turn_servers: vec![],
            video_config: VideoConfig::default(),
            network_config: NetworkConfig::default(),
            security_config: SecurityConfig::default(),
        }
    }
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            framerate: 30,
            codec: VideoCodec::H264,
            bitrate: 4000, // 4 Mbps
            hardware_acceleration: true,
            resolution: None,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            max_packet_size: 65536,
            connection_timeout: 30,
            enable_ipv6: true,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            e2e_encryption: true,
            require_auth: true,
            cert_path: None,
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn load(path: &PathBuf) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, path: &PathBuf) -> crate::error::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
