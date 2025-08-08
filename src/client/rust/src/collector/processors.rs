//! Log processor implementations for the collector

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;
use std::time::Duration;

use crate::collector::config::{ProcessorConfig, FilterConfig, MatchConfig, MatchType, ActionType, AttributeAction, TransformAction, TransformType};
use crate::collector::sources::LogEntry;

/// Interface for log processors
#[async_trait]
pub trait LogProcessor: Send + Sync {
    /// Process a log entry
    async fn process(&self, log: LogEntry) -> Result<Option<LogEntry>>;
    /// Get the name of this processor
    fn name(&self) -> &str;
}

/// Create a log processor from configuration
pub fn create_processor(config: &ProcessorConfig) -> Result<Box<dyn LogProcessor>> {
    match config {
        ProcessorConfig::Resource { name, attributes } => {
            Ok(Box::new(ResourceProcessor::new(
                name.clone(),
                attributes.clone(),
            )?))
        },
        ProcessorConfig::Filter { name, logs } => {
            Ok(Box::new(FilterProcessor::new(
                name.clone(),
                logs.clone(),
            )?))
        },
        ProcessorConfig::Batch { name, timeout, send_batch_size } => {
            Ok(Box::new(BatchProcessor::new(
                name.clone(),
                *timeout,
                *send_batch_size,
            )?))
        },
        ProcessorConfig::Transform { name, transforms } => {
            Ok(Box::new(TransformProcessor::new(
                name.clone(),
                transforms.clone(),
            )?))
        },
    }
}

/// Resource processor adds metadata to logs
pub struct ResourceProcessor {
    name: String,
    attributes: Vec<AttributeAction>,
}

impl ResourceProcessor {
    /// Create a new resource processor
    pub fn new(
        name: String,
        attributes: Vec<AttributeAction>,
    ) -> Result<Self> {
        Ok(Self {
            name,
            attributes,
        })
    }

    /// Replace environment variables in a string
    fn replace_env_vars(&self, value: &str) -> String {
        let mut result = value.to_string();

        // Find all ${ENV_VAR} patterns and replace them with environment variable values
        let env_var_regex = Regex::new(r"\$\{([^}]+)\}").unwrap();

        for captures in env_var_regex.captures_iter(value) {
            if let Some(env_var_name) = captures.get(1) {
                let env_var_name = env_var_name.as_str();
                if let Ok(env_value) = std::env::var(env_var_name) {
                    result = result.replace(&format!("${{{}}}", env_var_name), &env_value);
                }
            }
        }

        result
    }
}

