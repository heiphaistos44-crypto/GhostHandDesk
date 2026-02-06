//! Module de persistance des données utilisateur
//!
//! Ce module gère la sauvegarde et le chargement des données utilisateur
//! telles que l'historique des connexions, les pairs favoris, et les préférences.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

/// Historique d'une connexion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionHistory {
    /// ID unique de la connexion
    pub id: String,
    /// ID du pair
    pub peer_id: String,
    /// Timestamp de début (Unix milliseconds)
    pub timestamp: u64,
    /// Durée en secondes (None si en cours)
    pub duration_secs: Option<u64>,
    /// Direction : "outgoing" ou "incoming"
    pub direction: String,
    /// Succès ou échec
    pub success: bool,
    /// Raison de la déconnexion (si applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disconnect_reason: Option<String>,
}

/// Information sur un pair connu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownPeer {
    /// ID du pair
    pub peer_id: String,
    /// Nom d'affichage personnalisé
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Dernière fois vu (Unix milliseconds)
    pub last_seen: u64,
    /// Marqué comme favori
    #[serde(default)]
    pub favorite: bool,
    /// Nombre de connexions réussies
    #[serde(default)]
    pub connection_count: u32,
    /// Notes personnelles
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Structure principale de stockage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageData {
    /// Version du schéma de données (pour migrations futures)
    #[serde(default = "default_version")]
    pub version: u32,

    /// Historique des connexions (max 1000 entrées)
    #[serde(default)]
    pub connections_history: Vec<ConnectionHistory>,

    /// Pairs connus
    #[serde(default)]
    pub known_peers: HashMap<String, KnownPeer>,

    /// Préférences utilisateur (clé-valeur)
    #[serde(default)]
    pub user_preferences: HashMap<String, String>,

    /// Timestamp de dernière sauvegarde
    #[serde(default)]
    pub last_saved: u64,
}

fn default_version() -> u32 {
    1
}

/// Gestionnaire de persistance
pub struct Storage {
    /// Chemin du fichier de stockage
    storage_path: PathBuf,

    /// Chemin du fichier de backup
    backup_path: PathBuf,

    /// Données en mémoire
    data: StorageData,

    /// Maximum d'entrées d'historique à conserver
    max_history_entries: usize,
}

impl Storage {
    /// Créer un nouveau gestionnaire de stockage
    ///
    /// # Arguments
    /// * `storage_dir` - Dossier où stocker les données
    /// * `max_history_entries` - Nombre max d'entrées d'historique (défaut: 1000)
    pub fn new(storage_dir: impl AsRef<Path>, max_history_entries: Option<usize>) -> io::Result<Self> {
        let storage_dir = storage_dir.as_ref();

        // Créer le dossier s'il n'existe pas
        fs::create_dir_all(storage_dir)?;

        let storage_path = storage_dir.join("storage.json");
        let backup_path = storage_dir.join("storage.backup.json");

        // Charger les données existantes ou créer nouvelles
        let data = Self::load_from_file(&storage_path, &backup_path)?;

        Ok(Self {
            storage_path,
            backup_path,
            data,
            max_history_entries: max_history_entries.unwrap_or(1000),
        })
    }

    /// Charger les données depuis le fichier
    fn load_from_file(storage_path: &Path, backup_path: &Path) -> io::Result<StorageData> {
        // Essayer de charger depuis le fichier principal
        if storage_path.exists() {
            match Self::read_json_file(storage_path) {
                Ok(data) => {
                    info!("Storage chargé depuis: {}", storage_path.display());
                    return Ok(data);
                }
                Err(e) => {
                    error!("Erreur lecture storage principal: {}", e);
                    warn!("Tentative de restauration depuis le backup...");
                }
            }
        }

        // Essayer le backup si le principal échoue
        if backup_path.exists() {
            match Self::read_json_file(backup_path) {
                Ok(data) => {
                    info!("Storage restauré depuis backup: {}", backup_path.display());
                    return Ok(data);
                }
                Err(e) => {
                    error!("Erreur lecture backup: {}", e);
                }
            }
        }

        // Si tout échoue, créer des données vides
        info!("Création d'un nouveau storage");
        Ok(StorageData::default())
    }

