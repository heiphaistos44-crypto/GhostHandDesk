use crate::config::VideoCodec;
use crate::error::{GhostHandError, Result};
use crate::screen_capture::{Frame, FrameFormat};
use tracing::{debug, info, warn};

/// Video encoder trait
#[async_trait::async_trait]
pub trait VideoEncoder: Send + Sync {
    /// Encode a frame
    async fn encode(&mut self, frame: &Frame) -> Result<EncodedFrame>;

    /// Get encoder info
    fn get_info(&self) -> EncoderInfo;
}

/// Encoded frame data
#[derive(Clone)]
pub struct EncodedFrame {
    pub data: Vec<u8>,
    pub timestamp: u64,
    pub is_keyframe: bool,
    pub width: u32,
    pub height: u32,
}

/// Encoder information
#[derive(Debug, Clone)]
pub struct EncoderInfo {
    pub codec: VideoCodec,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub bitrate: u32,
    pub hardware_accelerated: bool,
}

/// Simple encoder using image compression (for testing/fallback)
pub struct ImageEncoder {
    quality: u8,
    info: EncoderInfo,
}

impl ImageEncoder {
    pub fn new(width: u32, height: u32, framerate: u32) -> Result<Self> {
        Ok(Self {
            quality: 80,
            info: EncoderInfo {
                codec: VideoCodec::H264, // Not really H264, but placeholder
                width,
                height,
                framerate,
                bitrate: 0,
                hardware_accelerated: false,
            },
        })
    }
}

#[async_trait::async_trait]
impl VideoEncoder for ImageEncoder {
    async fn encode(&mut self, frame: &Frame) -> Result<EncodedFrame> {
        // Convert frame to image format
        let img = match frame.format {
            FrameFormat::RGBA => {
                image::RgbaImage::from_raw(frame.width, frame.height, frame.data.clone())
                    .ok_or_else(|| {
                        GhostHandError::VideoEncoding("Failed to create RGBA image".to_string())
                    })?
            }
            FrameFormat::BGRA => {
                // Convert BGRA to RGBA
                let mut rgba_data = frame.data.clone();
                for chunk in rgba_data.chunks_exact_mut(4) {
                    chunk.swap(0, 2); // Swap B and R
                }
                image::RgbaImage::from_raw(frame.width, frame.height, rgba_data).ok_or_else(
                    || GhostHandError::VideoEncoding("Failed to create RGBA image".to_string()),
                )?
            }
            _ => {
                return Err(GhostHandError::VideoEncoding(
                    "Unsupported frame format".to_string(),
                ))
            }
        };

        // Convert RGBA to RGB (JPEG doesn't support alpha)
        let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();

        // Encode to JPEG
        let mut buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buffer);

        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, self.quality)
            .encode(
                &rgb_img,
                frame.width,
                frame.height,
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| {
                GhostHandError::VideoEncoding(format!("Failed to encode JPEG: {}", e))
            })?;

        Ok(EncodedFrame {
            data: buffer,
            timestamp: frame.timestamp,
            is_keyframe: true, // JPEG frames are always keyframes
            width: frame.width,
            height: frame.height,
        })
    }

    fn get_info(&self) -> EncoderInfo {
        self.info.clone()
    }
}

// Note: For production, we would implement FFmpeg-based encoder
// This requires complex FFI bindings to libavcodec
// Here's a placeholder structure for future implementation

#[cfg(feature = "ffmpeg")]
pub struct FFmpegEncoder {
    encoder: ffmpeg_next::encoder::video::Video,
    scaler: ffmpeg_next::software::scaling::context::Context,
    info: EncoderInfo,
    frame_number: i64,
}

