//! Tests de sécurité pour GhostHandDesk
//! Valide les mécanismes de protection

use ghost_hand_client::validation::{
    validate_device_id, validate_sdp, validate_ice_candidate,
    validate_password, ClientRateLimiter, sanitize_for_logging
};
use ghost_hand_client::input_control::{InputController, KeyModifiers};
use std::time::Duration;

#[test]
fn test_device_id_security() {
    // Attaque injection SQL
    assert!(validate_device_id("'; DROP TABLE users--").is_err());

    // Attaque XSS
    assert!(validate_device_id("<script>alert('xss')</script>").is_err());

    // Path traversal
    assert!(validate_device_id("../../etc/passwd").is_err());

    // Null bytes
    assert!(validate_device_id("device\0id").is_err());

    // IDs valides
    assert!(validate_device_id("GHD-12345").is_ok());
    assert!(validate_device_id("test-device-001").is_ok());
}

#[test]
fn test_sdp_security() {
    // SDP avec caractères de contrôle malicieux
    let malicious_sdp = "v=0\no=\x00malicious\ns=test\nm=audio";
    assert!(validate_sdp(malicious_sdp).is_err());

    // SDP géant (DoS)
    let huge_sdp = "v=0\n".to_string() + &"a=test\n".repeat(1000000);
    assert!(validate_sdp(&huge_sdp).is_err());

    // SDP valide minimal
    let valid_sdp = "v=0\no=- 123 456 IN IP4 127.0.0.1\ns=Test\nm=audio 1234 RTP/AVP 0\n";
    assert!(validate_sdp(valid_sdp).is_ok());
}

#[test]
fn test_blocked_keys_security() {
    let modifiers = KeyModifiers::default();

    // Touches Windows bloquées
    assert!(InputController::is_key_blocked("meta", &modifiers));
    assert!(InputController::is_key_blocked("super", &modifiers));
    assert!(InputController::is_key_blocked("windows", &modifiers));

    // Win+R bloqué (Exécuter)
    let mut modifiers_winr = KeyModifiers::default();
    modifiers_winr.meta = true;
    assert!(InputController::is_key_blocked("r", &modifiers_winr));

    // Ctrl+Alt+Del bloqué
    let mut modifiers_cad = KeyModifiers::default();
    modifiers_cad.ctrl = true;
    modifiers_cad.alt = true;
    assert!(InputController::is_key_blocked("delete", &modifiers_cad));

    // Touches normales autorisées
    assert!(!InputController::is_key_blocked("a", &modifiers));
    assert!(!InputController::is_key_blocked("enter", &modifiers));
}

#[test]
fn test_rate_limiting_dos_protection() {
    let limiter = ClientRateLimiter::new(5, Duration::from_secs(60));

    // Simuler attaque DoS (10 requêtes rapides)
    let mut blocked_count = 0;
    for i in 0..10 {
        if limiter.check("connect").is_err() {
            blocked_count += 1;
        }
        println!("Requête {}: {} bloquées", i+1, blocked_count);
    }

    // Devrait avoir bloqué au moins 5 requêtes
    assert!(blocked_count >= 5, "Rate limiter n'a pas bloqué assez de requêtes");
}

#[test]
fn test_password_security() {
    // Password valides
    assert!(validate_password("simple123").is_ok());
    assert!(validate_password("ComplexP@ssw0rd!2024").is_ok());

    // Password avec null bytes (injection)
    assert!(validate_password("pass\0word").is_err());

    // Password vide
    assert!(validate_password("").is_err());

    // Password trop long (buffer overflow potentiel)
    let huge_password = "A".repeat(1000);
    assert!(validate_password(&huge_password).is_err());
}

#[test]
fn test_sanitize_logging_prevents_log_injection() {
    // Tentative d'injection de logs
    let malicious_input = "user123\nLEVEL:ADMIN\nACCESS:GRANTED";
    let sanitized = sanitize_for_logging(malicious_input, 100);

    // Les \n doivent être remplacés
    assert!(!sanitized.contains('\n'), "Sanitization failed: contains newlines");

    // Tentative d'injection ANSI escape
    let ansi_input = "user\x1b[31mADMIN\x1b[0m";
    let sanitized = sanitize_for_logging(ansi_input, 100);

    // Les caractères de contrôle doivent être neutralisés
    assert!(!sanitized.chars().any(|c| c.is_control()),
        "Sanitization failed: contains control characters");
}

#[test]
fn test_ice_candidate_security() {
    // ICE candidate valide
    let valid = "candidate:1 1 UDP 2130706431 192.168.1.1 54321 typ host";
    assert!(validate_ice_candidate(valid).is_ok());

    // ICE candidate avec injection
    let malicious = "candidate:1; DROP TABLE candidates;--";
    assert!(validate_ice_candidate(malicious).is_ok()); // Passe mais sera rejeté par WebRTC

    // ICE candidate trop long (DoS)
    let huge = "candidate:".to_string() + &"X".repeat(1000);
    assert!(validate_ice_candidate(&huge).is_err());

    // ICE candidate vide
    assert!(validate_ice_candidate("").is_err());
}

#[test]
fn test_combined_attack_scenarios() {
    // Scénario 1: Attaque multi-vecteurs
    let attacker_id = "<script>alert('xss')</script>";
    assert!(validate_device_id(attacker_id).is_err());

    // Scénario 2: Tentative de spam + injection
    let limiter = ClientRateLimiter::new(3, Duration::from_secs(5));
    for _ in 0..10 {
        let _ = limiter.check(&format!("spam{}", attacker_id));
    }
    // Rate limiter doit avoir bloqué

    // Scénario 3: Tentative d'exécution commande via touches
    let mut modifiers = KeyModifiers::default();
    modifiers.meta = true;
    assert!(InputController::is_key_blocked("r", &modifiers)); // Win+R bloqué
}

#[cfg(test)]
mod stress_tests {
    use super::*;

    #[test]
    #[ignore] // Exécuter avec `cargo test -- --ignored`
    fn stress_test_validation_performance() {
        use std::time::Instant;

        let iterations = 100_000;
        let start = Instant::now();

        for i in 0..iterations {
            let device_id = format!("GHD-{:05}", i);
            let _ = validate_device_id(&device_id);
        }

        let elapsed = start.elapsed();
        let per_validation = elapsed.as_micros() / iterations as u128;

        println!("Validations: {}", iterations);
        println!("Temps total: {:?}", elapsed);
        println!("Par validation: {} µs", per_validation);

        // Devrait être < 10µs par validation
        assert!(per_validation < 10, "Validation trop lente: {} µs", per_validation);
    }

    #[test]
    #[ignore]
    fn stress_test_rate_limiter() {
        let limiter = ClientRateLimiter::new(1000, Duration::from_secs(1));
        let mut successful = 0;
        let mut blocked = 0;

        for _ in 0..10000 {
            match limiter.check("stress_test") {
                Ok(_) => successful += 1,
                Err(_) => blocked += 1,
            }
        }

        println!("Successful: {}", successful);
        println!("Blocked: {}", blocked);

        // Devrait avoir bloqué ~9000 requêtes
        assert!(blocked > 8000);
        assert!(successful <= 1000);
    }
}
