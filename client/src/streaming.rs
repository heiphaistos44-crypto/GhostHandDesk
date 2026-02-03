//! Module de streaming vidéo en temps réel
//!
//! Ce module gère la boucle de capture, encodage et transmission vidéo.

use crate::config::Config;
use crate::error::Result;
use crate::network::WebRTCConnection;
use crate::screen_capture::ScreenCapturer;
use crate::video_encoder::VideoEncoder;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// Streamer principal : capture → encode → send
pub struct Streamer {
    capturer: Arc<Mutex<Box<dyn ScreenCapturer>>>,
    encoder: Arc<Mutex<Box<dyn VideoEncoder>>>,
    webrtc: Arc<Mutex<WebRTCConnection>>,
    framerate: u32,
    running: Arc<Mutex<bool>>,
}

impl Streamer {
    /// Créer un nouveau streamer
    pub fn new(
        capturer: Box<dyn ScreenCapturer>,
        encoder: Box<dyn VideoEncoder>,
        webrtc: WebRTCConnection,
        framerate: u32,
    ) -> Self {
        Self {
            capturer: Arc::new(Mutex::new(capturer)),
            encoder: Arc::new(Mutex::new(encoder)),
            webrtc: Arc::new(Mutex::new(webrtc)),
            framerate,
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Démarrer le streaming
    pub async fn start(&self) -> Result<()> {
        info!("Démarrage du streaming vidéo à {} FPS", self.framerate);

        // Marquer comme running
        *self.running.lock().await = true;

        // Créer un interval pour le framerate
        let frame_duration = Duration::from_millis(1000 / self.framerate as u64);
        let mut ticker = interval(frame_duration);

        let mut frame_count = 0u64;
        let mut error_count = 0u32;

        while *self.running.lock().await {
            ticker.tick().await;

            // 1. Capturer frame
            let frame = match self.capturer.lock().await.capture() {
                Ok(f) => f,
                Err(e) => {
                    error_count += 1;
                    if error_count < 5 {
                        warn!("Erreur de capture ({}): {}", error_count, e);
                        continue;
                    } else {
                        error!("Trop d'erreurs de capture, arrêt du streaming");
                        break;
                    }
                }
            };

            // Reset error count on success
            if error_count > 0 {
                error_count = 0;
            }

            // 2. Encoder frame
            let encoded = match self.encoder.lock().await.encode(&frame).await {
                Ok(e) => e,
                Err(e) => {
                    warn!("Erreur d'encodage: {}", e);
                    continue;
                }
            };

            // 3. Envoyer via WebRTC
            if let Err(e) = self.webrtc.lock().await.send_data(&encoded.data).await {
                warn!("Erreur d'envoi WebRTC: {}", e);
                // Ne pas arrêter, juste logger
            }

            frame_count += 1;
            if frame_count % (self.framerate as u64 * 10) == 0 {
                debug!(
                    "Streaming: {} frames envoyées ({} secondes)",
                    frame_count,
                    frame_count / self.framerate as u64
                );
            }
        }

        info!("Streaming arrêté. Total frames: {}", frame_count);
        Ok(())
    }

    /// Arrêter le streaming
    pub async fn stop(&self) {
        info!("Arrêt du streaming demandé");
        *self.running.lock().await = false;
    }

    /// Vérifier si le streaming est actif
    pub async fn is_running(&self) -> bool {
        *self.running.lock().await
    }
}

/// Receiver : réception et décodage vidéo
pub struct Receiver {
    webrtc: Arc<Mutex<WebRTCConnection>>,
    frame_callback: Option<Box<dyn Fn(Vec<u8>) + Send + Sync>>,
}

impl Receiver {
    /// Créer un nouveau receiver
    pub fn new(webrtc: WebRTCConnection) -> Self {
        Self {
            webrtc: Arc::new(Mutex::new(webrtc)),
            frame_callback: None,
        }
    }

    /// Configurer le callback pour les frames reçues
    pub fn on_frame<F>(&mut self, callback: F)
    where
        F: Fn(Vec<u8>) + Send + Sync + 'static,
    {
        self.frame_callback = Some(Box::new(callback));
    }

    /// Démarrer la réception
    pub async fn start(self: Arc<Self>) -> Result<()> {
        info!("Démarrage de la réception vidéo");

        // Setup data channel callback
        if self.frame_callback.is_none() {
            warn!("Aucun callback configuré pour les frames");
            return Ok(());
        }

        // Note: Pour l'instant, on configure juste le receiver
        // L'implémentation complète nécessite de refactorer on_data_channel_message
        info!("Réception vidéo configurée");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_streamer_creation() {
        // Test basique de création
        // Note: nécessite des mocks pour les vrais tests
    }
}
