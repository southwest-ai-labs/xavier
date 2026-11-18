//! AST-to-Entity Markdown Compiler.
//!
//! Transforms Tree-sitter AST symbols and call graph edges into rich Markdown dossiers
//! with structured YAML frontmatter under `.xavier/wiki/entities/<lang>/`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use chrono::Utc;
use code_graph::types::{CodeEdge, Symbol, SymbolKind};
use tracing::{debug, info};

use crate::wiki::schema::WikiEntity;

/// Statistics produced after compiling AST symbols into wiki entities.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompileStats {
    /// Number of entity dossier files written or updated.
    pub entities_written: usize,
    /// Total compilation duration in milliseconds.
    pub duration_ms: u64,
}

/// Sanitizes a symbol name for safe cross-platform filesystem filenames.
pub fn sanitize_symbol_filename(name: &str) -> String {
    let mut safe = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            ':' | '/' | '\\' | '<' | '>' | '"' | '|' | '?' | '*' | ' ' | '#' => safe.push('_'),
            _ => safe.push(ch),
        }
    }
    // Trim leading or trailing underscores
    let trimmed = safe.trim_matches('_');
    if trimmed.is_empty() {
        "symbol".to_string()
    } else {
        trimmed.to_string()
    }
}

/// AST-to-Entity Markdown compiler.
pub struct EntityCompiler {
    wiki_root: PathBuf,
}

impl EntityCompiler {
    /// Creates a new compiler targeting `.xavier/wiki/` (or a custom directory).
    pub fn new(wiki_root: impl Into<PathBuf>) -> Self {
        Self {
            wiki_root: wiki_root.into(),
        }
    }

    /// Path to the entities directory: `<wiki_root>/entities/`.
    pub fn entities_dir(&self) -> PathBuf {
        self.wiki_root.join("entities")
    }

    /// Compiles symbols and call edges into Markdown dossiers with YAML frontmatter.
    pub fn compile(&self, symbols: &[Symbol], edges: &[CodeEdge]) -> Result<CompileStats> {
        let start = Instant::now();
        let entities_dir = self.entities_dir();

        // Build adjacency maps for calls and called_by
        let mut outgoing_calls: HashMap<&str, Vec<String>> = HashMap::new();
        let mut incoming_calls: HashMap<&str, Vec<String>> = HashMap::new();

        for edge in edges {
            outgoing_calls
                .entry(&edge.from_symbol)
                .or_default()
                .push(edge.to_symbol.clone());
            incoming_calls
                .entry(&edge.to_symbol)
                .or_default()
                .push(edge.from_symbol.clone());
        }

        let mut written_count = 0;

        for sym in symbols {
            // Filter: focus on key architectural symbols (functions, structs, traits, classes, interfaces, modules)
            if !is_indexable_symbol(&sym.kind) {
                continue;
            }

            let sym_id = sym
                .stable_id
                .clone()
                .unwrap_or_else(|| sym.deterministic_id("wiki"));

            let lang_str = format!("{:?}", sym.lang).to_lowercase();
            let safe_name = sanitize_symbol_filename(&sym.name);
            let target_dir = entities_dir.join(&lang_str);
            std::fs::create_dir_all(&target_dir).with_context(|| {
                format!("Failed to create entity dir: {}", target_dir.display())
            })?;

            let target_path = target_dir.join(format!("{safe_name}.md"));

            let calls = outgoing_calls
                .get(sym_id.as_str())
                .cloned()
                .unwrap_or_default();
            let called_by = incoming_calls
                .get(sym_id.as_str())
                .cloned()
                .unwrap_or_default();

            // Collect wikilinks to referenced symbols
            let mut wikilinks = Vec::new();
            for callee in &calls {
                wikilinks.push(format!("[[{callee}]]"));
            }
            for caller in &called_by {
                wikilinks.push(format!("[[{caller}]]"));
            }
            wikilinks.sort();
            wikilinks.dedup();

            let summary_body = render_entity_markdown_body(sym, &calls, &called_by);

            let mut entity = WikiEntity::new(
                sym_id,
                sym.name.clone(),
                format!("{:?}", sym.kind),
                lang_str,
                sym.file_path.clone(),
            );
            entity.signature = sym.signature.clone();
            entity.summary = summary_body;
            entity.calls = calls;
            entity.called_by = called_by;
            entity.wikilinks = wikilinks;
            entity.updated_at = Utc::now();

            let markdown_content = entity.to_markdown_with_frontmatter();
            std::fs::write(&target_path, markdown_content).with_context(|| {
                format!("Failed to write entity note: {}", target_path.display())
            })?;

            written_count += 1;
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        info!(
            entities_written = written_count,
            duration_ms, "Compiled AST symbols into Living Wiki dossiers"
        );

        Ok(CompileStats {
            entities_written: written_count,
            duration_ms,
        })
    }
}

/// Determines whether a symbol kind warrants a standalone wiki dossier.
fn is_indexable_symbol(kind: &SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Function
            | SymbolKind::Method
            | SymbolKind::Struct
            | SymbolKind::Class
            | SymbolKind::Trait
            | SymbolKind::Component
            | SymbolKind::Module
            | SymbolKind::Enum
    )
}

