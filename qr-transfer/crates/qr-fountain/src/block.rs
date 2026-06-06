//! Encoded block structure for fountain codes

/// A single encoded block from the fountain encoder
#[derive(Debug, Clone)]
pub struct EncodedBlock {
    /// Block identifier (sequence number)
    pub block_id: u32,
    /// Encoded data
    pub data: Vec<u8>,
    /// Size of source data (for reconstruction)
    pub source_size: usize,
}

impl EncodedBlock {
    /// Create a new encoded block
    pub fn new(block_id: u32, data: Vec<u8>, source_size: usize) -> Self {
        Self {
            block_id,
            data,
            source_size,
        }
    }
    
    /// Serialize block for QR frame payload
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(4 + self.data.len());
        buf.extend_from_slice(&self.block_id.to_le_bytes());
        buf.extend_from_slice(&self.data);
        buf
    }
    
    /// Deserialize from QR frame payload
    pub fn deserialize(data: &[u8], source_size: usize) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        
        let block_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let block_data = data[4..].to_vec();
        
        Some(Self {
            block_id,
            data: block_data,
            source_size,
        })
    }
}