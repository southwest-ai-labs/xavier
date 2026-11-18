//! Living Wiki Consistency and Integrity Linter Engine.
//!
//! Checks for broken wikilinks, orphaned notes without incoming backlinks,
//! and Code-Doc drift where an entity note's underlying code symbol no longer exists.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use code_graph::db::CodeGraphDB;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::wiki::parser::parse_markdown_content;
use crate::wiki::storage::{WikiStorageEngine, WIKI_SQLITE_FILE};

/// A broken wikilink report item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokenLinkItem {
    pub source_file: String,
    pub target: String,
}

/// A Code-to-Documentation drift warning item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeDocDriftItem {
    pub entity_file: String,
    pub symbol_name: String,
    pub missing_file_or_symbol: String,
}

/// Comprehensive report produced by the wiki linter.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintReport {
    pub total_checked: usize,
    pub broken_links: Vec<BrokenLinkItem>,
    pub orphans: Vec<String>,
    pub drift_warnings: Vec<CodeDocDriftItem>,
}

impl LintReport {
    /// True if zero errors or warnings were detected.
    pub fn is_clean(&self) -> bool {
        self.broken_links.is_empty() && self.orphans.is_empty() && self.drift_warnings.is_empty()
    }
}

/// Living Wiki Linter.
pub struct WikiLinter;

impl WikiLinter {
    /// Audits a wiki directory for broken links, orphans, and drift against `CodeGraphDB`.
    pub fn lint_wiki(wiki_root: &Path, code_db: Option<&CodeGraphDB>) -> Result<LintReport> {
        Self::lint_wiki_with_storage(wiki_root, code_db, None)
    }

    /// Audits a wiki directory with an optional explicit SQLite storage engine fallback.
    ///
    /// If `storage` is None, attempts to auto-detect `wiki.sqlite` in the wiki root
    /// or its parent directories to validate wikilinks against indexed `wiki_nodes`.
    pub fn lint_wiki_with_storage(
        wiki_root: &Path,
        code_db: Option<&CodeGraphDB>,
        storage: Option<&WikiStorageEngine>,
    ) -> Result<LintReport> {
        let mut report = LintReport::default();

        if !wiki_root.exists() {
            return Ok(report);
        }

        let mut sqlite_known_nodes = HashSet::new();

        if let Some(store) = storage {
            if let Ok(ids) = store.get_known_node_identifiers() {
                sqlite_known_nodes.extend(ids);
            }
        } else {
            let candidates = [
                wiki_root.join(WIKI_SQLITE_FILE),
                wiki_root.join("wiki.sqlite"),
                wiki_root.join(".xavier").join(WIKI_SQLITE_FILE),
                wiki_root
                    .parent()
                    .map(|p| p.join(WIKI_SQLITE_FILE))
                    .unwrap_or_else(|| wiki_root.to_path_buf()),
                wiki_root
                    .parent()
                    .map(|p| p.join("wiki.sqlite"))
                    .unwrap_or_else(|| wiki_root.to_path_buf()),
                wiki_root
                    .parent()
                    .map(|p| p.join(".xavier").join(WIKI_SQLITE_FILE))
                    .unwrap_or_else(|| wiki_root.to_path_buf()),
            ];

            for cand in &candidates {
                if cand.is_file() {
                    if let Ok(store) = WikiStorageEngine::open(cand) {
                        if let Ok(ids) = store.get_known_node_identifiers() {
                            sqlite_known_nodes.extend(ids);
                            break;
                        }
                    }
                }
            }
        }

        let mut all_stems = HashSet::new();
        let mut note_files = Vec::new();
        let mut outgoing_map: Vec<(String, String, Vec<String>)> = Vec::new();
        let mut all_referenced_targets = HashSet::new();

        // 1. Collect all markdown notes across wiki
        for entry in walkdir::WalkDir::new(wiki_root).into_iter().flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                let rel = path
                    .strip_prefix(wiki_root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .replace('\\', "/");

                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                all_stems.insert(stem.clone());
                all_stems.insert(rel.clone());
                note_files.push((path.to_path_buf(), rel, stem));
            }
        }

        report.total_checked = note_files.len();

        // 2. Parse links and verify targets
        let mut note_stable_ids: Vec<String> = Vec::with_capacity(note_files.len());

