//! Markdown and Wikilink AST Parser for Living Wiki.
//!
//! Extracts frontmatter metadata, inline tags, section anchors, aliases, and transclusions
//! (`[[TargetNote#Heading|Display Text]]`, `![[EmbeddedNote]]`) from Markdown documents.

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Detailed representation of an extracted WikiLink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    /// Target note or entity identifier (e.g. `Compiler`, `src/lib.rs`).
    pub target: String,
    /// Optional section heading anchor (e.g. `#Architecture`).
    pub section: Option<String>,
    /// Optional display alias text (e.g. `|Execution Pipeline`).
    pub alias: Option<String>,
    /// Whether this is a transclusion / file embed (`![[...]]`).
    pub is_embed: bool,
}

/// Structure representing parsed markdown document fields.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedMarkdownNote {
    /// Note title extracted from frontmatter or first H1 heading.
    pub title: Option<String>,
    /// Inline and frontmatter tags (e.g. `["wiki", "brain2"]`).
    pub tags: Vec<String>,
    /// Simple string targets of referenced notes (for backward compatibility).
    pub wikilinks: Vec<String>,
    /// Rich parsed wikilinks with anchors, aliases, and embed metadata.
    pub rich_links: Vec<WikiLink>,
    /// Markdown body content (stripped of YAML frontmatter).
    pub body: String,
    /// Key-value frontmatter metadata map.
    pub frontmatter: HashMap<String, serde_json::Value>,
}

static RE_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|\s)#([a-zA-Z0-9_\-\/]+)").unwrap());

static RE_RICH_WIKILINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(!?)\[\[([^\]\|#]+)(?:#([^\]\|]+))?(?:\|([^\]]+))?\]\]").unwrap()
});

/// Helper to recursively extract tags from YAML frontmatter values.
fn extract_tags_from_value(val: &serde_json::Value, tags: &mut Vec<String>) {
    match val {
        serde_json::Value::String(s) => {
            let t = s.trim().trim_start_matches('#').to_string();
            if !t.is_empty() && !tags.contains(&t) {
                tags.push(t);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                extract_tags_from_value(item, tags);
            }
        }
        _ => {}
    }
}

/// Parse markdown frontmatter (YAML `---` block), inline tags (`#tag`), and wikilinks (`[[link]]`).
pub fn parse_markdown_content(raw_content: &str) -> ParsedMarkdownNote {
    let mut note = ParsedMarkdownNote::default();
    let trimmed = raw_content.trim_start();
    let mut body_str = raw_content.to_string();

    // 1. Extract YAML Frontmatter if present
    if trimmed.starts_with("---") {
        let lines: Vec<&str> = trimmed.lines().collect();
        if lines.len() > 1 {
            let mut end_idx = None;
            for (i, line) in lines.iter().enumerate().skip(1) {
                if line.trim() == "---" {
                    end_idx = Some(i);
                    break;
                }
            }

            if let Some(end) = end_idx {
                let yaml_str = lines[1..end].join("\n");
                if let Ok(yaml_val) = serde_yaml::from_str::<serde_json::Value>(&yaml_str) {
                    if let Some(obj) = yaml_val.as_object() {
                        for (k, v) in obj {
                            note.frontmatter.insert(k.clone(), v.clone());
                        }

                        // Extract special frontmatter keys
                        if let Some(t) = obj.get("title").and_then(|v| v.as_str()) {
                            note.title = Some(t.to_string());
                        }

                        if let Some(tags_val) = obj.get("tags") {
                            extract_tags_from_value(tags_val, &mut note.tags);
                        }
                    }
                }
                body_str = lines[(end + 1)..].join("\n");
            }
        }
    }

    note.body = body_str.trim().to_string();

    // 2. Derive title if not in frontmatter
    if note.title.is_none() {
        for line in note.body.lines() {
            let line_trim = line.trim();
            if let Some(stripped) = line_trim.strip_prefix("# ") {
                note.title = Some(stripped.trim().to_string());
                break;
            }
        }
    }

    // 3. Extract Wikilinks with full anchor and alias support
    for cap in RE_RICH_WIKILINK.captures_iter(&note.body) {
        let is_embed = cap.get(1).map(|m| !m.as_str().is_empty()).unwrap_or(false);
        let target = cap
            .get(2)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        let section = cap
            .get(3)
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.is_empty());
        let alias = cap
            .get(4)
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.is_empty());

        if !target.is_empty() {
            if !note.wikilinks.contains(&target) {
                note.wikilinks.push(target.clone());
            }

            note.rich_links.push(WikiLink {
                target,
                section,
                alias,
                is_embed,
            });
        }
    }

    // 4. Extract inline hashtags: #tag_name
    for cap in RE_TAG.captures_iter(&note.body) {
        if let Some(matched) = cap.get(1) {
            let tag = matched.as_str().trim().to_string();
            if !tag.is_empty()
                && !tag.chars().all(|c| c.is_ascii_digit())
                && !note.tags.contains(&tag)
            {
                note.tags.push(tag);
            }
        }
    }

    note
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rich_wikilinks() {
        let md = r#"---
title: Test Document
tags: [architecture, brain2]
---
# Main Document Title

Check out [[Compiler]] and [[StorageEngine#Transactions|Atomic DB]].
Also see this embedded diagram: ![[diagram.png]].
And another link: [[WikiEntity#Methods]].
Tags: #core #engine
"#;

        let note = parse_markdown_content(md);
        assert_eq!(note.title.as_deref(), Some("Test Document"));
        assert!(note.tags.contains(&"architecture".to_string()));
        assert!(note.tags.contains(&"brain2".to_string()));
        assert!(note.tags.contains(&"core".to_string()));

        assert_eq!(
            note.wikilinks,
            vec![
                "Compiler".to_string(),
                "StorageEngine".to_string(),
                "diagram.png".to_string(),
                "WikiEntity".to_string(),
            ]
        );

        assert_eq!(note.rich_links.len(), 4);
        assert_eq!(
            note.rich_links[1],
            WikiLink {
                target: "StorageEngine".to_string(),
                section: Some("Transactions".to_string()),
                alias: Some("Atomic DB".to_string()),
                is_embed: false,
            }
        );
        assert_eq!(
            note.rich_links[2],
            WikiLink {
                target: "diagram.png".to_string(),
                section: None,
                alias: None,
                is_embed: true,
            }
        );
    }
}
