use crate::error::{GhostHandError, Result};
use base64::prelude::*;
use ring::aead::{Aad, BoundKey, Nonce, NonceSequence, SealingKey, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

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

/// Key exchange using X25519 ECDH (Elliptic Curve Diffie-Hellman)
pub struct KeyExchange {
    rng: SystemRandom,
}

impl KeyExchange {
    pub fn new() -> Self {
        Self {
            rng: SystemRandom::new(),
        }
    }

    /// Generate a key pair for key exchange using X25519
    /// Returns (private_key_bytes, public_key_bytes)
    pub fn generate_keypair(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        // Générer 32 bytes aléatoires pour la clé privée
        let mut private_key_bytes = [0u8; 32];
        self.rng.fill(&mut private_key_bytes)
            .map_err(|e| GhostHandError::Crypto(format!("Failed to generate random bytes: {:?}", e)))?;

        // Créer la clé privée X25519 à partir des bytes
        let private_key = StaticSecret::from(private_key_bytes);

        // Calculer la clé publique correspondante
        let public_key = X25519PublicKey::from(&private_key);

        Ok((private_key_bytes.to_vec(), public_key.as_bytes().to_vec()))
    }

    /// Derive shared secret from private key and peer's public key using X25519 ECDH
    ///
    /// # Arguments
    /// * `private_key` - Notre clé privée (32 bytes)
    /// * `peer_public_key` - La clé publique du pair (32 bytes)
    ///
    /// # Returns
    /// Le secret partagé dérivé (32 bytes) qui peut être utilisé comme clé de chiffrement
    pub fn derive_shared_secret(
        &self,
        private_key: &[u8],
        peer_public_key: &[u8],
    ) -> Result<Vec<u8>> {
        if private_key.len() != 32 {
            return Err(GhostHandError::Crypto(format!(
                "Invalid private key length: expected 32, got {}",
                private_key.len()
            )));
        }

        if peer_public_key.len() != 32 {
            return Err(GhostHandError::Crypto(format!(
                "Invalid public key length: expected 32, got {}",
                peer_public_key.len()
            )));
        }

        // Reconstruire notre clé privée à partir des bytes
        let mut private_key_array = [0u8; 32];
        private_key_array.copy_from_slice(private_key);
        let my_private_key = StaticSecret::from(private_key_array);

        // Reconstruire la clé publique du pair à partir des bytes
        let mut peer_public_key_array = [0u8; 32];
        peer_public_key_array.copy_from_slice(peer_public_key);
        let peer_public = X25519PublicKey::from(peer_public_key_array);

        // Effectuer l'échange de clés ECDH pour obtenir le secret partagé
        let shared_secret = my_private_key.diffie_hellman(&peer_public);

        // Le secret partagé est de 32 bytes, parfait pour AES-256
        Ok(shared_secret.as_bytes().to_vec())
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

    #[test]
    fn test_key_exchange_ecdh() -> Result<()> {
        let kex = KeyExchange::new();

        // Alice génère sa paire de clés
        let (alice_private, alice_public) = kex.generate_keypair()?;
        assert_eq!(alice_private.len(), 32);
        assert_eq!(alice_public.len(), 32);

        // Bob génère sa paire de clés
        let (bob_private, bob_public) = kex.generate_keypair()?;
        assert_eq!(bob_private.len(), 32);
        assert_eq!(bob_public.len(), 32);

        // Alice calcule le secret partagé avec sa clé privée + la clé publique de Bob
        let alice_shared = kex.derive_shared_secret(&alice_private, &bob_public)?;

        // Bob calcule le secret partagé avec sa clé privée + la clé publique d'Alice
        let bob_shared = kex.derive_shared_secret(&bob_private, &alice_public)?;

        // Les deux secrets partagés doivent être identiques
        assert_eq!(alice_shared, bob_shared);
        assert_eq!(alice_shared.len(), 32); // 32 bytes pour AES-256

        Ok(())
    }

    #[test]
    fn test_e2e_encryption_with_key_exchange() -> Result<()> {
        let kex = KeyExchange::new();
        let crypto = CryptoManager::new();

        // Simulation d'un échange de clés entre Alice et Bob
        let (alice_private, alice_public) = kex.generate_keypair()?;
        let (bob_private, bob_public) = kex.generate_keypair()?;

        // Dériver le secret partagé
        let shared_secret = kex.derive_shared_secret(&alice_private, &bob_public)?;

        // Alice chiffre un message avec le secret partagé
        let plaintext = b"Message secret d'Alice a Bob";
        let encrypted = crypto.encrypt(&shared_secret, plaintext)?;

        // Bob déchiffre le message avec le même secret partagé
        let decrypted = crypto.decrypt(&shared_secret, &encrypted)?;

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());

        Ok(())
    }
}
