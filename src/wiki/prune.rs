//! Stale Concept and Deprecation Pruner for Living Wiki.
//!
//! Applies soft deprecation notices to code-drifted entity notes and safely
//! archives unreferenced orphan pages to `.xavier/wiki/archive/`.

use anyhow::{Context, Result};
use jiff::Timestamp;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::wiki::linter::LintReport;

/// Statistics produced after a wiki prune/deprecation pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PruneStats {
    /// Number of notes marked with deprecation banners.
    pub deprecations_applied: usize,
    /// Number of orphan notes moved to archive/.
    pub notes_archived: usize,
}

/// Living Wiki Pruner.
pub struct WikiPruner;

impl WikiPruner {
    /// Applies deprecation tags and archives orphans based on a `LintReport`.
    pub fn prune_wiki(
        wiki_root: &Path,
        lint_report: &LintReport,
        commit_hash: Option<&str>,
        archive_orphans: bool,
    ) -> Result<PruneStats> {
        let mut stats = PruneStats::default();

        if !wiki_root.exists() {
            return Ok(stats);
        }

        let hash_label = commit_hash.unwrap_or("HEAD");
        let date_str = Timestamp::now().strftime("%Y-%m-%d").to_string();

        // 1. Mark code-doc drift with soft deprecation banner
        for drift in &lint_report.drift_warnings {
            let file_path = wiki_root.join(&drift.entity_file);
            if file_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&file_path) {
                    if !content.contains("DEPRECATED / ARCHIVED") {
                        let deprecation_banner = format!(
                            "> [!WARNING]\n> **DEPRECATED / ARCHIVED:** The underlying code symbol (`{}`) was removed or renamed in commit `{}` on {}.\n\n",
                            drift.symbol_name, hash_label, date_str
                        );

                        let updated_content = if let Some(stripped) = content.strip_prefix("---") {
                            // Insert right after closing delimiter
                            if let Some(pos) = stripped.find("\n---") {
                                let split_point = 3 + pos + 4;
                                format!(
                                    "{}\n\n{}{}",
                                    &content[..split_point],
                                    deprecation_banner,
                                    content[split_point..].trim_start()
                                )
                            } else {
                                format!("{deprecation_banner}{content}")
                            }
                        } else {
                            format!("{deprecation_banner}{content}")
                        };

                        std::fs::write(&file_path, updated_content)?;
                        stats.deprecations_applied += 1;
                    }
                }
            }
        }

        // 2. Archive orphans if requested
        if archive_orphans && !lint_report.orphans.is_empty() {
            let archive_dir = wiki_root.join("archive");
            std::fs::create_dir_all(&archive_dir).with_context(|| {
                format!("Failed to create archive dir: {}", archive_dir.display())
            })?;

            for orphan_rel in &lint_report.orphans {
                let src_path = wiki_root.join(orphan_rel);
                if src_path.exists() {
                    let file_name = src_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("orphan.md");
                    let dest_path = archive_dir.join(file_name);
                    std::fs::rename(&src_path, &dest_path)?;
                    stats.notes_archived += 1;
                }
            }
        }

        info!(
            deprecations = stats.deprecations_applied,
            archived = stats.notes_archived,
            "Completed wiki deprecation and archive pruning"
        );

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wiki::linter::{CodeDocDriftItem, LintReport};
    use tempfile::TempDir;

    #[test]
    fn test_wiki_pruner_deprecation_and_archive() {
        let temp = TempDir::new().expect("tempdir");
        let wiki_root = temp.path();

        let entity_path = wiki_root.join("entities/rust/old_fn.md");
        std::fs::create_dir_all(entity_path.parent().unwrap()).unwrap();
        std::fs::write(&entity_path, "# old_fn\n\nExisting documentation.").unwrap();

        let orphan_path = wiki_root.join("orphan_note.md");
        std::fs::write(&orphan_path, "# Orphan\n\nNot linked.").unwrap();

        let mut report = LintReport::default();
        report.drift_warnings.push(CodeDocDriftItem {
            entity_file: "entities/rust/old_fn.md".to_string(),
            symbol_name: "old_fn".to_string(),
            missing_file_or_symbol: "old_fn in src/lib.rs".to_string(),
        });
        report.orphans.push("orphan_note.md".to_string());

        let stats = WikiPruner::prune_wiki(wiki_root, &report, Some("abc1234"), true).unwrap();
        assert_eq!(stats.deprecations_applied, 1);
        assert_eq!(stats.notes_archived, 1);

        // Verify banner was added
        let updated_entity = std::fs::read_to_string(&entity_path).unwrap();
        assert!(updated_entity.contains("DEPRECATED / ARCHIVED"));
        assert!(updated_entity.contains("abc1234"));

        // Verify orphan was moved to archive/
        assert!(!orphan_path.exists());
        assert!(wiki_root.join("archive/orphan_note.md").exists());
    }
}
