//! Database module for the MCP client
//!
//! This module handles local storage using SQLite for caching logs
//! and storing action execution history.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Database {
    conn: Connection,
}

/// Log entry structure
pub struct LogEntry {
    pub id: Option<i64>,
    pub timestamp: i64,
    pub source: String,
    pub content: String,
    pub encrypted: bool,
    pub sent: bool,
}

/// Action execution record
pub struct ActionRecord {
    pub id: Option<i64>,
    pub timestamp: i64,
    pub action_id: String,
    pub parameters: String,
    pub status: String,
    pub result: String,
}

impl Database {
    /// Open or create a database connection
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)
            .context("Failed to open database")?;

        let db = Self { conn };
        db.initialize()?;

        Ok(db)
    }

    /// Initialize the database schema
    fn initialize(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY,
                timestamp INTEGER NOT NULL,
                source TEXT NOT NULL,
                content TEXT NOT NULL,
                encrypted BOOLEAN NOT NULL,
                sent BOOLEAN NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS actions (
                id INTEGER PRIMARY KEY,
                timestamp INTEGER NOT NULL,
                action_id TEXT NOT NULL,
                parameters TEXT NOT NULL,
                status TEXT NOT NULL,
                result TEXT NOT NULL
            )",
            [],
        )?;

        // Create indices for better performance
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_logs_sent ON logs (sent)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_actions_timestamp ON actions (timestamp)",
            [],
        )?;

        Ok(())
    }

    /// Store a log entry
    pub fn store_log(&self, entry: &LogEntry) -> Result<i64> {
        let timestamp = entry.timestamp;

        let id = self.conn.execute(
            "INSERT INTO logs (timestamp, source, content, encrypted, sent)
             VALUES (?, ?, ?, ?, ?)",
            params![timestamp, entry.source, entry.content, entry.encrypted, entry.sent],
        )?;

        Ok(id as i64)
    }

    /// Get unsent logs
    pub fn get_unsent_logs(&self, limit: usize) -> Result<Vec<LogEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, source, content, encrypted, sent
             FROM logs
             WHERE sent = 0
             ORDER BY timestamp
             LIMIT ?",
        )?;

        let log_iter = stmt.query_map([limit as i64], |row| {
            Ok(LogEntry {
                id: Some(row.get(0)?),
                timestamp: row.get(1)?,
                source: row.get(2)?,
                content: row.get(3)?,
                encrypted: row.get(4)?,
                sent: row.get(5)?,
            })
        })?;

        let logs: Result<Vec<_>, _> = log_iter.collect();
        Ok(logs?)
    }

    /// Mark logs as sent
    pub fn mark_logs_sent(&self, ids: &[i64]) -> Result<()> {
        let id_list = ids.iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        if !id_list.is_empty() {
            let query = format!(
                "UPDATE logs SET sent = 1 WHERE id IN ({})",
                id_list
            );

            self.conn.execute(&query, [])?;
        }

        Ok(())
    }

    /// Record an action execution
    pub fn record_action(&self, record: &ActionRecord) -> Result<i64> {
        let timestamp = record.timestamp;

        let id = self.conn.execute(
            "INSERT INTO actions (timestamp, action_id, parameters, status, result)
             VALUES (?, ?, ?, ?, ?)",
            params![
                timestamp,
                record.action_id,
                record.parameters,
                record.status,
                record.result
            ],
        )?;

        Ok(id as i64)
    }

    /// Get recent action executions
    pub fn get_recent_actions(&self, limit: usize) -> Result<Vec<ActionRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, action_id, parameters, status, result
             FROM actions
             ORDER BY timestamp DESC
             LIMIT ?",
        )?;

        let action_iter = stmt.query_map([limit as i64], |row| {
            Ok(ActionRecord {
                id: Some(row.get(0)?),
                timestamp: row.get(1)?,
                action_id: row.get(2)?,
                parameters: row.get(3)?,
                status: row.get(4)?,
                result: row.get(5)?,
            })
        })?;

        let actions: Result<Vec<_>, _> = action_iter.collect();
        Ok(actions?)
    }

    /// Clean up old logs
    pub fn cleanup_old_logs(&self, max_age_days: u64) -> Result<usize> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let max_age_secs = max_age_days * 24 * 60 * 60;
        let cutoff = now - (max_age_secs as i64);

        let rows = self.conn.execute(
            "DELETE FROM logs WHERE timestamp < ? AND sent = 1",
            params![cutoff],
        )?;

        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use tempfile::tempdir;

    #[test]
    fn test_database_operations() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.db");

        let db = Database::open(db_path)?;

        // Test storing logs
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        let log = LogEntry {
            id: None,
            timestamp,
            source: "test".to_string(),
            content: "test log".to_string(),
            encrypted: false,
            sent: false,
        };

        let id = db.store_log(&log)?;
        assert!(id > 0);

        // Test retrieving unsent logs
        let unsent = db.get_unsent_logs(10)?;
        assert_eq!(unsent.len(), 1);
        assert_eq!(unsent[0].content, "test log");

        // Test marking logs as sent
        db.mark_logs_sent(&[id])?;

        let unsent_after = db.get_unsent_logs(10)?;
        assert_eq!(unsent_after.len(), 0);

        // Test action recording
        let action = ActionRecord {
            id: None,
            timestamp,
            action_id: "test.action".to_string(),
            parameters: "{\"param\": \"value\"}".to_string(),
            status: "success".to_string(),
            result: "OK".to_string(),
        };

        let action_id = db.record_action(&action)?;
        assert!(action_id > 0);

        // Test retrieving recent actions
        let recent = db.get_recent_actions(10)?;
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].action_id, "test.action");

        Ok(())
    }
}
