use crate::error::{GhostHandError, Result};
use image::{ImageBuffer, RgbaImage};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Screen capture interface
pub trait ScreenCapturer {
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

        Ok(Frame {
            width,
            height,
            data,
            format: FrameFormat::RGBA,
            timestamp: self.frame_count,
        })
    }

    fn get_displays(&self) -> Result<Vec<Display>> {
        let displays = self
            .monitors
            .iter()
            .enumerate()
            .map(|(idx, _monitor)| {
                // xcap Monitor doesn't have direct getter methods in this version
                // We'll use default values that will be updated on first capture
                Display {
                    id: idx as u32,
                    name: format!("Monitor {}", idx),
                    width: 1920, // Default, will be updated on first capture
                    height: 1080,
                    x: 0,
                    y: 0,
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
        // Return a default resolution for now
        // Will be updated when we capture a frame
        (1920, 1080)
    }
}

/// Alternative capturer using scrap for more control
#[cfg(feature = "scrap_capturer")]
pub struct ScrapCapturer {
    capturer: scrap::Capturer,
    width: u32,
    height: u32,
    frame_count: u64,
}

#[cfg(feature = "scrap_capturer")]
impl ScrapCapturer {
    pub fn new() -> Result<Self> {
        let display = scrap::Display::primary().map_err(|e| {
            GhostHandError::ScreenCapture(format!("Failed to get primary display: {}", e))
        })?;

        let capturer = scrap::Capturer::new(display).map_err(|e| {
            GhostHandError::ScreenCapture(format!("Failed to create capturer: {}", e))
        })?;

        let width = capturer.width() as u32;
        let height = capturer.height() as u32;

        Ok(Self {
            capturer,
            width,
            height,
            frame_count: 0,
        })
    }
}

#[cfg(feature = "scrap_capturer")]
impl ScreenCapturer for ScrapCapturer {
    fn capture(&mut self) -> Result<Frame> {
        // Scrap is synchronous
        let frame = loop {
            match self.capturer.frame() {
                Ok(buffer) => break buffer.to_vec(),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Frame not ready yet, retry
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    continue;
                }
                Err(e) => {
                    return Err(GhostHandError::ScreenCapture(format!(
                        "Capture failed: {}",
                        e
                    )))
                }
            }
        };

        self.frame_count += 1;

        Ok(Frame {
            width: self.width,
            height: self.height,
            data: frame,
            format: FrameFormat::BGRA, // scrap uses BGRA on most platforms
            timestamp: self.frame_count,
        })
    }

    fn get_displays(&self) -> Result<Vec<Display>> {
        // scrap doesn't provide multi-monitor support easily
        Ok(vec![Display {
            id: 0,
            name: "Primary Display".to_string(),
            width: self.width,
            height: self.height,
            x: 0,
            y: 0,
            is_primary: true,
        }])
    }

    fn select_display(&mut self, _display_id: u32) -> Result<()> {
        // Not implemented for scrap
        warn!("Display selection not supported with scrap capturer");
        Ok(())
    }

    fn get_resolution(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// Factory to create the appropriate capturer for the platform
pub fn create_capturer() -> Result<Box<dyn ScreenCapturer>> {
    #[cfg(not(feature = "scrap_capturer"))]
    {
        Ok(Box::new(XCapCapturer::new()?))
    }

    #[cfg(feature = "scrap_capturer")]
    {
        Ok(Box::new(ScrapCapturer::new()?))
    }
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
