//! Fountain decoder that reconstructs source data from encoded blocks

use crate::{EncodedBlock, FountainError};
use raptorq::{Decoder as RaptorQDecoder, ObjectTransmissionInformation};

/// Fountain decoder that accumulates blocks until reconstruction is possible
pub struct FountainDecoder {
    source_size: usize,
    block_size: usize,
    received_blocks: Vec<EncodedBlock>,
    block_bitmap: Vec<bool>,
    decoded: Option<Vec<u8>>,
}

impl FountainDecoder {
    /// Create a new fountain decoder
    pub fn new(
        source_size: usize,
        block_size: usize,
    ) -> Result<Self, FountainError> {
        let expected_blocks = (source_size + block_size - 1) / block_size;
        
        Ok(Self {
            source_size,
            block_size,
            received_blocks: Vec::new(),
            block_bitmap: vec![false; expected_blocks * 2], // Allow for overhead
            decoded: None,
        })
    }
    
    /// Add a received block. Returns true if decoding is complete.
    pub fn add_block(&mut self, block: EncodedBlock) -> Result<bool, FountainError> {
        // Check if we already have this block
        let idx = block.block_id as usize;
        if idx < self.block_bitmap.len() && self.block_bitmap[idx] {
            return Ok(self.decoded.is_some());
        }
        
        // Mark as received
        if idx >= self.block_bitmap.len() {
            self.block_bitmap.resize(idx * 2, false);
        }
        self.block_bitmap[idx] = true;
        self.received_blocks.push(block);
        
        // Try to decode if we have enough blocks
        if self.received_blocks.len() >= self.min_blocks_needed() {
            self.try_decode()?;
        }
        
        Ok(self.decoded.is_some())
    }
    
    /// Check if decoding is complete
    pub fn is_complete(&self) -> bool {
        self.decoded.is_some()
    }
    
    /// Get decoded data (if complete)
    pub fn decode(&self) -> Option<&[u8]> {
        self.decoded.as_deref()
    }
    
    /// Get decoded data, consuming the decoder
    pub fn into_decode(self) -> Option<Vec<u8>> {
        self.decoded
    }
    
    /// Get number of blocks received
    pub fn blocks_received(&self) -> usize {
        self.received_blocks.len()
    }
    
    /// Get minimum blocks needed
    fn min_blocks_needed(&self) -> usize {
        (self.source_size + self.block_size - 1) / self.block_size
    }
    
    /// Attempt to decode with current blocks
    fn try_decode(&mut self) -> Result<(), FountainError> {
        // Create RaptorQ decoder
        let oti = ObjectTransmissionInformation::new(
            self.source_size as u64,
            self.block_size as u16,
            1,
            1,
            1,
        );
        
        let decoder = RaptorQDecoder::new(oti)
            .ok_or_else(|| FountainError::DecodingFailed(
                "Failed to create decoder".into()
            ))?;
        
        // Convert blocks to RaptorQ packets
        let packets: Vec<_> = self.received_blocks.iter()
            .map(|b| {
                raptorq::EncodingPacket::deserialize(&b.data)
            })
            .filter(|p| p.is_some())
            .map(|p| p.unwrap())
            .collect();
        
        // Attempt decode
        if let Some(decoded) = decoder.decode(packets) {
            self.decoded = Some(decoded);
        }
        
        Ok(())
    }
    
    /// Get progress as fraction (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        let min_needed = self.min_blocks_needed();
        let received = self.received_blocks.len();
        
        if received >= min_needed && self.decoded.is_some() {
            1.0
        } else {
            (received as f64) / (min_needed as f64 * 1.05) // Account for overhead
        }.min(0.99)
    }
}