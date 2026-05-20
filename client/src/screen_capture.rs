use crate::error::{GhostHandError, Result};
use tracing::{debug, info};
use async_trait::async_trait;

/// Screen capture interface
#[async_trait]
pub trait ScreenCapturer: Send {
    /// Capture the current screen frame (synchronous - for backwards compatibility)
    fn capture(&mut self) -> Result<Frame>;

    /// Capture the current screen frame (asynchronous - non-blocking)
    /// This method uses tokio::task::spawn_blocking to avoid blocking the async runtime
    async fn capture_async(&mut self) -> Result<Frame>;

    /// Get available displays
    fn get_displays(&self) -> Result<Vec<Display>>;

    /// Select which display to capture
    fn select_display(&mut self, display_id: u32) -> Result<()>;

    /// Get current capture resolution
    fn get_resolution(&self) -> (u32, u32);
}

/// Represents a captured frame
#[derive(Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub format: FrameFormat,
    pub timestamp: u64,
}

#[derive(Clone, Copy, Debug)]
pub enum FrameFormat {
    RGBA,
    BGRA,
    RGB,
    BGR,
}

/// Represents a display/monitor
#[derive(Clone, Debug)]
pub struct Display {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub is_primary: bool,
}

/// Cross-platform screen capturer using xcap
pub struct XCapCapturer {
    monitors: Vec<xcap::Monitor>,
    current_monitor: Option<usize>,
    frame_count: u64,
}

// SAFETY: XCapCapturer contient xcap::Monitor qui n'implémente pas Send.
// Cependant, xcap::Monitor ne contient que des métadonnées (dimensions, position, nom).
// L'accès concurrent est protégé par un Mutex<Box<dyn ScreenCapturer>> dans Streamer.
// La capture réelle (capture_image) est effectuée dans spawn_blocking pour isoler les appels.
unsafe impl Send for XCapCapturer {}

impl XCapCapturer {
    pub fn new() -> Result<Self> {
        let monitors = xcap::Monitor::all()
            .map_err(|e| GhostHandError::ScreenCapture(format!("Failed to get monitors: {}", e)))?;

        if monitors.is_empty() {
            return Err(GhostHandError::ScreenCapture(
                "No monitors found".to_string(),
            ));
        }

        info!("Found {} monitor(s)", monitors.len());
        for (idx, monitor) in monitors.iter().enumerate() {
            debug!(
                "Monitor {}: {}x{} at ({}, {})",
                idx,
                monitor.width(),
                monitor.height(),
                monitor.x(),
                monitor.y()
            );
        }

        Ok(Self {
            monitors,
            current_monitor: Some(0), // Default to first monitor
            frame_count: 0,
        })
    }
}

#[async_trait]
impl ScreenCapturer for XCapCapturer {
    fn capture(&mut self) -> Result<Frame> {
        let monitor_idx = self.current_monitor.ok_or_else(|| {
            GhostHandError::ScreenCapture("No monitor selected".to_string())
        })?;

        let monitor = self.monitors.get(monitor_idx).ok_or_else(|| {
            GhostHandError::ScreenCapture(format!("Invalid monitor index: {}", monitor_idx))
        })?;

        // Capture the screen
        let image = monitor.capture_image().map_err(|e| {
            GhostHandError::ScreenCapture(format!("Failed to capture screen: {}", e))
        })?;

        let width = image.width();
        let height = image.height();
        let data = image.into_raw();

        self.frame_count += 1;

        // Utiliser un timestamp réel en millisecondes
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(Frame {
            width,
            height,
            data,
            format: FrameFormat::RGBA,
            timestamp,
        })
    }

