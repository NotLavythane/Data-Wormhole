//! # qr-crypto
//! 
//! Cryptographic primitives for QR Transfer.
//! 
//! Provides: X25519 key exchange, Ed25519 identity signing, AES-256-GCM encryption,
//! HKDF key derivation, BLAKE3 hashing, session key rotation, and Safety Number verification.

pub mod error;
pub mod identity;
pub mod session;
pub mod fountain_seed;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use blake3::Hasher as Blake3Hasher;
use hkdf::Hkdf;
use rand::{rngs::OsRng, RngCore};
use sha2::Sha256;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub use error::CryptoError;
pub use identity::{IdentityKeyPair, IdentityPublicKey};
pub use session::{SessionKeys, TransferSession};
pub use fountain_seed::derive_fountain_seed;

// Re-export key types for convenience
pub use x25519_dalek::{PublicKey, StaticSecret};

/// Version string for protocol identification
pub const PROTOCOL_VERSION: &str = "qr-transfer-v1";

/// Size of X25519 public key in bytes
pub const X25519_PUBLIC_KEY_SIZE: usize = 32;

/// Size of AES-256 key in bytes  
pub const AES_KEY_SIZE: usize = 32;

/// Size of AES-GCM nonce/IV in bytes
pub const AES_NONCE_SIZE: usize = 12;

/// Size of AES-GCM authentication tag in bytes
pub const AES_TAG_SIZE: usize = 16;

/// Size of BLAKE3 hash in bytes
pub const BLAKE3_HASH_SIZE: usize = 32;

/// Size of file hash prefix used in frame headers
pub const FILE_HASH_PREFIX_SIZE: usize = 8;

/// Size of CRC-32 checksum in bytes
pub const CRC32_SIZE: usize = 4;

/// Frame magic bytes: "QR" = 0x5152
pub const FRAME_MAGIC: [u8; 2] = [0x51, 0x52];

/// Derive an AES-256-GCM key and nonce from shared secret using HKDF-SHA256
pub fn derive_session_keys(shared_secret: &[u8; 32]) -> Result<SessionKeyMaterial, CryptoError> {
    let hkdf = Hkdf::<Sha256>::from_okm(shared_secret);
    
    // Derive AES-256 key
    let mut key_bytes = [0u8; AES_KEY_SIZE];
    hkdf_expand(&hkdf, b"qr-aes-v1-key", &mut key_bytes)?;
    
    // Derive base nonce/IV  
    let mut nonce_bytes = [0u8; AES_NONCE_SIZE];
    hkdf_expand(&hkdf, b"qr-aes-v1-nonce", &mut nonce_bytes)?;
    
    Ok(SessionKeyMaterial {
        aes_key: key_bytes,
        base_nonce: nonce_bytes,
    })
}

/// Derive a per-chunk IV from session salt and chunk index
/// IV = HKDF(session_salt, "chunk" || chunk_index || key_epoch)
pub fn derive_chunk_iv(
    session_salt: &[u8; 32], 
    chunk_index: u32,
    key_epoch: u16,
) -> Result<[u8; AES_NONCE_SIZE], CryptoError> {
    let hkdf = Hkdf::<Sha256>::from_okm(session_salt);
    
    let mut info = Vec::with_capacity(14);
    info.extend_from_slice(b"chunk");
    info.extend_from_slice(&chunk_index.to_le_bytes());
    info.extend_from_slice(&key_epoch.to_le_bytes());
    
    let mut iv = [0u8; AES_NONCE_SIZE];
    hkdf_expand(&hkdf, &info, &mut iv)?;
    
    Ok(iv)
}

/// Derive metadata encryption IV
pub fn derive_metadata_iv(session_salt: &[u8; 32]) -> Result<[u8; AES_NONCE_SIZE], CryptoError> {
    let hkdf = Hkdf::<Sha256>::from_okm(session_salt);
    let mut iv = [0u8; AES_NONCE_SIZE];
    hkdf_expand(&hkdf, b"metadata", &mut iv)?;
    Ok(iv)
}

/// Compute a Safety Number for MITM detection
/// SAFETY_NUMBER = SHA3-256(sender_pubkey || receiver_pubkey || session_salt)
pub fn compute_safety_number(
    sender_pubkey: &[u8; 32],
    receiver_pubkey: &[u8; 32],
    session_salt: &[u8; 32],
) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = sha2::Sha3_256::new();
    hasher.update(sender_pubkey);
    hasher.update(receiver_pubkey);
    hasher.update(session_salt);
    hasher.finalize().into()
}

