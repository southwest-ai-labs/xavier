//! Unified SQLite Living Wiki Container with ZSTD Compression.
//!
//! Packages compiled wiki entities, concept dossiers, and interconnected links into
//! `.xavier/wiki.sqlite`, enabling atomic multi-file updates, sub-millisecond lookups,
//! and bidirectional export/import with Markdown directory vaults.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use tracing::{debug, info};

use crate::wiki::parser::parse_markdown_content;

/// Name of the unified SQLite wiki container database.
pub const WIKI_SQLITE_FILE: &str = "wiki.sqlite";

/// Header information of a stored wiki node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredWikiNodeHeader {
    pub node_id: i64,
    pub stable_id: String,
    pub title: String,
    pub kind: String,
    pub file_path: String,
    pub token_count: usize,
    pub updated_at: i64,
}

/// Full content of a stored wiki node (decompressed from ZSTD).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredWikiNode {
    pub node_id: i64,
    pub stable_id: String,
    pub title: String,
    pub kind: String,
    pub file_path: String,
    pub markdown_content: String,
    pub token_count: usize,
    pub updated_at: i64,
}

/// Storage engine managing `.xavier/wiki.sqlite`.
#[derive(Clone)]
pub struct WikiStorageEngine {
    db_path: Option<PathBuf>,
    conn: Arc<Mutex<Connection>>,
}

