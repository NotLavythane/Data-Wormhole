//! Benchmark command - performance testing

use indicatif::{ProgressBar, ProgressStyle};
use std::time::{Duration, Instant};

pub async fn execute(decoder: String, fps: u32) -> anyhow::Result<()> {
    println!("⚡ QR Transfer Benchmark");
    println!("=======================");
    println!("Decoder: {}", decoder);
    println!("Target FPS: {}", fps);
    println!();
    
    // Benchmark 1: Key generation
    println!("Benchmarking key generation...");
    let start = Instant::now();
    let iterations = 100;
    for _ in 0..iterations {
        let _ = qr_crypto::generate_ephemeral_keypair();
    }
    let keygen_time = start.elapsed() / iterations;
    println!("  X25519 keypair: {:?} ({} ops/sec)", 
        keygen_time,
        1_000_000_000u64 / keygen_time.as_nanos().max(1) as u64
    );
    
    // Benchmark 2: BLAKE3 hashing
    println!("\nBenchmarking BLAKE3 hashing...");
    let test_sizes = [1024, 1024 * 1024, 10 * 1024 * 1024];
    for size in &test_sizes {
        let data = vec![0x42u8; *size];
        let start = Instant::now();
        let iterations = (100_000_000 / size.max(&1)).max(10) as usize;
        for _ in 0..iterations.min(1000) {
            let _ = qr_crypto::hash_file(&data);
        }
        let elapsed = start.elapsed();
        let throughput = (*size as f64 * iterations.min(1000) as f64) / elapsed.as_secs_f64();
        println!("  {} MB: {:.0} MB/s", size / (1024 * 1024), throughput / (1024.0 * 1024.0));
    }
    
    // Benchmark 3: AES-GCM encryption
    println!("\nBenchmarking AES-GCM encryption...");
    let key = [0x42u8; 32];
    let iv = [0x12u8; 12];
    let plaintext_sizes = [1900, 10000, 100000];
    for size in &plaintext_sizes {
        let data = vec![0x42u8; *size];
        let start = Instant::now();
        let iterations = 1000;
        for _ in 0..iterations {
            let _ = qr_crypto::encrypt_chunk(&key, &iv, &data[..1900.min(*size)]);
        }
        let elapsed = start.elapsed();
        let throughput = (*size as f64 * iterations as f64) / elapsed.as_secs_f64();
        println!("  {} KB: {:.0} KB/s", size / 1024, throughput / 1024.0);
    }
    
    // Benchmark 4: Fountain encoding
    println!("\nBenchmarking fountain codes...");
    let data = vec![0x42u8; 1_000_000]; // 1MB
    let seed = [0x42u8; 32];
    let start = Instant::now();
    let mut encoder = qr_fountain::FountainEncoder::new(&data, 1900, &seed)?;
    let init_time = start.elapsed();
    println!("  Encoder init: {:?}", init_time);
    
    let start = Instant::now();
    let block_count = 1000;
    for _ in 0..block_count {
        let _ = encoder.next_block();
    }
    let encode_time = start.elapsed();
    let throughput = (1900.0 * block_count as f64) / encode_time.as_secs_f64();
    println!("  Encode throughput: {:.0} MB/s", throughput / (1024.0 * 1024.0));
    
    println!("\n✅ Benchmark complete");
    
    Ok(())
}