        for (path, rel, stem) in &note_files {
            if let Ok(content) = std::fs::read_to_string(path) {
                let parsed = parse_markdown_content(&content);

                let stable_id = parsed
                    .frontmatter
                    .get("stable_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                note_stable_ids.push(stable_id);

                for link in &parsed.wikilinks {
                    all_referenced_targets.insert(link.clone());

                    // Check if target exists in stems or file paths or SQLite wiki_nodes table
                    let exists = all_stems.contains(link.as_str())
                        || all_stems.contains(&format!("{link}.md"))
                        || all_stems.contains(&format!("entities/{link}"))
                        || all_stems.contains(&format!("concepts/{link}"))
                        || sqlite_known_nodes.contains(link.as_str())
                        || sqlite_known_nodes.contains(&format!("{link}.md"));

                    if !exists {
                        report.broken_links.push(BrokenLinkItem {
                            source_file: rel.clone(),
                            target: link.clone(),
                        });
                    }
                }

                // Check Code-Doc drift if CodeGraphDB is provided
                if let Some(db) = code_db {
                    if rel.starts_with("entities/") {
                        let symbol_name = parsed
                            .frontmatter
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(stem);

                        let file_path = parsed
                            .frontmatter
                            .get("file_path")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        // Query CodeGraphDB: does the symbol still exist?
                        let found = db.find_by_name(symbol_name, 1).unwrap_or_default();
                        if found.is_empty() {
                            report.drift_warnings.push(CodeDocDriftItem {
                                entity_file: rel.clone(),
                                symbol_name: symbol_name.to_string(),
                                missing_file_or_symbol: format!("{symbol_name} in {file_path}"),
                            });
                        }
                    }
                }

                outgoing_map.push((rel.clone(), stem.clone(), parsed.wikilinks));
            } else {
                note_stable_ids.push(String::new());
            }
        }

        // 3. Identify orphan notes (excluding index.md and log.md)
        for (i, (_, rel, stem)) in note_files.iter().enumerate() {
            if rel == "index.md" || rel == "log.md" {
                continue;
            }

            let sid = note_stable_ids.get(i).map(|s| s.as_str()).unwrap_or("");

            let is_referenced = all_referenced_targets.contains(stem.as_str())
                || all_referenced_targets.contains(rel.as_str())
                || (!sid.is_empty() && all_referenced_targets.contains(sid))
                || sqlite_known_nodes.contains(stem.as_str())
                || sqlite_known_nodes.contains(rel.as_str());

            if !is_referenced {
                report.orphans.push(rel.clone());
            }
        }

        info!(
            checked = report.total_checked,
            broken = report.broken_links.len(),
            orphans = report.orphans.len(),
            drift = report.drift_warnings.len(),
            "Completed wiki consistency linting"
        );

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_wiki_linter_detects_broken_link_and_orphan() {
        let temp = TempDir::new().expect("tempdir");
        let wiki_root = temp.path();

        let index_md = "# Index\n\n- [[NoteA]]\n";
        let note_a_md = "# Note A\n\nPoints to [[MissingTarget]] and [[NoteB]].";
        let note_b_md = "# Note B\n\nValid leaf note.";
        let orphan_md = "# Orphan Note\n\nNobody links here.";

        std::fs::write(wiki_root.join("index.md"), index_md).unwrap();
        std::fs::write(wiki_root.join("NoteA.md"), note_a_md).unwrap();
        std::fs::write(wiki_root.join("NoteB.md"), note_b_md).unwrap();
        std::fs::write(wiki_root.join("Orphan.md"), orphan_md).unwrap();

        let report = WikiLinter::lint_wiki(wiki_root, None).expect("lint");
        assert_eq!(report.total_checked, 4);

        assert_eq!(report.broken_links.len(), 1);
        assert_eq!(report.broken_links[0].target, "MissingTarget");

        assert_eq!(report.orphans, vec!["Orphan.md".to_string()]);
    }

    #[test]
    fn test_wiki_linter_sqlite_fallback_resolves_false_positives() {
        let temp = TempDir::new().expect("tempdir");
        let wiki_root = temp.path();

        // Create a note that references a symbol by its stable_id: [[stable_calc_hash_1234]]
        let note_a = "# Note A\n\nCalls [[stable_calc_hash_1234]].\n";
        std::fs::write(wiki_root.join("NoteA.md"), note_a).unwrap();

        // 1. Without SQLite storage, it's flagged as a broken link
        let report_no_db = WikiLinter::lint_wiki(wiki_root, None).expect("lint");
        assert_eq!(report_no_db.broken_links.len(), 1);
        assert_eq!(report_no_db.broken_links[0].target, "stable_calc_hash_1234");

        // 2. With SQLite storage containing the node, fallback resolves it cleanly
        let storage = WikiStorageEngine::in_memory().expect("in-memory storage");
        storage
            .upsert_node(
                "stable_calc_hash_1234",
                "calculate_tax",
                "Function",
                "src/billing.rs",
                "# Calculate Tax",
                &[],
            )
            .expect("upsert");

        let report_with_db =
            WikiLinter::lint_wiki_with_storage(wiki_root, None, Some(&storage)).expect("lint");
        assert_eq!(
            report_with_db.broken_links.len(),
            0,
            "SQLite fallback must resolve false positive broken links"
        );
    }
}
