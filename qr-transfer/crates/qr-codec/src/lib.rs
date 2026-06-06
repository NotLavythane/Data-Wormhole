//! # qr-codec
//! 
//! QR code generation, grid packing, and interleave patterns for QR Transfer.

pub mod generator;
pub mod grid;
pub mod accessibility;

pub use generator::QrGenerator;
pub use grid::GridPacker;
pub use accessibility::high_contrast_mode;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CodecError {
    #[error("QR generation failed: {0}")]
    GenerationFailed(String),
    
    #[error("Grid layout invalid: {0}")]
    InvalidGridLayout(String),
    
    #[error("Image encoding failed: {0}")]
    ImageEncodingFailed(String),
    
    #[error("Payload too large: {size} > {max}")]
    PayloadTooLarge { size: usize, max: usize },
}

/// QR code color mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Normal,
    HighContrast,
    Inverted,
}

/// Generate a single QR code from binary data
pub fn generate_qr(
    data: &[u8],
    version: i16,
    color_mode: ColorMode,
) -> Result<image::DynamicImage, CodecError> {
    use qrcode::{QrCode, EcLevel};
    
    let ec_level = match version {
        v if v < 10 => EcLevel::H,
        v if v < 25 => EcLevel::Q,
        _ => EcLevel::M,
    };
    
    let code = QrCode::with_error_correction_level(data, ec_level)
        .map_err(|e| CodecError::GenerationFailed(format!("{:?}", e)))?;
    
    let image = code.render::<image::Luma<u8>>()
        .quiet_zone(true)
        .module_dimensions(4, 4)
        .build();
    
    // Apply color mode
    let img = image::DynamicImage::ImageLuma8(image);
    
    match color_mode {
        ColorMode::HighContrast => Ok(high_contrast_mode(img)),
        ColorMode::Inverted => Ok(invert_colors(img)),
        ColorMode::Normal => Ok(img),
    }
}

/// Invert QR colors (black ↔ white)
pub fn invert_colors(img: image::DynamicImage) -> image::DynamicImage {
    use image::GenericImageView;
    let mut buf = img.to_luma8();
    for pixel in buf.pixels_mut() {
        pixel.0[0] = 255 - pixel.0[0];
    }
    image::DynamicImage::ImageLuma8(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_qr() {
        let data = b"Hello QR Transfer";
        let img = generate_qr(data, 40, ColorMode::Normal).unwrap();
        assert!(img.width() > 0);
        assert!(img.height() > 0);
    }
    
    #[test]
    fn test_high_contrast() {
        let data = b"Test";
        let img = generate_qr(data, 40, ColorMode::HighContrast).unwrap();
        // Verify pure black and white
        let luma = img.to_luma8();
        for pixel in luma.pixels() {
            assert!(pixel.0[0] == 0 || pixel.0[0] == 255);
        }
    }
}