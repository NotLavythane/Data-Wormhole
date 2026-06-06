//! File chunking for QR transfer

use crate::ProtocolError;

/// Chunk a file into equal-sized pieces
pub fn chunk_file(data: &[u8], chunk_size: usize) -> Vec<Vec<u8>> {
    data.chunks(chunk_size)
        .map(|c| c.to_vec())
        .collect()
}

/// Calculate number of chunks needed
pub fn num_chunks(data_len: usize, chunk_size: usize) -> u32 {
    ((data_len + chunk_size - 1) / chunk_size) as u32
}

/// Reassemble chunks into a single buffer
pub fn reassemble_chunks(chunks: &[Vec<u8>]) -> Vec<u8> {
    let total_len: usize = chunks.iter().map(|c| c.len()).sum();
    let mut result = Vec::with_capacity(total_len);
    
    for chunk in chunks {
        result.extend_from_slice(chunk);
    }
    
    result
}

/// Create chunk index to byte range mapping
pub fn chunk_range(chunk_idx: u32, chunk_size: usize, total_size: usize) -> (usize, usize) {
    let start = (chunk_idx as usize) * chunk_size;
    let end = ((start + chunk_size).min(total_size));
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chunking() {
        let data = vec![0u8; 10000];
        let chunks = chunk_file(&data, 1900);
        
        assert_eq!(chunks.len(), 6);
        assert_eq!(chunks[0].len(), 1900);
        assert_eq!(chunks[5].len(), 500); // Last chunk
    }
    
    #[test]
    fn test_reassemble() {
        let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        let chunks = chunk_file(&data, 1900);
        let reassembled = reassemble_chunks(&chunks);
        
        assert_eq!(data, reassembled);
    }
    
    #[test]
    fn test_num_chunks() {
        assert_eq!(num_chunks(1000, 1900), 1);
        assert_eq!(num_chunks(2000, 1900), 2);
        assert_eq!(num_chunks(3800, 1900), 2);
    }
}