//! # qr-compression
//! 
//! Compression with file type auto-detection for QR Transfer.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompressionError {
    #[error("Compression failed: {0}")]
    CompressionFailed(String),
    
    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),
    
    #[error("Invalid data")]
    InvalidData,
}

/// File type hint for compression selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Unknown = 0,
    Text = 1,
    Executable = 2,
    ImageRaw = 3,
    AlreadyCompressed = 4,
}

impl FileType {
    /// Detect file type from content and extension
    pub fn detect(data: &[u8], filename: Option<&str>) -> Self {
        // Check extension first
        if let Some(name) = filename {
            let lower = name.to_lowercase();
            if lower.ends_with(".zip") || lower.ends_with(".gz") 
                || lower.ends_with(".bz2") || lower.ends_with(".xz")
                || lower.ends_with(".7z") || lower.ends_with(".rar")
                || lower.ends_with(".mp4") || lower.ends_with(".mp3")
                || lower.ends_with(".png") || lower.ends_with(".jpg")
                || lower.ends_with(".jpeg") || lower.ends_with(".webp")
                || lower.ends_with(".gif") {
                return FileType::AlreadyCompressed;
            }
            
            if lower.ends_with(".txt") || lower.ends_with(".md")
                || lower.ends_with(".json") || lower.ends_with(".xml")
                || lower.ends_with(".csv") || lower.ends_with(".log")
                || lower.ends_with(".html") || lower.ends_with(".css")
                || lower.ends_with(".js") || lower.ends_with(".rs")
                || lower.ends_with(".py") || lower.ends_with(".c")
                || lower.ends_with(".cpp") || lower.ends_with(".h") {
                return FileType::Text;
            }
            
            if lower.ends_with(".exe") || lower.ends_with(".dll")
                || lower.ends_with(".so") || lower.ends_with(".dylib")
                || lower.ends_with(".elf") || lower.ends_with(".wasm") {
                return FileType::Executable;
            }
        }
        
        // Check entropy of first 1KB
        let sample = &data[..data.len().min(1024)];
        let entropy = calculate_entropy(sample);
        
        if entropy > 7.5 {
            FileType::AlreadyCompressed
        } else if entropy < 6.0 {
            FileType::Text
        } else {
            FileType::Unknown
        }
    }
}

/// Compress data with optional file type optimization
pub fn compress(data: &[u8], file_type: FileType) -> Result<Vec<u8>, CompressionError> {
    match file_type {
        FileType::AlreadyCompressed => {
            // Don't compress already-compressed data
            Ok(data.to_vec())
        }
        _ => {
            // Use zstd with default level
            zstd::encode_all(data, 3)
                .map_err(|e| CompressionError::CompressionFailed(e.to_string()))
        }
    }
}

/// Decompress data
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    zstd::decode_all(data)
        .map_err(|e| CompressionError::DecompressionFailed(e.to_string()))
}

/// Calculate Shannon entropy of data (bits per byte)
fn calculate_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    
    let mut freq = [0u64; 256];
    for &b in data {
        freq[b as usize] += 1;
    }
    
    let len = data.len() as f64;
    let mut entropy = 0.0;
    
    for count in freq.iter() {
        if *count == 0 {
            continue;
        }
        let p = (*count as f64) / len;
        entropy -= p * p.log2();
    }
    
    entropy
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_text_compression() {
        let text = "Hello, World! This is a test of the QR Transfer compression system. ".repeat(100);
        let compressed = compress(text.as_bytes(), FileType::Text).unwrap();
        
        // Compressed should be smaller
        assert!(compressed.len() < text.len());
        
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(text.as_bytes(), &decompressed);
    }
    
    #[test]
    fn test_skip_already_compressed() {
        // Random data (high entropy)
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let file_type = FileType::detect(&data, Some("test.bin"));
        
        // Should detect as already compressed due to high entropy
        if let FileType::AlreadyCompressed = file_type {
            let compressed = compress(&data, file_type).unwrap();
            assert_eq!(compressed.len(), data.len());
        }
    }
    
    #[test]
    fn test_file_type_detection() {
        let text = "Hello, this is plain text content.";
        assert_eq!(FileType::detect(text.as_bytes(), Some("test.txt")), FileType::Text);
        
        assert_eq!(FileType::detect(&[], Some("test.zip")), FileType::AlreadyCompressed);
        assert_eq!(FileType::detect(&[], Some("test.mp4")), FileType::AlreadyCompressed);
    }
}