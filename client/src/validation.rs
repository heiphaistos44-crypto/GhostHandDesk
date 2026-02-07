//! Module de validation stricte des entrées réseau
//!
//! Ce module fournit des fonctions de validation pour prévenir les attaques d'injection,
//! XSS, et autres vecteurs d'attaque via les entrées utilisateur et réseau.

use crate::error::{GhostHandError, Result};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::warn;

// Constantes de validation
const MAX_DEVICE_ID_LENGTH: usize = 64;
const MIN_DEVICE_ID_LENGTH: usize = 5;
const MAX_SDP_SIZE: usize = 100 * 1024; // 100 KB
const MAX_ICE_CANDIDATE_LENGTH: usize = 512;
const MAX_PASSWORD_LENGTH: usize = 128;

// Caractères autorisés dans un Device ID (alphanumeric + tiret)
const ALLOWED_DEVICE_ID_CHARS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-";

/// Valider un Device ID
pub fn validate_device_id(device_id: &str) -> Result<()> {
    // Vérifier longueur
    if device_id.len() < MIN_DEVICE_ID_LENGTH {
        return Err(GhostHandError::Validation(format!(
            "Device ID trop court: {} caractères (min: {})",
            device_id.len(), MIN_DEVICE_ID_LENGTH
        )));
    }

    if device_id.len() > MAX_DEVICE_ID_LENGTH {
        return Err(GhostHandError::Validation(format!(
            "Device ID trop long: {} caractères (max: {})",
            device_id.len(), MAX_DEVICE_ID_LENGTH
        )));
    }

    // Vérifier format (alphanumeric + tiret uniquement)
    if !device_id.chars().all(|c| ALLOWED_DEVICE_ID_CHARS.contains(c)) {
        return Err(GhostHandError::Validation(
            "Device ID contient des caractères invalides (seuls a-z, A-Z, 0-9, - sont autorisés)".to_string()
        ));
    }

    // Vérifier que ce n'est pas seulement des tirets
    if device_id.chars().all(|c| c == '-') {
        return Err(GhostHandError::Validation(
            "Device ID invalide: ne peut contenir que des tirets".to_string()
        ));
    }

    Ok(())
}

/// Valider un SDP (Session Description Protocol)
pub fn validate_sdp(sdp: &str) -> Result<()> {
    // Vérifier taille
    if sdp.is_empty() {
        return Err(GhostHandError::Validation("SDP vide".to_string()));
    }

    if sdp.len() > MAX_SDP_SIZE {
        return Err(GhostHandError::Validation(format!(
            "SDP trop grand: {} bytes (max: {} bytes)",
            sdp.len(), MAX_SDP_SIZE
        )));
    }

    // Vérifier format SDP de base (commence par v=)
    if !sdp.starts_with("v=0") && !sdp.starts_with("v=") {
        return Err(GhostHandError::Validation(
            "SDP invalide: doit commencer par 'v='".to_string()
        ));
    }

    // Vérifier présence de lignes obligatoires
    let has_origin = sdp.contains("o=");
    let has_session = sdp.contains("s=");
    let has_media = sdp.contains("m=");

    if !has_origin || !has_session || !has_media {
        return Err(GhostHandError::Validation(
            "SDP invalide: lignes obligatoires manquantes (o=, s=, m=)".to_string()
        ));
    }

    // Vérifier qu'il n'y a pas de caractères de contrôle dangereux (sauf \r\n)
    for ch in sdp.chars() {
        if ch.is_control() && ch != '\r' && ch != '\n' {
            return Err(GhostHandError::Validation(
                "SDP contient des caractères de contrôle invalides".to_string()
            ));
        }
    }

    Ok(())
}

