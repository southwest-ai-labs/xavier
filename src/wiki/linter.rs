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
        let mut report = LintReport::default();

        if !wiki_root.exists() {
            return Ok(report);
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
        for (path, rel, stem) in &note_files {
            if let Ok(content) = std::fs::read_to_string(path) {
                let parsed = parse_markdown_content(&content);

                for link in &parsed.wikilinks {
                    all_referenced_targets.insert(link.clone());

                    // Check if target exists in stems or file paths
                    let exists = all_stems.contains(link.as_str())
                        || all_stems.contains(&format!("{link}.md"))
                        || all_stems.contains(&format!("entities/{link}"))
                        || all_stems.contains(&format!("concepts/{link}"));

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
            }
        }

        // 3. Identify orphan notes (excluding index.md and log.md)
        for (_, rel, stem) in &note_files {
            if rel == "index.md" || rel == "log.md" {
                continue;
            }

            let is_referenced = all_referenced_targets.contains(stem.as_str())
                || all_referenced_targets.contains(rel.as_str());

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
}
