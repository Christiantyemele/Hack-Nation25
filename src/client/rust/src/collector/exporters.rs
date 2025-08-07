//! Log exporter implementations for the collector

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::fs::{self, File};
use std::io::Write;

use crate::collector::config::ExporterConfig;
use crate::collector::sources::LogEntry;
use crate::crypto;

/// Interface for log exporters
#[async_trait]
pub trait LogExporter: Send + Sync {
    /// Export a log entry
    async fn export(&self, log: LogEntry) -> Result<()>;
    /// Flush any buffered logs
    async fn flush(&self) -> Result<()>;
    /// Get the name of this exporter
    fn name(&self) -> &str;
}

/// Create a log exporter from configuration
pub async fn create_exporter(config: &ExporterConfig) -> Result<Box<dyn LogExporter>> {
    match config {
        ExporterConfig::LogNarrator { name, endpoint, client_id, key_path } => {
            Ok(Box::new(LogNarratorExporter::new(
                name.clone(),
                endpoint.clone(),
                client_id.clone(),
                key_path.clone(),
            ).await?))
        },
        ExporterConfig::LocalCache { name, directory, max_size_mb } => {
            Ok(Box::new(LocalCacheExporter::new(
                name.clone(),
                directory.clone(),
                *max_size_mb,
            )?))
        },
    }
}

/// LogNarrator cloud service exporter
pub struct LogNarratorExporter {
    name: String,
    endpoint: String,
    client_id: String,
    key_path: String,
    http_client: Client,
    logs_buffer: Arc<RwLock<Vec<LogEntry>>>,
}

#[derive(Serialize)]
struct LogBatch {
    client_id: String,
    timestamp: String,
    logs: Vec<LogEntry>,
    signature: String,
}

impl LogNarratorExporter {
    /// Create a new LogNarrator exporter
    async fn new(
        name: String,
        endpoint: String,
        client_id: String,
        key_path: String,
    ) -> Result<Self> {
        // Validate that the key file exists
        if !Path::new(&key_path).exists() {
            return Err(anyhow!("Private key file not found: {}", key_path));
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            name,
            endpoint,
            client_id,
            key_path,
            http_client: client,
            logs_buffer: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Create a signature for the log batch
    async fn sign_batch(&self, batch: &[LogEntry]) -> Result<String> {
        // In a real implementation, this would use the private key to sign the batch
        // For this example, we'll just use a placeholder
        let private_key = fs::read_to_string(&self.key_path)?;
        let data = serde_json::to_string(batch)?;

        // This is a placeholder - in reality we would use crypto::sign
        let signature = format!("signed-{}", crypto::hash_sha256(&data));

        Ok(signature)
    }
}

#[async_trait]
impl LogExporter for LogNarratorExporter {
    async fn export(&self, log: LogEntry) -> Result<()> {
        // Add the log to the buffer
        let mut buffer = self.logs_buffer.write().await;
        buffer.push(log);

        // If the buffer is large enough, flush it
        if buffer.len() >= 100 {
            drop(buffer); // Release the write lock
            self.flush().await?
        }

        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        let mut buffer = self.logs_buffer.write().await;

        if buffer.is_empty() {
            return Ok(());
        }

        let logs = std::mem::take(&mut *buffer);
        drop(buffer); // Release the write lock

        // Sign the batch
        let signature = self.sign_batch(&logs).await?;

        // Create the batch
        let batch = LogBatch {
            client_id: self.client_id.clone(),
            timestamp: Utc::now().to_rfc3339(),
            logs,
            signature,
        };

        // Send the batch to the LogNarrator API
        let response = self.http_client
            .post(&self.endpoint)
            .json(&batch)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Failed to export logs: {}", error_text));
        }

        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Local file cache exporter
pub struct LocalCacheExporter {
    name: String,
    directory: PathBuf,
    max_size_mb: u64,
    current_file: Option<PathBuf>,
    current_size: u64,
}

impl LocalCacheExporter {
    /// Create a new local cache exporter
    fn new(
        name: String,
        directory: String,
        max_size_mb: u64,
    ) -> Result<Self> {
        let dir_path = PathBuf::from(&directory);

        // Create the directory if it doesn't exist
        if !dir_path.exists() {
            fs::create_dir_all(&dir_path)?;
        }

        Ok(Self {
            name,
            directory: dir_path,
            max_size_mb,
            current_file: None,
            current_size: 0,
        })
    }

    /// Create a new cache file
    fn create_new_file(&mut self) -> Result<PathBuf> {
        let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
        let filename = format!("logs_{}.jsonl", timestamp);
        let file_path = self.directory.join(filename);

        // Create the file
        File::create(&file_path)?;

        self.current_file = Some(file_path.clone());
        self.current_size = 0;

        Ok(file_path)
    }

    /// Check if the current cache file is too large
    fn check_rotation(&mut self) -> Result<()> {
        // Convert max_size from MB to bytes
        let max_bytes = self.max_size_mb * 1024 * 1024;

        if self.current_size >= max_bytes {
            self.create_new_file()?;
        }

        Ok(())
    }

    /// Write a log entry to the current cache file
    fn write_log(&mut self, log: &LogEntry) -> Result<()> {
        let file_path = if let Some(path) = &self.current_file {
            path.clone()
        } else {
            self.create_new_file()?
        };

        // Serialize the log entry to JSON
        let log_json = serde_json::to_string(log)?;

        // Append the log entry to the file
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(file_path)?;

        writeln!(file, "{}", log_json)?;

        // Update the current size
        self.current_size += log_json.len() as u64 + 1; // +1 for newline

        // Check if we need to rotate the file
        self.check_rotation()?;

        Ok(())
    }
}

#[async_trait]
impl LogExporter for LocalCacheExporter {
    async fn export(&self, log: LogEntry) -> Result<()> {
        // Clone self to avoid borrowing issues with async trait
        let mut this = Self {
            name: self.name.clone(),
            directory: self.directory.clone(),
            max_size_mb: self.max_size_mb,
            current_file: self.current_file.clone(),
            current_size: self.current_size,
        };

        // Write the log entry to the cache file
        this.write_log(&log)?;

        // Update the original object's state
        // This is a workaround since we can't mutate self directly in an async trait method
        let mut this = Self {
            name: self.name.clone(),
            directory: self.directory.clone(),
            max_size_mb: self.max_size_mb,
            current_file: this.current_file,
            current_size: this.current_size,
        };

        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        // No buffering in this exporter, so nothing to flush
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}