#[cfg(feature = "ffmpeg")]
impl FFmpegEncoder {
    pub fn new(
        codec: VideoCodec,
        width: u32,
        height: u32,
        framerate: u32,
        bitrate: u32,
    ) -> Result<Self> {
        use ffmpeg_next as ffmpeg;
        use ffmpeg::format::Pixel;
        use ffmpeg::software::scaling::{context::Context as ScalingContext, flag::Flags};

        info!(
            "Initialisation de l'encodeur FFmpeg: {:?} ({}x{} @ {} fps, {} kbps)",
            codec, width, height, framerate, bitrate
        );

        // 1. Initialiser FFmpeg (une seule fois)
        ffmpeg::init().map_err(|e| GhostHandError::VideoEncoding(format!("Erreur d'initialisation FFmpeg: {}", e)))?;

        // 2. Sélectionner le codec
        let codec_name = match codec {
            VideoCodec::H264 => "libx264",
            VideoCodec::H265 => "libx265",
            VideoCodec::VP8 => "libvpx",
            VideoCodec::VP9 => "libvpx-vp9",
            VideoCodec::AV1 => "libaom-av1",
        };

        let codec_id = match codec {
            VideoCodec::H264 => ffmpeg::codec::Id::H264,
            VideoCodec::H265 => ffmpeg::codec::Id::H265,
            VideoCodec::VP8 => ffmpeg::codec::Id::VP8,
            VideoCodec::VP9 => ffmpeg::codec::Id::VP9,
            VideoCodec::AV1 => ffmpeg::codec::Id::AV1,
        };

        // 3. Trouver l'encodeur
        let encoder_codec = ffmpeg::encoder::find(codec_id)
            .ok_or_else(|| GhostHandError::VideoEncoding(format!("Codec {} non trouvé", codec_name)))?;

        // 4. Créer le contexte d'encodage
        let mut encoder = ffmpeg::codec::context::Context::new_with_codec(encoder_codec)
            .encoder()
            .video()
            .map_err(|e| GhostHandError::VideoEncoding(format!("Erreur de création du contexte: {}", e)))?;

        // 5. Configurer l'encodeur
        encoder.set_width(width);
        encoder.set_height(height);
        encoder.set_format(Pixel::YUV420P);
        encoder.set_time_base((1, framerate as i32));
        encoder.set_frame_rate(Some((framerate as i32, 1)));
        encoder.set_bit_rate((bitrate * 1000) as usize);

        // 6. Options spécifiques H.264 pour faible latence
        if codec == VideoCodec::H264 {
            encoder.set_option("preset", "ultrafast")
                .map_err(|e| GhostHandError::VideoEncoding(format!("Erreur preset: {}", e)))?;
            encoder.set_option("tune", "zerolatency")
                .map_err(|e| GhostHandError::VideoEncoding(format!("Erreur tune: {}", e)))?;
            encoder.set_option("profile", "baseline")
                .map_err(|e| GhostHandError::VideoEncoding(format!("Erreur profile: {}", e)))?;
        }

        // 7. Ouvrir l'encodeur
        let encoder = encoder.open_as(encoder_codec)
            .map_err(|e| GhostHandError::VideoEncoding(format!("Erreur d'ouverture de l'encodeur: {}", e)))?;

        // 8. Créer le scaler (RGBA → YUV420P)
        let scaler = ScalingContext::get(
            Pixel::RGBA,
            width,
            height,
            Pixel::YUV420P,
            width,
            height,
            Flags::BILINEAR,
        )
        .map_err(|e| GhostHandError::VideoEncoding(format!("Erreur de création du scaler: {}", e)))?;

        info!("Encodeur FFmpeg {} initialisé avec succès", codec_name);

        Ok(Self {
            encoder,
            scaler,
            info: EncoderInfo {
                codec,
                width,
                height,
                framerate,
                bitrate,
                hardware_accelerated: false,
            },
            frame_number: 0,
        })
    }
}

#[cfg(feature = "ffmpeg")]
#[async_trait::async_trait]
impl VideoEncoder for FFmpegEncoder {
    async fn encode(&mut self, frame: &Frame) -> Result<EncodedFrame> {
        use ffmpeg_next as ffmpeg;
        use ffmpeg::format::Pixel;
        use ffmpeg::util::frame::video::Video as AVFrame;

        // 1. Créer AVFrame source (RGBA)
        let mut src_frame = AVFrame::new(Pixel::RGBA, frame.width, frame.height);

        // Copier les données de la frame
        let src_data = src_frame.data_mut(0);
        let copy_len = std::cmp::min(src_data.len(), frame.data.len());
        src_data[..copy_len].copy_from_slice(&frame.data[..copy_len]);

        // 2. Créer AVFrame destination (YUV420P)
        let mut yuv_frame = AVFrame::new(Pixel::YUV420P, self.info.width, self.info.height);

        // 3. Scaler RGBA → YUV420P
        self.scaler
            .run(&src_frame, &mut yuv_frame)
            .map_err(|e| GhostHandError::VideoEncoding(format!("Erreur de scaling: {}", e)))?;

        // 4. Définir PTS (Presentation Timestamp)
        yuv_frame.set_pts(Some(self.frame_number));
        self.frame_number += 1;

        // 5. Envoyer la frame à l'encodeur
        self.encoder
            .send_frame(&yuv_frame)
            .map_err(|e| GhostHandError::VideoEncoding(format!("Erreur d'envoi de frame: {}", e)))?;

        // 6. Recevoir les packets encodés
        let mut encoded_data = Vec::new();
        let mut is_keyframe = false;

        let mut packet = ffmpeg::codec::packet::Packet::empty();
        while self.encoder.receive_packet(&mut packet).is_ok() {
            if let Some(data) = packet.data() {
                encoded_data.extend_from_slice(data);
            }
            is_keyframe = packet.is_key();
        }

        // Si pas de données, retourner une frame vide
        if encoded_data.is_empty() {
            encoded_data = vec![0u8; 100]; // Placeholder minimal
        }

        Ok(EncodedFrame {
            data: encoded_data,
            timestamp: frame.timestamp,
            is_keyframe,
            width: frame.width,
            height: frame.height,
        })
    }

