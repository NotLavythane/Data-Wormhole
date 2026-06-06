//! Ed25519 identity keys for device authentication

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::CryptoError;

/// Long-term identity keypair for device authentication
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct IdentityKeyPair {
    #[zeroize(skip)]
    signing_key: SigningKey,
}

impl IdentityKeyPair {
    /// Generate a new random identity keypair
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }
    
    /// Get the public key for sharing
    pub fn public_key(&self) -> IdentityPublicKey {
        IdentityPublicKey {
            verifying_key: self.signing_key.verifying_key(),
        }
    }
    
    /// Sign a message (typically the ephemeral X25519 public key)
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }
    
    /// Export raw public key bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }
    
    /// Import from raw key bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(bytes);
        Self { signing_key }
    }
}

/// Public portion of identity key
#[derive(Debug, Clone)]
pub struct IdentityPublicKey {
    verifying_key: VerifyingKey,
}

impl IdentityPublicKey {
    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), CryptoError> {
        self.verifying_key
            .verify(message, signature)
            .map_err(|_| CryptoError::InvalidSignature)
    }
    
    /// Export raw public key bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }
    
    /// Import from raw bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, CryptoError> {
        let verifying_key = VerifyingKey::from_bytes(bytes)
            .map_err(|_| CryptoError::InvalidPublicKey)?;
        Ok(Self { verifying_key })
    }
    
    /// Get fingerprint for display (first 8 bytes of hash)
    pub fn fingerprint(&self) -> String {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(&self.to_bytes());
        let hash = hasher.finalize();
        hex::encode(&hash.as_bytes()[..8])
    }
}

/// Sign an ephemeral X25519 public key with an identity key
pub fn sign_ephemeral_key(
    identity: &IdentityKeyPair,
    ephemeral_public: &[u8; 32],
) -> Signature {
    identity.sign(ephemeral_public)
}

/// Verify an ephemeral key signature
pub fn verify_ephemeral_key(
    identity: &IdentityPublicKey,
    ephemeral_public: &[u8; 32],
    signature: &Signature,
) -> Result<(), CryptoError> {
    identity.verify(ephemeral_public, signature)
}

/// Simple hex encoding helper
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_identity_generate() {
        let id = IdentityKeyPair::generate();
        let pubkey = id.public_key();
        assert_eq!(pubkey.to_bytes().len(), 32);
    }
    
    #[test]
    fn test_sign_and_verify() {
        let identity = IdentityKeyPair::generate();
        let ephemeral = [0x42u8; 32];
        
        let sig = sign_ephemeral_key(&identity, &ephemeral);
        let result = verify_ephemeral_key(&identity.public_key(), &ephemeral, &sig);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_invalid_signature() {
        let identity = IdentityKeyPair::generate();
        let wrong_identity = IdentityKeyPair::generate();
        let ephemeral = [0x42u8; 32];
        
        let sig = sign_ephemeral_key(&identity, &ephemeral);
        let result = verify_ephemeral_key(&wrong_identity.public_key(), &ephemeral, &sig);
        assert!(result.is_err());
    }
}