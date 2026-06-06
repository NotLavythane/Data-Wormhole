//! Verify command - verify file against BLAKE3 hash

use qr_crypto::hash_file;
use std::path::PathBuf;

pub async fn execute(file: PathBuf, expected_hash: String) -> anyhow::Result<()> {
    println!("🔍 Verifying file integrity...");
    println!("File: {}", file.display());
    
    let data = tokio::fs::read(&file).await?;
    let actual_hash = hash_file(&data);
    let actual_hex = hex::encode(&actual_hash);
    
    println!("Expected hash: {}", expected_hash);
    println!("Actual hash:   {}", actual_hex);
    
    if actual_hex.eq_ignore_ascii_case(&expected_hash) {
        println!("\n✅ File integrity verified!");
    } else {
        println!("\n❌ Hash mismatch! File may be corrupted or tampered with.");
        std::process::exit(1);
    }
    
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