//! Frame format for QR Transfer protocol
//! 
//! Frame layout:
//! | Field           | Size   | Offset |
//! |-----------------|--------|--------|
//! | Magic           | 2      | 0      |
//! | Flags           | 2      | 2      |
//! | Chunk Index     | 4      | 4      |
//! | Total Chunks    | 4      | 8      |
//! | File Hash Prefix| 8      | 12     |
//! | Key Epoch       | 2      | 20     |
//! | Payload         | ~1900  | 22     |
//! | CRC-32          | 4      | end    |

use crate::{AuthType, Compression, EccLevel, GridLayout, ProtocolError};
use qr_crypto::crc32;

/// Frame header size (bytes before payload)
pub const FRAME_HEADER_SIZE: usize = 22;

/// Frame footer size (CRC-32)
pub const FRAME_FOOTER_SIZE: usize = 4;

/// Total frame overhead
pub const FRAME_OVERHEAD: usize = FRAME_HEADER_SIZE + FRAME_FOOTER_SIZE;

/// Maximum payload per frame
pub const MAX_PAYLOAD_SIZE: usize = 1900;

/// QR Transfer protocol frame
#[derive(Debug, Clone)]
pub struct Frame {
    pub magic: [u8; 2],
    pub flags: FrameFlags,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub file_hash_prefix: [u8; 8],
    pub key_epoch: u16,
    pub payload: Vec<u8>,
    pub crc32: u32,
}

/// Frame flags encoded in 2 bytes
#[derive(Debug, Clone)]
pub struct FrameFlags {
    pub protocol_version: u8,
    pub qr_version: u8,
    pub ecc_level: EccLevel,
    pub auth_type: AuthType,
    pub grid_layout: GridLayout,
    pub streamable: bool,
    pub compression: Compression,
}

impl FrameFlags {
    /// Encode flags to 2 bytes
    pub fn encode(&self) -> [u8; 2] {
        let byte0 = ((self.protocol_version & 0x0F) << 4) 
                   | (self.qr_version.min(40) & 0x0F);
        let byte1 = ((self.ecc_level as u8) & 0x03) << 6
                   | ((self.auth_type as u8) & 0x03) << 4
                   | ((self.grid_layout.rows - 1) & 0x03) << 2
                   | ((self.grid_layout.cols - 1) & 0x03);
        [byte0, byte1]
    }
    
    /// Decode flags from 2 bytes
    pub fn decode(bytes: [u8; 2]) -> Result<Self, ProtocolError> {
        let byte0 = bytes[0];
        let byte1 = bytes[1];
        
        let protocol_version = (byte0 >> 4) & 0x0F;
        let qr_version = byte0 & 0x0F;
        
        let ecc_level = EccLevel::from_u8((byte1 >> 6) & 0x03)
            .ok_or_else(|| ProtocolError::InvalidFrame("Invalid ECC level".into()))?;
        let auth_type = AuthType::from_u8((byte1 >> 4) & 0x03)
            .ok_or_else(|| ProtocolError::InvalidFrame("Invalid auth type".into()))?;
        let rows = ((byte1 >> 2) & 0x03) + 1;
        let cols = (byte1 & 0x03) + 1;
        
        Ok(FrameFlags {
            protocol_version,
            qr_version,
            ecc_level,
            auth_type,
            grid_layout: GridLayout { rows, cols },
            streamable: false, // TODO: encode in flags
            compression: Compression::None, // TODO: encode in flags
        })
    }
}

impl Frame {
    /// Create a new data frame
    pub fn new(
        chunk_index: u32,
        total_chunks: u32,
        file_hash_prefix: [u8; 8],
        key_epoch: u16,
        payload: Vec<u8>,
    ) -> Result<Self, ProtocolError> {
        if payload.len() > MAX_PAYLOAD_SIZE {
            return Err(ProtocolError::FrameTooLarge { 
                size: payload.len(), 
                max: MAX_PAYLOAD_SIZE 
            });
        }
        
        let mut frame = Self {
            magic: qr_crypto::FRAME_MAGIC,
            flags: FrameFlags {
                protocol_version: crate::PROTOCOL_VERSION_MINOR as u8,
                qr_version: crate::DEFAULT_QR_VERSION,
                ecc_level: crate::DEFAULT_ECC_LEVEL,
                auth_type: AuthType::Anonymous,
                grid_layout: GridLayout::SINGLE,
                streamable: false,
                compression: Compression::None,
            },
            chunk_index,
            total_chunks,
            file_hash_prefix,
            key_epoch,
            payload,
            crc32: 0,
        };
        
        frame.crc32 = frame.compute_crc();
        Ok(frame)
    }
    
