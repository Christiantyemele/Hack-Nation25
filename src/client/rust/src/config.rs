//! Configuration handling for the MCP client

use anyhow::Result;
use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::path::Path;

/// Main configuration structure for the MCP client
#[derive(Debug, Deserialize, Clone)]
pub struct McpConfig {
    pub server: ServerConfig,
    pub security: SecurityConfig,
    pub database: DatabaseConfig,
    pub actions: ActionsConfig,
}

/// Server connection configuration
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    /// LogNarrator API endpoint URL
    pub api_url: String,
    /// API connection timeout in seconds
    pub timeout_seconds: u64,
    /// Client identifier UUID
    pub client_id: String,
}

/// Security configuration
#[derive(Debug, Deserialize, Clone)]
pub struct SecurityConfig {
    /// Path to the private key file
    pub private_key_path: String,
    /// Enable TLS certificate verification
    pub verify_certs: bool,
    /// Path to CA certificates if using custom CA
    pub ca_cert_path: Option<String>,
}

/// Database configuration for local storage
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    /// Path to the SQLite database file
    pub db_path: String,
    /// Maximum number of cached log entries
    pub max_cache_entries: usize,
}

/// Configuration for the actions subsystem
#[derive(Debug, Deserialize, Clone)]
pub struct ActionsConfig {
    /// Path to the actions definition directory
    pub actions_dir: String,
    /// Path to the permissions policy file
    pub permissions_path: String,
    /// Whether to require confirmation for high-risk actions
    pub require_confirmation: bool,
    /// Maximum time to wait for action execution in seconds
    pub execution_timeout: u64,
}

/// Load the configuration from a file
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<McpConfig> {
    let config = Config::builder()
        .add_source(File::with_name(path.as_ref().to_str().unwrap()))
        .add_source(config::Environment::with_prefix("MCP").separator("_"))
        .build()?
        .try_deserialize()?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_valid_config() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("config.yaml");

        let mut file = File::create(&config_path)?;
        write!(file, r#"
            server:
              api_url: "https://api.lognarrator.com"
              timeout_seconds: 30
              client_id: "12345678-1234-1234-1234-123456789012"
            security:
              private_key_path: "/app/config/private.key"
              verify_certs: true
            database:
              db_path: "/app/data/mcp.db"
              max_cache_entries: 10000
            actions:
              actions_dir: "/app/config/actions"
              permissions_path: "/app/config/permissions.yaml"
              require_confirmation: true
              execution_timeout: 60
        "#)?;

        let config = load_config(config_path)?;

        assert_eq!(config.server.api_url, "https://api.lognarrator.com");
        assert_eq!(config.server.timeout_seconds, 30);
        assert_eq!(config.security.verify_certs, true);
        assert_eq!(config.database.max_cache_entries, 10000);
        assert_eq!(config.actions.require_confirmation, true);

        Ok(())
    }
}