    /// Lire un fichier JSON
    fn read_json_file(path: &Path) -> io::Result<StorageData> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("Erreur parsing JSON: {}", e))
        })
    }

    /// Sauvegarder les données sur disque
    pub fn save(&mut self) -> io::Result<()> {
        // Mettre à jour le timestamp
        self.data.last_saved = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Créer un backup avant d'écraser
        if self.storage_path.exists() {
            if let Err(e) = fs::copy(&self.storage_path, &self.backup_path) {
                warn!("Impossible de créer backup: {}", e);
            }
        }

        // Écrire le nouveau fichier
        let file = File::create(&self.storage_path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &self.data).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Erreur sérialisation JSON: {}", e))
        })?;

        info!("Storage sauvegardé: {}", self.storage_path.display());
        Ok(())
    }

    /// Ajouter une entrée à l'historique des connexions
    pub fn add_connection_history(&mut self, entry: ConnectionHistory) {
        self.data.connections_history.push(entry);

        // Limiter la taille de l'historique
        if self.data.connections_history.len() > self.max_history_entries {
            // Garder seulement les plus récentes
            self.data.connections_history.sort_by_key(|e| std::cmp::Reverse(e.timestamp));
            self.data.connections_history.truncate(self.max_history_entries);
        }
    }

    /// Mettre à jour une entrée d'historique (par exemple, ajouter la durée)
    pub fn update_connection_history<F>(&mut self, id: &str, update_fn: F) -> bool
    where
        F: FnOnce(&mut ConnectionHistory),
    {
        if let Some(entry) = self.data.connections_history.iter_mut().find(|e| e.id == id) {
            update_fn(entry);
            true
        } else {
            false
        }
    }

    /// Obtenir l'historique des connexions (les plus récentes en premier)
    pub fn get_connection_history(&self, limit: Option<usize>) -> Vec<&ConnectionHistory> {
        let mut history: Vec<_> = self.data.connections_history.iter().collect();
        history.sort_by_key(|e| std::cmp::Reverse(e.timestamp));

        if let Some(limit) = limit {
            history.truncate(limit);
        }

        history
    }

    /// Ajouter ou mettre à jour un pair connu
    pub fn upsert_known_peer(&mut self, peer: KnownPeer) {
        self.data.known_peers.insert(peer.peer_id.clone(), peer);
    }

    /// Obtenir un pair connu
    pub fn get_known_peer(&self, peer_id: &str) -> Option<&KnownPeer> {
        self.data.known_peers.get(peer_id)
    }

    /// Obtenir tous les pairs favoris
    pub fn get_favorite_peers(&self) -> Vec<&KnownPeer> {
        self.data.known_peers.values()
            .filter(|p| p.favorite)
            .collect()
    }

    /// Obtenir tous les pairs connus (triés par dernière visite)
    pub fn get_all_known_peers(&self) -> Vec<&KnownPeer> {
        let mut peers: Vec<_> = self.data.known_peers.values().collect();
        peers.sort_by_key(|p| std::cmp::Reverse(p.last_seen));
        peers
    }

    /// Marquer un pair comme favori
    pub fn set_peer_favorite(&mut self, peer_id: &str, favorite: bool) -> bool {
        if let Some(peer) = self.data.known_peers.get_mut(peer_id) {
            peer.favorite = favorite;
            true
        } else {
            false
        }
    }

    /// Définir une préférence utilisateur
    pub fn set_preference(&mut self, key: String, value: String) {
        self.data.user_preferences.insert(key, value);
    }

    /// Obtenir une préférence utilisateur
    pub fn get_preference(&self, key: &str) -> Option<&String> {
        self.data.user_preferences.get(key)
    }

    /// Supprimer une préférence utilisateur
    pub fn remove_preference(&mut self, key: &str) -> Option<String> {
        self.data.user_preferences.remove(key)
    }

    /// Nettoyer l'historique (supprimer les entrées plus anciennes que N jours)
    pub fn cleanup_history(&mut self, days_to_keep: u64) {
        let cutoff_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() - (days_to_keep * 24 * 60 * 60);
        let cutoff_ms = cutoff_timestamp * 1000;

        let before = self.data.connections_history.len();
        self.data.connections_history.retain(|e| e.timestamp >= cutoff_ms);
        let after = self.data.connections_history.len();

        if before > after {
            info!("Nettoyage historique: {} entrées supprimées", before - after);
        }
    }

    /// Obtenir des statistiques sur les données stockées
    pub fn get_stats(&self) -> StorageStats {
        StorageStats {
            total_connections: self.data.connections_history.len(),
            known_peers: self.data.known_peers.len(),
            favorite_peers: self.data.known_peers.values().filter(|p| p.favorite).count(),
            preferences: self.data.user_preferences.len(),
            last_saved: self.data.last_saved,
        }
    }
}

