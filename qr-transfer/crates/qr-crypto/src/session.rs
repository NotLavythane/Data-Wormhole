//! Session management: key rotation, chunk encryption, state tracking

use crate::{
    compute_shared_secret, derive_chunk_iv, derive_metadata_iv, derive_session_keys,
    encrypt_chunk, decrypt_chunk, rotate_key,
    CryptoError, IdentityKeyPair, SessionKeyMaterial,
    AES_KEY_SIZE, AES_NONCE_SIZE,
};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Maximum key epoch before requiring re-key
pub const MAX_KEY_EPOCH: u16 = 65535;

/// Rotate key every N chunks
pub const KEY_ROTATION_CHUNK_INTERVAL: u64 = 10_000;

/// Rotate key every N seconds
pub const KEY_ROTATION_TIME_INTERVAL_SECS: u64 = 900; // 15 minutes

/// A complete transfer session with key management
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct TransferSession {
    #[zeroize(skip)]
    my_ephemeral_secret: X25519StaticSecret,
    my_ephemeral_public: X25519PublicKey,
    their_ephemeral_public: Option<X25519PublicKey>,
    session_keys: Option<SessionKeyMaterial>,
    current_epoch: u16,
    chunks_in_epoch: u64,
    epoch_start_time: Option<std::time::Instant>,
}

impl TransferSession {
    /// Create a new session (generates ephemeral keypair)
    pub fn new() -> Self {
        let (secret, public) = crate::generate_ephemeral_keypair();
        Self {
            my_ephemeral_secret: secret,
            my_ephemeral_public: public,
            their_ephemeral_public: None,
            session_keys: None,
            current_epoch: 0,
            chunks_in_epoch: 0,
            epoch_start_time: None,
        }
    }
    
    /// Get our ephemeral public key to send to peer
    pub fn my_public_key(&self) -> [u8; 32] {
        *self.my_ephemeral_public.as_bytes()
    }
    
    /// Complete key exchange with peer's public key
    pub fn complete_key_exchange(
        &mut self,
        their_public: [u8; 32],
    ) -> Result<(), CryptoError> {
        let their_pk = X25519PublicKey::from(their_public);
        self.their_ephemeral_public = Some(their_pk);
        
        let shared_secret = compute_shared_secret(&self.my_ephemeral_secret, &their_pk);
        let keys = derive_session_keys(&shared_secret)?;
        
        self.session_keys = Some(keys);
        self.current_epoch = 0;
        self.chunks_in_epoch = 0;
        self.epoch_start_time = Some(std::time::Instant::now());
        
        Ok(())
    }
    
    /// Encrypt a chunk, handling key rotation automatically
    pub fn encrypt_chunk(
        &mut self,
        chunk_index: u32,
        plaintext: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        self.check_rotation()?;
        
        let keys = self.session_keys.as_ref()
            .ok_or(CryptoError::SessionExpired)?;
        
        let iv = derive_chunk_iv(&keys.base_nonce, chunk_index, self.current_epoch)?;
        let ciphertext = encrypt_chunk(&keys.aes_key, &iv, plaintext)?;
        
        self.chunks_in_epoch += 1;
        
        Ok(ciphertext)
    }
    
    /// Decrypt a chunk, handling key rotation
    pub fn decrypt_chunk(
        &mut self,
        chunk_index: u32,
        key_epoch: u16,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        // If the chunk uses a different epoch, derive that key
        let keys = if key_epoch != self.current_epoch {
            self.derive_key_for_epoch(key_epoch)?
        } else {
            self.session_keys.as_ref()
                .ok_or(CryptoError::SessionExpired)?
                .clone()
        };
        
        let iv = derive_chunk_iv(&keys.base_nonce, chunk_index, key_epoch)?;
        let plaintext = decrypt_chunk(&keys.aes_key, &iv, ciphertext)?;
        
        Ok(plaintext)
    }
    
    /// Get current key epoch
    pub fn current_epoch(&self) -> u16 {
        self.current_epoch
    }
    
    /// Check if key rotation is needed
    fn check_rotation(&mut self) -> Result<(), CryptoError> {
        let should_rotate = self.chunks_in_epoch >= KEY_ROTATION_CHUNK_INTERVAL
            || self.epoch_start_time.map_or(false, |t| {
                t.elapsed().as_secs() >= KEY_ROTATION_TIME_INTERVAL_SECS
            });
        
        if should_rotate {
            self.perform_rotation()?;
        }
        
        Ok(())
    }
    
    /// Perform key rotation
    fn perform_rotation(&mut self) -> Result<(), CryptoError> {
        if self.current_epoch >= MAX_KEY_EPOCH {
            return Err(CryptoError::SessionExpired);
        }
        
        let old_keys = self.session_keys.as_ref()
            .ok_or(CryptoError::SessionExpired)?;
        
        let new_key = rotate_key(&old_keys.aes_key, self.current_epoch + 1)?;
        
        // Derive new session material with rotated key
        let hkdf = hkdf::Hkdf::<sha2::Sha256>::from_okm(&new_key);
        let mut nonce = [0u8; AES_NONCE_SIZE];
        hkdf.expand(b"qr-aes-v1-nonce", &mut nonce)
            .map_err(|_| CryptoError::KeyDerivationFailed)?;
        
        self.session_keys = Some(SessionKeyMaterial {
            aes_key: new_key,
            base_nonce: nonce,
        });
        
        self.current_epoch += 1;
        self.chunks_in_epoch = 0;
        self.epoch_start_time = Some(std::time::Instant::now());
        
        Ok(())
    }
    
    /// Derive key material for a specific epoch (for decryption)
    fn derive_key_for_epoch(&self, epoch: u16) -> Result<SessionKeyMaterial, CryptoError> {
        let base_keys = self.session_keys.as_ref()
            .ok_or(CryptoError::SessionExpired)?;
        
        let mut current_key = base_keys.aes_key;
        for _ in 0..epoch {
            current_key = rotate_key(&current_key, _ as u16 + 1)?;
        }
        
        let hkdf = hkdf::Hkdf::<sha2::Sha256>::from_okm(&current_key);
        let mut nonce = [0u8; AES_NONCE_SIZE];
        hkdf.expand(b"qr-aes-v1-nonce", &mut nonce)
            .map_err(|_| CryptoError::KeyDerivationFailed)?;
        
        Ok(SessionKeyMaterial {
            aes_key: current_key,
            base_nonce: nonce,
        })
    }
}

impl Default for TransferSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Session keys for a transfer
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct SessionKeys {
    pub aes_key: [u8; AES_KEY_SIZE],
    pub base_nonce: [u8; AES_NONCE_SIZE],
    pub current_epoch: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate_ephemeral_keypair;
    
    #[test]
    fn test_session_lifecycle() {
        let mut alice = TransferSession::new();
        let mut bob = TransferSession::new();
        
        // Exchange public keys
        let alice_pub = alice.my_public_key();
        let bob_pub = bob.my_public_key();
        
        alice.complete_key_exchange(bob_pub).unwrap();
        bob.complete_key_exchange(alice_pub).unwrap();
        
        // Encrypt on Alice, decrypt on Bob
        let plaintext = b"Secret message for transfer";
        let ciphertext = alice.encrypt_chunk(0, plaintext).unwrap();
        let decrypted = bob.decrypt_chunk(0, 0, &ciphertext).unwrap();
        
        assert_eq!(plaintext.to_vec(), decrypted);
    }
}