    /// Serialize frame to bytes for QR encoding
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(FRAME_HEADER_SIZE + self.payload.len() + FRAME_FOOTER_SIZE);
        
        buf.extend_from_slice(&self.magic);
        buf.extend_from_slice(&self.flags.encode());
        buf.extend_from_slice(&self.chunk_index.to_le_bytes());
        buf.extend_from_slice(&self.total_chunks.to_le_bytes());
        buf.extend_from_slice(&self.file_hash_prefix);
        buf.extend_from_slice(&self.key_epoch.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf.extend_from_slice(&self.crc32.to_le_bytes());
        
        buf
    }
    
    /// Deserialize frame from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() < FRAME_OVERHEAD + 1 {
            return Err(ProtocolError::InvalidFrame(
                format!("Frame too short: {} bytes", data.len())
            ));
        }
        
        // Check magic
        if &data[0..2] != &qr_crypto::FRAME_MAGIC[..] {
            return Err(ProtocolError::InvalidFrame("Invalid magic bytes".into()));
        }
        
        let flags = FrameFlags::decode([data[2], data[3]])?;
        let chunk_index = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let total_chunks = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let mut file_hash_prefix = [0u8; 8];
        file_hash_prefix.copy_from_slice(&data[12..20]);
        let key_epoch = u16::from_le_bytes([data[20], data[21]]);
        
        let payload_end = data.len() - FRAME_FOOTER_SIZE;
        let payload = data[FRAME_HEADER_SIZE..payload_end].to_vec();
        let crc32 = u32::from_le_bytes([
            data[data.len()-4], 
            data[data.len()-3], 
            data[data.len()-2], 
            data[data.len()-1]
        ]);
        
        let frame = Self {
            magic: qr_crypto::FRAME_MAGIC,
            flags,
            chunk_index,
            total_chunks,
            file_hash_prefix,
            key_epoch,
            payload,
            crc32,
        };
        
        // Verify CRC
        let computed_crc = frame.compute_crc();
        if computed_crc != crc32 {
            return Err(ProtocolError::CrcMismatch);
        }
        
        Ok(frame)
    }
    
    /// Compute CRC-32 over header + payload
    pub fn compute_crc(&self) -> u32 {
        let mut buf = Vec::with_capacity(FRAME_HEADER_SIZE + self.payload.len());
        
        buf.extend_from_slice(&self.magic);
        buf.extend_from_slice(&self.flags.encode());
        buf.extend_from_slice(&self.chunk_index.to_le_bytes());
        buf.extend_from_slice(&self.total_chunks.to_le_bytes());
        buf.extend_from_slice(&self.file_hash_prefix);
        buf.extend_from_slice(&self.key_epoch.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        
        crc32(&buf)
    }
    
    /// Verify frame integrity
    pub fn verify(&self) -> bool {
        self.compute_crc() == self.crc32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_frame_serde() {
        let payload = vec![0x42u8; 100];
        let frame = Frame::new(
            42,
            1000,
            [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0],
            0,
            payload.clone(),
        ).unwrap();
        
        let serialized = frame.serialize();
        let deserialized = Frame::deserialize(&serialized).unwrap();
        
        assert_eq!(frame.chunk_index, deserialized.chunk_index);
        assert_eq!(frame.total_chunks, deserialized.total_chunks);
        assert_eq!(frame.payload, deserialized.payload);
        assert!(deserialized.verify());
    }
    
    #[test]
    fn test_frame_too_large() {
        let payload = vec![0u8; MAX_PAYLOAD_SIZE + 1];
        let result = Frame::new(0, 1, [0u8; 8], 0, payload);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_invalid_magic() {
        let data = vec![0x00, 0x00, 0x00, 0x00]; // Wrong magic
        let result = Frame::deserialize(&data);
        assert!(result.is_err());
    }
}