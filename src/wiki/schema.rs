//! Living Wiki Core Schema & Data Types.
//!
//! Represents compiled codebase knowledge, architectural concepts, and chronological logs.

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

/// Custom error types for living wiki schema serialization and parsing.
#[derive(Debug, thiserror::Error)]
pub enum WikiSchemaError {
    /// Frontmatter opening delimiter `---` is missing.
    #[error("Missing frontmatter opening delimiter '---'")]
    MissingOpeningDelimiter,

    /// Frontmatter closing delimiter `---` is missing.
    #[error("Missing frontmatter closing delimiter '---'")]
    MissingClosingDelimiter,

    /// YAML deserialization error.
    #[error("YAML deserialization error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    /// JSON error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Convenience result alias for wiki schema operations.
pub type Result<T> = std::result::Result<T, WikiSchemaError>;

/// Core schema representing a code symbol (struct, function, trait, module, etc.) in the living wiki.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiEntity {
    /// Unique identifier for the symbol (e.g. ULID or deterministic hash).
    pub id: String,
    /// Human-readable symbol name (e.g. `WikiEntity` or `compile_ast`).
    pub name: String,
    /// Symbol classification: "struct" | "function" | "trait" | "module" | etc.
    pub kind: String,
    /// Source programming language (e.g. "rust", "typescript", "python").
    pub lang: String,
    /// Relative or canonical file path where the entity is declared.
    pub file_path: String,
    /// Optional function/type signature extracted from AST.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Markdown documentation summary or description.
    #[serde(default)]
    pub summary: String,
    /// Outgoing symbol dependencies/calls.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calls: Vec<String>,
    /// Incoming call edges (callers referencing this symbol).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub called_by: Vec<String>,
    /// Wiki cross-references (e.g. `[[WikiConcept]]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wikilinks: Vec<String>,
    /// Last updated timestamp in UTC.
    pub updated_at: Timestamp,
}

#[derive(Debug, Serialize, Deserialize)]
struct WikiEntityFrontmatter {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub lang: String,
    pub file_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calls: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub called_by: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wikilinks: Vec<String>,
    pub updated_at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

impl WikiEntity {
    /// Creates a new `WikiEntity` with required identification and metadata fields.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        kind: impl Into<String>,
        lang: impl Into<String>,
        file_path: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind: kind.into(),
            lang: lang.into(),
            file_path: file_path.into(),
            signature: None,
            summary: String::new(),
            calls: Vec::new(),
            called_by: Vec::new(),
            wikilinks: Vec::new(),
            updated_at: Timestamp::now(),
        }
    }

    /// Renders this entity as a Markdown document with YAML frontmatter.
    ///
    /// Structured metadata is written to the frontmatter header while the documentation
    /// summary constitutes the Markdown body.
    pub fn to_markdown_with_frontmatter(&self) -> String {
        let fm = WikiEntityFrontmatter {
            id: self.id.clone(),
            name: self.name.clone(),
            kind: self.kind.clone(),
            lang: self.lang.clone(),
            file_path: self.file_path.clone(),
            signature: self.signature.clone(),
            calls: self.calls.clone(),
            called_by: self.called_by.clone(),
            wikilinks: self.wikilinks.clone(),
            updated_at: self.updated_at,
            summary: None,
        };
        render_frontmatter(&fm, &self.summary)
    }

    /// Parses a Markdown document with YAML frontmatter into a `WikiEntity`.
    pub fn from_markdown_with_frontmatter(content: &str) -> Result<Self> {
        let (yaml_str, body_str) = split_frontmatter(content)?;
        let fm: WikiEntityFrontmatter = serde_yaml::from_str(yaml_str)?;
        let summary = if !body_str.is_empty() {
            body_str.to_string()
        } else {
            fm.summary.unwrap_or_default()
        };

        Ok(Self {
            id: fm.id,
            name: fm.name,
            kind: fm.kind,
            lang: fm.lang,
            file_path: fm.file_path,
            signature: fm.signature,
            summary,
            calls: fm.calls,
            called_by: fm.called_by,
            wikilinks: fm.wikilinks,
            updated_at: fm.updated_at,
        })
    }
}

