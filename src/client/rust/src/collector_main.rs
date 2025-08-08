//! LogNarrator Log Collector
//! 
//! This binary provides the Phase 1B log collection functionality for LogNarrator.
//! It handles secure log collection from various sources, encryption, and transport
//! to the LogNarrator cloud service.

use anyhow::{Context, Result};
use clap::Parser;

mod config;
mod crypto;
mod db;
mod collector;

/// Command-line arguments for the log collector
#[derive(Parser, Debug)]
#[clap(author, version, about = "LogNarrator secure log collector")]
struct Args {
    /// Path to the collector configuration file
    #[clap(short, long, default_value = "/app/config/collector.yaml")]
    config: String,

    /// Enable verbose logging
    #[clap(short, long)]
    verbose: bool,

    /// Generate a new keypair and save to specified path
    #[clap(long)]
    generate_keys: Option<String>,
}

/// Collector configuration structure
#[derive(Debug, serde::Deserialize, Clone)]
pub struct LogCollectorConfig {
    pub collector: collector::config::CollectorConfig,
    pub encryption: EncryptionConfig,
    pub database: DatabaseConfig,
}

/// Encryption configuration
#[derive(Debug, serde::Deserialize, Clone)]
pub struct EncryptionConfig {
    /// Path to the private key file
    pub private_key_path: String,
    /// Path to the server's public key file
    pub server_public_key_path: String,
}

/// Database configuration
#[derive(Debug, serde::Deserialize, Clone)]
pub struct DatabaseConfig {
    /// Path to the SQLite database file
    pub db_path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Initialize logging
    init_logging(args.verbose)?;

    // Initialize cryptography
    crypto::init()?;

    // Handle key generation if requested
    if let Some(key_path) = args.generate_keys {
        return generate_keypair(&key_path).await;
    }

    // Load configuration
    let config = load_collector_config(&args.config)
        .context("Failed to load collector configuration")?;

    tracing::info!("Starting LogNarrator Log Collector");
    tracing::debug!("Loaded configuration from {}", args.config);

    // Initialize database
    let _db = db::Database::open(&config.database.db_path)
        .context("Failed to open database")?;

    // Create and start the collector pipeline
    let mut collector = collector::LogCollector::new(config.collector)
        .context("Failed to create log collector")?;

    // Setup graceful shutdown
    let shutdown_signal = setup_shutdown_signal();

    // Start the collector
    collector.start().await
        .context("Failed to start log collector")?;

    tracing::info!("Log collector started successfully");

    // Wait for shutdown signal
    shutdown_signal.await;

    tracing::info!("Shutdown signal received, stopping log collector");

    // Stop the collector
    collector.stop().await
        .context("Failed to stop log collector")?;

    tracing::info!("LogNarrator Log Collector stopped");
    Ok(())
}

/// Initialize the logging system based on verbosity level
fn init_logging(verbose: bool) -> Result<()> {
    let filter = if verbose {
        "debug".to_string()
    } else {
        std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string())
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();

    Ok(())
}

/// Load collector configuration from file
fn load_collector_config(path: &str) -> Result<LogCollectorConfig> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read configuration file")?;
    
    let config: LogCollectorConfig = serde_yaml::from_str(&content)
        .context("Failed to parse configuration file")?;
    
    Ok(config)
}

/// Generate a new keypair and save to files
async fn generate_keypair(base_path: &str) -> Result<()> {
    let (public_key, secret_key) = crypto::generate_keypair();
    
    let private_key_path = format!("{}.private", base_path);
    let public_key_path = format!("{}.public", base_path);
    
    crypto::write_secret_key(&private_key_path, &secret_key)
        .context("Failed to write private key")?;
    
    crypto::write_public_key(&public_key_path, &public_key)
        .context("Failed to write public key")?;
    
    println!("Generated keypair:");
    println!("  Private key: {}", private_key_path);
    println!("  Public key: {}", public_key_path);
    
    Ok(())
}

/// Setup graceful shutdown signal handling
async fn setup_shutdown_signal() {
    use tokio::signal;
    
    #[cfg(unix)]
    {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to register SIGTERM handler");
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
            .expect("Failed to register SIGINT handler");
        
        tokio::select! {
            _ = sigterm.recv() => {
                tracing::info!("Received SIGTERM");
            }
            _ = sigint.recv() => {
                tracing::info!("Received SIGINT");
            }
        }
    }
    
    #[cfg(not(unix))]
    {
        signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
        tracing::info!("Received Ctrl+C");
    }
}