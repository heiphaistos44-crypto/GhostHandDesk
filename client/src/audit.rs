//! Module d'audit trail pour traçabilité des actions
//!
//! Ce module enregistre toutes les actions sensibles pour audit de sécurité
//! et diagnostic. Les logs sont structurés en JSON pour faciliter le parsing.

use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::{error, info};

/// Niveau d'importance d'un événement d'audit
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuditLevel {
    Info,
    Warning,
    Error,
    Security, // Événements de sécurité critiques
}

/// Types d'événements audités
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum AuditEvent {
    /// Connexion établie avec un pair
    ConnectionEstablished {
        peer_id: String,
        direction: String, // "outgoing" ou "incoming"
        #[serde(skip_serializing_if = "Option::is_none")]
        password_used: Option<bool>,
    },

    /// Connexion terminée
    ConnectionClosed {
        peer_id: String,
        reason: String,
    },

    /// Demande de connexion reçue
    ConnectionRequestReceived {
        from: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        password_required: Option<bool>,
    },

    /// Demande de connexion acceptée
    ConnectionRequestAccepted {
        peer_id: String,
    },

    /// Demande de connexion rejetée
    ConnectionRequestRejected {
        peer_id: String,
        reason: String,
    },

    /// Événement de contrôle input (souris)
    MouseControl {
        peer_id: String,
        action: String, // "move", "click", "scroll"
        #[serde(skip_serializing_if = "Option::is_none")]
        position: Option<(i32, i32)>,
    },

    /// Événement de contrôle input (clavier)
    KeyboardControl {
        peer_id: String,
        action: String, // "press", "release"
        // Ne pas logger les touches pour la confidentialité
    },

    /// Erreur de sécurité détectée
    SecurityError {
        error_code: String,
        description: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        peer_id: Option<String>,
    },

    /// Tentative suspecte détectée
    SuspiciousActivity {
        description: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        peer_id: Option<String>,
    },

    /// Changement de configuration
    ConfigurationChange {
        setting: String,
        old_value: String,
        new_value: String,
    },

    /// Streaming démarré/arrêté
    StreamingStateChange {
        peer_id: String,
        state: String, // "started", "stopped"
    },
}

/// Entrée d'audit complète
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Timestamp Unix en millisecondes
    pub timestamp: u64,
    /// Niveau d'importance
    pub level: AuditLevel,
    /// Device ID local
    pub device_id: String,
    /// Événement audité
    #[serde(flatten)]
    pub event: AuditEvent,
    /// Informations supplémentaires optionnelles
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Logger d'audit avec rotation automatique
pub struct AuditLogger {
    file: Mutex<Option<File>>,
    log_path: PathBuf,
    device_id: String,
    max_size_bytes: u64,
}

impl AuditLogger {
    /// Créer un nouveau logger d'audit
    ///
    /// # Arguments
    /// * `log_dir` - Dossier où stocker les logs
    /// * `device_id` - ID du device local
    /// * `max_size_bytes` - Taille max d'un fichier de log (défaut: 10MB)
    pub fn new(log_dir: impl AsRef<Path>, device_id: String, max_size_bytes: Option<u64>) -> std::io::Result<Self> {
        let log_dir = log_dir.as_ref();

        // Créer le dossier s'il n'existe pas
        std::fs::create_dir_all(log_dir)?;

        let log_path = log_dir.join("audit.jsonl");

        // Ouvrir ou créer le fichier de log
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        Ok(Self {
            file: Mutex::new(Some(file)),
            log_path,
            device_id,
            max_size_bytes: max_size_bytes.unwrap_or(10 * 1024 * 1024), // 10 MB par défaut
        })
    }

    /// Logger un événement d'audit
    pub fn log(&self, level: AuditLevel, event: AuditEvent) {
        self.log_with_metadata(level, event, None);
    }

    /// Logger un événement d'audit avec metadata supplémentaires
    pub fn log_with_metadata(&self, level: AuditLevel, event: AuditEvent, metadata: Option<serde_json::Value>) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let entry = AuditEntry {
            timestamp,
            level: level.clone(),
            device_id: self.device_id.clone(),
            event,
            metadata,
        };

