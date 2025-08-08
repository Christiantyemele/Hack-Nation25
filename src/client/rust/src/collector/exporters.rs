//! Log exporter implementations for the collector

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use std::fs::{self, File};
use std::io::Write;
use std::time::Duration;
use tokio::time::{interval, Instant};

use crate::collector::config::ExporterConfig;
use crate::collector::sources::LogEntry;
use crate::crypto;
use crate::db::Database;

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
        ExporterConfig::LogNarrator { name, endpoint, client_id, key_path, batch_size, flush_interval_seconds } => {
            Ok(Box::new(LogNarratorExporter::new(
                name.clone(),
                endpoint.clone(),
                client_id.clone(),
                key_path.clone(),
                batch_size.unwrap_or(100),
                flush_interval_seconds.unwrap_or(30),
            ).await?))
        },
        ExporterConfig::LocalCache { name, directory, max_size_mb } => {
            Ok(Box::new(LocalCacheExporter::new(
                name.clone(),
                directory.clone(),
                *max_size_mb,
            )?))
        },
        ExporterConfig::Database { name, db_path, batch_size } => {
            Ok(Box::new(DatabaseExporter::new(
                name.clone(),
                db_path.clone(),
                batch_size.unwrap_or(100),
            ).await?))
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
    batch_size: u32,
    flush_interval_seconds: u64,
    last_flush: Arc<RwLock<Instant>>,
}

#[derive(Serialize)]
struct LogBatch {
    client_id: String,
    timestamp: String,
    logs: Vec<LogEntry>,
    signature: String,
}

#[derive(Serialize)]
struct EncryptedData {
    client_id: String,
    timestamp: i64,
    version: i32,
    algorithm: String,
    nonce: String,
    data: String,
    compressed: bool,
}

impl LogNarratorExporter {
    /// Create a new LogNarrator exporter
    async fn new(
        name: String,
        endpoint: String,
        client_id: String,
        key_path: String,
        batch_size: u32,
        flush_interval_seconds: u64,
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
            batch_size,
            flush_interval_seconds,
            last_flush: Arc::new(RwLock::new(Instant::now())),
        })
    }

    /// Convert LogEntry to server-compatible format and encrypt
    async fn encrypt_batch(&self, batch: &[LogEntry]) -> Result<Vec<u8>> {
        // Convert LogEntry format to server-expected LogRecord format
        let log_records: Vec<serde_json::Value> = batch.iter().map(|entry| {
            serde_json::json!({
                "timestamp": entry.timestamp.timestamp_millis(),
                "severity": entry.level.as_ref().unwrap_or(&"INFO".to_string()).to_uppercase(),
                "body": entry.message,
                "attributes": entry.attributes,
                "resource": {
                    "source": entry.source
                },
                "trace_id": null,
                "span_id": null,
                "severity_num": match entry.level.as_ref().unwrap_or(&"INFO".to_string()).to_uppercase().as_str() {
                    "TRACE" => 1,
                    "DEBUG" => 5,
                    "INFO" => 9,
                    "WARN" => 13,
                    "ERROR" => 17,
                    "FATAL" => 21,
                    _ => 9
                }
            })
        }).collect();

        // Create the batch in server-expected format
        let log_batch = serde_json::json!({
            "records": log_records
        });

        // Serialize to JSON
        let data = serde_json::to_string(&log_batch)?;
        let data_bytes = data.as_bytes();

        // Read the private key
        let secret_key = crypto::read_secret_key(&self.key_path)?;
        
        // Sign the data
        let signed_data = crypto::sign(data_bytes, &secret_key);

        Ok(signed_data)
    }
}

#[async_trait]
impl LogExporter for LogNarratorExporter {
    async fn export(&self, log: LogEntry) -> Result<()> {
        // Add the log to the buffer
        let mut buffer = self.logs_buffer.write().await;
        buffer.push(log);

        // Check if we should flush based on buffer size
        let should_flush_by_size = buffer.len() >= self.batch_size as usize;
        
        // Check if we should flush based on time
        let last_flush = *self.last_flush.read().await;
        let should_flush_by_time = last_flush.elapsed() >= Duration::from_secs(self.flush_interval_seconds);
        
        drop(buffer); // Release the write lock

        // Flush if either condition is met
        if should_flush_by_size || should_flush_by_time {
            self.flush().await?;
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

        // Encrypt and sign the batch
        let encrypted_data = self.encrypt_batch(&logs).await?;

        // Create the encrypted batch payload
        let batch = EncryptedData {
            client_id: self.client_id.clone(),
            timestamp: Utc::now().timestamp_millis(),
            version: 1,
            algorithm: "nacl.signing".to_string(),
            nonce: "".to_string(), // Not used for signing
            data: base64::encode(&encrypted_data),
            compressed: false,
        };

        // Send the batch to the LogNarrator API with encrypted content type
        let response = self.http_client
            .post(&self.endpoint)
            .header("Content-Type", "application/json+encrypted")
            .json(&batch)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Failed to export logs: {}", error_text));
        }

        // Update the last flush timestamp
        *self.last_flush.write().await = Instant::now();
        
        tracing::debug!("Successfully exported {} logs", logs.len());
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
    state: Arc<Mutex<LocalCacheState>>,
}

/// Mutable state for LocalCacheExporter
struct LocalCacheState {
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

