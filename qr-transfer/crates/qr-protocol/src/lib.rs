//! # qr-protocol
//! 
//! Core protocol engine for QR Transfer.
//! 
//! Provides: Frame format, state machine, capability handshake, chunking.

pub mod frame;
pub mod state;
pub mod handshake;
pub mod chunking;

pub use frame::*;
pub use state::*;
pub use handshake::*;
pub use chunking::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid frame format: {0}")]
    InvalidFrame(String),
    
    #[error("Invalid state transition: {from:?} -> {to:?}")]
    InvalidStateTransition { from: TransferState, to: TransferState },
    
    #[error("Capability mismatch: {0}")]
    CapabilityMismatch(String),
    
    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u16, actual: u16 },
    
    #[error("Chunk index out of bounds: {index} >= {total}")]
    ChunkIndexOutOfBounds { index: u32, total: u32 },
    
    #[error("Frame too large: {size} > max {max}")]
    FrameTooLarge { size: usize, max: usize },
    
    #[error("CRC mismatch")]
    CrcMismatch,
    
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Current protocol version
pub const PROTOCOL_VERSION_MAJOR: u8 = 0;
pub const PROTOCOL_VERSION_MINOR: u8 = 1;
pub const PROTOCOL_VERSION: u16 = ((PROTOCOL_VERSION_MAJOR as u16) << 8) | (PROTOCOL_VERSION_MINOR as u16);

/// Maximum frame payload size (Version 40 QR, Level M, minus headers)
pub const MAX_FRAME_PAYLOAD: usize = 1900;

/// QR Version to use by default
pub const DEFAULT_QR_VERSION: u8 = 40;

/// Default error correction level
pub const DEFAULT_ECC_LEVEL: EccLevel = EccLevel::M;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EccLevel {
    L = 0, // ~7% recovery
    M = 1, // ~15% recovery
    Q = 2, // ~25% recovery
    H = 3, // ~30% recovery
}

impl EccLevel {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(EccLevel::L),
            1 => Some(EccLevel::M),
            2 => Some(EccLevel::Q),
            3 => Some(EccLevel::H),
            _ => None,
        }
    }
    
    /// Capacity for this ECC level at Version 40
    pub fn capacity_v40(&self) -> usize {
        match self {
            EccLevel::L => 2953,
            EccLevel::M => 2331,
            EccLevel::Q => 1663,
            EccLevel::H => 1273,
        }
    }
}

/// Authentication type for transfer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthType {
    Anonymous = 0,
    Signed = 1,
    Tofu = 2,
}

impl AuthType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(AuthType::Anonymous),
            1 => Some(AuthType::Signed),
            2 => Some(AuthType::Tofu),
            _ => None,
        }
    }
}

/// Grid layout for multi-QR frames
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridLayout {
    pub rows: u8,
    pub cols: u8,
}

impl GridLayout {
    pub const SINGLE: Self = Self { rows: 1, cols: 1 };
    pub const GRID_1X2: Self = Self { rows: 1, cols: 2 };
    pub const GRID_2X2: Self = Self { rows: 2, cols: 2 };
    pub const GRID_2X3: Self = Self { rows: 2, cols: 3 };
    pub const GRID_3X3: Self = Self { rows: 3, cols: 3 };
    
    pub fn total_cells(&self) -> usize {
        (self.rows as usize) * (self.cols as usize)
    }
    
    pub fn is_valid(&self) -> bool {
        self.rows >= 1 && self.rows <= 4 && self.cols >= 1 && self.cols <= 4
    }
}

/// Compression algorithm selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None = 0,
    ZstdGeneric = 1,
    ZstdText = 2,
    ZstdExecutable = 3,
    ZstdImage = 4,
}

impl Compression {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Compression::None),
            1 => Some(Compression::ZstdGeneric),
            2 => Some(Compression::ZstdText),
            3 => Some(Compression::ZstdExecutable),
            4 => Some(Compression::ZstdImage),
            _ => None,
        }
    }
}

/// File metadata (encrypted in chunk 0)
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub filename: String,
    pub mime_type: String,
    pub mod_time: u64,
    pub original_size: u64,
    pub compression: Compression,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ecc_capacity() {
        assert_eq!(EccLevel::L.capacity_v40(), 2953);
        assert_eq!(EccLevel::M.capacity_v40(), 2331);
        assert_eq!(EccLevel::H.capacity_v40(), 1273);
    }
    
    #[test]
    fn test_grid_layout() {
        assert_eq!(GridLayout::SINGLE.total_cells(), 1);
        assert_eq!(GridLayout::GRID_2X2.total_cells(), 4);
        assert_eq!(GridLayout::GRID_3X3.total_cells(), 9);
    }
}