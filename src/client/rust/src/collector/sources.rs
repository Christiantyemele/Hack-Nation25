//! Log source implementations for the collector

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use tower_http::cors::CorsLayer;
use std::convert::Infallible;
use std::net::SocketAddr;

use crate::collector::config::{SourceConfig, StartAt};

/// A log entry collected from a source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Timestamp when the log was generated
    pub timestamp: DateTime<Utc>,
    /// Source that generated the log
    pub source: String,
    /// Log level or severity
    pub level: Option<String>,
    /// Log message content
    pub message: String,
    /// Additional attributes/metadata
    pub attributes: HashMap<String, String>,
}

/// Channel for sending log entries
pub type LogSender = mpsc::Sender<LogEntry>;

/// Interface for log sources
#[async_trait]
pub trait LogSource: Send + Sync {
    /// Start collecting logs
    async fn start(&mut self, sender: LogSender) -> Result<()>;
    /// Stop collecting logs
    async fn stop(&mut self) -> Result<()>;
    /// Get the name of this source
    fn name(&self) -> &str;
}

/// Create a log source from configuration
pub async fn create_source(config: &SourceConfig) -> Result<Box<dyn LogSource>> {
    match config {
        SourceConfig::File { name, include, exclude_filename_pattern, start_at } => {
            Ok(Box::new(FileSource::new(
                name.clone(),
                include.clone(),
                exclude_filename_pattern.clone(),
                *start_at,
            )?))
        },
        #[cfg(target_os = "linux")]
        SourceConfig::Journald { name, directory, units } => {
            Ok(Box::new(JournaldSource::new(
                name.clone(),
                directory.clone(),
                units.clone(),
            )?))
        },
        SourceConfig::Docker { name, containers, all_containers } => {
            Ok(Box::new(DockerSource::new(
                name.clone(),
                containers.clone(),
                *all_containers,
            )?))
        },
        SourceConfig::Otlp { name, port, interface } => {
            Ok(Box::new(OtlpSource::new(
                name.clone(),
                *port,
                interface.clone(),
            )?))
        },
    }
}

/// File-based log source
pub struct FileSource {
    name: String,
    file_paths: Vec<PathBuf>,
    exclude_pattern: Option<regex::Regex>,
    start_at: StartAt,
    running: bool,
}

impl FileSource {
    /// Create a new file source
    pub fn new(
        name: String,
        include: Vec<String>,
        exclude_pattern: Option<String>,
        start_at: StartAt,
    ) -> Result<Self> {
        let exclude_regex = match exclude_pattern {
            Some(pattern) => Some(regex::Regex::new(&pattern)?),
            None => None,
        };

        let file_paths = include
            .iter()
            .map(|path| PathBuf::from(path))
            .collect();

        Ok(Self {
            name,
            file_paths,
            exclude_pattern: exclude_regex,
            start_at,
            running: false,
        })
    }
}

