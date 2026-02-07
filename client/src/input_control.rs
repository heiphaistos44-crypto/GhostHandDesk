use crate::error::{GhostHandError, Result};
use crate::audit::{audit_log, AuditEvent, AuditLevel};
use enigo::{
    Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

// WHITELIST SÉCURITÉ: Touches système bloquées pour prévenir les actions dangereuses
// Ces touches sont bloquées indépendamment des modifiers
const BLOCKED_KEYS: &[&str] = &[
    // Touches Windows système
    "meta",       // Touche Windows seule
    "super",      // Alias pour Meta
    "windows",    // Alias pour Meta
    "command",    // Alias pour Meta (macOS)
];

// Combinaisons de touches bloquées (modifier + touche)
const BLOCKED_COMBINATIONS: &[(&str, &str)] = &[
    // Windows dangereuses
    ("meta", "r"),       // Win+R (Exécuter)
    ("meta", "x"),       // Win+X (Menu système)
    ("meta", "l"),       // Win+L (Verrouiller)
    ("meta", "d"),       // Win+D (Bureau) - Désactivé pour éviter distraction
    ("ctrl", "alt"),     // Ctrl+Alt (préfixe pour Ctrl+Alt+Del)

    // Shortcuts système critiques
    ("alt", "f4"),       // Alt+F4 (Fermer application)
    ("ctrl", "shift"),   // Ctrl+Shift+Esc combo (si suivi de Esc)
];

/// Input control manager for keyboard and mouse events
pub struct InputController {
    enigo: Enigo,
    screen_width: i32,
    screen_height: i32,
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

/// Keyboard modifiers (Ctrl, Shift, Alt, Meta)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

impl InputController {
    /// Vérifier si une touche est bloquée par la whitelist de sécurité
    pub fn is_key_blocked(key: &str, modifiers: &KeyModifiers) -> bool {
        let key_lower = key.to_lowercase();

        // Bloquer les touches système individuelles
        if BLOCKED_KEYS.contains(&key_lower.as_str()) {
            return true;
        }

        // Bloquer les combinaisons dangereuses
        for (modifier, blocked_key) in BLOCKED_COMBINATIONS {
            let key_matches = key_lower == *blocked_key;
            let modifier_active = match *modifier {
                "meta" => modifiers.meta,
                "ctrl" => modifiers.ctrl,
                "alt" => modifiers.alt,
                "shift" => modifiers.shift,
                _ => false,
            };

            if key_matches && modifier_active {
                return true;
            }
        }

        // Bloquer Ctrl+Alt+Del spécifiquement
        if modifiers.ctrl && modifiers.alt && (key_lower == "delete" || key_lower == "del") {
            return true;
        }

        false
    }

    /// Create a new input controller with default resolution
    pub fn new() -> Result<Self> {
        Self::new_with_resolution(3840, 2160)
    }

    /// Create a new input controller with specific screen resolution
    pub fn new_with_resolution(screen_width: i32, screen_height: i32) -> Result<Self> {
        let enigo = Enigo::new(&Settings::default()).map_err(|e| {
            GhostHandError::InputControl(format!("Failed to initialize input control: {}", e))
        })?;

        Ok(Self { enigo, screen_width, screen_height })
    }

    /// Update the screen resolution dynamically
    pub fn set_resolution(&mut self, width: i32, height: i32) {
        self.screen_width = width;
        self.screen_height = height;
        debug!("InputController resolution updated: {}x{}", width, height);
    }

    /// Handle a mouse event
    pub fn handle_mouse_event(&mut self, event: MouseEvent) -> Result<()> {
        match event {
            MouseEvent::Move { x, y } => {
                // Normaliser les coordonnées pour éviter les débordements
                let (max_width, max_height) = (self.screen_width, self.screen_height);
                let clamped_x = x.max(0).min(max_width - 1);
                let clamped_y = y.max(0).min(max_height - 1);

                if clamped_x != x || clamped_y != y {
                    debug!("Coordonnées clampées: ({}, {}) → ({}, {})", x, y, clamped_x, clamped_y);
                }

                debug!("Mouse move to ({}, {})", clamped_x, clamped_y);
                self.enigo
                    .move_mouse(clamped_x, clamped_y, Coordinate::Abs)
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

    /// Handle a keyboard event with optional modifiers
    pub fn handle_keyboard_event(&mut self, event: KeyboardEvent, modifiers: KeyModifiers) -> Result<()> {
        // SÉCURITÉ: Vérifier la whitelist avant d'exécuter
        if let KeyboardEvent::Press { ref key } | KeyboardEvent::Release { ref key } = event {
            if Self::is_key_blocked(key, &modifiers) {
                warn!("⚠️  SÉCURITÉ: Touche bloquée par whitelist: {} (modifiers: {:?})", key, modifiers);

                // AUDIT: Logger la tentative bloquée
                audit_log(
                    AuditLevel::Security,
                    AuditEvent::SecurityError {
                        error_code: "BLOCKED_KEY".to_string(),
                        description: format!(
                            "Tentative d'utilisation d'une touche système bloquée: {} (ctrl={}, alt={}, shift={}, meta={})",
                            key, modifiers.ctrl, modifiers.alt, modifiers.shift, modifiers.meta
                        ),
                        peer_id: None,
                    },
                );

                return Err(GhostHandError::InputControl(
                    format!("Touche système bloquée pour sécurité: {}", key)
                ));
            }
        }

        // Appliquer les modifiers AVANT la touche principale
        if modifiers.ctrl {
            self.enigo.key(Key::Control, Direction::Press).map_err(|e| {
                GhostHandError::InputControl(format!("Failed to press Ctrl: {}", e))
            })?;
        }
        if modifiers.shift {
            self.enigo.key(Key::Shift, Direction::Press).map_err(|e| {
                GhostHandError::InputControl(format!("Failed to press Shift: {}", e))
            })?;
        }
        if modifiers.alt {
            self.enigo.key(Key::Alt, Direction::Press).map_err(|e| {
                GhostHandError::InputControl(format!("Failed to press Alt: {}", e))
            })?;
        }
        if modifiers.meta {
            self.enigo.key(Key::Meta, Direction::Press).map_err(|e| {
                GhostHandError::InputControl(format!("Failed to press Meta: {}", e))
            })?;
        }

        // Exécuter la touche principale
        match event {
            KeyboardEvent::Press { key } => {
                debug!("Key press: {} (modifiers: {:?})", key, modifiers);
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

        // Relâcher les modifiers APRÈS la touche principale (dans l'ordre inverse)
        if modifiers.meta {
            self.enigo.key(Key::Meta, Direction::Release).map_err(|e| {
                GhostHandError::InputControl(format!("Failed to release Meta: {}", e))
            })?;
        }
        if modifiers.alt {
            self.enigo.key(Key::Alt, Direction::Release).map_err(|e| {
                GhostHandError::InputControl(format!("Failed to release Alt: {}", e))
            })?;
        }
        if modifiers.shift {
            self.enigo.key(Key::Shift, Direction::Release).map_err(|e| {
                GhostHandError::InputControl(format!("Failed to release Shift: {}", e))
            })?;
        }
        if modifiers.ctrl {
            self.enigo.key(Key::Control, Direction::Release).map_err(|e| {
                GhostHandError::InputControl(format!("Failed to release Ctrl: {}", e))
            })?;
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

    #[test]
    fn test_blocked_keys() {
        let modifiers = KeyModifiers::default();

        // Touche Windows seule doit être bloquée
        assert!(InputController::is_key_blocked("meta", &modifiers));
        assert!(InputController::is_key_blocked("super", &modifiers));
        assert!(InputController::is_key_blocked("windows", &modifiers));

        // Touches normales ne doivent pas être bloquées
        assert!(!InputController::is_key_blocked("a", &modifiers));
        assert!(!InputController::is_key_blocked("enter", &modifiers));
    }

    #[test]
    fn test_blocked_combinations() {
        // Win+R doit être bloqué
        let mut modifiers = KeyModifiers::default();
        modifiers.meta = true;
        assert!(InputController::is_key_blocked("r", &modifiers));

        // Win+L doit être bloqué
        assert!(InputController::is_key_blocked("l", &modifiers));

        // Ctrl+Alt+Delete doit être bloqué
        let mut modifiers2 = KeyModifiers::default();
        modifiers2.ctrl = true;
        modifiers2.alt = true;
        assert!(InputController::is_key_blocked("delete", &modifiers2));

        // Alt+F4 doit être bloqué
        let mut modifiers3 = KeyModifiers::default();
        modifiers3.alt = true;
        assert!(InputController::is_key_blocked("f4", &modifiers3));
    }

    #[test]
    fn test_safe_combinations() {
        // Ctrl+C doit être autorisé
        let mut modifiers = KeyModifiers::default();
        modifiers.ctrl = true;
        assert!(!InputController::is_key_blocked("c", &modifiers));

        // Ctrl+V doit être autorisé
        assert!(!InputController::is_key_blocked("v", &modifiers));

        // Alt+Tab devrait être autorisé (pas dans la liste bloquée)
        let mut modifiers2 = KeyModifiers::default();
        modifiers2.alt = true;
        assert!(!InputController::is_key_blocked("tab", &modifiers2));
    }
}