/// Renders the Markdown body of the entity note.
fn render_entity_markdown_body(sym: &Symbol, calls: &[String], called_by: &[String]) -> String {
    let mut out = String::new();
    let lang_str = format!("{:?}", sym.lang).to_lowercase();

    out.push_str(&format!("# {}\n\n", sym.name));
    out.push_str(&format!(
        "**Kind:** `{:?}` | **Language:** `{}` | **File:** `{}:{}`\n\n",
        sym.kind, lang_str, sym.file_path, sym.start_line
    ));

    if let Some(ref sig) = sym.signature {
        out.push_str("## Signature\n\n");
        out.push_str(&format!("```{lang_str}\n{sig}\n```\n\n"));
    }

    if let Some(ref parent) = sym.parent {
        out.push_str(&format!("**Parent:** `[[{parent}]]`\n\n"));
    }

    if !calls.is_empty() {
        out.push_str("## Outgoing Calls\n\n");
        for target in calls {
            out.push_str(&format!("- [[{target}]]\n"));
        }
        out.push('\n');
    }

    if !called_by.is_empty() {
        out.push_str("## Incoming Callers\n\n");
        for caller in called_by {
            out.push_str(&format!("- [[{caller}]]\n"));
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use code_graph::types::Language;
    use tempfile::TempDir;

    #[test]
    fn test_sanitize_symbol_filename() {
        assert_eq!(sanitize_symbol_filename("Foo::bar"), "Foo__bar");
        assert_eq!(sanitize_symbol_filename("path/to/file"), "path_to_file");
        assert_eq!(sanitize_symbol_filename("generic<T>"), "generic_T");
        assert_eq!(sanitize_symbol_filename(":::"), "symbol");
    }

    #[test]
    fn test_compile_mock_symbols_to_markdown() {
        let temp = TempDir::new().expect("tempdir");
        let compiler = EntityCompiler::new(temp.path());

        let sym = Symbol {
            id: Some(1),
            stable_id: Some("stable_calc".to_string()),
            name: "calculate_tax".to_string(),
            kind: SymbolKind::Function,
            lang: Language::Rust,
            file_path: "src/billing.rs".to_string(),
            start_line: 42,
            end_line: 58,
            start_col: 0,
            end_col: 1,
            signature: Some("pub fn calculate_tax(amount: f64) -> f64".to_string()),
            parent: None,
            complexity: Some(2.5),
            project_root: None,
        };

        let edge = CodeEdge {
            id: None,
            from_symbol: "stable_calc".to_string(),
            to_symbol: "stable_get_rates".to_string(),
            edge_type: code_graph::types::EdgeType::Calls,
            file_path: "src/billing.rs".to_string(),
            line: 45,
            confidence: 1.0,
            metadata: None,
        };

        let stats = compiler
            .compile(&[sym], &[edge])
            .expect("compilation should succeed");

        assert_eq!(stats.entities_written, 1);

        let entity_file = temp
            .path()
            .join("entities")
            .join("rust")
            .join("calculate_tax.md");

        assert!(entity_file.exists(), "entity markdown file must exist");

        let content = std::fs::read_to_string(&entity_file).expect("read content");
        let parsed = WikiEntity::from_markdown_with_frontmatter(&content)
            .expect("markdown must parse with frontmatter");

        assert_eq!(parsed.name, "calculate_tax");
        assert_eq!(parsed.kind, "Function");
        assert_eq!(parsed.lang, "rust");
        assert_eq!(parsed.file_path, "src/billing.rs");
        assert_eq!(parsed.calls, vec!["stable_get_rates".to_string()]);
        assert!(parsed
            .wikilinks
            .contains(&"[[stable_get_rates]]".to_string()));
    }
}
