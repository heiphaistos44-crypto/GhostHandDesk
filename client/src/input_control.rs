use crate::error::{GhostHandError, Result};
use enigo::{
    Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Input control manager for keyboard and mouse events
pub struct InputController {
    enigo: Enigo,
}

/// Represents a mouse event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseEvent {
    Move { x: i32, y: i32 },
    Click { button: MouseButton },
    Press { button: MouseButton },
    Release { button: MouseButton },
    Scroll { delta_x: i32, delta_y: i32 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Represents a keyboard event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyboardEvent {
    Press { key: String },
    Release { key: String },
    Type { text: String },
}

impl InputController {
    /// Create a new input controller
    pub fn new() -> Result<Self> {
        let enigo = Enigo::new(&Settings::default()).map_err(|e| {
            GhostHandError::InputControl(format!("Failed to initialize input control: {}", e))
        })?;

        Ok(Self { enigo })
    }

    /// Handle a mouse event
    pub fn handle_mouse_event(&mut self, event: MouseEvent) -> Result<()> {
        match event {
            MouseEvent::Move { x, y } => {
                debug!("Mouse move to ({}, {})", x, y);
                self.enigo
                    .move_mouse(x, y, Coordinate::Abs)
                    .map_err(|e| {
                        GhostHandError::InputControl(format!("Failed to move mouse: {}", e))
                    })?;
            }
            MouseEvent::Click { button } => {
                debug!("Mouse click: {:?}", button);
                let btn = Self::convert_button(button);
                self.enigo.button(btn, Direction::Click).map_err(|e| {
                    GhostHandError::InputControl(format!("Failed to click mouse: {}", e))
                })?;
            }
            MouseEvent::Press { button } => {
                debug!("Mouse press: {:?}", button);
                let btn = Self::convert_button(button);
                self.enigo.button(btn, Direction::Press).map_err(|e| {
                    GhostHandError::InputControl(format!("Failed to press mouse: {}", e))
                })?;
            }
            MouseEvent::Release { button } => {
                debug!("Mouse release: {:?}", button);
                let btn = Self::convert_button(button);
                self.enigo.button(btn, Direction::Release).map_err(|e| {
                    GhostHandError::InputControl(format!("Failed to release mouse: {}", e))
                })?;
            }
            MouseEvent::Scroll { delta_x, delta_y } => {
                debug!("Mouse scroll: dx={}, dy={}", delta_x, delta_y);
                if delta_y != 0 {
                    self.enigo.scroll(delta_y, enigo::Axis::Vertical).map_err(|e| {
                        GhostHandError::InputControl(format!("Failed to scroll: {}", e))
                    })?;
                }
                if delta_x != 0 {
                    self.enigo.scroll(delta_x, enigo::Axis::Horizontal).map_err(|e| {
                        GhostHandError::InputControl(format!("Failed to scroll: {}", e))
                    })?;
                }
            }
        }
        Ok(())
    }

    /// Handle a keyboard event
    pub fn handle_keyboard_event(&mut self, event: KeyboardEvent) -> Result<()> {
        match event {
            KeyboardEvent::Press { key } => {
                debug!("Key press: {}", key);
                if let Some(k) = Self::parse_key(&key) {
                    self.enigo.key(k, Direction::Press).map_err(|e| {
                        GhostHandError::InputControl(format!("Failed to press key: {}", e))
                    })?;
                } else {
                    warn!("Unknown key: {}", key);
                }
            }
            KeyboardEvent::Release { key } => {
                debug!("Key release: {}", key);
                if let Some(k) = Self::parse_key(&key) {
                    self.enigo.key(k, Direction::Release).map_err(|e| {
                        GhostHandError::InputControl(format!("Failed to release key: {}", e))
                    })?;
                } else {
                    warn!("Unknown key: {}", key);
                }
            }
            KeyboardEvent::Type { text } => {
                debug!("Type text: {}", text);
                self.enigo.text(&text).map_err(|e| {
                    GhostHandError::InputControl(format!("Failed to type text: {}", e))
                })?;
            }
        }
        Ok(())
    }

    /// Convert MouseButton to enigo Button
    fn convert_button(button: MouseButton) -> Button {
        match button {
            MouseButton::Left => Button::Left,
            MouseButton::Right => Button::Right,
            MouseButton::Middle => Button::Middle,
        }
    }

    /// Parse a key string to enigo Key
    fn parse_key(key_str: &str) -> Option<Key> {
        match key_str.to_lowercase().as_str() {
            // Letters
            "a" => Some(Key::Unicode('a')),
            "b" => Some(Key::Unicode('b')),
            "c" => Some(Key::Unicode('c')),
            "d" => Some(Key::Unicode('d')),
            "e" => Some(Key::Unicode('e')),
            "f" => Some(Key::Unicode('f')),
            "g" => Some(Key::Unicode('g')),
            "h" => Some(Key::Unicode('h')),
            "i" => Some(Key::Unicode('i')),
            "j" => Some(Key::Unicode('j')),
            "k" => Some(Key::Unicode('k')),
            "l" => Some(Key::Unicode('l')),
            "m" => Some(Key::Unicode('m')),
            "n" => Some(Key::Unicode('n')),
            "o" => Some(Key::Unicode('o')),
            "p" => Some(Key::Unicode('p')),
            "q" => Some(Key::Unicode('q')),
            "r" => Some(Key::Unicode('r')),
            "s" => Some(Key::Unicode('s')),
            "t" => Some(Key::Unicode('t')),
            "u" => Some(Key::Unicode('u')),
            "v" => Some(Key::Unicode('v')),
            "w" => Some(Key::Unicode('w')),
            "x" => Some(Key::Unicode('x')),
            "y" => Some(Key::Unicode('y')),
            "z" => Some(Key::Unicode('z')),

            // Special keys
            "enter" | "return" => Some(Key::Return),
            "tab" => Some(Key::Tab),
            "space" => Some(Key::Space),
            "backspace" => Some(Key::Backspace),
            "delete" => Some(Key::Delete),
            "escape" | "esc" => Some(Key::Escape),

            // Modifiers
            "control" | "ctrl" => Some(Key::Control),
            "alt" => Some(Key::Alt),
            "shift" => Some(Key::Shift),
            "meta" | "super" | "windows" | "command" => Some(Key::Meta),

            // Arrow keys
            "up" => Some(Key::UpArrow),
            "down" => Some(Key::DownArrow),
            "left" => Some(Key::LeftArrow),
            "right" => Some(Key::RightArrow),

            // Function keys
            "f1" => Some(Key::F1),
            "f2" => Some(Key::F2),
            "f3" => Some(Key::F3),
            "f4" => Some(Key::F4),
            "f5" => Some(Key::F5),
            "f6" => Some(Key::F6),
            "f7" => Some(Key::F7),
            "f8" => Some(Key::F8),
            "f9" => Some(Key::F9),
            "f10" => Some(Key::F10),
            "f11" => Some(Key::F11),
            "f12" => Some(Key::F12),

            // Other
            "home" => Some(Key::Home),
            "end" => Some(Key::End),
            "pageup" => Some(Key::PageUp),
            "pagedown" => Some(Key::PageDown),

            // If single unicode character
            s if s.chars().count() == 1 => Some(Key::Unicode(s.chars().next().unwrap())),

            _ => None,
        }
    }
}

/// Input event listener (for capturing local input)
pub struct InputListener {
    // This would use rdev to listen to global input events
    // Useful for the host side to capture and send events
}

impl InputListener {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    /// Start listening to input events
    pub async fn start<F>(&self, _callback: F) -> Result<()>
    where
        F: Fn(InputEvent) + Send + 'static,
    {
        // In a real implementation, we'd use rdev::listen
        // For now, this is a placeholder
        tokio::spawn(async move {
            // rdev::listen implementation would go here
            // This would capture all mouse and keyboard events
        });

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    Mouse(MouseEvent),
    Keyboard(KeyboardEvent),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_controller_creation() {
        let result = InputController::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_key_parsing() {
        assert!(InputController::parse_key("a").is_some());
        assert!(InputController::parse_key("enter").is_some());
        assert!(InputController::parse_key("ctrl").is_some());
        assert!(InputController::parse_key("f1").is_some());
    }
}