        // Sérialiser en JSON
        match serde_json::to_string(&entry) {
            Ok(json) => {
                // Logger aussi via tracing pour monitoring en temps réel
                match level {
                    AuditLevel::Info => info!("[AUDIT] {}", json),
                    AuditLevel::Warning => tracing::warn!("[AUDIT] {}", json),
                    AuditLevel::Error => tracing::error!("[AUDIT] {}", json),
                    AuditLevel::Security => tracing::error!("[AUDIT-SECURITY] {}", json),
                }

                // Écrire dans le fichier
                if let Err(e) = self.write_to_file(&json) {
                    error!("Erreur écriture audit log: {}", e);
                }
            }
            Err(e) => {
                error!("Erreur sérialisation audit entry: {}", e);
            }
        }
    }

    /// Écrire une ligne dans le fichier de log avec rotation automatique
    fn write_to_file(&self, json: &str) -> std::io::Result<()> {
        let mut file_guard = self.file.lock().unwrap();

        if let Some(file) = file_guard.as_mut() {
            // Vérifier la taille du fichier
            let metadata = file.metadata()?;
            if metadata.len() > self.max_size_bytes {
                // Rotation : renommer le fichier actuel et créer un nouveau
                drop(file_guard); // Libérer le lock
                self.rotate_log()?;
                file_guard = self.file.lock().unwrap();
            }

            if let Some(file) = file_guard.as_mut() {
                writeln!(file, "{}", json)?;
                file.flush()?;
            }
        }

        Ok(())
    }

    /// Rotation des logs : renommer l'ancien fichier et créer un nouveau
    fn rotate_log(&self) -> std::io::Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let archived_name = format!("audit_{}.jsonl", timestamp);
        let archived_path = self.log_path.parent().unwrap().join(archived_name);

        // Fermer le fichier actuel
        *self.file.lock().unwrap() = None;

        // Renommer
        std::fs::rename(&self.log_path, archived_path)?;

        // Créer un nouveau fichier
        let new_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        *self.file.lock().unwrap() = Some(new_file);

        info!("Audit log rotation effectuée");

        Ok(())
    }

    /// Obtenir le chemin du fichier de log actuel
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }
}

/// Logger global (singleton) pour faciliter l'usage
static GLOBAL_LOGGER: std::sync::OnceLock<AuditLogger> = std::sync::OnceLock::new();

/// Initialiser le logger global
pub fn init_global_logger(log_dir: impl AsRef<Path>, device_id: String) -> std::io::Result<()> {
    let logger = AuditLogger::new(log_dir, device_id, None)?;
    GLOBAL_LOGGER.set(logger).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::AlreadyExists, "Global logger already initialized")
    })?;
    Ok(())
}

/// Logger un événement via le logger global
pub fn audit_log(level: AuditLevel, event: AuditEvent) {
    if let Some(logger) = GLOBAL_LOGGER.get() {
        logger.log(level, event);
    } else {
        error!("Audit logger non initialisé");
    }
}

/// Logger un événement avec metadata via le logger global
pub fn audit_log_with_metadata(level: AuditLevel, event: AuditEvent, metadata: serde_json::Value) {
    if let Some(logger) = GLOBAL_LOGGER.get() {
        logger.log_with_metadata(level, event, Some(metadata));
    } else {
        error!("Audit logger non initialisé");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_audit_logger_creation() {
        let temp_dir = std::env::temp_dir().join("ghosthand_audit_test");
        let _ = fs::remove_dir_all(&temp_dir); // Nettoyer

        let logger = AuditLogger::new(&temp_dir, "TEST-123".to_string(), None).unwrap();

        assert!(logger.log_path().exists());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_audit_logging() {
        let temp_dir = std::env::temp_dir().join("ghosthand_audit_test_log");
        let _ = fs::remove_dir_all(&temp_dir);

        let logger = AuditLogger::new(&temp_dir, "TEST-456".to_string(), None).unwrap();

        logger.log(
            AuditLevel::Info,
            AuditEvent::ConnectionEstablished {
                peer_id: "PEER-789".to_string(),
                direction: "outgoing".to_string(),
                password_used: Some(false),
            },
        );

        // Vérifier que le fichier contient des données
        let content = fs::read_to_string(logger.log_path()).unwrap();
        assert!(content.contains("TEST-456"));
        assert!(content.contains("PEER-789"));
        assert!(content.contains("connection_established"));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
