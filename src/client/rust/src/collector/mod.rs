//! Log collector module for LogNarrator client
//!
//! This module provides functionality for collecting logs from various sources,
//! processing them through a configurable pipeline, and exporting them to
//! configured destinations.

pub mod config;
pub mod sources;
pub mod processors;
pub mod exporters;
pub mod pipeline;

use anyhow::Result;
use config::CollectorConfig;
use pipeline::Pipeline;

/// LogCollector manages the collection, processing, and export of logs
pub struct LogCollector {
    pipeline: Pipeline,
}

impl LogCollector {
    /// Create a new LogCollector from configuration
    pub fn new(config: CollectorConfig) -> Result<Self> {
        let pipeline = Pipeline::new(config)?;
        Ok(Self { pipeline })
    }

    /// Start the log collection process
    pub async fn start(&mut self) -> Result<()> {
        self.pipeline.start().await
    }

    /// Stop the log collection process
    pub async fn stop(&mut self) -> Result<()> {
        self.pipeline.stop().await
    }
}