/// Core schema representing architectural concepts, ADRs, or workflow notes in the living wiki.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiConcept {
    /// Unique identifier for the concept note.
    pub id: String,
    /// Title of the concept note.
    pub title: String,
    /// Category classification: "architecture" | "adr" | "workflow" | etc.
    pub category: String,
    /// Markdown body content of the concept note.
    #[serde(default)]
    pub content: String,
    /// Associated code entities or symbols.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_entities: Vec<String>,
    /// Descriptive taxonomy tags (e.g. `["wiki", "brain2"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Last updated timestamp in UTC.
    pub updated_at: Timestamp,
}

#[derive(Debug, Serialize, Deserialize)]
struct WikiConceptFrontmatter {
    pub id: String,
    pub title: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_entities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub updated_at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

impl WikiConcept {
    /// Creates a new `WikiConcept` note.
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        category: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            category: category.into(),
            content: content.into(),
            related_entities: Vec::new(),
            tags: Vec::new(),
            updated_at: Timestamp::now(),
        }
    }

    /// Renders this concept note as a Markdown document with YAML frontmatter.
    pub fn to_markdown_with_frontmatter(&self) -> String {
        let fm = WikiConceptFrontmatter {
            id: self.id.clone(),
            title: self.title.clone(),
            category: self.category.clone(),
            related_entities: self.related_entities.clone(),
            tags: self.tags.clone(),
            updated_at: self.updated_at,
            content: None,
        };
        render_frontmatter(&fm, &self.content)
    }

    /// Parses a Markdown document with YAML frontmatter into a `WikiConcept`.
    pub fn from_markdown_with_frontmatter(content: &str) -> Result<Self> {
        let (yaml_str, body_str) = split_frontmatter(content)?;
        let fm: WikiConceptFrontmatter = serde_yaml::from_str(yaml_str)?;
        let concept_content = if !body_str.is_empty() {
            body_str.to_string()
        } else {
            fm.content.unwrap_or_default()
        };

        Ok(Self {
            id: fm.id,
            title: fm.title,
            category: fm.category,
            content: concept_content,
            related_entities: fm.related_entities,
            tags: fm.tags,
            updated_at: fm.updated_at,
        })
    }
}

/// Core schema representing an append-only change log entry for the living wiki chronicle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiLogEntry {
    /// Event occurrence timestamp in UTC.
    pub timestamp: Timestamp,
    /// Author user or AI agent identifier responsible for the change.
    pub author_or_agent: String,
    /// Classification of event: "ingest" | "compile" | "edit" | "prune" | etc.
    pub event_type: String,
    /// Detailed description or summary of what was changed.
    #[serde(default)]
    pub summary: String,
    /// List of wiki entities or files touched by this event.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub touched_entities: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WikiLogEntryFrontmatter {
    pub timestamp: Timestamp,
    pub author_or_agent: String,
    pub event_type: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub touched_entities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

impl WikiLogEntry {
    /// Creates a new `WikiLogEntry`.
    pub fn new(
        author_or_agent: impl Into<String>,
        event_type: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            timestamp: Timestamp::now(),
            author_or_agent: author_or_agent.into(),
            event_type: event_type.into(),
            summary: summary.into(),
            touched_entities: Vec::new(),
        }
    }

    /// Renders this log entry as a Markdown document with YAML frontmatter.
    pub fn to_markdown_with_frontmatter(&self) -> String {
        let fm = WikiLogEntryFrontmatter {
            timestamp: self.timestamp,
            author_or_agent: self.author_or_agent.clone(),
            event_type: self.event_type.clone(),
            touched_entities: self.touched_entities.clone(),
            summary: None,
        };
        render_frontmatter(&fm, &self.summary)
    }

    /// Parses a Markdown document with YAML frontmatter into a `WikiLogEntry`.
    pub fn from_markdown_with_frontmatter(content: &str) -> Result<Self> {
        let (yaml_str, body_str) = split_frontmatter(content)?;
        let fm: WikiLogEntryFrontmatter = serde_yaml::from_str(yaml_str)?;
        let summary = if !body_str.is_empty() {
            body_str.to_string()
        } else {
            fm.summary.unwrap_or_default()
        };

        Ok(Self {
            timestamp: fm.timestamp,
            author_or_agent: fm.author_or_agent,
            event_type: fm.event_type,
            summary,
            touched_entities: fm.touched_entities,
        })
    }
}