/// Convert safety number to 4-word BIP39-style mnemonic for human verification
pub fn safety_number_to_words(hash: &[u8; 32]) -> String {
    // Use first 8 bytes to select 4 words from a reduced word list
    let word_list = &BIP39_WORDS;
    let mut words = Vec::with_capacity(4);
    
    for i in 0..4 {
        let idx = ((hash[i * 2] as usize) << 8 | (hash[i * 2 + 1] as usize)) % word_list.len();
        words.push(word_list[idx]);
    }
    
    words.join(" ")
}

/// Encrypt a chunk using AES-256-GCM
pub fn encrypt_chunk(
    key: &[u8; AES_KEY_SIZE],
    iv: &[u8; AES_NONCE_SIZE],
    plaintext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| CryptoError::InvalidKeySize)?;
    let nonce = Nonce::from_slice(iv);
    
    cipher.encrypt(nonce, plaintext)
        .map_err(|_| CryptoError::EncryptionFailed)
}

/// Decrypt a chunk using AES-256-GCM
pub fn decrypt_chunk(
    key: &[u8; AES_KEY_SIZE],
    iv: &[u8; AES_NONCE_SIZE],
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| CryptoError::InvalidKeySize)?;
    let nonce = Nonce::from_slice(iv);
    
    cipher.decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::DecryptionFailed)
}

