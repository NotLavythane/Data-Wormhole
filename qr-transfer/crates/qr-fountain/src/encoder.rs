//! Fountain encoder using RaptorQ with deterministic seeding

use crate::{EncodedBlock, FountainError, DEFAULT_BLOCK_SIZE};
use raptorq::{Encoder as RaptorQEncoder, ObjectTransmissionInformation};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

/// Fountain encoder that generates an infinite stream of blocks
pub struct FountainEncoder {
    source_data: Vec<u8>,
    block_size: usize,
    raptor_encoder: RaptorQEncoder,
    prng: ChaCha20Rng,
    next_block_id: u32,
    source_blocks_count: usize,
}

impl FountainEncoder {
    /// Create a new fountain encoder
    /// 
    /// # Arguments
    /// * `data` - Source data to encode
    /// * `block_size` - Size of each encoded block
    /// * `seed` - 32-byte deterministic seed from HKDF(shared_secret)
    pub fn new(
        data: &[u8],
        block_size: usize,
        seed: &[u8; 32],
    ) -> Result<Self, FountainError> {
        let block_size = block_size.min(DEFAULT_BLOCK_SIZE);
        
        // Create RaptorQ encoder
        let oti = ObjectTransmissionInformation::new(
            data.len() as u64,
            block_size as u16,
            1, // source blocks
            1, // sub blocks
            1, // alignment
        );
        
        let raptor_encoder = RaptorQEncoder::new(data, oti);
        
        // Create deterministic PRNG
        let prng = ChaCha20Rng::from_seed(*seed);
        
        let source_blocks_count = (data.len() + block_size - 1) / block_size;
        
        Ok(Self {
            source_data: data.to_vec(),
            block_size,
            raptor_encoder,
            prng,
            next_block_id: 0,
            source_blocks_count,
        })
    }
    
    /// Get the next encoded block (infinite stream)
    pub fn next_block(&mut self) -> EncodedBlock {
        // Get encoding packets from RaptorQ
        let packets = self.raptor_encoder.get_encoded_packets(1);
        let packet = packets.into_iter().next().unwrap();
        
        let block_id = self.next_block_id;
        self.next_block_id = self.next_block_id.wrapping_add(1);
        
        EncodedBlock::new(
            block_id,
            packet.serialize(),
            self.source_data.len(),
        )
    }
    
    /// Get multiple blocks at once
    pub fn next_blocks(&mut self, count: usize) -> Vec<EncodedBlock> {
        (0..count).map(|_| self.next_block()).collect()
    }
    
    /// Get total source blocks
    pub fn source_blocks_count(&self) -> usize {
        self.source_blocks_count
    }
    
    /// Get block size
    pub fn block_size(&self) -> usize {
        self.block_size
    }
    
    /// Get source data size
    pub fn source_size(&self) -> usize {
        self.source_data.len()
    }
}