/// Splits a Markdown string with YAML frontmatter into YAML frontmatter string and body string.
fn split_frontmatter(content: &str) -> Result<(&str, &str)> {
    let trimmed = content.trim_start();
    let first_line = trimmed.lines().next().unwrap_or("");
    if first_line.trim() != "---" {
        return Err(WikiSchemaError::MissingOpeningDelimiter);
    }

    let first_line_len = trimmed
        .split_inclusive('\n')
        .next()
        .map(|l| l.len())
        .unwrap_or(first_line.len());
    let rest = &trimmed[first_line_len..];

    let mut offset = 0;
    let mut found = None;

    for line in rest.split_inclusive('\n') {
        let trimmed_line = line.trim_end_matches(['\r', '\n']).trim();
        if trimmed_line == "---" {
            let yaml_part = &rest[..offset];
            let body_part = &rest[offset + line.len()..];
            found = Some((yaml_part, body_part));
            break;
        }
        offset += line.len();
    }

    match found {
        Some((yaml, body)) => Ok((yaml.trim(), body.trim())),
        None => Err(WikiSchemaError::MissingClosingDelimiter),
    }
}

/// Formats a frontmatter-serializable struct and Markdown body into a formatted document.
fn render_frontmatter<T: Serialize>(frontmatter: &T, body: &str) -> String {
    let yaml = serde_yaml::to_string(frontmatter).unwrap_or_default();
    let trimmed_yaml = yaml.trim();
    let clean_yaml = if let Some(stripped) = trimmed_yaml.strip_prefix("---") {
        stripped.trim()
    } else {
        trimmed_yaml
    };

    let trimmed_body = body.trim();
    if trimmed_body.is_empty() {
        format!("---\n{}\n---\n", clean_yaml)
    } else {
        format!("---\n{}\n---\n\n{}\n", clean_yaml, trimmed_body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_time() -> Timestamp {
        "2026-11-17T16:00:00Z".parse::<Timestamp>().unwrap()
    }

    #[test]
    fn test_wiki_entity_json_roundtrip() {
        let entity = WikiEntity {
            id: "entity-uuid-1".to_string(),
            name: "SymbolName".to_string(),
            kind: "struct".to_string(),
            lang: "rust".to_string(),
            file_path: "src/wiki/schema.rs".to_string(),
            signature: Some("pub struct SymbolName;".to_string()),
            summary: "Entity summary description.".to_string(),
            calls: vec!["DependencyA".to_string(), "DependencyB".to_string()],
            called_by: vec!["CallerX".to_string()],
            wikilinks: vec!["[[WikiConcept]]".to_string()],
            updated_at: fixed_time(),
        };

        let json = serde_json::to_string(&entity).expect("json serialize");
        let deserialized: WikiEntity = serde_json::from_str(&json).expect("json deserialize");
        assert_eq!(entity, deserialized);
    }

    #[test]
    fn test_wiki_entity_markdown_frontmatter_roundtrip() {
        let entity = WikiEntity {
            id: "entity-uuid-2".to_string(),
            name: "WikiEntity".to_string(),
            kind: "struct".to_string(),
            lang: "rust".to_string(),
            file_path: "src/wiki/schema.rs".to_string(),
            signature: Some("pub struct WikiEntity".to_string()),
            summary: "Living wiki core symbol schema representation.".to_string(),
            calls: vec!["render_frontmatter".to_string()],
            called_by: vec!["EntityCompiler".to_string()],
            wikilinks: vec!["[[LivingWiki]]".to_string()],
            updated_at: fixed_time(),
        };

        let md = entity.to_markdown_with_frontmatter();
        assert!(md.starts_with("---\n"));
        assert!(md.contains("id: entity-uuid-2"));
        assert!(md.contains("name: WikiEntity"));
        assert!(md.contains("calls:\n- render_frontmatter"));
        assert!(md.ends_with("Living wiki core symbol schema representation.\n"));

        let parsed = WikiEntity::from_markdown_with_frontmatter(&md).expect("parse markdown");
        assert_eq!(entity, parsed);
    }

    #[test]
    fn test_wiki_entity_empty_summary_and_none_signature() {
        let mut entity = WikiEntity::new("ent-3", "EmptyEntity", "module", "rust", "src/lib.rs");
        entity.updated_at = fixed_time();

        let md = entity.to_markdown_with_frontmatter();
        let parsed = WikiEntity::from_markdown_with_frontmatter(&md).expect("parse markdown");
        assert_eq!(entity, parsed);
        assert_eq!(parsed.summary, "");
        assert_eq!(parsed.signature, None);
    }

    #[test]
    fn test_wiki_concept_json_roundtrip() {
        let concept = WikiConcept {
            id: "concept-adr-001".to_string(),
            title: "Living Wiki Architecture".to_string(),
            category: "architecture".to_string(),
            content: "Detailed markdown document content explaining living wiki.".to_string(),
            related_entities: vec!["WikiEntity".to_string(), "WikiConcept".to_string()],
            tags: vec!["wiki".to_string(), "architecture".to_string()],
            updated_at: fixed_time(),
        };

        let json = serde_json::to_string(&concept).expect("json serialize");
        let deserialized: WikiConcept = serde_json::from_str(&json).expect("json deserialize");
        assert_eq!(concept, deserialized);
    }

    #[test]
    fn test_wiki_concept_markdown_frontmatter_roundtrip() {
        let concept = WikiConcept {
            id: "concept-adr-002".to_string(),
            title: "ADR: Flat vs Hierarchical Dossiers".to_string(),
            category: "adr".to_string(),
            content: "We decide to store dossiers under language directories.".to_string(),
            related_entities: vec!["WikiEntity".to_string()],
            tags: vec!["adr".to_string()],
            updated_at: fixed_time(),
        };

        let md = concept.to_markdown_with_frontmatter();
        assert!(md.starts_with("---\n"));
        assert!(md.contains("id: concept-adr-002"));
        assert!(md.contains("category: adr"));

        let parsed = WikiConcept::from_markdown_with_frontmatter(&md).expect("parse markdown");
        assert_eq!(concept, parsed);
    }

    #[test]
    fn test_wiki_log_entry_json_and_markdown_roundtrip() {
        let entry = WikiLogEntry {
            timestamp: fixed_time(),
            author_or_agent: "agent-antigravity".to_string(),
            event_type: "compile".to_string(),
            summary: "Compiled 42 AST symbols into living wiki entities.".to_string(),
            touched_entities: vec!["WikiEntity".to_string(), "WikiConcept".to_string()],
        };

        // JSON roundtrip
        let json = serde_json::to_string(&entry).expect("json serialize");
        let deserialized: WikiLogEntry = serde_json::from_str(&json).expect("json deserialize");
        assert_eq!(entry, deserialized);

        // Markdown frontmatter roundtrip
        let md = entry.to_markdown_with_frontmatter();
        let parsed = WikiLogEntry::from_markdown_with_frontmatter(&md).expect("parse markdown");
        assert_eq!(entry, parsed);
    }

    #[test]
    fn test_crlf_line_endings_handling() {
        let raw = "---\r\nid: ent-crlf\r\nname: CRLFTest\r\nkind: struct\r\nlang: rust\r\nfile_path: src/test.rs\r\nupdated_at: 2026-11-17T16:00:00Z\r\n---\r\n\r\nBody with CRLF endings.\r\n";
        let parsed = WikiEntity::from_markdown_with_frontmatter(raw).expect("parse crlf");
        assert_eq!(parsed.id, "ent-crlf");
        assert_eq!(parsed.name, "CRLFTest");
        assert_eq!(parsed.summary, "Body with CRLF endings.");
    }

    #[test]
    fn test_invalid_markdown_errors() {
        // Missing opening delimiter
        let err1 = WikiEntity::from_markdown_with_frontmatter("No frontmatter here").unwrap_err();
        assert!(matches!(err1, WikiSchemaError::MissingOpeningDelimiter));

        // Missing closing delimiter
        let err2 =
            WikiEntity::from_markdown_with_frontmatter("---\nid: ent-incomplete\n").unwrap_err();
        assert!(matches!(err2, WikiSchemaError::MissingClosingDelimiter));
    }
}
