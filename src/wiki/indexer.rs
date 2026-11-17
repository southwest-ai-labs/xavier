//! Master Catalog and Map of Content (MOC) Indexer Generator.
//!
//! Generates `.xavier/wiki/index.md`, synthesizing repository metadata, architectural hubs,
//! categorized domain entities, and recent changelog chronicle entries.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use chrono::Utc;
use code_graph::types::{HubNode, IndexStats};
use tracing::info;

use crate::wiki::log::WikiLogManager;

/// Name of the master wiki index file.
pub const WIKI_INDEX_FILE: &str = "index.md";

/// Builder for generating the master Map of Content (MOC) `index.md`.
pub struct WikiCatalogBuilder {
    wiki_root: PathBuf,
}

impl WikiCatalogBuilder {
    /// Creates a new catalog builder targeting the given wiki directory.
    pub fn new(wiki_root: impl Into<PathBuf>) -> Self {
        Self {
            wiki_root: wiki_root.into(),
        }
    }

    /// Path to the master index: `<wiki_root>/index.md`.
    pub fn index_path(&self) -> PathBuf {
        self.wiki_root.join(WIKI_INDEX_FILE)
    }

    /// Generates `.xavier/wiki/index.md` from repository stats, hubs, and entities.
    pub fn generate_index(
        &self,
        repo_name: &str,
        stats: Option<&IndexStats>,
        hubs: &[HubNode],
    ) -> Result<String> {
        let _start = Instant::now();
        let mut out = String::new();

        out.push_str(&format!("# {}\n\n", repo_name));
        out.push_str("> Living Knowledge Wiki — compiled via Xavier Brain 2 engine.\n\n");
        out.push_str(&format!("*Last compiled: {}*\n\n", Utc::now().to_rfc3339()));

        // Section 1: Overview & Stats
        out.push_str("## 1. Repository Overview\n\n");
        if let Some(s) = stats {
            out.push_str("| Metric | Count |\n");
            out.push_str("| --- | ---: |\n");
            out.push_str(&format!(
                "| **Total Indexed Files** | {} |\n",
                s.total_files
            ));
            out.push_str(&format!(
                "| **Total Code Symbols** | {} |\n",
                s.total_symbols
            ));
            out.push_str(&format!("| **Total Imports** | {} |\n\n", s.total_imports));

            if !s.languages.is_empty() {
                out.push_str("### Language Breakdown\n\n");
                out.push_str("| Language | Files |\n");
                out.push_str("| --- | ---: |\n");
                for lc in &s.languages {
                    out.push_str(&format!("| `{}` | {} |\n", lc.lang.as_str(), lc.count));
                }
                out.push('\n');
            }
        } else {
            out.push_str("*Repository statistics currently being indexed.*\n\n");
        }

        // Section 2: Architectural Hubs (High Degree Centrality)
        out.push_str("## 2. Key Architectural Hubs\n\n");
        if hubs.is_empty() {
            out.push_str("*No high-degree hubs detected yet.*\n\n");
        } else {
            out.push_str("| Symbol | In-Degree | Out-Degree | Total Connections |\n");
            out.push_str("| --- | ---: | ---: | ---: |\n");
            for hub in hubs {
                let name = &hub.symbol.name;
                out.push_str(&format!(
                    "| [[{name}]] | {} | {} | {} |\n",
                    hub.incoming, hub.outgoing, hub.total
                ));
            }
            out.push('\n');
        }

        // Section 3: Entity Catalog by Language & Domain
        out.push_str("## 3. Entity Catalog\n\n");
        let entities_dir = self.wiki_root.join("entities");
        if entities_dir.exists() {
            let mut by_lang: BTreeMap<String, Vec<String>> = BTreeMap::new();
            if let Ok(entries) = std::fs::read_dir(&entities_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let lang = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("other")
                            .to_string();
                        if let Ok(files) = std::fs::read_dir(&path) {
                            for f in files.flatten() {
                                let fpath = f.path();
                                if fpath.extension().and_then(|e| e.to_str()) == Some("md") {
                                    if let Some(stem) = fpath.file_stem().and_then(|s| s.to_str()) {
                                        by_lang
                                            .entry(lang.clone())
                                            .or_default()
                                            .push(stem.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if by_lang.is_empty() {
                out.push_str("*No compiled entity dossiers found.*\n\n");
            } else {
                for (lang, mut entities) in by_lang {
                    entities.sort();
                    out.push_str(&format!(
                        "### Language: `{lang}` ({} entities)\n\n",
                        entities.len()
                    ));
                    for ent in entities {
                        out.push_str(&format!("- [[{ent}]]\n"));
                    }
                    out.push('\n');
                }
            }
        } else {
            out.push_str("*Entities directory not yet generated.*\n\n");
        }

        // Section 4: Recent Chronicle Changes
        out.push_str("## 4. Recent Chronicle Changes\n\n");
        let log_mgr = WikiLogManager::new(&self.wiki_root);
        if let Ok(log_content) = log_mgr.read_log() {
            let recent_lines: Vec<&str> = log_content
                .lines()
                .filter(|line| line.starts_with("## "))
                .take(5)
                .collect();

            if recent_lines.is_empty() {
                out.push_str("*No chronicle history recorded yet.*\n\n");
            } else {
                for header in recent_lines {
                    out.push_str(&format!("- {header}\n"));
                }
                out.push('\n');
                out.push_str("*(See `[[log]]` for full chronological history)*\n\n");
            }
        } else {
            out.push_str("*Chronicle log not found.*\n\n");
        }

        // Write index file
        std::fs::create_dir_all(&self.wiki_root).with_context(|| {
            format!(
                "Failed to create wiki directory: {}",
                self.wiki_root.display()
            )
        })?;
        std::fs::write(self.index_path(), &out).with_context(|| {
            format!(
                "Failed to write index file: {}",
                self.index_path().display()
            )
        })?;

        info!(path = %self.index_path().display(), "Master wiki catalog index generated");

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_index_with_stats_and_hubs() {
        let temp = TempDir::new().expect("tempdir");
        let builder = WikiCatalogBuilder::new(temp.path());

        let stats = IndexStats {
            total_files: 42,
            total_symbols: 350,
            total_imports: 120,
            duration_ms: 15,
            languages: vec![
                code_graph::types::LanguageCount {
                    lang: code_graph::types::Language::Rust,
                    count: 35,
                },
                code_graph::types::LanguageCount {
                    lang: code_graph::types::Language::TypeScript,
                    count: 7,
                },
            ],
        };

        let mut sym = code_graph::types::Symbol::default();
        sym.name = "ExecutionEngine".to_string();

        let hubs = vec![HubNode {
            symbol: sym,
            incoming: 25,
            outgoing: 12,
            total: 37,
        }];

        let index_content = builder
            .generate_index("Xavier Engine", Some(&stats), &hubs)
            .expect("generate index");

        assert!(index_content.contains("# Xavier Engine"));
        assert!(index_content.contains("**Total Indexed Files** | 42"));
        assert!(index_content.contains("**Total Code Symbols** | 350"));
        assert!(index_content.contains("[[ExecutionEngine]]"));
        assert!(builder.index_path().exists());
    }
}