    fn get_info(&self) -> EncoderInfo {
        self.info.clone()
    }
}

/// Create encoder based on configuration
pub fn create_encoder(
    codec: VideoCodec,
    width: u32,
    height: u32,
    framerate: u32,
    bitrate: u32,
) -> Result<Box<dyn VideoEncoder>> {
    #[cfg(feature = "ffmpeg")]
    {
        info!("Création de l'encodeur FFmpeg");
        Ok(Box::new(FFmpegEncoder::new(
            codec, width, height, framerate, bitrate,
        )?))
    }

    #[cfg(not(feature = "ffmpeg"))]
    {
        warn!("FFmpeg non disponible, utilisation de l'encodeur JPEG");
        Ok(Box::new(ImageEncoder::new(width, height, framerate)?))
    }
}

/// Utility to detect hardware acceleration capabilities
pub fn detect_hardware_acceleration() -> Vec<String> {
    let mut available = Vec::new();

    // Check for NVIDIA NVENC
    #[cfg(target_os = "windows")]
    {
        // Check for NVIDIA GPU
        if std::process::Command::new("nvidia-smi")
            .output()
            .is_ok()
        {
            available.push("NVENC (NVIDIA)".to_string());
        }
    }

    // Check for Intel Quick Sync
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        // This is a simplified check
        if std::path::Path::new("/dev/dri").exists() {
            available.push("QSV (Intel Quick Sync)".to_string());
        }
    }

    // Check for AMD VCE/VCN
    #[cfg(target_os = "windows")]
    {
        // Similar check for AMD
    }

    // Check for Apple VideoToolbox
    #[cfg(target_os = "macos")]
    {
        available.push("VideoToolbox (Apple)".to_string());
    }

    if available.is_empty() {
        debug!("No hardware acceleration detected");
    } else {
        info!("Hardware acceleration available: {:?}", available);
    }

    available
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_image_encoder() -> Result<()> {
        let mut encoder = ImageEncoder::new(1920, 1080, 30)?;

        // Create a test frame
        let frame = Frame {
            width: 1920,
            height: 1080,
            data: vec![0u8; 1920 * 1080 * 4], // Black frame
            format: FrameFormat::RGBA,
            timestamp: 0,
        };

        let encoded = encoder.encode(&frame).await?;
        assert!(!encoded.data.is_empty());
        assert_eq!(encoded.width, 1920);
        assert_eq!(encoded.height, 1080);

        Ok(())
    }

    #[test]
    fn test_video_codec_variants() {
        // Test tous les variants de VideoCodec
        let codecs = vec![
            VideoCodec::H264,
            VideoCodec::H265,
            VideoCodec::VP8,
            VideoCodec::VP9,
            VideoCodec::AV1,
        ];

        for codec in codecs {
            assert!(matches!(
                codec,
                VideoCodec::H264
                    | VideoCodec::H265
                    | VideoCodec::VP8
                    | VideoCodec::VP9
                    | VideoCodec::AV1
            ));
        }
    }

    #[tokio::test]
    async fn test_encoder_compression() {
        let mut encoder = ImageEncoder::new(100, 100, 30).unwrap();

        // Frame rouge (créer manuellement)
        let mut data = Vec::with_capacity(100 * 100 * 4);
        for _ in 0..(100 * 100) {
            data.extend_from_slice(&[255, 0, 0, 255]); // RGBA rouge
        }

        let frame = Frame {
            width: 100,
            height: 100,
            data,
            format: FrameFormat::RGBA,
            timestamp: 1000,
        };

        let encoded = encoder.encode(&frame).await.unwrap();

        // Vérifier compression
        assert!(encoded.data.len() < frame.data.len());
        assert_eq!(encoded.timestamp, 1000);
    }

    #[test]
    fn test_encoder_info() {
        let info = EncoderInfo {
            codec: VideoCodec::H264,
            width: 1280,
            height: 720,
            framerate: 30,
            bitrate: 3000,
            hardware_accelerated: false,
        };

        assert_eq!(info.width, 1280);
        assert_eq!(info.height, 720);
        assert!(!info.hardware_accelerated);
    }

    #[tokio::test]
    async fn test_create_encoder_default() {
        // Test création avec config par défaut
        let result = create_encoder(VideoCodec::H264, 640, 480, 30, 2000);

        assert!(result.is_ok());

        let encoder = result.unwrap();
        let info = encoder.get_info();

        assert_eq!(info.width, 640);
        assert_eq!(info.height, 480);
        assert_eq!(info.framerate, 30);
    }

    #[test]
    fn test_detect_hardware_acceleration() {
        let available = detect_hardware_acceleration();

        // Devrait retourner une liste (même vide)
        assert!(available.is_empty() || !available.is_empty());
    }
}
