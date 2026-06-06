//! # qr-transfer CLI
//! 
//! Command-line reference implementation for QR Transfer.
//! 
//! Usage:
//!   qr-transfer send <file> [--grid 2x2] [--ecc M] [--compress auto]
//!   qr-transfer receive [--output ./] [--verify]
//!   qr-transfer keygen [--export ./identity.pem]
//!   qr-transfer verify <file> <hash>
//!   qr-transfer benchmark

use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use tracing::{info, warn};

mod commands;

use commands::*;

#[derive(Parser)]
#[command(name = "qr-transfer")]
#[command(about = "Decentralized encrypted file transfer via animated QR codes")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a file via animated QR codes
    Send {
        /// File to send
        file: PathBuf,
        
        /// Grid layout (1x1, 1x2, 2x2, 2x3, 3x3)
        #[arg(long, default_value = "1x1")]
        grid: String,
        
        /// Error correction level (L, M, Q, H)
        #[arg(long, default_value = "M")]
        ecc: String,
        
        /// Compression mode (auto, none, zstd)
        #[arg(long, default_value = "auto")]
        compress: String,
        
        /// Read from stdin instead of file
        #[arg(long)]
        stdin: bool,
        
        /// Display QR in terminal (ASCII art)
        #[arg(long)]
        terminal: bool,
    },
    
    /// Receive a file by scanning QR codes
    Receive {
        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        
        /// Verify file integrity after transfer
        #[arg(long)]
        verify: bool,
        
        /// Write to stdout instead of file
        #[arg(long)]
        stdout: bool,
    },
    
    /// Generate identity keypair
    Keygen {
        /// Export key to file
        #[arg(long)]
        export: Option<PathBuf>,
    },
    
    /// Verify file against BLAKE3 hash
    Verify {
        /// File to verify
        file: PathBuf,
        
        /// Expected BLAKE3 hash (hex)
        hash: String,
    },
    
    /// Run performance benchmark
    Benchmark {
        /// Decoder to benchmark (zbar, quirc)
        #[arg(long, default_value = "zbar")]
        decoder: String,
        
        /// Target FPS
        #[arg(long, default_value = "60")]
        fps: u32,
    },
    
    /// Show configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    Show,
    Set {
        key: String,
        value: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(if cli.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)?;
    
    match cli.command {
        Commands::Send { file, grid, ecc, compress, stdin, terminal } => {
            info!("Starting send command");
            let grid_layout = parse_grid(&grid)?;
            let ecc_level = parse_ecc(&ecc)?;
            
            let data = if stdin {
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut std::io::stdin(), &mut buf)?;
                buf
            } else {
                tokio::fs::read(&file).await?
            };
            
            let filename = if stdin {
                "stdin".to_string()
            } else {
                file.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            };
            
            commands::send::execute(
                data,
                filename,
                grid_layout,
                ecc_level,
                compress,
                terminal,
            ).await?;
        }
        
        Commands::Receive { output, verify, stdout } => {
            info!("Starting receive command");
            commands::receive::execute(output, verify, stdout).await?;
        }
        
        Commands::Keygen { export } => {
            info!("Generating identity keypair");
            commands::keygen::execute(export).await?;
        }
        
        Commands::Verify { file, hash } => {
            info!("Verifying file integrity");
            commands::verify::execute(file, hash).await?;
        }
        
        Commands::Benchmark { decoder, fps } => {
            info!("Running benchmark");
            commands::benchmark::execute(decoder, fps).await?;
        }
        
        Commands::Config { action } => {
            match action {
                Some(ConfigAction::Show) | None => {
                    println!("QR Transfer Configuration");
                    println!("========================");
                    println!("Protocol version: {}.{}", 
                        qr_protocol::PROTOCOL_VERSION_MAJOR,
                        qr_protocol::PROTOCOL_VERSION_MINOR
                    );
                    println!("Default QR version: {}", qr_protocol::DEFAULT_QR_VERSION);
                    println!("Max frame payload: {} bytes", qr_protocol::MAX_FRAME_PAYLOAD);
                }
                Some(ConfigAction::Set { key, value }) => {
                    println!("Setting {} = {} (not yet implemented)", key, value);
                }
            }
        }
    }
    
    Ok(())
}

fn parse_grid(grid: &str) -> anyhow::Result<qr_protocol::GridLayout> {
    let parts: Vec<&str> = grid.split('x').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid grid format. Use: ROWSxCOLS (e.g., 2x2)");
    }
    
    let rows: u8 = parts[0].parse()?;
    let cols: u8 = parts[1].parse()?;
    
    if rows < 1 || rows > 4 || cols < 1 || cols > 4 {
        anyhow::bail!("Grid dimensions must be 1-4");
    }
    
    Ok(qr_protocol::GridLayout { rows, cols })
}

fn parse_ecc(ecc: &str) -> anyhow::Result<qr_protocol::EccLevel> {
    match ecc.to_uppercase().as_str() {
        "L" => Ok(qr_protocol::EccLevel::L),
        "M" => Ok(qr_protocol::EccLevel::M),
        "Q" => Ok(qr_protocol::EccLevel::Q),
        "H" => Ok(qr_protocol::EccLevel::H),
        _ => anyhow::bail!("Invalid ECC level. Use: L, M, Q, or H"),
    }
}