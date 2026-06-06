//! Multi-QR grid packing for increased throughput

use crate::CodecError;
use qr_protocol::GridLayout;

/// Packs multiple QR codes into a single frame image
pub struct GridPacker {
    layout: GridLayout,
    cell_size: u32,
    padding: u32,
}

impl GridPacker {
    pub fn new(layout: GridLayout, cell_size: u32) -> Self {
        Self {
            layout,
            cell_size,
            padding: 10,
        }
    }
    
    /// Pack QR images into a grid layout
    pub fn pack_grid(
        &self,
        qr_images: &[image::DynamicImage],
    ) -> Result<image::DynamicImage, CodecError> {
        if qr_images.len() > self.layout.total_cells() {
            return Err(CodecError::InvalidGridLayout(
                format!("Too many images: {} > {} cells", 
                    qr_images.len(), 
                    self.layout.total_cells()
                )
            ));
        }
        
        let total_width = self.layout.cols as u32 * self.cell_size 
            + (self.layout.cols as u32 + 1) * self.padding;
        let total_height = self.layout.rows as u32 * self.cell_size 
            + (self.layout.rows as u32 + 1) * self.padding;
        
        // Create canvas
        let mut canvas = image::DynamicImage::new_rgb8(total_width, total_height);
        
        // Fill background white
        for x in 0..total_width {
            for y in 0..total_height {
                canvas.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            }
        }
        
        // Place QR images in grid (checkerboard interleave)
        for (idx, qr_img) in qr_images.iter().enumerate() {
            let (row, col) = self.interleave_position(idx);
            let x = self.padding + col as u32 * (self.cell_size + self.padding);
            let y = self.padding + row as u32 * (self.cell_size + self.padding);
            
            // Resize QR to cell size
            let resized = qr_img.resize(
                self.cell_size, 
                self.cell_size, 
                image::imageops::Lanczos3
            );
            
            image::imageops::overlay(&mut canvas, &resized, x as i64, y as i64);
        }
        
        Ok(canvas)
    }
    
    /// Calculate interleave position for block index
    fn interleave_position(&self, index: usize) -> (usize, usize) {
        let total = self.layout.total_cells();
        let idx = index % total;
        
        match (self.layout.rows, self.layout.cols) {
            // Checkerboard pattern for 2x2 and larger
            (r, c) if r >= 2 && c >= 2 => {
                let row = idx / c;
                let col = if row % 2 == 0 {
                    idx % c
                } else {
                    c - 1 - (idx % c)
                };
                (row, col)
            }
            // Sequential for single row/column
            _ => {
                (idx / self.layout.cols as usize, idx % self.layout.cols as usize)
            }
        }
    }
    
    /// Get canvas dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        let w = self.layout.cols as u32 * self.cell_size 
            + (self.layout.cols as u32 + 1) * self.padding;
        let h = self.layout.rows as u32 * self.cell_size 
            + (self.layout.rows as u32 + 1) * self.padding;
        (w, h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_grid_packer() {
        let packer = GridPacker::new(GridLayout::GRID_2X2, 300);
        let (w, h) = packer.dimensions();
        assert_eq!(w, 2 * 300 + 3 * 10); // 2 cells + 3 padding gaps
        assert_eq!(h, 2 * 300 + 3 * 10);
    }
    
    #[test]
    fn test_interleave() {
        let packer = GridPacker::new(GridLayout::GRID_2X2, 100);
        
        // Checkerboard pattern
        assert_eq!(packer.interleave_position(0), (0, 0));
        assert_eq!(packer.interleave_position(1), (0, 1));
        assert_eq!(packer.interleave_position(2), (1, 1)); // Reversed row
        assert_eq!(packer.interleave_position(3), (1, 0));
    }
}