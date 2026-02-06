use crate::error::{GhostHandError, Result};
use tracing::{debug, info};

/// Screen capture interface
pub trait ScreenCapturer: Send {
    /// Capture the current screen frame
    fn capture(&mut self) -> Result<Frame>;

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

// SAFETY: xcap::Monitor is safe to send between threads as it only contains metadata
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

    fn get_displays(&self) -> Result<Vec<Display>> {
        let displays = self
            .monitors
            .iter()
            .enumerate()
            .map(|(idx, monitor)| {
                Display {
                    id: idx as u32,
                    name: format!("Monitor {}", idx),
                    width: monitor.width() as u32,
                    height: monitor.height() as u32,
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
                return (monitor.width() as u32, monitor.height() as u32);
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
}