        let state = Arc::new(Mutex::new(LocalCacheState {
            current_file: None,
            current_size: 0,
        }));

        Ok(Self {
            name,
            directory: dir_path,
            max_size_mb,
            state,
        })
    }

    /// Create a new cache file
    async fn create_new_file(&self, state: &mut LocalCacheState) -> Result<PathBuf> {
        let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
        let filename = format!("logs_{}.jsonl", timestamp);
        let file_path = self.directory.join(filename);

        // Create the file
        File::create(&file_path)?;

        state.current_file = Some(file_path.clone());
        state.current_size = 0;

        Ok(file_path)
    }

    /// Check if the current cache file is too large
    async fn check_rotation(&self, state: &mut LocalCacheState) -> Result<()> {
        // Convert max_size from MB to bytes
        let max_bytes = self.max_size_mb * 1024 * 1024;

        if state.current_size >= max_bytes {
            self.create_new_file(state).await?;
        }

        Ok(())
    }

    /// Write a log entry to the current cache file
    async fn write_log(&self, state: &mut LocalCacheState, log: &LogEntry) -> Result<()> {
        let file_path = if let Some(path) = &state.current_file {
            path.clone()
        } else {
            self.create_new_file(state).await?
        };

        // Serialize the log entry to JSON
        let log_json = serde_json::to_string(log)?;

        // Append the log entry to the file
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(file_path)?;

        writeln!(file, "{}", log_json)?;

        // Update the current size
        state.current_size += log_json.len() as u64 + 1; // +1 for newline

        // Check if we need to rotate the file
        self.check_rotation(state).await?;

        Ok(())
    }
}

#[async_trait]
impl LogExporter for LocalCacheExporter {
    async fn export(&self, log: LogEntry) -> Result<()> {
        let mut state = self.state.lock().await;
        self.write_log(&mut state, &log).await?;
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

/// SQLite database exporter
pub struct DatabaseExporter {
    name: String,
    db: Arc<Mutex<Database>>,
    logs_buffer: Arc<RwLock<Vec<LogEntry>>>,
    batch_size: u32,
}

impl DatabaseExporter {
    /// Create a new database exporter
    pub async fn new(name: String, db_path: String, batch_size: u32) -> Result<Self> {
        let db = Database::open(&db_path)?;
        
        Ok(Self {
            name,
            db: Arc::new(Mutex::new(db)),
            logs_buffer: Arc::new(RwLock::new(Vec::new())),
            batch_size,
        })
    }

    /// Insert logs into the database
    async fn insert_logs(&self, logs: &[LogEntry]) -> Result<()> {
        let db = self.db.lock().await;
        
        for log in logs {
            // Convert collector::sources::LogEntry to db::LogEntry
            let db_log_entry = crate::db::LogEntry {
                id: None,
                timestamp: log.timestamp.timestamp_millis(),
                source: log.source.clone(),
                content: serde_json::to_string(log)?,
                encrypted: false,
                sent: false,
            };
            
            db.store_log(&db_log_entry)?;
        }
        
        Ok(())
    }
}

#[async_trait]
impl LogExporter for DatabaseExporter {
    async fn export(&self, log: LogEntry) -> Result<()> {
        // Add the log to the buffer
        let mut buffer = self.logs_buffer.write().await;
        buffer.push(log);

        // If the buffer is large enough, flush it
        if buffer.len() >= self.batch_size as usize {
            let logs_to_insert = buffer.clone();
            buffer.clear();
            drop(buffer); // Release the write lock

            self.insert_logs(&logs_to_insert).await?;
        }

        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        let mut buffer = self.logs_buffer.write().await;

        if !buffer.is_empty() {
            let logs_to_insert = buffer.clone();
            buffer.clear();
            drop(buffer); // Release the write lock

            self.insert_logs(&logs_to_insert).await?;
        }

        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}
