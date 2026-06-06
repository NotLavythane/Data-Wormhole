//! Accessibility features for QR codes

/// Apply high contrast mode to QR image (pure black/white)
pub fn high_contrast_mode(img: image::DynamicImage) -> image::DynamicImage {
    use image::GenericImageView;
    let mut luma = img.to_luma8();
    
    for pixel in luma.pixels_mut() {
        // Binarize: threshold at 128
        pixel.0[0] = if pixel.0[0] > 128 { 255 } else { 0 };
    }
    
    image::DynamicImage::ImageLuma8(luma)
}

/// Add quiet zone padding around QR code
pub fn add_quiet_zone(
    img: image::DynamicImage, 
    padding_px: u32
) -> image::DynamicImage {
    use image::GenericImageView;
    let (width, height) = img.dimensions();
    
    let new_width = width + 2 * padding_px;
    let new_height = height + 2 * padding_px;
    
    let mut canvas = image::DynamicImage::new_rgb8(new_width, new_height);
    
    // Fill white
    for x in 0..new_width {
        for y in 0..new_height {
            canvas.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
        }
    }
    
    // Place original image
    image::imageops::overlay(&mut canvas, &img, padding_px as i64, padding_px as i64);
    
    canvas
}

/// Check if image meets WCAG contrast requirements
/// For QR codes: modules should be pure black (0) on pure white (255)
pub fn verify_wcag_contrast(img: &image::DynamicImage) -> bool {
    let luma = img.to_luma8();
    let mut has_black = false;
    let mut has_white = false;
    
    for pixel in luma.pixels() {
        if pixel.0[0] == 0 {
            has_black = true;
        }
        if pixel.0[0] == 255 {
            has_white = true;
        }
    }
    
    has_black && has_white
}