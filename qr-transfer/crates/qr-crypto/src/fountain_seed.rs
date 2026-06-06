//! Deterministic fountain code seeding from shared secret

use hkdf::Hkdf;
use sha2::Sha256;
use rand_chacha::ChaCha20Rng;
use rand::SeedableRng;

use crate::CryptoError;

/// Derive fountain encoder seed from shared secret
/// FountainSeed = HKDF-Expand(shared_secret, "qr-fountain-v1-seed", 32)
pub fn derive_fountain_seed(shared_secret: &[u8; 32]) -> Result<[u8; 32], CryptoError> {
    let hkdf = Hkdf::<Sha256>::from_okm(shared_secret);
    let mut seed = [0u8; 32];
    hkdf.expand(b"qr-fountain-v1-seed", &mut seed)
        .map_err(|_| CryptoError::KeyDerivationFailed)?;
    Ok(seed)
}

/// Create a deterministic CSPRNG from fountain seed
pub fn create_fountain_prng(seed: &[u8; 32]) -> ChaCha20Rng {
    ChaCha20Rng::from_seed(*seed)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_deterministic_seeding() {
        let secret = [0x42u8; 32];
        
        let seed1 = derive_fountain_seed(&secret).unwrap();
        let seed2 = derive_fountain_seed(&secret).unwrap();
        
        // Same secret produces same seed
        assert_eq!(seed1, seed2);
        
        // Seed is different from secret
        assert_ne!(seed1, secret);
    }
    
    #[test]
    fn test_prng_determinism() {
        let secret = [0x42u8; 32];
        let seed = derive_fountain_seed(&secret).unwrap();
        
        let mut prng1 = create_fountain_prng(&seed);
        let mut prng2 = create_fountain_prng(&seed);
        
        use rand::RngCore;
        let mut buf1 = [0u8; 32];
        let mut buf2 = [0u8; 32];
        
        prng1.fill_bytes(&mut buf1);
        prng2.fill_bytes(&mut buf2);
        
        // Same seed produces same sequence
        assert_eq!(buf1, buf2);
    }
}