/// Valider un ICE candidate
pub fn validate_ice_candidate(candidate: &str) -> Result<()> {
    // Vérifier taille
    if candidate.is_empty() {
        return Err(GhostHandError::Validation("ICE candidate vide".to_string()));
    }

    if candidate.len() > MAX_ICE_CANDIDATE_LENGTH {
        return Err(GhostHandError::Validation(format!(
            "ICE candidate trop long: {} caractères (max: {})",
            candidate.len(), MAX_ICE_CANDIDATE_LENGTH
        )));
    }

    // Vérifier format de base (doit contenir "candidate:")
    if !candidate.contains("candidate:") {
        return Err(GhostHandError::Validation(
            "ICE candidate invalide: doit contenir 'candidate:'".to_string()
        ));
    }

    // Vérifier qu'il n'y a pas de caractères dangereux
    for ch in candidate.chars() {
        if ch.is_control() && ch != ' ' {
            return Err(GhostHandError::Validation(
                "ICE candidate contient des caractères de contrôle invalides".to_string()
            ));
        }
    }

    Ok(())
}

/// Valider un password
pub fn validate_password(password: &str) -> Result<()> {
    if password.is_empty() {
        return Err(GhostHandError::Validation("Password vide".to_string()));
    }

    if password.len() > MAX_PASSWORD_LENGTH {
        return Err(GhostHandError::Validation(format!(
            "Password trop long: {} caractères (max: {})",
            password.len(), MAX_PASSWORD_LENGTH
        )));
    }

    // Pas de validation de complexité ici (c'est au choix de l'utilisateur)
    // Mais on vérifie qu'il n'y a pas de null bytes
    if password.contains('\0') {
        return Err(GhostHandError::Validation(
            "Password contient des caractères nuls".to_string()
        ));
    }

    Ok(())
}

