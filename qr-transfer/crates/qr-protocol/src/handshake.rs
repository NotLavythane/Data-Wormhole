//! Capability negotiation handshake
//! 
//! Sender displays advertisement → Receiver scans and responds → Sender confirms

use crate::{AuthType, Compression, EccLevel, GridLayout, ProtocolError};

/// Protocol capability advertisement (sender → receiver)
#[derive(Debug, Clone)]
pub struct CapabilityAdvertisement {
    pub protocol_version: u16,
    pub supported_qr_versions: Vec<u8>,
    pub supported_ecc_levels: Vec<EccLevel>,
    pub max_grid: GridLayout,
    pub webrtc_capable: bool,
    pub compression_algorithms: Vec<Compression>,
    pub identity_public_key: Option<[u8; 32]>,
}

/// Capability response (receiver → sender)
#[derive(Debug, Clone)]
pub struct CapabilityResponse {
    pub selected_qr_version: u8,
    pub selected_ecc: EccLevel,
    pub selected_grid: GridLayout,
    pub webrtc_accept: bool,
    pub selected_compression: Compression,
    pub identity_public_key: Option<[u8; 32]>,
}

/// Negotiated capabilities for the session
#[derive(Debug, Clone)]
pub struct NegotiatedCapabilities {
    pub qr_version: u8,
    pub ecc_level: EccLevel,
    pub grid: GridLayout,
    pub use_webrtc: bool,
    pub compression: Compression,
    pub protocol_version: u16,
}

impl CapabilityAdvertisement {
    /// Create default advertisement
    pub fn default_advertisement() -> Self {
        Self {
            protocol_version: crate::PROTOCOL_VERSION,
            supported_qr_versions: vec![20, 30, 40],
            supported_ecc_levels: vec![EccLevel::M, EccLevel::Q, EccLevel::L],
            max_grid: GridLayout::GRID_2X2,
            webrtc_capable: false,
            compression_algorithms: vec![
                Compression::None,
                Compression::ZstdGeneric,
            ],
            identity_public_key: None,
        }
    }
    
    /// Serialize to CBOR bytes
    pub fn serialize(&self) -> Result<Vec<u8>, ProtocolError> {
        // Simple binary serialization for QR payload
        let mut buf = Vec::new();
        
        buf.extend_from_slice(&self.protocol_version.to_le_bytes());
        buf.push(self.supported_qr_versions.len() as u8);
        buf.extend_from_slice(&self.supported_qr_versions);
        buf.push(self.supported_ecc_levels.len() as u8);
        for ecc in &self.supported_ecc_levels {
            buf.push(*ecc as u8);
        }
        buf.push(self.max_grid.rows);
        buf.push(self.max_grid.cols);
        buf.push(if self.webrtc_capable { 1 } else { 0 });
        buf.push(self.compression_algorithms.len() as u8);
        for comp in &self.compression_algorithms {
            buf.push(*comp as u8);
        }
        
        if let Some(key) = &self.identity_public_key {
            buf.push(1);
            buf.extend_from_slice(key);
        } else {
            buf.push(0);
        }
        
        Ok(buf)
    }
    
    /// Deserialize from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() < 4 {
            return Err(ProtocolError::InvalidFrame("Adv too short".into()));
        }
        
        let protocol_version = u16::from_le_bytes([data[0], data[1]]);
        
        let mut offset = 2;
        let qr_count = data[offset] as usize;
        offset += 1;
        let supported_qr_versions = data[offset..offset+qr_count].to_vec();
        offset += qr_count;
        
        let ecc_count = data[offset] as usize;
        offset += 1;
        let mut supported_ecc_levels = Vec::with_capacity(ecc_count);
        for i in 0..ecc_count {
            if let Some(ecc) = EccLevel::from_u8(data[offset + i]) {
                supported_ecc_levels.push(ecc);
            }
        }
        offset += ecc_count;
        
        let rows = data[offset];
        let cols = data[offset + 1];
        offset += 2;
        
        let webrtc_capable = data[offset] == 1;
        offset += 1;
        
        let comp_count = data[offset] as usize;
        offset += 1;
        let mut compression_algorithms = Vec::with_capacity(comp_count);
        for i in 0..comp_count {
            if let Some(comp) = Compression::from_u8(data[offset + i]) {
                compression_algorithms.push(comp);
            }
        }
        offset += comp_count;
        
        let has_key = data[offset] == 1;
        offset += 1;
        let identity_public_key = if has_key {
            let mut key = [0u8; 32];
            key.copy_from_slice(&data[offset..offset+32]);
            Some(key)
        } else {
            None
        };
        
        Ok(Self {
            protocol_version,
            supported_qr_versions,
            supported_ecc_levels,
            max_grid: GridLayout { rows, cols },
            webrtc_capable,
            compression_algorithms,
            identity_public_key,
        })
    }
}

