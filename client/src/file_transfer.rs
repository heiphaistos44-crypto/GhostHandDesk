use crate::error::{GhostHandError, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{info, warn};

const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100 MB
const CHUNK_SIZE: usize = 48 * 1024; // 48 KB par chunk

/// État d'un transfert en cours
pub struct FileTransferState {
    pub name: String,
    pub size: u64,
    pub received: u64,
    pub data: Vec<u8>,
}

/// Gestionnaire de transferts de fichiers
pub struct FileTransferManager {
    transfers: HashMap<String, FileTransferState>,
    download_dir: PathBuf,
}

impl Default for FileTransferManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FileTransferManager {
    pub fn new() -> Self {
        let download_dir = dirs_next::download_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("GhostHandDesk");

        // Créer le dossier si nécessaire
        let _ = std::fs::create_dir_all(&download_dir);

        Self {
            transfers: HashMap::new(),
            download_dir,
        }
    }

    /// Démarrer un nouveau transfert (côté réception)
    pub fn start_receive(&mut self, id: String, name: String, size: u64) -> Result<()> {
        if size > MAX_FILE_SIZE {
            return Err(GhostHandError::Internal(format!(
                "Fichier trop gros: {} bytes (max: {} bytes)", size, MAX_FILE_SIZE
            )));
        }

        info!("Début réception fichier: {} ({} bytes)", name, size);
        self.transfers.insert(id, FileTransferState {
            name,
            size,
            received: 0,
            data: Vec::with_capacity(size as usize),
        });
        Ok(())
    }

    /// Recevoir un chunk de données
    pub fn receive_chunk(&mut self, id: &str, data: &[u8], offset: u64) -> Result<f64> {
        let state = self.transfers.get_mut(id).ok_or_else(|| {
            GhostHandError::Internal(format!("Transfert {} non trouvé", id))
        })?;

        // Vérifier l'offset
        if offset != state.received {
            warn!("Offset inattendu: attendu {}, reçu {}", state.received, offset);
        }

        state.data.extend_from_slice(data);
        state.received += data.len() as u64;

        // Retourner la progression (0.0 → 1.0)
        Ok(state.received as f64 / state.size as f64)
    }

    /// Finaliser un transfert
    pub fn complete(&mut self, id: &str) -> Result<PathBuf> {
        let state = self.transfers.remove(id).ok_or_else(|| {
            GhostHandError::Internal(format!("Transfert {} non trouvé", id))
        })?;

        let file_path = self.download_dir.join(&state.name);
        std::fs::write(&file_path, &state.data).map_err(|e| {
            GhostHandError::Internal(format!("Erreur écriture fichier: {}", e))
        })?;

        info!("Fichier reçu: {} ({} bytes)", file_path.display(), state.data.len());
        Ok(file_path)
    }

    /// Préparer un fichier pour l'envoi (retourne les chunks)
    pub fn prepare_send(path: &std::path::Path) -> Result<(String, String, u64, Vec<Vec<u8>>)> {
        let data = std::fs::read(path).map_err(|e| {
            GhostHandError::Internal(format!("Erreur lecture fichier: {}", e))
        })?;

        let size = data.len() as u64;
        if size > MAX_FILE_SIZE {
            return Err(GhostHandError::Internal(format!(
                "Fichier trop gros: {} bytes (max: {} bytes)", size, MAX_FILE_SIZE
            )));
        }

        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let id = format!("ft-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis());

        let chunks: Vec<Vec<u8>> = data.chunks(CHUNK_SIZE)
            .map(|c| c.to_vec())
            .collect();

        Ok((id, name, size, chunks))
    }
}