/// Sanitize une chaîne pour logs (remplacer caractères sensibles)
pub fn sanitize_for_logging(input: &str, max_len: usize) -> String {
    let truncated = if input.len() > max_len {
        format!("{}... (tronqué)", &input[..max_len])
    } else {
        input.to_string()
    };

    // Remplacer les caractères de contrôle par des espaces
    truncated.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// Rate limiter simple pour les opérations réseau côté client
pub struct ClientRateLimiter {
    requests: Mutex<HashMap<String, VecDeque<Instant>>>,
    max_requests: usize,
    window: Duration,
}

use std::collections::VecDeque;

impl ClientRateLimiter {
    /// Créer un rate limiter
    ///
    /// # Arguments
    /// * `max_requests` - Nombre max de requêtes
    /// * `window` - Fenêtre de temps
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
            max_requests,
            window,
        }
    }

    /// Vérifier si une opération est autorisée
    ///
    /// # Arguments
    /// * `key` - Clé d'identification (ex: "connect_to_device", "send_message")
    ///
    /// # Returns
    /// `Ok(())` si autorisé, `Err` si rate limit atteint
    pub fn check(&self, key: &str) -> Result<()> {
        let mut requests = self.requests.lock().map_err(|e| {
            GhostHandError::Internal(format!("Rate limiter lock poisoned: {}", e))
        })?;

        let now = Instant::now();
        let cutoff = now - self.window;

        // Récupérer ou créer l'historique pour cette clé
        let history = requests.entry(key.to_string())
            .or_insert_with(VecDeque::new);

        // Nettoyer les anciennes entrées
        while history.front().map_or(false, |&time| time < cutoff) {
            history.pop_front();
        }

        // Vérifier la limite
        if history.len() >= self.max_requests {
            warn!("⚠️  Rate limit atteint pour '{}': {}/{} requêtes en {:?}",
                key, history.len(), self.max_requests, self.window);

            return Err(GhostHandError::RateLimit(format!(
                "Trop de requêtes '{}': max {} par {:?}",
                key, self.max_requests, self.window
            )));
        }

        // Ajouter la requête actuelle
        history.push_back(now);

        Ok(())
    }

    /// Réinitialiser le compteur pour une clé
    pub fn reset(&self, key: &str) -> Result<()> {
        let mut requests = self.requests.lock().map_err(|e| {
            GhostHandError::Internal(format!("Rate limiter lock poisoned: {}", e))
        })?;

        requests.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_device_id_valid() {
        assert!(validate_device_id("GHD-12345").is_ok());
        assert!(validate_device_id("test-device-001").is_ok());
        assert!(validate_device_id("ABCD1234").is_ok());
    }

    #[test]
    fn test_validate_device_id_invalid() {
        // Trop court
        assert!(validate_device_id("ABC").is_err());

        // Trop long
        let long_id = "A".repeat(100);
        assert!(validate_device_id(&long_id).is_err());

        // Caractères invalides
        assert!(validate_device_id("device@123").is_err());
        assert!(validate_device_id("device#123").is_err());
        assert!(validate_device_id("device 123").is_err());

        // Seulement tirets
        assert!(validate_device_id("-----").is_err());
    }

    #[test]
    fn test_validate_sdp_valid() {
        let valid_sdp = "v=0\no=- 123 456 IN IP4 127.0.0.1\ns=Test\nm=audio 1234 RTP/AVP 0\n";
        assert!(validate_sdp(valid_sdp).is_ok());
    }

    #[test]
    fn test_validate_sdp_invalid() {
        // SDP vide
        assert!(validate_sdp("").is_err());

        // Pas de v=
        assert!(validate_sdp("o=test\ns=test\nm=audio").is_err());

        // Lignes manquantes
        assert!(validate_sdp("v=0\no=test").is_err());

        // Trop grand
        let huge_sdp = "v=0\n".to_string() + &"a=test\n".repeat(100000);
        assert!(validate_sdp(&huge_sdp).is_err());
    }

    #[test]
    fn test_validate_ice_candidate_valid() {
        let valid_candidate = "candidate:1 1 UDP 2130706431 192.168.1.1 54321 typ host";
        assert!(validate_ice_candidate(valid_candidate).is_ok());
    }

    #[test]
    fn test_validate_ice_candidate_invalid() {
        // Vide
        assert!(validate_ice_candidate("").is_err());

        // Pas de "candidate:"
        assert!(validate_ice_candidate("invalid data").is_err());

        // Trop long
        let long_candidate = "candidate:".to_string() + &"A".repeat(600);
        assert!(validate_ice_candidate(&long_candidate).is_err());
    }

    #[test]
    fn test_validate_password() {
        assert!(validate_password("test123").is_ok());
        assert!(validate_password("ComplexP@ssw0rd!").is_ok());

        // Vide
        assert!(validate_password("").is_err());

        // Trop long
        let long_pass = "A".repeat(200);
        assert!(validate_password(&long_pass).is_err());

        // Null bytes
        assert!(validate_password("pass\0word").is_err());
    }

    #[test]
    fn test_sanitize_for_logging() {
        let result = sanitize_for_logging("normal text", 50);
        assert_eq!(result, "normal text");

        // Avec caractères de contrôle
        let result = sanitize_for_logging("text\nwith\tcontrol", 50);
        assert_eq!(result, "text with control");

        // Troncation
        let result = sanitize_for_logging("very long text here", 10);
        assert!(result.starts_with("very long "));
        assert!(result.contains("tronqué"));
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = ClientRateLimiter::new(3, Duration::from_secs(1));

        // Premières requêtes OK
        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_ok());

        // 4ème requête bloquée
        assert!(limiter.check("test").is_err());

        // Clé différente OK
        assert!(limiter.check("other").is_ok());
    }

    #[test]
    fn test_rate_limiter_reset() {
        let limiter = ClientRateLimiter::new(2, Duration::from_secs(10));

        // Atteindre la limite
        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_err());

        // Reset
        limiter.reset("test").unwrap();

        // Devrait fonctionner à nouveau
        assert!(limiter.check("test").is_ok());
    }
}
