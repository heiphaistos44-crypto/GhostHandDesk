use crate::error::{GhostHandError, Result};
use std::sync::{Arc, Mutex};
use tracing::{debug, warn};

/// Gestionnaire de presse-papiers bidirectionnel
pub struct ClipboardManager {
    last_content: Arc<Mutex<String>>,
}

impl Default for ClipboardManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardManager {
    pub fn new() -> Self {
        Self {
            last_content: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Lire le contenu actuel du presse-papiers
    pub fn get_clipboard(&self) -> Result<String> {
        let mut ctx = arboard::Clipboard::new().map_err(|e| {
            GhostHandError::Internal(format!("Impossible d'accéder au presse-papiers: {}", e))
        })?;
        ctx.get_text().map_err(|e| {
            GhostHandError::Internal(format!("Erreur lecture presse-papiers: {}", e))
        })
    }

    /// Écrire dans le presse-papiers local
    pub fn set_clipboard(&self, content: &str) -> Result<()> {
        let mut ctx = arboard::Clipboard::new().map_err(|e| {
            GhostHandError::Internal(format!("Impossible d'accéder au presse-papiers: {}", e))
        })?;
        ctx.set_text(content).map_err(|e| {
            GhostHandError::Internal(format!("Erreur écriture presse-papiers: {}", e))
        })?;

        // Mettre à jour le cache pour éviter les boucles
        if let Ok(mut last) = self.last_content.lock() {
            *last = content.to_string();
        }

        debug!("Presse-papiers mis à jour ({} chars)", content.len());
        Ok(())
    }

    /// Vérifier si le presse-papiers a changé depuis la dernière vérification
    pub fn has_changed(&self) -> Option<String> {
        match self.get_clipboard() {
            Ok(current) => {
                if let Ok(mut last) = self.last_content.lock() {
                    if current != *last && !current.is_empty() {
                        *last = current.clone();
                        return Some(current);
                    }
                }
                None
            }
            Err(e) => {
                warn!("Erreur vérification presse-papiers: {}", e);
                None
            }
        }
    }
}
