/// Protocol de messages échangés via le data channel WebRTC
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ControlMessage {
    // Stream control
    StartStream {
        resolution: (u32, u32),
        framerate: u32,
    },
    StopStream,
    StreamStarted,

    // Video frame
    VideoFrame {
        data: Vec<u8>,
        width: u32,
        height: u32,
        timestamp: u64,
        format: String, // "jpeg", "h264"
    },

    // Input control
    MouseMove {
        x: i32,
        y: i32,
    },
    MouseClick {
        button: String,
        pressed: bool,
    },
    MouseScroll {
        delta: i32,
    },
    KeyPress {
        key: String,
        pressed: bool,
    },

    // System
    Ping,
    Pong,
    Error {
        message: String,
    },
}

impl ControlMessage {
    /// Sérialiser le message en bytes pour envoi via WebRTC
    /// Utilise un format binaire optimisé pour VideoFrame, JSON pour les autres
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        match self {
            ControlMessage::VideoFrame { data, width, height, timestamp, format: _ } => {
                // Format binaire optimisé pour les frames vidéo
                // Header: [Magic: 4 bytes][Width: 4 bytes][Height: 4 bytes][Timestamp: 8 bytes][DataLen: 4 bytes]
                let mut buf = Vec::with_capacity(24 + data.len());

                // Magic number "VFRM"
                buf.extend_from_slice(b"VFRM");

                // Width (u32 little-endian)
                buf.extend_from_slice(&width.to_le_bytes());

                // Height (u32 little-endian)
                buf.extend_from_slice(&height.to_le_bytes());

                // Timestamp (u64 little-endian)
                buf.extend_from_slice(&timestamp.to_le_bytes());

                // Data length (u32 little-endian)
                buf.extend_from_slice(&(data.len() as u32).to_le_bytes());

                // Data (JPEG/H264)
                buf.extend_from_slice(data);

                Ok(buf)
            }
            _ => {
                // Utiliser JSON pour les autres messages
                serde_json::to_vec(self)
            }
        }
    }

    /// Désérialiser depuis bytes reçus via WebRTC
    pub fn from_bytes(data: &[u8]) -> Result<Self, serde_json::Error> {
        // Vérifier si c'est un VideoFrame en format binaire
        if data.len() >= 24 && &data[0..4] == b"VFRM" {
            // Désérialiser format binaire
            let width = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
            let height = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
            let timestamp = u64::from_le_bytes([
                data[12], data[13], data[14], data[15],
                data[16], data[17], data[18], data[19],
            ]);
            let data_len = u32::from_le_bytes([data[20], data[21], data[22], data[23]]) as usize;

            if data.len() >= 24 + data_len {
                let frame_data = data[24..24 + data_len].to_vec();
                Ok(ControlMessage::VideoFrame {
                    data: frame_data,
                    width,
                    height,
                    timestamp,
                    format: "jpeg".to_string(), // Assumer JPEG par défaut
                })
            } else {
                Err(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "VideoFrame data truncated"
                )))
            }
        } else {
            // Utiliser JSON pour les autres messages
            serde_json::from_slice(data)
        }
    }
}
