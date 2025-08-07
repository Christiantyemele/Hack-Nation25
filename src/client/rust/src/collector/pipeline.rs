//! Log processing pipeline implementation

use anyhow::{anyhow, Result};
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;

use crate::collector::config::CollectorConfig;
use crate::collector::exporters::{self, LogExporter};
use crate::collector::processors::{self, LogProcessor};
use crate::collector::sources::{self, LogSource, LogEntry, LogSender};

/// Pipeline for log processing
pub struct Pipeline {
    config: CollectorConfig,
    sources: Vec<Box<dyn LogSource>>,
    processors: Vec<Box<dyn LogProcessor>>,
    exporters: Vec<Box<dyn LogExporter>>,
    task_handles: Vec<JoinHandle<()>>,
    log_channel: (LogSender, mpsc::Receiver<LogEntry>),
    running: bool,
}

impl Pipeline {
    /// Create a new pipeline from configuration
    pub fn new(config: CollectorConfig) -> Result<Self> {
        let (sender, receiver) = mpsc::channel(1000); // Buffer up to 1000 log entries

        Ok(Self {
            config,
            sources: Vec::new(),
            processors: Vec::new(),
            exporters: Vec::new(),
            task_handles: Vec::new(),
            log_channel: (sender, receiver),
            running: false,
        })
    }

    /// Initialize the pipeline components
    async fn initialize(&mut self) -> Result<()> {
        // Initialize sources
        for source_config in &self.config.sources {
            let source = sources::create_source(source_config).await?;
            self.sources.push(source);
        }

        // Initialize processors
        for processor_config in &self.config.processors {
            let processor = processors::create_processor(processor_config)?;
            self.processors.push(processor);
        }

        // Initialize exporters
        for exporter_config in &self.config.exporters {
            let exporter = exporters::create_exporter(exporter_config).await?;
            self.exporters.push(exporter);
        }

        Ok(())
    }

    /// Start the log processor task
    async fn start_processor_task(&mut self) -> Result<()> {
        let processors = Arc::new(RwLock::new(self.processors.clone()));
        let exporters = Arc::new(RwLock::new(self.exporters.clone()));
        let mut receiver = self.log_channel.1.clone();

        // Start the processor task
        let handle = tokio::spawn(async move {
            while let Some(log) = receiver.recv().await {
                // Process the log through the processor chain
                let processors_guard = processors.read().await;
                let mut current_log = Some(log);

                for processor in processors_guard.iter() {
                    if let Some(log) = current_log {
                        match processor.process(log).await {
                            Ok(processed_log) => current_log = processed_log,
                            Err(e) => {
                                tracing::error!("Error processing log: {}", e);
                                current_log = None;
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }

                // If the log was processed successfully, export it
                if let Some(log) = current_log {
                    let exporters_guard = exporters.read().await;

                    // Export to all exporters in parallel
                    let export_futures = exporters_guard.iter().map(|exporter| {
                        let log_clone = log.clone();
                        async move {
                            if let Err(e) = exporter.export(log_clone).await {
                                tracing::error!("Error exporting log to {}: {}", exporter.name(), e);
                            }
                        }
                    });

                    stream::iter(export_futures)
                        .buffer_unordered(10) // Process up to 10 exports in parallel
                        .collect::<Vec<_>>()
                        .await;
                }
            }
        });

        self.task_handles.push(handle);

        Ok(())
    }

    /// Start the log collection pipeline
    pub async fn start(&mut self) -> Result<()> {
        if self.running {
            return Err(anyhow!("Pipeline already running"));
        }

        // Initialize components
        self.initialize().await?;

        if self.sources.is_empty() {
            return Err(anyhow!("No log sources configured"));
        }

        if self.exporters.is_empty() {
            return Err(anyhow!("No log exporters configured"));
        }

        // Start the processor task
        self.start_processor_task().await?;

        // Start all sources
        for source in &mut self.sources {
            let sender = self.log_channel.0.clone();
            source.start(sender).await?;
        }

        self.running = true;
        tracing::info!("Log collection pipeline started");

        Ok(())
    }

    /// Stop the log collection pipeline
    pub async fn stop(&mut self) -> Result<()> {
        if !self.running {
            return Err(anyhow!("Pipeline not running"));
        }

        // Stop all sources
        for source in &mut self.sources {
            if let Err(e) = source.stop().await {
                tracing::error!("Error stopping source {}: {}", source.name(), e);
            }
        }

        // Flush all exporters
        for exporter in &self.exporters {
            if let Err(e) = exporter.flush().await {
                tracing::error!("Error flushing exporter {}: {}", exporter.name(), e);
            }
        }

        // Cancel all tasks
        for handle in self.task_handles.drain(..) {
            handle.abort();
        }

        self.running = false;
        tracing::info!("Log collection pipeline stopped");

        Ok(())
    }
}
