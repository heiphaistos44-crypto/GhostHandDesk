use ghost_hand_client::storage::{global_storage, ConnectionHistory};

/// Obtenir l'historique des connexions
#[tauri::command]
pub fn get_connection_history(limit: Option<usize>) -> Result<Vec<ConnectionHistory>, String> {
    if let Some(storage_mutex) = global_storage() {
        if let Ok(storage) = storage_mutex.lock() {
            let history = storage.get_connection_history(limit);
            Ok(history.into_iter().cloned().collect())
        } else {
            Err("Impossible de verrouiller le storage".to_string())
        }
    } else {
        Err("Storage non initialisé".to_string())
    }
}

/// Obtenir les pairs connus
#[tauri::command]
pub fn get_known_peers() -> Result<Vec<ghost_hand_client::storage::KnownPeer>, String> {
    if let Some(storage_mutex) = global_storage() {
        if let Ok(storage) = storage_mutex.lock() {
            let peers = storage.get_all_known_peers();
            Ok(peers.into_iter().cloned().collect())
        } else {
            Err("Impossible de verrouiller le storage".to_string())
        }
    } else {
        Err("Storage non initialisé".to_string())
    }
}

/// Obtenir les pairs favoris
#[tauri::command]
pub fn get_favorite_peers() -> Result<Vec<ghost_hand_client::storage::KnownPeer>, String> {
    if let Some(storage_mutex) = global_storage() {
        if let Ok(storage) = storage_mutex.lock() {
            let peers = storage.get_favorite_peers();
            Ok(peers.into_iter().cloned().collect())
        } else {
            Err("Impossible de verrouiller le storage".to_string())
        }
    } else {
        Err("Storage non initialisé".to_string())
    }
}

/// Marquer un pair comme favori
#[tauri::command]
pub fn set_peer_favorite(peer_id: String, favorite: bool) -> Result<(), String> {
    if let Some(storage_mutex) = global_storage() {
        if let Ok(mut storage) = storage_mutex.lock() {
            if storage.set_peer_favorite(&peer_id, favorite) {
                storage.save().map_err(|e| format!("Erreur sauvegarde: {}", e))?;
                Ok(())
            } else {
                Err(format!("Pair {} introuvable", peer_id))
            }
        } else {
            Err("Impossible de verrouiller le storage".to_string())
        }
    } else {
        Err("Storage non initialisé".to_string())
    }
}

/// Obtenir les statistiques du storage
#[tauri::command]
pub async fn get_storage_stats() -> Result<ghost_hand_client::storage::StorageStats, String> {
    if let Some(storage_mutex) = global_storage() {
        if let Ok(storage) = storage_mutex.lock() {
            Ok(storage.get_stats())
        } else {
            Err("Impossible de verrouiller le storage".to_string())
        }
    } else {
        Err("Storage non initialisé".to_string())
    }
}
