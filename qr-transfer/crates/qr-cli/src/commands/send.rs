//! Send command - display animated QR codes for file transfer

use indicatif::{ProgressBar, ProgressStyle};
use qr_codec::{QrGenerator, ColorMode, grid::GridPacker};
use qr_protocol::{GridLayout, EccLevel, frame::Frame, chunking::chunk_file, handshake::*};
use qr_crypto::{hash_file, hash_prefix, derive_fountain_seed};
use qr_fountain::FountainEncoder;
use qr_compression::{compress, FileType};
use std::io::Write;

pub async fn execute(
    data: Vec<u8>,
    filename: String,
    grid: GridLayout,
    ecc: EccLevel,
    compress_mode: String,
    terminal: bool,
) -> anyhow::Result<()> {
    println!("📤 QR Transfer - Send Mode");
    println!("==========================");
    println!("File: {} ({} bytes)", filename, data.len());
    
    // Step 1: Compress if applicable
    let file_type = FileType::detect(&data, Some(&filename));
    let data = if compress_mode == "auto" || compress_mode == "zstd" {
        let compressed = compress(&data, file_type)?;
        if compressed.len() < data.len() {
            println!("✓ Compressed: {} → {} bytes", data.len(), compressed.len());
            compressed
        } else {
            data
        }
    } else {
        data
    };
    
    // Step 2: Compute file hash
    let file_hash = hash_file(&data);
    let hash_prefix = hash_prefix(&file_hash);
    println!("✓ File hash: {}", hex::encode(&file_hash));
    
    // Step 3: Generate capability advertisement
    let adv = CapabilityAdvertisement::default_advertisement();
    println!("✓ Protocol version: {}.{}", 
        qr_protocol::PROTOCOL_VERSION_MAJOR,
        qr_protocol::PROTOCOL_VERSION_MINOR
    );
    println!("✓ Grid layout: {}x{}", grid.rows, grid.cols);
    
    // Step 4: Setup fountain encoder
    let session_secret = [0x42u8; 32]; // In real impl, this comes from key exchange
    let fountain_seed = derive_fountain_seed(&session_secret)?;
    let chunk_size = qr_protocol::MAX_FRAME_PAYLOAD - 64; // Leave room for fountain overhead
    let chunks = chunk_file(&data, chunk_size);
    let total_chunks = chunks.len() as u32;
    
    println!("✓ Chunks: {} ({} bytes each)", total_chunks, chunk_size);
    
    let mut encoder = FountainEncoder::new(&data, chunk_size, &fountain_seed)?;
    
    // Step 5: Display QR codes
    println!("\n📱 Displaying animated QR codes...");
    println!("   Press Ctrl+C to stop\n");
    
    // Setup progress bar
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message("Streaming...");
    
    let mut block_count = 0;
    let generator = QrGenerator::new(40, ecc)
        .with_module_size(4);
    
    // Generate and display frames in a loop
    loop {
        let blocks = encoder.next_blocks(grid.total_cells());
        
        if terminal {
            // Terminal display: show ASCII QR
            for block in &blocks {
                let frame = Frame::new(
                    block.block_id,
                    total_chunks,
                    hash_prefix,
                    0,
                    block.data.clone(),
                )?;
                
                let serialized = frame.serialize();
                display_terminal_qr(&serialized)?;
                
                block_count += 1;
                pb.set_position((block_count % 100) as u64);
                
                // Small delay between frames
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                
                // Clear screen
                print!("\x1B[2J\x1B[H");
            }
        }
    }
}

fn display_terminal_qr(data: &[u8]) -> anyhow::Result<()> {
    use qrcode::{QrCode, EcLevel};
    
    let code = QrCode::with_error_correction_level(data, EcLevel::M)?;
    let string = code.render::<char>()
        .quiet_zone(true)
        .module_dimensions(1, 1)
        .build();
    
    println!("{}", string);
    Ok(())
}

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }
}