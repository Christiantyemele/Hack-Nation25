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
//! Database utilities for the LogNarrator client

use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::Path;

/// Database connection for the LogNarrator client
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open a database connection
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    /// Initialize the database schema
    fn initialize(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                source TEXT NOT NULL,
                level TEXT,
                message TEXT NOT NULL,
                attributes TEXT,
                exported INTEGER DEFAULT 0
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_logs_exported ON logs(exported)",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    /// Store a log entry
    pub fn store_log(
        &self,
        timestamp: &str,
        source: &str,
        level: Option<&str>,
        message: &str,
        attributes: &str,
    ) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO logs (timestamp, source, level, message, attributes)
             VALUES (?, ?, ?, ?, ?)",
        )?;

        stmt.execute(params![timestamp, source, level, message, attributes])?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Mark log entries as exported
    pub fn mark_exported(&self, ids: &[i64]) -> Result<usize> {
        let id_list = ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<String>>()
            .join(", ");

        let sql = format!(
            "UPDATE logs SET exported = 1 WHERE id IN ({})",
            id_list
        );

        let count = self.conn.execute(&sql, [])?;

        Ok(count)
    }

    /// Get unexported log entries
    pub fn get_unexported_logs(&self, limit: usize) -> Result<Vec<(i64, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, source, level, message, attributes
             FROM logs
             WHERE exported = 0
             ORDER BY id
             LIMIT ?",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            let id: i64 = row.get(0)?;
            let timestamp: String = row.get(1)?;
            let source: String = row.get(2)?;
            let level: Option<String> = row.get(3)?;
            let message: String = row.get(4)?;
            let attributes: String = row.get(5)?;

            // Construct a JSON representation of the log entry
            let log_json = format!(
                "{{\"timestamp\":\"{}\",\"source\":\"{}\",\"level\":{},\"message\":\"{}\",\"attributes\":{}}}",
                timestamp,
                source,
                level.map_or("null".to_string(), |l| format!("\"{}\"", l)),
                message.replace('"', "\\\""),
                attributes
            );

            Ok((id, log_json))
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }

        Ok(result)
    }

    /// Set a metadata value
    pub fn set_metadata(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?, ?)",
            params![key, value],
        )?;

        Ok(())
    }

    /// Get a metadata value
    pub fn get_metadata(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT value FROM metadata WHERE key = ?",
        )?;

        let mut rows = stmt.query(params![key])?;

        if let Some(row) = rows.next()? {
            let value: String = row.get(0)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Delete old log entries
    pub fn delete_old_logs(&self, days_to_keep: u32) -> Result<usize> {
        let sql = format!(
            "DELETE FROM logs WHERE datetime(timestamp) < datetime('now', '-{} days')",
            days_to_keep
        );

        let count = self.conn.execute(&sql, [])?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_operations() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.db");

        let db = Database::open(&db_path)?;

        // Test storing a log entry
        let id = db.store_log(
            "2023-01-01T12:00:00Z",
            "test-source",
            Some("INFO"),
            "Test message",
            "{\"attr1\":\"value1\"}",
        )?;

        assert!(id > 0);

        // Test getting unexported logs
        let logs = db.get_unexported_logs(10)?;
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].0, id);

        // Test marking logs as exported
        let count = db.mark_exported(&[id])?;
        assert_eq!(count, 1);

        // Test that the log is no longer unexported
        let logs = db.get_unexported_logs(10)?;
        assert_eq!(logs.len(), 0);

        // Test metadata operations
        db.set_metadata("test-key", "test-value")?;
        let value = db.get_metadata("test-key")?;
        assert_eq!(value, Some("test-value".to_string()));

        Ok(())
    }
}
        Ok(())
    }
}
