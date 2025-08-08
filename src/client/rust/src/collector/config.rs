//! Configuration handling for the log collector module

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Main configuration structure for the log collector
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CollectorConfig {
    /// Log sources configuration
    pub sources: Vec<SourceConfig>,
    /// Log processors configuration
    pub processors: Vec<ProcessorConfig>,
    /// Exporters configuration (where to send logs)
    pub exporters: Vec<ExporterConfig>,
}

/// Configuration for log sources
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "source_type", rename_all = "lowercase")]
pub enum SourceConfig {
    /// File-based log source
    File {
        /// Unique name for the source
        name: String,
        /// List of file paths to include
        include: Vec<String>,
        /// Optional regex pattern to exclude files
        exclude_filename_pattern: Option<String>,
        /// Where to start reading (beginning or end of file)
        #[serde(default = "default_start_at")]
        start_at: StartAt,
    },
    /// Journald log source (Linux only)
    #[cfg(target_os = "linux")]
    Journald {
        /// Unique name for the source
        name: String,
        /// Optional journal directory path
        directory: Option<String>,
        /// List of systemd units to collect logs from
        units: Vec<String>,
    },
    /// Docker container logs
    Docker {
        /// Unique name for the source
        name: String,
        /// List of container names or IDs to collect logs from
        containers: Vec<String>,
        /// Whether to collect logs from all containers
        #[serde(default)]
        all_containers: bool,
    },
    /// OpenTelemetry Protocol HTTP receiver
    Otlp {
        /// Unique name for the source
        name: String,
        /// Port to listen on
        port: u16,
        /// Interface to bind to
        #[serde(default = "default_interface")]
        interface: String,
    },
}

/// Configuration for log processors
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "processor_type", rename_all = "lowercase")]
pub enum ProcessorConfig {
    /// Resource processor adds metadata to logs
    Resource {
        /// Unique name for the processor
        name: String,
        /// Attributes to add to logs
        attributes: Vec<AttributeAction>,
    },
    /// Filter processor includes or excludes logs based on patterns
    Filter {
        /// Unique name for the processor
        name: String,
        /// Filter configuration
        logs: FilterConfig,
    },
    /// Batch processor groups logs for efficient transmission
    Batch {
        /// Unique name for the processor
        name: String,
        /// Timeout before sending a batch (in seconds)
        timeout: u64,
        /// Maximum batch size
        send_batch_size: usize,
    },
    /// Transform processor modifies log content
    Transform {
        /// Unique name for the processor
        name: String,
        /// List of transformations to apply
        transforms: Vec<TransformAction>,
    },
}

/// Configuration for log exporters
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "exporter_type", rename_all = "lowercase")]
pub enum ExporterConfig {
    /// LogNarrator cloud service exporter
    LogNarrator {
        /// Unique name for the exporter
        name: String,
        /// API endpoint URL
        endpoint: String,
        /// Client identifier
        client_id: String,
        /// Path to private key for authentication
        key_path: String,
        /// Maximum number of logs to buffer before sending (default: 100)
        batch_size: Option<u32>,
        /// Interval in seconds to automatically flush logs (default: 30)
        flush_interval_seconds: Option<u64>,
    },
    /// Local file cache exporter
    LocalCache {
        /// Unique name for the exporter
        name: String,
        /// Directory path for the cache
        directory: String,
        /// Maximum cache size in MB
        max_size_mb: u64,
    },
    /// SQLite database exporter
    Database {
        /// Unique name for the exporter
        name: String,
        /// Path to the SQLite database file
        db_path: String,
        /// Maximum number of logs to buffer before writing
        batch_size: Option<u32>,
    },
}

/// Position to start reading logs from
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StartAt {
    /// Start from the beginning of the file
    Beginning,
    /// Start from the end of the file
    End,
}

/// Default value for start_at
fn default_start_at() -> StartAt {
    StartAt::End
}

/// Default interface to bind to
fn default_interface() -> String {
    "0.0.0.0".to_string()
}

/// Action to perform on an attribute
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AttributeAction {
    /// Action type
    pub action: ActionType,
    /// Attribute key
    pub key: String,
    /// Attribute value (supports environment variable interpolation)
    pub value: String,
}

/// Type of action to perform on an attribute
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    /// Insert a new attribute
    Insert,
    /// Update an existing attribute
    Update,
    /// Insert if not exists, otherwise update
    Upsert,
    /// Delete an attribute
    Delete,
}

/// Filter configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FilterConfig {
    /// Patterns to include
    pub include: Option<MatchConfig>,
    /// Patterns to exclude
    pub exclude: Option<MatchConfig>,
}

/// Match configuration for filters
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MatchConfig {
    /// Type of matching to perform
    pub match_type: MatchType,
    /// List of exact match strings (used if match_type is exact)
    pub exact: Option<Vec<String>>,
    /// List of regular expressions (used if match_type is regexp)
    pub regexp: Option<Vec<String>>,
}

/// Type of matching to perform
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MatchType {
    /// Exact string matching
    Exact,
    /// Regular expression matching
    Regexp,
}

/// Transform action to apply to logs
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransformAction {
    /// Type of transformation
    pub transform_type: TransformType,
    /// Field to transform
    pub field: String,
    /// Parameters for the transformation
    #[serde(default)]
    pub parameters: HashMap<String, String>,
}

/// Type of transformation to apply
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransformType {
    /// Mask sensitive information
    Mask,
    /// Extract fields using a regular expression
    Extract,
    /// Convert field to a different format
    Convert,
    /// Rename a field
    Rename,
}

/// Load collector configuration from a file
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<CollectorConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: CollectorConfig = serde_yaml::from_str(&content)?;
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
        let config_path = dir.path().join("collector.yaml");

        let mut file = File::create(&config_path)?;
        write!(file, r#"
            sources:
              - source_type: file
                name: system-logs
                include:
                  - /var/log/syslog
                  - /var/log/messages
                exclude_filename_pattern: '.*\.gz$'
                start_at: end
            processors:
              - processor_type: filter
                name: error-filter
                logs:
                  include:
                    match_type: regexp
                    regexp:
                      - '.*error.*'
                      - '.*warning.*'
            exporters:
              - exporter_type: lognarrator
                name: cloud-export
                endpoint: https://api.lognarrator.com
                client_id: test-client
                key_path: /app/config/private.key
        "#)?;

        let config = load_config(config_path)?;

        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.processors.len(), 1);
        assert_eq!(config.exporters.len(), 1);

        if let SourceConfig::File { name, include, .. } = &config.sources[0] {
            assert_eq!(name, "system-logs");
            assert_eq!(include.len(), 2);
        } else {
            panic!("Expected File source");
        }

        Ok(())
    }
}
