//! Module de streaming vidéo en temps réel
//!
//! Ce module gère la boucle de capture, encodage et transmission vidéo.

use crate::error::{GhostHandError, Result};
use crate::input_control::{InputController, MouseButton, MouseEvent as InputMouseEvent, KeyboardEvent as InputKeyboardEvent, KeyModifiers};
use crate::network::WebRTCConnection;
use crate::protocol::ControlMessage;
use crate::screen_capture::ScreenCapturer;
use crate::video_encoder::VideoEncoder;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// Streamer principal : capture → encode → send
pub struct Streamer {
    capturer: Arc<Mutex<Box<dyn ScreenCapturer>>>,
    encoder: Arc<Mutex<Box<dyn VideoEncoder>>>,
    webrtc: Arc<Mutex<WebRTCConnection>>,
    framerate: u32,
    running: Arc<AtomicBool>,
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
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Démarrer le streaming
    pub async fn start(&self) -> Result<()> {
        info!("Démarrage du streaming vidéo à {} FPS", self.framerate);

        // Marquer comme running (AtomicBool = pas de lock)
        self.running.store(true, Ordering::SeqCst);

        // Créer un interval pour le framerate
        let frame_duration = Duration::from_millis(1000 / self.framerate as u64);
        let mut ticker = interval(frame_duration);

        let mut frame_count = 0u64;
        let mut error_count = 0u32;

        while self.running.load(Ordering::SeqCst) {
            ticker.tick().await;

            // 1. Capturer frame (scope explicite pour libérer le lock immédiatement)
            let frame = {
                let mut capturer_guard = self.capturer.lock().await;
                match capturer_guard.capture() {
                    Ok(f) => {
                        // Reset error count sur succès
                        error_count = 0;
                        f
                    },
                    Err(e) => {
                        error_count += 1;
                        if error_count >= 5 {
                            error!("Trop d'erreurs de capture consécutives ({}), arrêt du streaming", error_count);
                            return Err(GhostHandError::ScreenCapture(format!(
                                "Échec après {} erreurs consécutives de capture", error_count
                            )));
                        }
                        warn!("Erreur de capture ({}/5): {}", error_count, e);
                        continue;
                    }
                }
            }; // capturer_guard est drop ici

            // 2. Encoder frame (scope explicite pour libérer le lock immédiatement)
            let encoded = {
                let mut encoder_guard = self.encoder.lock().await;
                match encoder_guard.encode(&frame).await {
                    Ok(e) => e,
                    Err(e) => {
                        warn!("Erreur d'encodage: {}", e);
                        continue;
                    }
                }
            }; // encoder_guard est drop ici

            // 3. Créer le message ControlMessage
            let message = ControlMessage::VideoFrame {
                data: encoded.data,
                width: encoded.width,
                height: encoded.height,
                timestamp: encoded.timestamp,
                format: "jpeg".to_string(), // ou "h264" selon l'encoder
            };

            // 4. Envoyer via WebRTC (scope explicite)
            match message.to_bytes() {
                Ok(bytes) => {
                    let webrtc_guard = self.webrtc.lock().await;
                    if let Err(e) = webrtc_guard.send_data(&bytes).await {
                        warn!("Erreur d'envoi WebRTC: {}", e);
                        // Ne pas arrêter, juste logger
                    }
                    // webrtc_guard est drop ici automatiquement
                }
                Err(e) => {
                    warn!("Erreur de sérialisation du message: {}", e);
                }
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
    pub fn stop(&self) {
        info!("Arrêt du streaming demandé");
        self.running.store(false, Ordering::SeqCst);
    }

    /// Vérifier si le streaming est actif
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Receiver : réception et décodage vidéo
pub struct Receiver {
    webrtc: Arc<Mutex<WebRTCConnection>>,
}

impl Receiver {
    /// Créer un nouveau receiver
    pub fn new(webrtc: WebRTCConnection) -> Self {
        Self {
            webrtc: Arc::new(Mutex::new(webrtc)),
        }
    }

    /// Démarrer la réception avec un callback pour les frames
    pub async fn start<F>(self: Arc<Self>, frame_callback: F) -> Result<()>
    where
        F: Fn(Vec<u8>, u32, u32, u64) + Send + Sync + 'static,
    {
        info!("Démarrage de la réception vidéo");

        // Créer un canal mpsc pour éviter les "lost wakeups"
        // Le callback WebRTC envoie les données dans le canal,
        // et une task async les traite séparément
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

        let webrtc = self.webrtc.lock().await;

        // Setup callback pour recevoir les messages du data channel
        // Le callback envoie juste les données brutes dans le canal
        webrtc.on_data_channel_message(move |data: &[u8]| {
            let _ = tx.send(data.to_vec());
        }).await?;

        info!("Réception vidéo configurée");

        // Spawner une task pour traiter les messages du canal
        let callback = Arc::new(frame_callback);
        tokio::spawn(async move {
            while let Some(data) = rx.recv().await {
                // Parse le message
                if let Ok(msg) = ControlMessage::from_bytes(&data) {
                    match msg {
                        ControlMessage::VideoFrame { data, width, height, timestamp, .. } => {
                            // Appeler le callback avec les données de la frame
                            callback(data, width, height, timestamp);
                        }
                        _ => {
                            debug!("Message non-vidéo reçu: {:?}", msg);
                        }
                    }
                }
            }
        });

        Ok(())
    }
}

/// InputHandler : gestion des commandes input reçues
pub struct InputHandler {
    controller: Arc<Mutex<InputController>>,
}

impl InputHandler {
    /// Créer un nouveau InputHandler avec résolution par défaut
    pub fn new() -> Result<Self> {
        Ok(Self {
            controller: Arc::new(Mutex::new(InputController::new()?)),
        })
    }

    /// Créer un nouveau InputHandler avec une résolution spécifique
    pub fn new_with_resolution(width: i32, height: i32) -> Result<Self> {
        Ok(Self {
            controller: Arc::new(Mutex::new(InputController::new_with_resolution(width, height)?)),
        })
    }

    /// Traiter un message de contrôle reçu
    pub async fn handle_message(&self, msg: ControlMessage) -> Result<()> {
        match msg {
            ControlMessage::MouseMove { x, y } => {
                self.controller.lock().await.handle_mouse_event(InputMouseEvent::Move { x, y })?;
            }
            ControlMessage::MouseClick { button, pressed } => {
                let btn = match button.as_str() {
                    "left" => MouseButton::Left,
                    "right" => MouseButton::Right,
                    "middle" => MouseButton::Middle,
                    _ => return Ok(()), // Ignorer les boutons inconnus
                };
                if pressed {
                    self.controller.lock().await.handle_mouse_event(InputMouseEvent::Press { button: btn })?;
                } else {
                    self.controller.lock().await.handle_mouse_event(InputMouseEvent::Release { button: btn })?;
                }
            }
            ControlMessage::MouseScroll { delta } => {
                self.controller.lock().await.handle_mouse_event(InputMouseEvent::Scroll {
                    delta_x: 0,
                    delta_y: delta,
                })?;
            }
            ControlMessage::KeyPress { key, pressed } => {
                // Pour l'instant, pas de modifiers (TODO: ajouter au protocole)
                let modifiers = KeyModifiers::default();
                if pressed {
                    self.controller.lock().await.handle_keyboard_event(InputKeyboardEvent::Press { key }, modifiers)?;
                } else {
                    self.controller.lock().await.handle_keyboard_event(InputKeyboardEvent::Release { key }, modifiers)?;
                }
            }
            _ => {
                debug!("Message non-input reçu: {:?}", msg);
            }
        }
        Ok(())
    }

    /// Setup le handler sur une connexion WebRTC existante
    pub async fn attach_to_webrtc(self: Arc<Self>, webrtc: Arc<Mutex<WebRTCConnection>>) -> Result<()> {
        info!("Attachement du InputHandler au WebRTC");

        // Créer un canal mpsc pour éviter les "lost wakeups"
        // Le callback WebRTC envoie les données dans le canal,
        // et une task async les traite séparément
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

        let webrtc_locked = webrtc.lock().await;

        // Setup callback pour recevoir les messages du data channel
        // Le callback envoie juste les données brutes dans le canal
        webrtc_locked.on_data_channel_message(move |data: &[u8]| {
            let _ = tx.send(data.to_vec());
        }).await?;

        info!("InputHandler attaché avec succès");

        // Spawner une task pour traiter les messages du canal
        let handler = self.clone();
        tokio::spawn(async move {
            while let Some(data) = rx.recv().await {
                // Parse le message
                if let Ok(msg) = ControlMessage::from_bytes(&data) {
                    if let Err(e) = handler.handle_message(msg).await {
                        warn!("Erreur traitement message input: {}", e);
                    }
                }
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_streamer_creation() {
        // Test basique de création
        // Note: nécessite des mocks pour les vrais tests
    }
}