impl WikiStorageEngine {
    /// Opens or creates a persistent SQLite living wiki container at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create wiki database dir: {}", parent.display())
            })?;
        }

        let conn = Connection::open(p)
            .with_context(|| format!("Failed to open wiki SQLite database: {}", p.display()))?;

        let engine = Self {
            db_path: Some(p.to_path_buf()),
            conn: Arc::new(Mutex::new(conn)),
        };
        engine.init_schema()?;
        Ok(engine)
    }

    /// Creates an in-memory SQLite living wiki container (useful for isolated unit testing).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .with_context(|| "Failed to open in-memory wiki database")?;

        let engine = Self {
            db_path: None,
            conn: Arc::new(Mutex::new(conn)),
        };
        engine.init_schema()?;
        Ok(engine)
    }

    /// Initializes schema tables and indexes.
    fn init_schema(&self) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {e}"))?;

        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS wiki_nodes (
                node_id INTEGER PRIMARY KEY AUTOINCREMENT,
                stable_id TEXT UNIQUE NOT NULL,
                title TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'entity',
                file_path TEXT NOT NULL DEFAULT '',
                content_zstd BLOB NOT NULL,
                token_count INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_wiki_nodes_stable_id ON wiki_nodes(stable_id);
            CREATE INDEX IF NOT EXISTS idx_wiki_nodes_kind ON wiki_nodes(kind);

            CREATE TABLE IF NOT EXISTS wiki_edges (
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                edge_type TEXT NOT NULL DEFAULT 'wikilink',
                weight REAL NOT NULL DEFAULT 1.0,
                PRIMARY KEY (source_id, target_id, edge_type)
            );

            CREATE INDEX IF NOT EXISTS idx_wiki_edges_source ON wiki_edges(source_id);
            CREATE INDEX IF NOT EXISTS idx_wiki_edges_target ON wiki_edges(target_id);
            "#,
        )
        .with_context(|| "Failed to initialize wiki SQLite schema")?;

        Ok(())
    }

    /// Atomically upserts a node and optionally synchronizes outgoing edges.
    pub fn upsert_node(
        &self,
        stable_id: &str,
        title: &str,
        kind: &str,
        file_path: &str,
        markdown_content: &str,
        outgoing_links: &[String],
    ) -> Result<()> {
        let compressed = zstd::encode_all(markdown_content.as_bytes(), 3)
            .with_context(|| "Failed to compress wiki markdown content with ZSTD")?;

        let token_count = markdown_content.split_whitespace().count();
        let now = Utc::now().timestamp();

        let mut conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {e}"))?;

        let tx = conn.transaction()?;

        tx.execute(
            r#"
            INSERT INTO wiki_nodes (stable_id, title, kind, file_path, content_zstd, token_count, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(stable_id) DO UPDATE SET
                title = excluded.title,
                kind = excluded.kind,
                file_path = excluded.file_path,
                content_zstd = excluded.content_zstd,
                token_count = excluded.token_count,
                updated_at = excluded.updated_at
            "#,
            params![stable_id, title, kind, file_path, compressed, token_count, now],
        )?;

        // Refresh edges for this source
        tx.execute(
            "DELETE FROM wiki_edges WHERE source_id = ?1",
            params![stable_id],
        )?;

        for target in outgoing_links {
            tx.execute(
                r#"
                INSERT OR IGNORE INTO wiki_edges (source_id, target_id, edge_type, weight)
                VALUES (?1, ?2, 'wikilink', 1.0)
                "#,
                params![stable_id, target],
            )?;
        }

        tx.commit()?;

        debug!(stable_id, "Upserted wiki node into SQLite container");
        Ok(())
    }

    /// Retrieves a single node and decompresses its Markdown content.
    pub fn get_node(&self, stable_id: &str) -> Result<Option<StoredWikiNode>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {e}"))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT node_id, stable_id, title, kind, file_path, content_zstd, token_count, updated_at
            FROM wiki_nodes
            WHERE stable_id = ?1
            "#,
        )?;

        let mut rows = stmt.query(params![stable_id])?;
        if let Some(row) = rows.next()? {
            let node_id: i64 = row.get(0)?;
            let stable_id: String = row.get(1)?;
            let title: String = row.get(2)?;
            let kind: String = row.get(3)?;
            let file_path: String = row.get(4)?;
            let compressed: Vec<u8> = row.get(5)?;
            let token_count: usize = row.get(6)?;
            let updated_at: i64 = row.get(7)?;

            let decompressed_bytes = zstd::decode_all(&compressed[..])
                .with_context(|| "Failed to decompress ZSTD wiki node content")?;
            let markdown_content = String::from_utf8(decompressed_bytes)
                .with_context(|| "Decompressed content is invalid UTF-8")?;

            Ok(Some(StoredWikiNode {
                node_id,
                stable_id,
                title,
                kind,
                file_path,
                markdown_content,
                token_count,
                updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Lists all node headers stored in the database.
    pub fn list_nodes(&self) -> Result<Vec<StoredWikiNodeHeader>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {e}"))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT node_id, stable_id, title, kind, file_path, token_count, updated_at
            FROM wiki_nodes
            ORDER BY title ASC
            "#,
        )?;

        let nodes = stmt
            .query_map([], |row| {
                Ok(StoredWikiNodeHeader {
                    node_id: row.get(0)?,
                    stable_id: row.get(1)?,
                    title: row.get(2)?,
                    kind: row.get(3)?,
                    file_path: row.get(4)?,
                    token_count: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(nodes)
    }

    /// Returns all known node identifiers, titles, and file stems from `wiki_nodes`
    /// for fast in-memory validation of wikilinks and integrity checking.
    pub fn get_known_node_identifiers(&self) -> Result<HashSet<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {e}"))?;

        let mut stmt = conn.prepare("SELECT stable_id, title, file_path FROM wiki_nodes")?;
        let mut set = HashSet::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let stable_id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let file_path: String = row.get(2)?;

            if !stable_id.is_empty() {
                set.insert(stable_id);
            }
            if !title.is_empty() {
                set.insert(title);
            }
            if !file_path.is_empty() {
                set.insert(file_path.clone());
                if let Some(stem) = Path::new(&file_path).file_stem().and_then(|s| s.to_str()) {
                    set.insert(stem.to_string());
                }
            }
        }
        Ok(set)
    }

    /// Returns outgoing links from `source_id`.
    pub fn get_forward_links(&self, source_id: &str) -> Result<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {e}"))?;

        let mut stmt = conn.prepare(
            "SELECT target_id FROM wiki_edges WHERE source_id = ?1 ORDER BY target_id ASC",
        )?;

        let targets = stmt
            .query_map(params![source_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(targets)
    }

    /// Returns all directed edge pairs `(source_id, target_id)` stored in the wiki.
    pub fn get_all_edges(&self) -> Result<Vec<(String, String)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {e}"))?;

        let mut stmt = conn.prepare(
            "SELECT source_id, target_id FROM wiki_edges ORDER BY source_id ASC, target_id ASC",
        )?;

        let edges = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(edges)
    }

    /// Returns incoming backlinks to `target_id`.
    pub fn get_backlinks(&self, target_id: &str) -> Result<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {e}"))?;

        let mut stmt = conn.prepare(
            "SELECT source_id FROM wiki_edges WHERE target_id = ?1 ORDER BY source_id ASC",
        )?;

        let sources = stmt
            .query_map(params![target_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(sources)
    }

    /// Identifies orphan nodes (nodes that have 0 incoming backlinks and 0 forward links).
    pub fn find_orphan_nodes(&self) -> Result<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {e}"))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT n.stable_id
            FROM wiki_nodes n
            LEFT JOIN wiki_edges e_out ON n.stable_id = e_out.source_id
            LEFT JOIN wiki_edges e_in ON n.stable_id = e_in.target_id
            WHERE e_out.source_id IS NULL AND e_in.target_id IS NULL
            ORDER BY n.stable_id ASC
            "#,
        )?;

        let orphans = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(orphans)
    }

    /// Exports all nodes from SQLite container into Markdown files in `target_dir` (Obsidian / Git sync).
    pub fn export_to_markdown_dir(&self, target_dir: &Path) -> Result<usize> {
        std::fs::create_dir_all(target_dir)?;
        let headers = self.list_nodes()?;
        let mut count = 0;

        for h in headers {
            if let Some(node) = self.get_node(&h.stable_id)? {
                let safe_file = if node.file_path.is_empty() {
                    format!("{}.md", node.stable_id)
                } else if node.file_path.ends_with(".md") {
                    node.file_path.clone()
                } else {
                    format!("{}.md", node.file_path)
                };

                let dest = target_dir.join(&safe_file);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&dest, &node.markdown_content)?;
                count += 1;
            }
        }

        info!(count, target = %target_dir.display(), "Exported living wiki to markdown directory");
        Ok(count)
    }

    /// Imports all Markdown files from `source_dir` into the SQLite container.
    pub fn import_from_markdown_dir(&self, source_dir: &Path) -> Result<usize> {
        if !source_dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        for entry in walkdir::WalkDir::new(source_dir).into_iter().flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                let content = std::fs::read_to_string(path)?;
                let parsed = parse_markdown_content(&content);

                let rel_path = path
                    .strip_prefix(source_dir)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .replace('\\', "/");

                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("note")
                    .to_string();

                let stable_id = parsed
                    .frontmatter
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| stem.clone());

                let title = parsed.title.unwrap_or_else(|| stem.clone());
                let kind = parsed
                    .frontmatter
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("concept");

                self.upsert_node(
                    &stable_id,
                    &title,
                    kind,
                    &rel_path,
                    &content,
                    &parsed.wikilinks,
                )?;
                count += 1;
            }
        }

        info!(count, source = %source_dir.display(), "Imported markdown directory into SQLite container");
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_storage_engine_zstd_roundtrip_and_edges() {
        let storage = WikiStorageEngine::in_memory().expect("in-memory db");

        let content = "# Node A\n\nReferences [[NodeB]] and [[NodeC]].";
        storage
            .upsert_node(
                "NodeA",
                "Node A",
                "entity",
                "src/a.rs",
                content,
                &["NodeB".to_string(), "NodeC".to_string()],
            )
            .expect("upsert NodeA");

        storage
            .upsert_node("NodeB", "Node B", "entity", "src/b.rs", "# Node B", &[])
            .expect("upsert NodeB");

        let node_a = storage
            .get_node("NodeA")
            .expect("get NodeA")
            .expect("found");
        assert_eq!(node_a.title, "Node A");
        assert_eq!(node_a.markdown_content, content);

        let forward = storage.get_forward_links("NodeA").expect("forward");
        assert_eq!(forward, vec!["NodeB".to_string(), "NodeC".to_string()]);

        let back = storage.get_backlinks("NodeB").expect("backlinks");
        assert_eq!(back, vec!["NodeA".to_string()]);
    }

    #[test]
    fn test_export_and_import_roundtrip() {
        let storage = WikiStorageEngine::in_memory().expect("in-memory db");
        let temp = TempDir::new().expect("tempdir");

        let note_content = "---\ntitle: Concept 1\nkind: concept\n---\n# Concept 1\nBody text";
        storage
            .upsert_node(
                "concept_1",
                "Concept 1",
                "concept",
                "concepts/concept_1.md",
                note_content,
                &[],
            )
            .expect("upsert");

        let exported = storage.export_to_markdown_dir(temp.path()).expect("export");
        assert_eq!(exported, 1);

        let storage2 = WikiStorageEngine::in_memory().expect("in-memory db 2");
        let imported = storage2
            .import_from_markdown_dir(temp.path())
            .expect("import");
        assert_eq!(imported, 1);

        let roundtrip_node = storage2.get_node("concept_1").expect("get").expect("found");
        assert_eq!(roundtrip_node.title, "Concept 1");
        assert_eq!(roundtrip_node.markdown_content, note_content);
    }
}
