//! QR code generator with configurable parameters

use crate::{CodecError, ColorMode};
use qr_protocol::{EccLevel, MAX_FRAME_PAYLOAD};

/// QR generator with cached settings
pub struct QrGenerator {
    version: i16,
    ecc_level: EccLevel,
    color_mode: ColorMode,
    module_size: u32,
}

impl QrGenerator {
    pub fn new(version: i16, ecc_level: EccLevel) -> Self {
        Self {
            version,
            ecc_level,
            color_mode: ColorMode::Normal,
            module_size: 4,
        }
    }
    
    pub fn with_color_mode(mut self, mode: ColorMode) -> Self {
        self.color_mode = mode;
        self
    }
    
    pub fn with_module_size(mut self, size: u32) -> Self {
        self.module_size = size;
        self
    }
    
    /// Generate QR code image from binary payload
    pub fn generate(&self, payload: &[u8]) -> Result<image::DynamicImage, CodecError> {
        if payload.len() > MAX_FRAME_PAYLOAD {
            return Err(CodecError::PayloadTooLarge {
                size: payload.len(),
                max: MAX_FRAME_PAYLOAD,
            });
        }
        
        crate::generate_qr(payload, self.version, self.color_mode)
    }
    
    /// Generate QR as PNG bytes
    pub fn generate_png(&self, payload: &[u8]) -> Result<Vec<u8>, CodecError> {
        let img = self.generate(payload)?;
        
        let mut png_data = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png_data), image::ImageFormat::Png)
            .map_err(|e| CodecError::ImageEncodingFailed(e.to_string()))?;
        
        Ok(png_data)
    }
}