/// Compute BLAKE3 hash of data
pub fn hash_file(data: &[u8]) -> [u8; BLAKE3_HASH_SIZE] {
    let mut hasher = Blake3Hasher::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Derive hash prefix for frame headers
pub fn hash_prefix(hash: &[u8; BLAKE3_HASH_SIZE]) -> [u8; FILE_HASH_PREFIX_SIZE] {
    let mut prefix = [0u8; FILE_HASH_PREFIX_SIZE];
    prefix.copy_from_slice(&hash[..FILE_HASH_PREFIX_SIZE]);
    prefix
}

/// Rotate session key via HKDF ratchet
/// Key_n = HKDF-Expand(Key_{n-1}, "qr-ratchet-v1" || n, 32)
pub fn rotate_key(
    current_key: &[u8; AES_KEY_SIZE],
    epoch: u16,
) -> Result<[u8; AES_KEY_SIZE], CryptoError> {
    let hkdf = Hkdf::<Sha256>::from_okm(current_key);
    let mut info = Vec::with_capacity(18);
    info.extend_from_slice(b"qr-ratchet-v1");
    info.extend_from_slice(&epoch.to_le_bytes());
    
    let mut new_key = [0u8; AES_KEY_SIZE];
    hkdf_expand(&hkdf, &info, &mut new_key)?;
    
    Ok(new_key)
}

/// HKDF-Expand helper
fn hkdf_expand(hkdf: &Hkdf<Sha256>, info: &[u8], okm: &mut [u8]) -> Result<(), CryptoError> {
    hkdf.expand(info, okm)
        .map_err(|_| CryptoError::KeyDerivationFailed)
}

/// Session key material derived from shared secret
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct SessionKeyMaterial {
    pub aes_key: [u8; AES_KEY_SIZE],
    pub base_nonce: [u8; AES_NONCE_SIZE],
}

/// Generate ephemeral X25519 keypair
pub fn generate_ephemeral_keypair() -> (X25519StaticSecret, X25519PublicKey) {
    let secret = X25519StaticSecret::random_from_rng(OsRng);
    let public = X25519PublicKey::from(&secret);
    (secret, public)
}

/// Compute ECDH shared secret
pub fn compute_shared_secret(
    my_secret: &X25519StaticSecret,
    their_public: &X25519PublicKey,
) -> [u8; 32] {
    *my_secret.diffie_hellman(their_public).as_bytes()
}

/// CRC-32 computation for frame integrity
pub fn crc32(data: &[u8]) -> u32 {
    const CRC_TABLE: [u32; 256] = {
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut c = i as u32;
            let mut j = 0;
            while j < 8 {
                c = if (c & 1) != 0 {
                    0xedb88320 ^ (c >> 1)
                } else {
                    c >> 1
                };
                j += 1;
            }
            table[i] = c;
            i += 1;
        }
        table
    };
    
    let mut crc: u32 = 0xffffffff;
    for byte in data {
        crc = CRC_TABLE[((crc ^ (*byte as u32)) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffffffff
}

/// BIP39 word list (first 256 words for compact selection)
const BIP39_WORDS: [&str; 256] = [
    "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract",
    "absurd", "abuse", "access", "accident", "account", "accuse", "achieve", "acid",
    "acoustic", "acquire", "across", "act", "action", "actor", "actress", "actual",
    "adapt", "add", "addict", "address", "adjust", "admit", "adult", "advance",
    "advice", "aerobic", "affair", "afford", "afraid", "again", "age", "agent",
    "agree", "ahead", "aim", "air", "airport", "aisle", "alarm", "album",
    "alcohol", "alert", "alien", "all", "alley", "allow", "almost", "alone",
    "alpha", "already", "also", "alter", "always", "amateur", "amazing", "among",
    "amount", "amused", "analyst", "anchor", "ancient", "anger", "angle", "angry",
    "animal", "ankle", "announce", "annual", "another", "answer", "antenna", "antique",
    "anxiety", "any", "apart", "apology", "appear", "apple", "approve", "april",
    "arch", "arctic", "area", "arena", "argue", "arise", "armor", "army",
    "around", "arrange", "arrest", "arrive", "arrow", "art", "artefact", "artist",
    "artwork", "ask", "aspect", "assault", "asset", "assist", "assume", "asthma",
    "athlete", "atom", "attack", "attend", "attitude", "attract", "auction", "audit",
    "august", "aunt", "author", "auto", "autumn", "average", "avocado", "avoid",
    "awake", "aware", "away", "awesome", "awful", "awkward", "axis", "baby",
    "bachelor", "bacon", "badge", "bag", "balance", "balcony", "ball", "bamboo",
    "banana", "banner", "bar", "barely", "bargain", "barrel", "base", "basic",
    "basket", "battle", "beach", "bean", "beauty", "because", "become", "beef",
    "before", "begin", "behave", "behind", "believe", "below", "belt", "bench",
    "benefit", "best", "betray", "better", "between", "beyond", "bicycle", "bid",
    "bike", "bind", "biology", "bird", "birth", "bitter", "black", "blade",
    "blame", "blanket", "blast", "bleak", "bless", "blind", "blood", "blossom",
    "blouse", "blue", "blur", "blush", "board", "boat", "body", "boil",
    "bomb", "bone", "bonus", "book", "boost", "border", "borrow", "boss",
    "bottom", "bounce", "box", "boy", "bracket", "brain", "brand", "brass",
    "brave", "bread", "breeze", "brick", "bridge", "brief", "bright", "brilliant",
    "bring", "brisk", "broccoli", "broken", "bronze", "broom", "brother", "brown",
    "brush", "bubble", "buddy", "budget", "buffalo", "build", "bulb", "bulk",
    "bullet", "bundle", "bunker", "burden", "burger", "burst", "bus", "business",
    "busy", "butter", "buyer", "buzz", "cabbage", "cabin", "cable", "cactus"
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let (secret, public) = generate_ephemeral_keypair();
        assert_eq!(public.as_bytes().len(), 32);
    }

    #[test]
    fn test_key_exchange() {
        let (alice_secret, alice_public) = generate_ephemeral_keypair();
        let (bob_secret, bob_public) = generate_ephemeral_keypair();
        
        let alice_shared = compute_shared_secret(&alice_secret, &bob_public);
        let bob_shared = compute_shared_secret(&bob_secret, &alice_public);
        
        assert_eq!(alice_shared, bob_shared);
    }

    #[test]
    fn test_session_key_derivation() {
        let shared = [0x42u8; 32];
        let keys = derive_session_keys(&shared).unwrap();
        assert_eq!(keys.aes_key.len(), 32);
        assert_eq!(keys.base_nonce.len(), 12);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let key = [0x42u8; 32];
        let iv = [0x12u8; 12];
        let plaintext = b"Hello, QR Transfer!";
        
        let ciphertext = encrypt_chunk(&key, &iv, plaintext).unwrap();
        let decrypted = decrypt_chunk(&key, &iv, &ciphertext).unwrap();
        
        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_blake3_hash() {
        let data = b"test data";
        let hash = hash_file(data);
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_crc32() {
        let data = b"123456789";
        let checksum = crc32(data);
        // Known CRC-32 of "123456789" is 0xcbf43926
        assert_eq!(checksum, 0xcbf43926);
    }

    #[test]
    fn test_key_rotation() {
        let key = [0x42u8; 32];
        let new_key = rotate_key(&key, 1).unwrap();
        assert_eq!(new_key.len(), 32);
        assert_ne!(key, new_key);
    }

    #[test]
    fn test_safety_number() {
        let sender = [0x01u8; 32];
        let receiver = [0x02u8; 32];
        let salt = [0x03u8; 32];
        
        let sn = compute_safety_number(&sender, &receiver, &salt);
        assert_eq!(sn.len(), 32);
        
        let words = safety_number_to_words(&sn);
        assert!(!words.is_empty());
    }
}