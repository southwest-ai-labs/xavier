//! Compiled Project Dossier Engine.
//!
//! Synthesizes project overview, architectural hubs, active ADR concepts,
//! and recent changelog chronicle events into a high-density, noise-free dossier
//! for AI agent context injection.

use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, info};

use crate::context::builder::matches_project_context;
use crate::context::ContextDocument;
use crate::wiki::indexer::WikiCatalogBuilder;
use crate::wiki::log::WikiLogManager;

/// High-density compiled project dossier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledProjectDossier {
    pub project_id: String,
    pub overview: String,
    pub key_hubs: Vec<String>,
    pub active_concepts: Vec<String>,
    pub recent_chronicle: Vec<String>,
    pub reference_count: usize,
}

impl CompiledProjectDossier {
    /// Formats the dossier into a compact Markdown context block for agent injection.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Project Dossier: {}\n\n", self.project_id));

        if !self.overview.is_empty() {
            out.push_str("## Overview\n");
            out.push_str(&self.overview);
            out.push_str("\n\n");
        }

        if !self.key_hubs.is_empty() {
            out.push_str("## Key Architectural Hubs\n");
            for hub in &self.key_hubs {
                out.push_str(&format!("- [[{hub}]]\n"));
            }
            out.push('\n');
        }

        if !self.active_concepts.is_empty() {
            out.push_str("## Core Concepts & Guidelines\n");
            for c in &self.active_concepts {
                out.push_str(&format!("- [[{c}]]\n"));
            }
            out.push('\n');
        }

        if !self.recent_chronicle.is_empty() {
            out.push_str("## Recent Chronicle Events\n");
            for event in &self.recent_chronicle {
                out.push_str(&format!("- {event}\n"));
            }
            out.push('\n');
        }

        if self.reference_count > 0 {
            out.push_str(&format!(
                "*(Filtered out {} verbose execution logs; available on-demand via REF)*\n",
                self.reference_count
            ));
        }

        out
    }
}

/// Checks if a document represents raw terminal or runner execution noise.
pub fn is_execution_noise(doc: &ContextDocument) -> bool {
    let path = &doc.path;
    let content = &doc.content;

    path.contains("gestalt/bus/executions")
        || path.contains("tasks/runs/")
        || content.contains("[run_started]")
        || content.contains("[run_finished]")
        || content.contains("is_heartbeat_tick")
        || content.contains("PID ") && content.contains("%CPU")
}

/// Filter documents for project_id while stripping execution noise.
pub fn filter_noise_free_project_documents(
    documents: &[ContextDocument],
    project_id: &str,
) -> (Vec<ContextDocument>, usize) {
    let mut clean = Vec::new();
    let mut noise_count = 0;

    for doc in documents {
        if matches_project_context(doc, project_id) {
            if is_execution_noise(doc) {
                noise_count += 1;
            } else {
                clean.push(doc.clone());
            }
        }
    }

    (clean, noise_count)
}

/// Project Dossier Builder synthesizing wiki assets.
pub struct ProjectDossierBuilder;

impl ProjectDossierBuilder {
    /// Compiles a project dossier for `project_id` using wiki assets at `wiki_root`.
    pub fn build(
        project_id: &str,
        wiki_root: Option<&Path>,
        raw_documents: &[ContextDocument],
    ) -> CompiledProjectDossier {
        let (clean_docs, noise_count) =
            filter_noise_free_project_documents(raw_documents, project_id);

        let mut overview = String::new();
        let mut key_hubs = Vec::new();
        let mut active_concepts = Vec::new();
        let mut recent_chronicle = Vec::new();

        if let Some(root) = wiki_root {
            // 1. Read index.md if present
            let index_path = root.join("index.md");
            if let Ok(index_content) = std::fs::read_to_string(&index_path) {
                // Extract first 15 lines of overview
                let lines: Vec<&str> = index_content.lines().take(15).collect();
                overview = lines.join("\n");

                // Extract key architectural hubs
                for line in index_content.lines() {
                    if line.starts_with("| [[") {
                        if let Some(end) = line.find("]]") {
                            let hub_name = &line[4..end];
                            key_hubs.push(hub_name.to_string());
                        }
                    }
                }
            }

            // 2. Read recent entries from log.md
            let log_mgr = WikiLogManager::new(root);
            if let Ok(log_content) = log_mgr.read_log() {
                for line in log_content.lines().filter(|l| l.starts_with("## ")).take(3) {
                    recent_chronicle.push(line.trim_start_matches("## ").to_string());
                }
            }

            // 3. Scan concepts/ directory
            let concepts_dir = root.join("concepts");
            if concepts_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(concepts_dir) {
                    for entry in entries.flatten() {
                        if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                            active_concepts.push(stem.to_string());
                        }
                    }
                }
            }
        }

        // Add any non-noise docs to overview if overview is empty
        if overview.is_empty() && !clean_docs.is_empty() {
            overview = format!("Synthesized from {} project documents.", clean_docs.len());
        }

        CompiledProjectDossier {
            project_id: project_id.to_string(),
            overview,
            key_hubs,
            active_concepts,
            recent_chronicle,
            reference_count: noise_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_filtering() {
        let mut doc_noise = ContextDocument::new(
            "1",
            "sess_1",
            "system",
            "[run_started] agy.exe\n[run_finished]",
        )
        .with_path("gestalt/bus/executions/run_1.log");
        doc_noise.metadata = serde_json::json!({ "project": "xavier" });

        let mut doc_clean =
            ContextDocument::new("2", "sess_1", "assistant", "# Architecture Overview")
                .with_path("docs/architecture.md");
        doc_clean.metadata = serde_json::json!({ "project": "xavier" });

        assert!(is_execution_noise(&doc_noise));
        assert!(!is_execution_noise(&doc_clean));

        let (clean, noise) = filter_noise_free_project_documents(&[doc_noise, doc_clean], "xavier");
        assert_eq!(clean.len(), 1);
        assert_eq!(clean[0].id, "2");
        assert_eq!(noise, 1);
    }
}
