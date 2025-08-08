//! LogNarrator MCP Client
//! 
//! This binary provides the Multi-Command Protocol client for executing
//! secure, authorized actions on target systems based on LogNarrator analysis.

use anyhow::{Context, Result};
use clap::Parser;

mod config;
mod crypto;
mod db;
mod mcp;

/// Command-line arguments for the MCP client
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to the configuration file
    #[clap(short, long, default_value = "/app/config/mcp_client.yaml")]
    config: String,

    /// Enable verbose logging
    #[clap(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Initialize logging
    init_logging(args.verbose)?;

    // Load configuration
    let config = config::load_config(&args.config)
        .context("Failed to load configuration")?;

    tracing::info!("Starting LogNarrator MCP Client");
    tracing::debug!("Loaded configuration from {}", args.config);

    // TODO: Initialize MCP client components

    // Main service loop
    mcp::start_service(config).await?;

    tracing::info!("Shutting down LogNarrator MCP Client");
    Ok(())
}

/// Initialize the logging system based on verbosity level
fn init_logging(verbose: bool) -> Result<()> {
    let filter = if verbose {
        "debug"
    } else {
        &std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string())
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();

    Ok(())
}
