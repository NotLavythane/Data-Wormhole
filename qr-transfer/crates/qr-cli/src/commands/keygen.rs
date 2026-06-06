//! Keygen command - generate Ed25519 identity keypair

use qr_crypto::IdentityKeyPair;
use std::path::PathBuf;

pub async fn execute(export: Option<PathBuf>) -> anyhow::Result<()> {
    println!("🔑 Generating Ed25519 identity keypair...");
    
    let identity = IdentityKeyPair::generate();
    let public_key = identity.public_key();
    
    let fingerprint = public_key.fingerprint();
    println!("✓ Public key fingerprint: {}", fingerprint);
    
    if let Some(path) = export {
        let key_bytes = identity.to_bytes();
        let hex_key = hex::encode(&key_bytes);
        
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        tokio::fs::write(&path, hex_key).await?;
        println!("✓ Private key exported to: {}", path.display());
        println!("⚠️  Keep this file secure!");
    }
    
    println!("\nYour identity fingerprint is: {}", fingerprint);
    println!("Share this with contacts to verify your identity.");
    
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