#[async_trait]
impl LogSource for FileSource {
    async fn start(&mut self, sender: LogSender) -> Result<()> {
        if self.running {
            return Err(anyhow!("Source already running"));
        }

        self.running = true;

        // Setup file watchers and start collecting logs
        // Implementation will monitor files and send logs to the sender channel

        // For each file path
        for file_path in &self.file_paths {
            // Skip if file matches exclude pattern
            if let Some(ref pattern) = self.exclude_pattern {
                if let Some(file_name) = file_path.file_name() {
                    if let Some(name_str) = file_name.to_str() {
                        if pattern.is_match(name_str) {
                            continue;
                        }
                    }
                }
            }

            // Start a task to monitor this file
            // This is just a placeholder - actual implementation would be more complex
            let path = file_path.clone();
            let source_name = self.name.clone();
            let sender_clone = sender.clone();
            let start_at = self.start_at;

            tokio::spawn(async move {
                // Real implementation would use proper file monitoring
                // This is just a placeholder for the structure
                tracing::info!("Monitoring file: {:?}", path);

                // Example log entry creation
                let log = LogEntry {
                    timestamp: Utc::now(),
                    source: source_name.clone(),
                    level: Some("INFO".to_string()),
                    message: format!("Started monitoring file: {:?}", path),
                    attributes: HashMap::new(),
                };

                // Send the log entry
                if let Err(e) = sender_clone.send(log).await {
                    tracing::error!("Failed to send log: {}", e);
                }
            });
        }

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if !self.running {
            return Err(anyhow!("Source not running"));
        }

        self.running = false;
        // Stop file watchers and clean up resources

        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(target_os = "linux")]
/// Journald log source (Linux only)
pub struct JournaldSource {
    name: String,
    directory: Option<String>,
    units: Vec<String>,
    running: bool,
}

#[cfg(target_os = "linux")]
impl JournaldSource {
    /// Create a new journald source
    pub fn new(
        name: String,
        directory: Option<String>,
        units: Vec<String>,
    ) -> Result<Self> {
        Ok(Self {
            name,
            directory,
            units,
            running: false,
        })
    }
}

#[cfg(target_os = "linux")]
#[async_trait]
impl LogSource for JournaldSource {
    async fn start(&mut self, sender: LogSender) -> Result<()> {
        if self.running {
            return Err(anyhow!("Source already running"));
        }

        self.running = true;

        // Setup journal monitoring and start collecting logs
        // Implementation will monitor journald and send logs to the sender channel

        let source_name = self.name.clone();
        let units = self.units.clone();
        let directory = self.directory.clone();

        tokio::spawn(async move {
            // Real implementation would use systemd journal API
            // This is just a placeholder for the structure
            tracing::info!("Monitoring journald for units: {:?}", units);

            // Example log entry creation
            let log = LogEntry {
                timestamp: Utc::now(),
                source: source_name.clone(),
                level: Some("INFO".to_string()),
                message: format!("Started monitoring journald for units: {:?}", units),
                attributes: HashMap::new(),
            };

            // Send the log entry
            if let Err(e) = sender.send(log).await {
                tracing::error!("Failed to send log: {}", e);
            }
        });

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if !self.running {
            return Err(anyhow!("Source not running"));
        }

        self.running = false;
        // Stop journal monitoring and clean up resources

        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Docker container log source
pub struct DockerSource {
    name: String,
    containers: Vec<String>,
    all_containers: bool,
    running: bool,
}

impl DockerSource {
    /// Create a new Docker log source
    pub fn new(
        name: String,
        containers: Vec<String>,
        all_containers: bool,
    ) -> Result<Self> {
        Ok(Self {
            name,
            containers,
            all_containers,
            running: false,
        })
    }
}

#[async_trait]
impl LogSource for DockerSource {
    async fn start(&mut self, sender: LogSender) -> Result<()> {
        if self.running {
            return Err(anyhow!("Source already running"));
        }

        self.running = true;

        // Setup Docker API client and start collecting logs
        // Implementation will monitor Docker containers and send logs to the sender channel

        let source_name = self.name.clone();
        let containers = self.containers.clone();
        let all_containers = self.all_containers;

        tokio::spawn(async move {
            // Real implementation would use Docker API
            // This is just a placeholder for the structure
            tracing::info!("Monitoring Docker containers: {:?}, all: {}", containers, all_containers);

            // Example log entry creation
            let log = LogEntry {
                timestamp: Utc::now(),
                source: source_name.clone(),
                level: Some("INFO".to_string()),
                message: format!("Started monitoring Docker containers: {:?}", containers),
                attributes: HashMap::new(),
            };

            // Send the log entry
            if let Err(e) = sender.send(log).await {
                tracing::error!("Failed to send log: {}", e);
            }
        });

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if !self.running {
            return Err(anyhow!("Source not running"));
        }

        self.running = false;
        // Stop Docker monitoring and clean up resources

        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// OpenTelemetry Protocol HTTP receiver source
pub struct OtlpSource {
    name: String,
    port: u16,
    interface: String,
    running: bool,
}

impl OtlpSource {
    /// Create a new OTLP source
    pub fn new(
        name: String,
        port: u16,
        interface: String,
    ) -> Result<Self> {
        Ok(Self {
            name,
            port,
            interface,
            running: false,
        })
    }
}

#[async_trait]
impl LogSource for OtlpSource {
    async fn start(&mut self, sender: LogSender) -> Result<()> {
        if self.running {
            return Err(anyhow!("Source already running"));
        }

        self.running = true;

        let source_name = self.name.clone();
        let port = self.port;
        let interface = self.interface.clone();

        tokio::spawn(async move {
            if let Err(e) = start_otlp_server(interface, port, source_name, sender).await {
                tracing::error!("OTLP server error: {}", e);
            }
        });

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if !self.running {
            return Err(anyhow!("Source not running"));
        }

        self.running = false;
        // Stop HTTP server and clean up resources

        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Start an OTLP HTTP server to receive logs
async fn start_otlp_server(
    interface: String,
    port: u16,
    source_name: String,
    sender: LogSender,
) -> Result<()> {
    let addr: SocketAddr = format!("{}:{}", interface, port).parse()?;
    
    tracing::info!("Starting OTLP receiver on {}", addr);

    // Clone values before moving into closure
    let sender_clone = sender.clone();
    let source_name_clone = source_name.clone();

    // Create a service that handles OTLP requests
    let make_svc = make_service_fn(move |_conn| {
        let sender = sender_clone.clone();
        let source_name = source_name_clone.clone();
        
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_otlp_request(req, sender.clone(), source_name.clone())
            }))
        }
    });

    // Start the HTTP server
    let server = Server::bind(&addr).serve(make_svc);

    // Send a startup log entry
    let startup_log = LogEntry {
        timestamp: Utc::now(),
        source: source_name.clone(),
        level: Some("INFO".to_string()),
        message: format!("OTLP receiver started on {}", addr),
        attributes: HashMap::new(),
    };
    
    if let Err(e) = sender.send(startup_log).await {
        tracing::error!("Failed to send startup log: {}", e);
    }

    // Run the server
    if let Err(e) = server.await {
        tracing::error!("OTLP server error: {}", e);
    }

    Ok(())
}

/// Handle an OTLP HTTP request
async fn handle_otlp_request(
    req: Request<Body>,
    sender: LogSender,
    source_name: String,
) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/v1/logs") => {
            // Handle OTLP logs endpoint
            match process_otlp_logs(req, sender, source_name).await {
                Ok(response) => Ok(response),
                Err(e) => {
                    tracing::error!("Error processing OTLP logs: {}", e);
                    Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from("Internal server error"))
                        .unwrap())
                }
            }
        }
        (&Method::GET, "/health") => {
            // Health check endpoint
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from("OK"))
                .unwrap())
        }
        _ => {
            // Not found
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not found"))
                .unwrap())
        }
    }
}

/// Process OTLP logs from the request body
async fn process_otlp_logs(
    req: Request<Body>,
    sender: LogSender,
    source_name: String,
) -> Result<Response<Body>> {
    // Read the request body
    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
    
    // For now, we'll create a simple log entry from the raw OTLP data
    // In a full implementation, this would parse the OTLP protobuf format
    let log_entry = LogEntry {
        timestamp: Utc::now(),
        source: source_name,
        level: Some("INFO".to_string()),
        message: format!("Received OTLP log data ({} bytes)", body_bytes.len()),
        attributes: {
            let mut attrs = HashMap::new();
            attrs.insert("otlp_size".to_string(), body_bytes.len().to_string());
            attrs.insert("content_type".to_string(), "application/x-protobuf".to_string());
            attrs
        },
    };

    // Send the log entry to the pipeline
    if let Err(e) = sender.send(log_entry).await {
        tracing::error!("Failed to send OTLP log entry: {}", e);
        return Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Failed to process log"))
            .unwrap());
    }

    // Return success response
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("OK"))
        .unwrap())
}
