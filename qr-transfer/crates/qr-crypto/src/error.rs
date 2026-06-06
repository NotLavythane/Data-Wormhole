use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Invalid key size")]
    InvalidKeySize,
    
    #[error("Encryption failed")]
    EncryptionFailed,
    
    #[error("Decryption failed - authentication tag mismatch")]
    DecryptionFailed,
    
    #[error("Key derivation failed")]
    KeyDerivationFailed,
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Key exchange failed")]
    KeyExchangeFailed,
    
    #[error("Invalid public key")]
    InvalidPublicKey,
    
    #[error("Session expired")]
    SessionExpired,
    
    #[error("Memory sanitization failed")]
    SanitizationFailed,
}