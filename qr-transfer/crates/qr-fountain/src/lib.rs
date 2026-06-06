//! # qr-fountain
//! 
//! Fountain codes (RaptorQ) with deterministic PRNG seeding for QR Transfer.
//! 
//! The encoder generates an infinite stream of encoded blocks from source data.
//! The receiver needs only slightly more than K blocks to reconstruct the file.

pub mod encoder;
pub mod decoder;
pub mod block;

pub use encoder::FountainEncoder;
pub use decoder::FountainDecoder;
pub use block::EncodedBlock;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FountainError {
    #[error("Encoding failed: {0}")]
    EncodingFailed(String),
    
    #[error("Decoding failed: {0}")]
    DecodingFailed(String),
    
    #[error("Invalid block: {0}")]
    InvalidBlock(String),
    
    #[error("Insufficient blocks: got {got}, need {need}")]
    InsufficientBlocks { got: usize, need: usize },
    
    #[error("PRNG seeding failed")]
    PrngSeedingFailed,
}

/// Default source block size (matches QR payload)
pub const DEFAULT_BLOCK_SIZE: usize = 1900;

/// Target overhead (receiver needs ~105% of source blocks)
pub const DEFAULT_OVERHEAD_FACTOR: f64 = 1.05;

/// Size of block ID in bytes
pub const BLOCK_ID_SIZE: usize = 4;

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;
    
    #[test]
    fn test_roundtrip() {
        let data = vec![0x42u8; 10000];
        let seed = [0x42u8; 32];
        
        let mut encoder = FountainEncoder::new(&data, DEFAULT_BLOCK_SIZE, &seed).unwrap();
        let mut decoder = FountainDecoder::new(
            data.len(),
            DEFAULT_BLOCK_SIZE,
        ).unwrap();
        
        // Collect 120% of source blocks
        let num_source_blocks = (data.len() + DEFAULT_BLOCK_SIZE - 1) / DEFAULT_BLOCK_SIZE;
        let target_blocks = (num_source_blocks as f64 * 1.2) as usize;
        
        let mut collected = 0;
        while collected < target_blocks && !decoder.is_complete() {
            let block = encoder.next_block();
            if decoder.add_block(block).unwrap() {
                break;
            }
            collected += 1;
        }
        
        let decoded = decoder.decode().unwrap();
        assert_eq!(data, decoded);
    }
}