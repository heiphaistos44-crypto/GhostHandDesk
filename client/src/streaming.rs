//! Module de streaming vidéo en temps réel
//!
//! Ce module gère la boucle de capture, encodage et transmission vidéo.

use crate::adaptive_bitrate::AdaptiveBitrateController;
use crate::error::{GhostHandError, Result};
use crate::input_control::{InputController, MouseButton, MouseEvent as InputMouseEvent, KeyboardEvent as InputKeyboardEvent, KeyModifiers};
use crate::network::WebRTCConnection;
use crate::protocol::ControlMessage;
use crate::screen_capture::ScreenCapturer;
use crate::video_encoder::VideoEncoder;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use tracing::{debug, info, warn};

// Diagnostic temporaire - écriture directe dans un fichier log
fn stream_diag(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true).append(true)
        .open("C:\\Users\\Momo\\Documents\\GhostHandDesk\\diag.log")
    {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let _ = f.write_all(format!("[{}] {}\n", ts, msg).as_bytes());
    }
}

/// Callback pour recevoir les frames localement (preview sur le PC contrôlé)
pub type LocalFrameCallback = Arc<dyn Fn(Vec<u8>, u32, u32, u64) + Send + Sync>;

/// Streamer principal : capture → encode → send
pub struct Streamer {
    capturer: Arc<Mutex<Box<dyn ScreenCapturer>>>,
    encoder: Arc<Mutex<Box<dyn VideoEncoder>>>,
    webrtc: Arc<Mutex<WebRTCConnection>>,
    framerate: u32,
    running: Arc<AtomicBool>,
    adaptive_controller: Option<Arc<Mutex<AdaptiveBitrateController>>>,
    local_frame_callback: Option<LocalFrameCallback>,
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
            adaptive_controller: None,
            local_frame_callback: None,
        }
    }

    /// Activer le contrôle adaptatif du bitrate
    pub fn with_adaptive_bitrate(mut self, controller: AdaptiveBitrateController) -> Self {
        self.adaptive_controller = Some(Arc::new(Mutex::new(controller)));
        self
    }

    /// Ajouter un callback local pour le preview (PC contrôlé)
    pub fn with_local_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(Vec<u8>, u32, u32, u64) + Send + Sync + 'static,
    {
        self.local_frame_callback = Some(Arc::new(callback));
        self
    }

    /// Obtenir une référence partagée au capturer (pour switch de moniteur externe)
    pub fn capturer(&self) -> Arc<Mutex<Box<dyn ScreenCapturer>>> {
        self.capturer.clone()
    }

    /// Obtenir une référence partagée à l'encodeur (pour changer la résolution en live)
    pub fn encoder(&self) -> Arc<Mutex<Box<dyn VideoEncoder>>> {
        self.encoder.clone()
    }

    /// Démarrer le streaming
    pub async fn start(&self) -> Result<()> {
        info!("Démarrage du streaming vidéo à {} FPS", self.framerate);
        warn!("E2E encryption au niveau applicatif non intégré dans le pipeline vidéo - DTLS-SRTP actif par défaut");

        self.running.store(true, Ordering::SeqCst);

        // Channel pour découpler capture/encode de l'envoi réseau (capacity 2)
        // Si le sender est lent, try_send échoue → frame skippée (pas de backpressure)
        let (frame_tx, mut frame_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(2);

        // Durée du dernier envoi partagée via AtomicU64 (pour adaptive bitrate)
        let last_send_ns = Arc::new(AtomicU64::new(0));
        let last_send_ns_clone = last_send_ns.clone();

        // Task d'envoi séparée — tourne indépendamment de la boucle de capture
        let webrtc = self.webrtc.clone();
        tokio::spawn(async move {
            while let Some(bytes) = frame_rx.recv().await {
                let start = std::time::Instant::now();
                let webrtc_guard = webrtc.lock().await;
                if let Err(e) = webrtc_guard.send_data(&bytes).await {
                    stream_diag(&format!("SENDER: erreur envoi: {}", e));
                }
                last_send_ns_clone.store(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
            }
        });

        let frame_duration = Duration::from_millis(1000 / self.framerate as u64);
        let mut ticker = interval(frame_duration);

        let mut frame_count = 0u64;
        let mut error_count = 0u32;
        let mut skip_count = 0u64;

        while self.running.load(Ordering::SeqCst) {
            ticker.tick().await;

            // 1. Capturer frame
            let frame = {
                let mut capturer_guard = self.capturer.lock().await;
                match capturer_guard.capture_async().await {
                    Ok(f) => {
                        error_count = 0;
                        f
                    },
                    Err(_e) => {
                        error_count += 1;
                        if error_count >= 5 {
                            stream_diag("STREAMER: Trop d'erreurs capture, arrêt!");
                            return Err(GhostHandError::ScreenCapture(format!(
                                "Échec après {} erreurs consécutives de capture", error_count
                            )));
                        }
                        continue;
                    }
                }
            };

            // 2. Encoder frame
            let encoded = {
                let mut encoder_guard = self.encoder.lock().await;
                match encoder_guard.encode(&frame).await {
                    Ok(e) => e,
                    Err(e) => {
                        warn!("Erreur d'encodage: {}", e);
                        continue;
                    }
                }
            };

            // 2.5 Preview locale (1 frame sur 3 = ~10 FPS)
            if let Some(ref cb) = self.local_frame_callback {
                if frame_count.is_multiple_of(3) {
                    cb(encoded.data.clone(), encoded.width, encoded.height, encoded.timestamp);
                }
            }

            // 3. Sérialiser
            let message = ControlMessage::VideoFrame {
                data: encoded.data,
                width: encoded.width,
                height: encoded.height,
                timestamp: encoded.timestamp,
                format: "jpeg".to_string(),
            };

            // 4. Envoyer via channel (skip si le sender est occupé)
            match message.to_bytes() {
                Ok(bytes) => {
                    match frame_tx.try_send(bytes) {
                        Ok(_) => {},
                        Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                            skip_count += 1;
                        }
                        Err(_) => break, // Channel fermé
                    }
                }
                Err(e) => warn!("Erreur sérialisation: {}", e),
            }

            // 5. Adaptive bitrate basé sur la durée d'envoi du sender task
            if let Some(ref controller) = self.adaptive_controller {
                let send_ns = last_send_ns.load(Ordering::Relaxed);
                if send_ns > 0 {
                    let send_duration = Duration::from_nanos(send_ns);
                    let mut ctrl = controller.lock().await;
                    ctrl.update_rtt(send_duration);
                    let new_quality = ctrl.get_quality();
                    drop(ctrl);
                    let mut encoder_guard = self.encoder.lock().await;
                    encoder_guard.adjust_quality(new_quality);
                }
            }

            frame_count += 1;
            if frame_count.is_multiple_of(self.framerate as u64 * 10) {
                debug!(
                    "Streaming: {} frames envoyées, {} skipped ({} sec)",
                    frame_count, skip_count, frame_count / self.framerate as u64
                );
            }
        }

        info!("Streaming arrêté. Total frames: {}, skipped: {}", frame_count, skip_count);
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

    /// Démarrer la réception avec callbacks pour vidéo et messages de contrôle
    pub async fn start_with_message_handler<F, M>(
        self: Arc<Self>,
        frame_callback: F,
        message_callback: M,
    ) -> Result<()>
    where
        F: Fn(Vec<u8>, u32, u32, u64) + Send + Sync + 'static,
        M: Fn(ControlMessage) + Send + Sync + 'static,
    {
        info!("Démarrage de la réception vidéo");

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

        let webrtc = self.webrtc.lock().await;

        let rx_counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let rx_counter_clone = rx_counter.clone();
        webrtc.on_data_channel_message(move |data: &[u8]| {
            let count = rx_counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count < 3 {
                stream_diag(&format!("RECEIVER-RAW: message #{} reçu ({} bytes)", count, data.len()));
            }
            let _ = tx.send(data.to_vec());
        }).await?;

        stream_diag("Receiver: on_data_channel_message configuré OK");

        let frame_cb = Arc::new(frame_callback);
        let msg_cb = Arc::new(message_callback);
        tokio::spawn(async move {
            let mut reassembly: Option<(usize, Vec<u8>)> = None;

            while let Some(data) = rx.recv().await {
                // Vérifier si c'est un message fragmenté
                if data.len() >= 2 && data[0] == 0xFF {
                    match data[1] {
                        0x01 if data.len() >= 6 => {
                            let total_len = u32::from_le_bytes([data[2], data[3], data[4], data[5]]) as usize;
                            reassembly = Some((total_len, Vec::with_capacity(total_len)));
                            continue;
                        }
                        0x02 => {
                            if let Some((expected_len, ref mut buffer)) = reassembly {
                                buffer.extend_from_slice(&data[2..]);
                                if buffer.len() >= expected_len {
                                    let complete_data = std::mem::take(buffer);
                                    reassembly = None;
                                    if let Ok(msg) = ControlMessage::from_bytes(&complete_data) {
                                        match msg {
                                            ControlMessage::VideoFrame { data, width, height, timestamp, .. } => {
                                                frame_cb(data, width, height, timestamp);
                                            }
                                            other => {
                                                msg_cb(other);
                                            }
                                        }
                                    }
                                }
                            }
                            continue;
                        }
                        _ => {}
                    }
                }

                // Message normal (non fragmenté)
                if let Ok(msg) = ControlMessage::from_bytes(&data) {
                    match msg {
                        ControlMessage::VideoFrame { data, width, height, timestamp, .. } => {
                            frame_cb(data, width, height, timestamp);
                        }
                        other => {
                            msg_cb(other);
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
            ControlMessage::KeyPress { key, pressed, modifiers } => {
                // Convertir les modifiers du protocole vers input_control
                let key_modifiers = if let Some(m) = modifiers {
                    KeyModifiers {
                        ctrl: m.ctrl,
                        shift: m.shift,
                        alt: m.alt,
                        meta: m.meta,
                    }
                } else {
                    KeyModifiers::default()
                };
                if pressed {
                    self.controller.lock().await.handle_keyboard_event(InputKeyboardEvent::Press { key }, key_modifiers)?;
                } else {
                    self.controller.lock().await.handle_keyboard_event(InputKeyboardEvent::Release { key }, key_modifiers)?;
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