/// Statistiques sur le storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_connections: usize,
    pub known_peers: usize,
    pub favorite_peers: usize,
    pub preferences: usize,
    pub last_saved: u64,
}

/// Gestionnaire de storage global (singleton)
static GLOBAL_STORAGE: std::sync::OnceLock<std::sync::Mutex<Storage>> = std::sync::OnceLock::new();

/// Initialiser le storage global
pub fn init_global_storage(storage_dir: impl AsRef<Path>) -> io::Result<()> {
    let storage = Storage::new(storage_dir, None)?;
    GLOBAL_STORAGE.set(std::sync::Mutex::new(storage)).map_err(|_| {
        io::Error::new(io::ErrorKind::AlreadyExists, "Global storage already initialized")
    })?;
    Ok(())
}

/// Obtenir une référence au storage global
pub fn global_storage() -> Option<&'static std::sync::Mutex<Storage>> {
    GLOBAL_STORAGE.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_creation() {
        let temp_dir = std::env::temp_dir().join("ghosthand_storage_test");
        let _ = fs::remove_dir_all(&temp_dir);

        let storage = Storage::new(&temp_dir, None).unwrap();
        assert_eq!(storage.data.connections_history.len(), 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_connection_history() {
        let temp_dir = std::env::temp_dir().join("ghosthand_storage_test_history");
        let _ = fs::remove_dir_all(&temp_dir);

        let mut storage = Storage::new(&temp_dir, Some(10)).unwrap();

        // Ajouter des entrées
        for i in 0..15 {
            storage.add_connection_history(ConnectionHistory {
                id: format!("conn-{}", i),
                peer_id: format!("PEER-{}", i),
                timestamp: 1000 + i,
                duration_secs: Some(60),
                direction: "outgoing".to_string(),
                success: true,
                disconnect_reason: None,
            });
        }

        // Vérifier la limite
        assert_eq!(storage.data.connections_history.len(), 10);

        // Sauvegarder et recharger
        storage.save().unwrap();
        let storage2 = Storage::new(&temp_dir, None).unwrap();
        assert_eq!(storage2.data.connections_history.len(), 10);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_known_peers() {
        let temp_dir = std::env::temp_dir().join("ghosthand_storage_test_peers");
        let _ = fs::remove_dir_all(&temp_dir);

        let mut storage = Storage::new(&temp_dir, None).unwrap();

        // Ajouter un pair
        storage.upsert_known_peer(KnownPeer {
            peer_id: "PEER-123".to_string(),
            display_name: Some("Mon PC Bureau".to_string()),
            last_seen: 1000,
            favorite: false,
            connection_count: 1,
            notes: None,
        });

        // Récupérer
        let peer = storage.get_known_peer("PEER-123").unwrap();
        assert_eq!(peer.display_name.as_ref().unwrap(), "Mon PC Bureau");

        // Marquer comme favori
        storage.set_peer_favorite("PEER-123", true);
        assert_eq!(storage.get_favorite_peers().len(), 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