impl CapabilityResponse {
    /// Create response by intersecting with advertisement
    pub fn from_advertisement(adv: &CapabilityAdvertisement) -> Result<Self, ProtocolError> {
        // Select minimum capabilities
        let selected_qr_version = adv.supported_qr_versions.iter().min().copied()
            .unwrap_or(40);
        let selected_ecc = adv.supported_ecc_levels.first().copied()
            .unwrap_or(EccLevel::M);
        let selected_grid = GridLayout::SINGLE; // Conservative default
        let selected_compression = adv.compression_algorithms.first().copied()
            .unwrap_or(Compression::None);
        
        Ok(Self {
            selected_qr_version,
            selected_ecc,
            selected_grid,
            webrtc_accept: false, // Conservative
            selected_compression,
            identity_public_key: None,
        })
    }
    
    /// Serialize to bytes
    pub fn serialize(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = Vec::new();
        
        buf.push(self.selected_qr_version);
        buf.push(self.selected_ecc as u8);
        buf.push(self.selected_grid.rows);
        buf.push(self.selected_grid.cols);
        buf.push(if self.webrtc_accept { 1 } else { 0 });
        buf.push(self.selected_compression as u8);
        
        if let Some(key) = &self.identity_public_key {
            buf.push(1);
            buf.extend_from_slice(key);
        } else {
            buf.push(0);
        }
        
        Ok(buf)
    }
    
    /// Deserialize from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() < 6 {
            return Err(ProtocolError::InvalidFrame("Response too short".into()));
        }
        
        let selected_qr_version = data[0];
        let selected_ecc = EccLevel::from_u8(data[1])
            .ok_or_else(|| ProtocolError::InvalidFrame("Invalid ECC".into()))?;
        let selected_grid = GridLayout {
            rows: data[2],
            cols: data[3],
        };
        let webrtc_accept = data[4] == 1;
        let selected_compression = Compression::from_u8(data[5])
            .ok_or_else(|| ProtocolError::InvalidFrame("Invalid compression".into()))?;
        
        let mut offset = 6;
        let has_key = data[offset] == 1;
        offset += 1;
        let identity_public_key = if has_key && data.len() >= offset + 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&data[offset..offset+32]);
            Some(key)
        } else {
            None
        };
        
        Ok(Self {
            selected_qr_version,
            selected_ecc,
            selected_grid,
            webrtc_accept,
            selected_compression,
            identity_public_key,
        })
    }
}

/// Perform capability negotiation
pub fn negotiate_capabilities(
    adv: &CapabilityAdvertisement,
    resp: &CapabilityResponse,
) -> Result<NegotiatedCapabilities, ProtocolError> {
    // Verify protocol version compatibility
    let adv_major = (adv.protocol_version >> 8) as u8;
    let our_major = (crate::PROTOCOL_VERSION >> 8) as u8;
    
    if adv_major != our_major {
        return Err(ProtocolError::VersionMismatch {
            expected: crate::PROTOCOL_VERSION,
            actual: adv.protocol_version,
        });
    }
    
    Ok(NegotiatedCapabilities {
        qr_version: resp.selected_qr_version,
        ecc_level: resp.selected_ecc,
        grid: resp.selected_grid,
        use_webrtc: adv.webrtc_capable && resp.webrtc_accept,
        compression: resp.selected_compression,
        protocol_version: adv.protocol_version.min(crate::PROTOCOL_VERSION),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_advertisement_serde() {
        let adv = CapabilityAdvertisement::default_advertisement();
        let serialized = adv.serialize().unwrap();
        let deserialized = CapabilityAdvertisement::deserialize(&serialized).unwrap();
        
        assert_eq!(adv.protocol_version, deserialized.protocol_version);
        assert_eq!(adv.max_grid.rows, deserialized.max_grid.rows);
        assert_eq!(adv.max_grid.cols, deserialized.max_grid.cols);
    }
    
    #[test]
    fn test_capability_response() {
        let adv = CapabilityAdvertisement::default_advertisement();
        let resp = CapabilityResponse::from_advertisement(&adv).unwrap();
        
        assert!(adv.supported_qr_versions.contains(&resp.selected_qr_version));
    }
    
    #[test]
    fn test_negotiation() {
        let adv = CapabilityAdvertisement::default_advertisement();
        let resp = CapabilityResponse::from_advertisement(&adv).unwrap();
        let negotiated = negotiate_capabilities(&adv, &resp).unwrap();
        
        assert_eq!(negotiated.grid.rows, 1);
        assert_eq!(negotiated.grid.cols, 1);
    }
}