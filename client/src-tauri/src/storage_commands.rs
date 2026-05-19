use ghost_hand_client::storage::{global_storage, ConnectionHistory, KnownPeer, StorageStats};

#[tauri::command]
pub fn get_connection_history(limit: Option<usize>) -> Result<Vec<ConnectionHistory>, String> {
    let storage_mutex = global_storage().ok_or("Storage non initialisé")?;
    let storage = storage_mutex.lock().map_err(|_| "Impossible de verrouiller le storage")?;
    Ok(storage.get_connection_history(limit).into_iter().cloned().collect())
}

#[tauri::command]
pub fn get_known_peers() -> Result<Vec<KnownPeer>, String> {
    let storage_mutex = global_storage().ok_or("Storage non initialisé")?;
    let storage = storage_mutex.lock().map_err(|_| "Impossible de verrouiller le storage")?;
    Ok(storage.get_all_known_peers().into_iter().cloned().collect())
}

#[tauri::command]
pub fn get_favorite_peers() -> Result<Vec<KnownPeer>, String> {
    let storage_mutex = global_storage().ok_or("Storage non initialisé")?;
    let storage = storage_mutex.lock().map_err(|_| "Impossible de verrouiller le storage")?;
    Ok(storage.get_favorite_peers().into_iter().cloned().collect())
}

#[tauri::command]
pub fn set_peer_favorite(peer_id: String, favorite: bool) -> Result<(), String> {
    let storage_mutex = global_storage().ok_or("Storage non initialisé")?;
    let mut storage = storage_mutex.lock().map_err(|_| "Impossible de verrouiller le storage")?;
    if storage.set_peer_favorite(&peer_id, favorite) {
        storage.save().map_err(|e| format!("Erreur sauvegarde: {}", e))
    } else {
        Err(format!("Pair {} introuvable", peer_id))
    }
}

#[tauri::command]
pub async fn get_storage_stats() -> Result<StorageStats, String> {
    let storage_mutex = global_storage().ok_or("Storage non initialisé")?;
    let storage = storage_mutex.lock().map_err(|_| "Impossible de verrouiller le storage")?;
    Ok(storage.get_stats())
}
