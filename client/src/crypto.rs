use crate::error::{GhostHandError, Result};
use base64::prelude::*;
use ring::aead::{Aad, BoundKey, Nonce, NonceSequence, SealingKey, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

const NONCE_SIZE: usize = 12;
const KEY_SIZE: usize = 32;

/// Cryptography manager for E2E encryption
pub struct CryptoManager {
    rng: SystemRandom,
}

impl CryptoManager {
    pub fn new() -> Self {
        Self {
            rng: SystemRandom::new(),
        }
    }

    /// Generate a random encryption key
    pub fn generate_key(&self) -> Result<Vec<u8>> {
        let mut key = vec![0u8; KEY_SIZE];
        self.rng.fill(&mut key).map_err(|e| {
            GhostHandError::Crypto(format!("Failed to generate key: {:?}", e))
        })?;
        Ok(key)
    }

    /// Generate a random nonce
    pub fn generate_nonce(&self) -> Result<Vec<u8>> {
        let mut nonce = vec![0u8; NONCE_SIZE];
        self.rng.fill(&mut nonce).map_err(|e| {
            GhostHandError::Crypto(format!("Failed to generate nonce: {:?}", e))
        })?;
        Ok(nonce)
    }

    /// Encrypt data using AES-256-GCM
    pub fn encrypt(&self, key: &[u8], data: &[u8]) -> Result<EncryptedData> {
        if key.len() != KEY_SIZE {
            return Err(GhostHandError::Crypto(format!(
                "Invalid key size: expected {}, got {}",
                KEY_SIZE,
                key.len()
            )));
        }

        let nonce = self.generate_nonce()?;

        let unbound_key = UnboundKey::new(&AES_256_GCM, key).map_err(|e| {
            GhostHandError::Crypto(format!("Failed to create key: {:?}", e))
        })?;

        let mut sealing_key = SealingKey::new(unbound_key, SingleNonce::new(&nonce));

        let mut in_out = data.to_vec();
        sealing_key
            .seal_in_place_append_tag(Aad::empty(), &mut in_out)
            .map_err(|e| GhostHandError::Crypto(format!("Failed to encrypt: {:?}", e)))?;

        Ok(EncryptedData {
            ciphertext: in_out,
            nonce,
        })
    }

    /// Decrypt data using AES-256-GCM
    pub fn decrypt(&self, key: &[u8], encrypted: &EncryptedData) -> Result<Vec<u8>> {
        if key.len() != KEY_SIZE {
            return Err(GhostHandError::Crypto(format!(
                "Invalid key size: expected {}, got {}",
                KEY_SIZE,
                key.len()
            )));
        }

        let unbound_key = UnboundKey::new(&AES_256_GCM, key).map_err(|e| {
            GhostHandError::Crypto(format!("Failed to create key: {:?}", e))
        })?;

        let mut opening_key = ring::aead::OpeningKey::new(unbound_key, SingleNonce::new(&encrypted.nonce));

        let mut in_out = encrypted.ciphertext.clone();
        let plaintext = opening_key
            .open_in_place(Aad::empty(), &mut in_out)
            .map_err(|e| GhostHandError::Crypto(format!("Failed to decrypt: {:?}", e)))?;

        Ok(plaintext.to_vec())
    }

    /// Generate a password hash for authentication
    pub fn hash_password(&self, password: &str) -> Result<String> {
        use ring::digest;

        let salt = self.generate_nonce()?;
        let mut to_hash = password.as_bytes().to_vec();
        to_hash.extend_from_slice(&salt);

        let hash = digest::digest(&digest::SHA256, &to_hash);

        // Combine salt and hash
        let mut result = salt;
        result.extend_from_slice(hash.as_ref());

        Ok(BASE64_STANDARD.encode(result))
    }

    /// Verify a password against a hash
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        use ring::digest;

        let combined = BASE64_STANDARD
            .decode(hash)
            .map_err(|e| GhostHandError::Crypto(format!("Invalid hash format: {}", e)))?;

        if combined.len() < NONCE_SIZE {
            return Ok(false);
        }

        let (salt, stored_hash) = combined.split_at(NONCE_SIZE);

        let mut to_hash = password.as_bytes().to_vec();
        to_hash.extend_from_slice(salt);

        let computed_hash = digest::digest(&digest::SHA256, &to_hash);

        Ok(computed_hash.as_ref() == stored_hash)
    }
}

/// Encrypted data container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

/// Single-use nonce for AEAD operations
struct SingleNonce {
    nonce: Vec<u8>,
    used: bool,
}

impl SingleNonce {
    fn new(nonce: &[u8]) -> Self {
        Self {
            nonce: nonce.to_vec(),
            used: false,
        }
    }
}

impl NonceSequence for SingleNonce {
    fn advance(&mut self) -> std::result::Result<Nonce, ring::error::Unspecified> {
        if self.used {
            return Err(ring::error::Unspecified);
        }

        self.used = true;
        Nonce::try_assume_unique_for_key(&self.nonce)
    }
}

/// Key exchange using simplified Diffie-Hellman
/// In production, use proper ECDH with curve25519
pub struct KeyExchange {
    crypto: CryptoManager,
}

impl KeyExchange {
    pub fn new() -> Self {
        Self {
            crypto: CryptoManager::new(),
        }
    }

    /// Generate a key pair for key exchange
    pub fn generate_keypair(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        // This is a simplified version
        // In production, use ring::agreement or x25519_dalek

        let private_key = self.crypto.generate_key()?;
        let public_key = self.crypto.generate_key()?; // Simplified

        Ok((private_key, public_key))
    }

    /// Derive shared secret from private key and peer's public key
    pub fn derive_shared_secret(
        &self,
        _private_key: &[u8],
        _peer_public_key: &[u8],
    ) -> Result<Vec<u8>> {
        // This is a placeholder
        // In production, use proper ECDH
        // let agreement = ring::agreement::agree_ephemeral(...)?;

        // For now, just generate a random key
        self.crypto.generate_key()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() -> Result<()> {
        let crypto = CryptoManager::new();
        let key = crypto.generate_key()?;

        let plaintext = b"Hello, GhostHandDesk!";
        let encrypted = crypto.encrypt(&key, plaintext)?;
        let decrypted = crypto.decrypt(&key, &encrypted)?;

        assert_eq!(plaintext, decrypted.as_slice());

        Ok(())
    }

    #[test]
    fn test_password_hashing() -> Result<()> {
        let crypto = CryptoManager::new();
        let password = "super_secret_password";

        let hash = crypto.hash_password(password)?;
        assert!(crypto.verify_password(password, &hash)?);
        assert!(!crypto.verify_password("wrong_password", &hash)?);

        Ok(())
    }

    #[test]
    fn test_key_generation() -> Result<()> {
        let crypto = CryptoManager::new();

        let key1 = crypto.generate_key()?;
        let key2 = crypto.generate_key()?;

        assert_eq!(key1.len(), KEY_SIZE);
        assert_eq!(key2.len(), KEY_SIZE);
        assert_ne!(key1, key2); // Should be different

        Ok(())
    }
}