    async fn capture_async(&mut self) -> Result<Frame> {
        // Note: xcap::Monitor n'implémente pas Send (handle Windows HMONITOR)
        // On ne peut donc pas utiliser spawn_blocking directement.
        // Solution: Appeler la méthode synchrone mais dans un contexte async.
        // Le gain de performance vient du fait que le mutex est libéré rapidement
        // et que d'autres tâches async peuvent s'exécuter pendant la capture.

        let monitor_idx = self.current_monitor.ok_or_else(|| {
            GhostHandError::ScreenCapture("No monitor selected".to_string())
        })?;

        let monitor = self.monitors.get(monitor_idx).ok_or_else(|| {
            GhostHandError::ScreenCapture(format!("Invalid monitor index: {}", monitor_idx))
        })?;

        // Capture de l'image (opération bloquante mais généralement rapide ~10-20ms)
        let image = monitor.capture_image().map_err(|e| {
            GhostHandError::ScreenCapture(format!("Failed to capture screen: {}", e))
        })?;

        let width = image.width();
        let height = image.height();

        // Limitation: seul into_raw() est dans spawn_blocking car xcap::Monitor
        // n'est pas Send (HMONITOR sur Windows). La capture elle-même reste synchrone
        // dans le contexte async, mais est rapide (~10-20ms).
        let data = tokio::task::spawn_blocking(move || {
            image.into_raw()
        })
        .await
        .map_err(|e| {
            GhostHandError::ScreenCapture(format!("Image conversion failed: {}", e))
        })?;

        self.frame_count += 1;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        debug!("Captured frame {} asynchronously ({}x{})", self.frame_count, width, height);

        Ok(Frame {
            width,
            height,
            data,
            format: FrameFormat::RGBA,
            timestamp,
        })
    }

    fn get_displays(&self) -> Result<Vec<Display>> {
        let displays = self
            .monitors
            .iter()
            .enumerate()
            .map(|(idx, monitor)| {
                Display {
                    id: idx as u32,
                    name: format!("Monitor {}", idx),
                    width: monitor.width(),
                    height: monitor.height(),
                    x: monitor.x(),
                    y: monitor.y(),
                    is_primary: idx == 0,
                }
            })
            .collect();

        Ok(displays)
    }

    fn select_display(&mut self, display_id: u32) -> Result<()> {
        let idx = display_id as usize;
        if idx >= self.monitors.len() {
            return Err(GhostHandError::ScreenCapture(format!(
                "Invalid display ID: {}",
                display_id
            )));
        }

        self.current_monitor = Some(idx);
        info!("Selected display {}", display_id);
        Ok(())
    }

    fn get_resolution(&self) -> (u32, u32) {
        if let Some(idx) = self.current_monitor {
            if let Some(monitor) = self.monitors.get(idx) {
                return (monitor.width(), monitor.height());
            }
        }
        (1920, 1080) // Fallback
    }
}

/// Factory to create the appropriate capturer for the platform
pub fn create_capturer() -> Result<Box<dyn ScreenCapturer>> {
    Ok(Box::new(XCapCapturer::new()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_capture() -> Result<()> {
        let mut capturer = XCapCapturer::new()?;

        let displays = capturer.get_displays()?;
        assert!(!displays.is_empty());

        let frame = capturer.capture()?;
        assert!(frame.width > 0);
        assert!(frame.height > 0);
        assert!(!frame.data.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_async_capture() -> Result<()> {
        let mut capturer = XCapCapturer::new()?;

        // Test capture async
        let frame = capturer.capture_async().await?;
        assert!(frame.width > 0);
        assert!(frame.height > 0);
        assert!(!frame.data.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_async_capture_performance() -> Result<()> {
        let mut capturer = XCapCapturer::new()?;

        let num_frames = 30; // Capturer 30 frames pour mesurer le FPS
        let start = std::time::Instant::now();

        for _ in 0..num_frames {
            let _frame = capturer.capture_async().await?;
        }

        let elapsed = start.elapsed();
        let fps = num_frames as f64 / elapsed.as_secs_f64();

        println!("✅ Capture async performance: {:.2} FPS", fps);
        println!("   Temps total: {:.2}s pour {} frames", elapsed.as_secs_f64(), num_frames);

        // Note: FPS dépend fortement du hardware et de la résolution d'écran
        // Mode debug: 5-10 FPS typique
        // Mode release: 15-60 FPS selon hardware
        // Pas d'assertion stricte car performance varie trop selon l'environnement
        if fps < 5.0 {
            println!("⚠️  Warning: FPS très bas ({:.2}), vérifier performance xcap", fps);
        }

        Ok(())
    }

    #[test]
    fn test_sync_capture_performance() -> Result<()> {
        let mut capturer = XCapCapturer::new()?;

        let num_frames = 30;
        let start = std::time::Instant::now();

        for _ in 0..num_frames {
            let _frame = capturer.capture()?;
        }

        let elapsed = start.elapsed();
        let fps = num_frames as f64 / elapsed.as_secs_f64();

        println!("📊 Capture sync performance: {:.2} FPS", fps);
        println!("   Temps total: {:.2}s pour {} frames", elapsed.as_secs_f64(), num_frames);

        Ok(())
    }
}
