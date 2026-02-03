//! Tests d'intégration pour GhostHandDesk
//!
//! Ces tests vérifient que les composants fonctionnent ensemble correctement.

use ghost_hand_client::config::{Config, VideoCodec};
use ghost_hand_client::crypto::CryptoManager;
use ghost_hand_client::network::generate_device_id;
use ghost_hand_client::screen_capture::{create_capturer, FrameFormat};
use ghost_hand_client::video_encoder::create_encoder;
use ghost_hand_client::Result;

#[tokio::test]
async fn test_full_client_initialization() -> Result<()> {
    // 1. Configuration
    let config = Config::default();
    assert!(!config.server_url.is_empty());
    assert!(!config.stun_servers.is_empty());

    // 2. Device ID
    let device_id = generate_device_id();
    assert!(device_id.starts_with("GHD-"));
    assert!(device_id.len() > 8);

    // 3. Crypto
    let crypto = CryptoManager::new();
    let key = crypto.generate_key()?;
    assert_eq!(key.len(), 32); // AES-256

    Ok(())
}

#[test]
fn test_capture_and_encode_pipeline() -> Result<()> {
    // 1. Créer capturer
    let mut capturer = create_capturer()?;

    // 2. Lister les displays
    let displays = capturer.get_displays()?;
    assert!(!displays.is_empty(), "Au moins un display devrait être détecté");

    // 3. Capturer une frame
    let frame = capturer.capture()?;
    assert!(!frame.data.is_empty());
    assert!(frame.width > 0);
    assert!(frame.height > 0);
    assert!(matches!(frame.format, FrameFormat::RGBA));

    println!("Frame capturée: {}x{}, {} bytes", frame.width, frame.height, frame.data.len());

    Ok(())
}

#[tokio::test]
async fn test_complete_encoding_pipeline() -> Result<()> {
    // 1. Créer capturer
    let mut capturer = create_capturer()?;
    let frame = capturer.capture()?;

    let original_size = frame.data.len();

    // 2. Créer encodeur (JPEG fallback)
    let mut encoder = create_encoder(
        VideoCodec::H264,
        frame.width,
        frame.height,
        30,
        4000,
    )?;

    // 3. Encoder la frame
    let encoded = encoder.encode(&frame).await?;

    // Vérifications
    assert!(!encoded.data.is_empty(), "Les données encodées ne doivent pas être vides");
    assert_eq!(encoded.width, frame.width);
    assert_eq!(encoded.height, frame.height);

    // La compression devrait réduire la taille
    let compressed_size = encoded.data.len();
    let compression_ratio = original_size as f64 / compressed_size as f64;

    println!("Compression: {} bytes -> {} bytes (ratio: {:.2}x)",
        original_size, compressed_size, compression_ratio);

    assert!(compressed_size < original_size, "L'encodage devrait compresser les données");

    Ok(())
}

#[test]
fn test_config_defaults() {
    let config = Config::default();

    // Vérifier les valeurs par défaut
    assert!(!config.server_url.is_empty());
    assert!(config.server_url.starts_with("wss://"));
    assert_eq!(config.video_config.framerate, 30);
    assert_eq!(config.video_config.bitrate, 4000);
    assert!(!config.stun_servers.is_empty());
}

#[test]
fn test_crypto_encrypt_decrypt() -> Result<()> {
    let crypto = CryptoManager::new();

    // Générer clé
    let key = crypto.generate_key()?;

    // Test data
    let plaintext = b"Message secret de test";

    // Chiffrer
    let ciphertext = crypto.encrypt(&key, plaintext)?;

    // Les données chiffrées devraient être différentes
    assert!(!ciphertext.ciphertext.is_empty());
    assert!(!ciphertext.nonce.is_empty());

    // Déchiffrer
    let decrypted = crypto.decrypt(&key, &ciphertext)?;
    assert_eq!(plaintext.as_slice(), &decrypted[..]);

    Ok(())
}

#[test]
fn test_device_id_uniqueness() {
    let mut ids = std::collections::HashSet::new();

    // Générer 10 IDs avec délai
    for _ in 0..10 {
        let id = generate_device_id();
        assert!(id.starts_with("GHD-"));
        ids.insert(id);

        // Petit délai pour garantir timestamps différents
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    // Tous devraient être uniques
    assert_eq!(ids.len(), 10);
}

#[tokio::test]
async fn test_encoder_consistency() -> Result<()> {
    let mut encoder = create_encoder(VideoCodec::H264, 320, 240, 15, 1000)?;

    // Frame de test (pattern)
    let mut data = Vec::with_capacity(320 * 240 * 4);
    for y in 0..240 {
        for x in 0..320 {
            let r = ((x * 255) / 320) as u8;
            let g = ((y * 255) / 240) as u8;
            let b = 128u8;
            let a = 255u8;
            data.extend_from_slice(&[r, g, b, a]);
        }
    }

    let frame = ghost_hand_client::screen_capture::Frame {
        width: 320,
        height: 240,
        data,
        format: FrameFormat::RGBA,
        timestamp: 0,
    };

    // Encoder plusieurs fois
    for i in 0..5 {
        let mut test_frame = frame.clone();
        test_frame.timestamp = i;

        let encoded = encoder.encode(&test_frame).await?;

        assert!(!encoded.data.is_empty());
        assert_eq!(encoded.timestamp, i);
        assert_eq!(encoded.width, 320);
        assert_eq!(encoded.height, 240);
    }

    Ok(())
}

#[test]
fn test_multiple_displays() -> Result<()> {
    let capturer = create_capturer()?;
    let displays = capturer.get_displays()?;

    for display in &displays {
        println!("Display {}: {} ({}x{}) at ({}, {}) {}",
            display.id,
            display.name,
            display.width,
            display.height,
            display.x,
            display.y,
            if display.is_primary { "[PRIMARY]" } else { "" }
        );

        // Vérifications basiques
        assert!(display.width > 0);
        assert!(display.height > 0);
    }

    // Au moins un display devrait être primary
    assert!(displays.iter().any(|d| d.is_primary));

    Ok(())
}