#[async_trait]
impl LogProcessor for ResourceProcessor {
    async fn process(&self, mut log: LogEntry) -> Result<Option<LogEntry>> {
        // Apply attribute actions to the log entry
        for attr in &self.attributes {
            let value = self.replace_env_vars(&attr.value);

            match attr.action {
                ActionType::Insert => {
                    if !log.attributes.contains_key(&attr.key) {
                        log.attributes.insert(attr.key.clone(), value);
                    }
                },
                ActionType::Update => {
                    if log.attributes.contains_key(&attr.key) {
                        log.attributes.insert(attr.key.clone(), value);
                    }
                },
                ActionType::Upsert => {
                    log.attributes.insert(attr.key.clone(), value);
                },
                ActionType::Delete => {
                    log.attributes.remove(&attr.key);
                },
            }
        }

        Ok(Some(log))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Filter processor includes or excludes logs based on patterns
pub struct FilterProcessor {
    name: String,
    filter: FilterConfig,
    include_matchers: Vec<Matcher>,
    exclude_matchers: Vec<Matcher>,
}

enum Matcher {
    Exact(String),
    Regexp(Regex),
}

impl Matcher {
    fn matches(&self, text: &str) -> bool {
        match self {
            Matcher::Exact(pattern) => text.contains(pattern),
            Matcher::Regexp(regex) => regex.is_match(text),
        }
    }
}

impl FilterProcessor {
    /// Create a new filter processor
    pub fn new(
        name: String,
        filter: FilterConfig,
    ) -> Result<Self> {
        let mut include_matchers = Vec::new();
        let mut exclude_matchers = Vec::new();

        // Setup include matchers
        if let Some(include) = &filter.include {
            match include.match_type {
                MatchType::Exact => {
                    if let Some(patterns) = &include.exact {
                        for pattern in patterns {
                            include_matchers.push(Matcher::Exact(pattern.clone()));
                        }
                    }
                },
                MatchType::Regexp => {
                    if let Some(patterns) = &include.regexp {
                        for pattern in patterns {
                            let regex = Regex::new(pattern)?;
                            include_matchers.push(Matcher::Regexp(regex));
                        }
                    }
                },
            }
        }

        // Setup exclude matchers
        if let Some(exclude) = &filter.exclude {
            match exclude.match_type {
                MatchType::Exact => {
                    if let Some(patterns) = &exclude.exact {
                        for pattern in patterns {
                            exclude_matchers.push(Matcher::Exact(pattern.clone()));
                        }
                    }
                },
                MatchType::Regexp => {
                    if let Some(patterns) = &exclude.regexp {
                        for pattern in patterns {
                            let regex = Regex::new(pattern)?;
                            exclude_matchers.push(Matcher::Regexp(regex));
                        }
                    }
                },
            }
        }

        Ok(Self {
            name,
            filter,
            include_matchers,
            exclude_matchers,
        })
    }
}

#[async_trait]
impl LogProcessor for FilterProcessor {
    async fn process(&self, log: LogEntry) -> Result<Option<LogEntry>> {
        let message = &log.message;

        // Check exclude patterns first (if any log matches an exclude pattern, drop the log)
        for matcher in &self.exclude_matchers {
            if matcher.matches(message) {
                return Ok(None);
            }
        }

        // If there are include patterns, the log must match at least one to be included
        if !self.include_matchers.is_empty() {
            let mut included = false;

            for matcher in &self.include_matchers {
                if matcher.matches(message) {
                    included = true;
                    break;
                }
            }

            if !included {
                return Ok(None);
            }
        }

        // If we get here, the log passed all filters
        Ok(Some(log))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Batch processor groups logs for efficient transmission
pub struct BatchProcessor {
    name: String,
    timeout: Duration,
    batch_size: usize,
}

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new(
        name: String,
        timeout_seconds: u64,
        batch_size: usize,
    ) -> Result<Self> {
        Ok(Self {
            name,
            timeout: Duration::from_secs(timeout_seconds),
            batch_size,
        })
    }
}

#[async_trait]
impl LogProcessor for BatchProcessor {
    async fn process(&self, log: LogEntry) -> Result<Option<LogEntry>> {
        // The batch processor just passes logs through in this simple implementation
        // In a real implementation, it would buffer logs and only release them when the batch is full
        // or when the timeout expires
        Ok(Some(log))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Transform processor modifies log content
pub struct TransformProcessor {
    name: String,
    transforms: Vec<TransformAction>,
    regexes: HashMap<String, Regex>,
}

impl TransformProcessor {
    /// Create a new transform processor
    pub fn new(
        name: String,
        transforms: Vec<TransformAction>,
    ) -> Result<Self> {
        let mut regexes = HashMap::new();

        // Compile regexes used in transforms
        for transform in &transforms {
            if transform.transform_type == TransformType::Extract || transform.transform_type == TransformType::Mask {
                if let Some(pattern) = transform.parameters.get("pattern") {
                    let regex = Regex::new(pattern)?;
                    regexes.insert(transform.field.clone(), regex);
                }
            }
        }

        Ok(Self {
            name,
            transforms,
            regexes,
        })
    }

    /// Apply mask transformation
    fn apply_mask(&self, value: &str, field: &str, parameters: &HashMap<String, String>) -> String {
        if let Some(regex) = self.regexes.get(field) {
            let default_replacement = "*****".to_string();
            let replacement = parameters.get("replacement").unwrap_or(&default_replacement);
            regex.replace_all(value, replacement.as_str()).to_string()
        } else {
            value.to_string()
        }
    }

    /// Apply extract transformation
    fn apply_extract(&self, log: &mut LogEntry, field: &str) -> Result<()> {
        if let Some(regex) = self.regexes.get(field) {
            let value = if field == "message" {
                log.message.clone()
            } else if let Some(attr_value) = log.attributes.get(field) {
                attr_value.clone()
            } else {
                return Ok(());
            };

            if let Some(captures) = regex.captures(&value) {
                for name in regex.capture_names().flatten() {
                    if let Some(m) = captures.name(name) {
                        log.attributes.insert(name.to_string(), m.as_str().to_string());
                    }
                }
            }
        }

        Ok(())
    }

    /// Apply rename transformation
    fn apply_rename(&self, log: &mut LogEntry, field: &str, parameters: &HashMap<String, String>) -> Result<()> {
        if let Some(new_name) = parameters.get("new_name") {
            if let Some(value) = log.attributes.remove(field) {
                log.attributes.insert(new_name.clone(), value);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl LogProcessor for TransformProcessor {
    async fn process(&self, mut log: LogEntry) -> Result<Option<LogEntry>> {
        // Apply transformations to the log entry
        for transform in &self.transforms {
            match transform.transform_type {
                TransformType::Mask => {
                    if transform.field == "message" {
                        log.message = self.apply_mask(&log.message, &transform.field, &transform.parameters);
                    } else if let Some(value) = log.attributes.get_mut(&transform.field) {
                        *value = self.apply_mask(value, &transform.field, &transform.parameters);
                    }
                },
                TransformType::Extract => {
                    self.apply_extract(&mut log, &transform.field)?;
                },
                TransformType::Rename => {
                    self.apply_rename(&mut log, &transform.field, &transform.parameters)?;
                },
                TransformType::Convert => {
                    // Not implemented in this simple version
                    // Would convert field formats like timestamps
                },
            }
        }

        Ok(Some(log))
    }

    fn name(&self) -> &str {
        &self.name
    }
}
