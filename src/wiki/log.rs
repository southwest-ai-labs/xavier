//! Append-Only Chronicle Log Engine for Living Wiki.
//!
//! Journals all ingested sources, git commits, and agent tasks chronologically in `.xavier/wiki/log.md`.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::Mutex;
use tracing::info;

use crate::wiki::schema::WikiLogEntry;

/// Name of the chronicled log markdown file.
pub const WIKI_LOG_FILE: &str = "log.md";

/// Append-only chronicle log manager.
#[derive(Clone)]
pub struct WikiLogManager {
    log_path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl WikiLogManager {
    /// Creates a new `WikiLogManager` targeting the given wiki directory.
    pub fn new(wiki_root: &Path) -> Self {
        Self {
            log_path: wiki_root.join(WIKI_LOG_FILE),
            lock: Arc::new(Mutex::new(())),
        }
    }

    /// Path to the chronicle `log.md` file.
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// Appends a new event entry to `log.md` safely across threads/tasks.
    pub async fn append_entry(&self, entry: &WikiLogEntry) -> Result<()> {
        let _guard = self.lock.lock().await;

        if let Some(parent) = self.log_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create wiki directory: {}", parent.display())
            })?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .with_context(|| format!("Failed to open log file: {}", self.log_path.display()))?;

        let timestamp_str = entry.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let mut entry_str = String::new();

        entry_str.push_str(&format!(
            "\n## {} — [{}] {}\n\n",
            timestamp_str, entry.event_type, entry.summary
        ));
        entry_str.push_str(&format!(
            "- **Author / Agent:** `{}`\n",
            entry.author_or_agent
        ));

        if !entry.touched_entities.is_empty() {
            entry_str.push_str("- **Touched Entities:** ");
            let links: Vec<String> = entry
                .touched_entities
                .iter()
                .map(|e| {
                    if e.starts_with("[[") && e.ends_with("]]") {
                        e.clone()
                    } else {
                        format!("[[{e}]]")
                    }
                })
                .collect();
            entry_str.push_str(&links.join(", "));
            entry_str.push('\n');
        }

        file.write_all(entry_str.as_bytes())
            .with_context(|| format!("Failed to write to log file: {}", self.log_path.display()))?;

        info!(
            event_type = %entry.event_type,
            author = %entry.author_or_agent,
            "Appended entry to wiki chronicle log"
        );

        Ok(())
    }

    /// Reads the entire content of `log.md` if it exists.
    pub fn read_log(&self) -> Result<String> {
        if !self.log_path.exists() {
            return Ok(String::new());
        }
        std::fs::read_to_string(&self.log_path)
            .with_context(|| format!("Failed to read log file: {}", self.log_path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_wiki_log_manager_append_and_read() {
        let temp = TempDir::new().expect("tempdir");
        let log_mgr = WikiLogManager::new(temp.path());

        let mut entry1 = WikiLogEntry::new("agent-karpathy", "compile", "Initial AST compilation");
        entry1.touched_entities = vec!["calculate_tax".to_string(), "billing_service".to_string()];

        let entry2 = WikiLogEntry::new("belal", "edit", "Updated architecture note");

        log_mgr.append_entry(&entry1).await.expect("append 1");
        log_mgr.append_entry(&entry2).await.expect("append 2");

        let content = log_mgr.read_log().expect("read log");
        assert!(content.contains("[compile] Initial AST compilation"));
        assert!(content.contains("`agent-karpathy`"));
        assert!(content.contains("[[calculate_tax]]"));
        assert!(content.contains("[edit] Updated architecture note"));
        assert!(content.contains("`belal`"));
    }
}
