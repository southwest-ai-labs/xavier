//! Living Wiki core module.
//!
//! Provides schema, compiling, indexing, and logging for compiled codebase knowledge.

pub mod compiler;
pub mod gpu_vram;
pub mod graph;
pub mod indexer;
pub mod linter;
pub mod log;
pub mod parser;
pub mod prune;
pub mod schema;
pub mod storage;

pub use compiler::EntityCompiler;
pub use gpu_vram::{CsrGraph, WikiVramEngine};
pub use graph::{WikiGraph, WikiGraphEdge, WikiGraphNode, WikiSubGraph};
pub use indexer::WikiCatalogBuilder;
pub use linter::{BrokenLinkItem, CodeDocDriftItem, LintReport, WikiLinter};
pub use log::WikiLogManager;
pub use parser::{parse_markdown_content, ParsedMarkdownNote, WikiLink};
pub use prune::{PruneStats, WikiPruner};
pub use schema::{WikiConcept, WikiEntity, WikiLogEntry, WikiSchemaError};
pub use storage::{StoredWikiNode, StoredWikiNodeHeader, WikiStorageEngine, WIKI_SQLITE_FILE};
