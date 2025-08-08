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
    exporters_arc: Option<Arc<RwLock<Vec<Box<dyn LogExporter>>>>>,
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
            exporters_arc: None,
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

    /// Start the log processor and export tasks
    async fn start_processor_task(&mut self) -> Result<()> {
        // Create channel for processing task -> export task communication
        let (export_tx, mut export_rx) = mpsc::channel(1000);
        
        // Get the receiver from sources (this is where sources send logs)
        let mut source_receiver = std::mem::replace(&mut self.log_channel.1, mpsc::channel(1).1);
        
        // Wrap processors and exporters in Arc<RwLock<>> for sharing between tasks
        let processors = Arc::new(RwLock::new(std::mem::take(&mut self.processors)));
        let exporters = Arc::new(RwLock::new(std::mem::take(&mut self.exporters)));
        
        // Store the exporters Arc for use in stop()
        self.exporters_arc = Some(exporters.clone());
        
        // Clone the Arc references for the tasks
        let processors_clone = processors.clone();
        let exporters_clone = exporters.clone();
        
        // Start a processing task that processes logs through the processor chain
        let process_handle = tokio::spawn(async move {
            while let Some(log) = source_receiver.recv().await {
                tracing::debug!("Processing log: {:?}", log);
                
                // Process the log through the processor chain
                let mut current_log = Some(log);
                
                let processors_guard = processors_clone.read().await;
                for processor in processors_guard.iter() {
                    if let Some(log) = current_log {
                        match processor.process(log).await {
                            Ok(processed_log) => current_log = processed_log,
                            Err(e) => {
                                tracing::error!("Error processing log with {}: {}", processor.name(), e);
                                current_log = None;
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
                drop(processors_guard); // Release the read lock
                
                // If the log was processed successfully, forward it to the export task
                if let Some(processed_log) = current_log {
                    if let Err(e) = export_tx.send(processed_log).await {
                        tracing::error!("Failed to forward processed log to exporters: {}", e);
                        break;
                    }
                }
            }
        });

        // Start an export task that sends logs to all exporters
        let export_handle = tokio::spawn(async move {
            while let Some(log) = export_rx.recv().await {
                tracing::debug!("Exporting log: {:?}", log);
                
                let exporters_guard = exporters_clone.read().await;
                
                // Export to all exporters in parallel
                let export_futures = exporters_guard.iter().map(|exporter| {
                    let log_clone = log.clone();
                    async move {
                        if let Err(e) = exporter.export(log_clone).await {
                            tracing::error!("Error exporting log to {}: {}", exporter.name(), e);
                        } else {
                            tracing::debug!("Successfully exported log to {}", exporter.name());
                        }
                    }
                });

                // Execute all exports concurrently
                futures::future::join_all(export_futures).await;
                drop(exporters_guard); // Release the read lock
            }
        });

        self.task_handles.push(process_handle);
        self.task_handles.push(export_handle);

        // Note: processors and exporters are now moved into the tasks
        // The pipeline struct keeps empty vectors, but the actual processing
        // happens in the spawned tasks with the Arc<RwLock<>> references

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

        // Start the processor task (this will move exporters into Arc<RwLock<>>)
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
        if let Some(exporters_arc) = &self.exporters_arc {
            let exporters_guard = exporters_arc.read().await;
            for exporter in exporters_guard.iter() {
                if let Err(e) = exporter.flush().await {
                    tracing::error!("Error flushing exporter {}: {}", exporter.name(), e);
                }
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
