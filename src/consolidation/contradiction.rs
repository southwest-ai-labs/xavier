//! Contradiction Detection and Reconciliation for Memory Consolidation.
//!
//! Reconciles contradictory facts across clustered memories (e.g. configuration updates,
//! port changes, status transitions) in favor of newer/higher-quality statements.

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{debug, info};

use crate::memory::manager::ManagedMemory;

/// Represents a single contradiction between two statements that was resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedContradiction {
    /// Outdated or superseded statement.
    pub old_statement: String,
    /// Fresh or authoritative replacement statement.
    pub new_statement: String,
    /// Explanation of why this resolution was selected.
    pub reason: String,
}

/// Result of contradiction reconciliation across a cluster of memories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciledDocument {
    /// Canonical unified content with contradictions resolved.
    pub canonical_content: String,
    /// List of contradictions that were reconciled.
    pub contradictions_resolved: Vec<ResolvedContradiction>,
    /// Whether any potential conflicts could not be deterministically resolved.
    pub has_unresolved_conflicts: bool,
}

/// Detects contradictions and synthesizes a reconciled document from clustered memories.
pub fn reconcile_memories_deterministically(memories: &[ManagedMemory]) -> ReconciledDocument {
    if memories.is_empty() {
        return ReconciledDocument {
            canonical_content: String::new(),
            contradictions_resolved: Vec::new(),
            has_unresolved_conflicts: false,
        };
    }

    // Sort by: quality score descending, then recency descending
    let mut ordered: Vec<&ManagedMemory> = memories.iter().collect();
    ordered.sort_by(|a, b| {
        b.quality
            .overall
            .partial_cmp(&a.quality.overall)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.last_access.cmp(&a.last_access))
            .then_with(|| b.created_at.cmp(&a.created_at))
    });

    let primary = ordered[0];
    let mut final_sentences: Vec<String> = extract_clean_sentences(&primary.doc.content);
    let mut resolved_contradictions = Vec::new();
    let mut seen_normalized: HashSet<String> = final_sentences
        .iter()
        .map(|s| normalize_sentence(s))
        .collect();

    for secondary in ordered.iter().skip(1) {
        let sec_sentences = extract_clean_sentences(&secondary.doc.content);
        for sec_s in sec_sentences {
            let norm_sec = normalize_sentence(&sec_s);
            if seen_normalized.contains(&norm_sec) {
                continue;
            }

            // Check if sec_s contradicts any sentence in final_sentences
            let mut contradicted_idx = None;
            for (idx, prim_s) in final_sentences.iter().enumerate() {
                if is_contradictory_topic(prim_s, &sec_s) {
                    contradicted_idx = Some(idx);
                    break;
                }
            }

            if let Some(idx) = contradicted_idx {
                // Primary has higher quality/recency; retain primary and log superseded secondary
                let old_s = sec_s.clone();
                let prim_s = final_sentences[idx].clone();
                resolved_contradictions.push(ResolvedContradiction {
                    old_statement: old_s,
                    new_statement: prim_s,
                    reason: "Superseded by higher quality/more recent memory statement".to_string(),
                });
            } else {
                seen_normalized.insert(norm_sec);
                final_sentences.push(sec_s);
            }
        }
    }

    let canonical_content = final_sentences.join(" ");

    ReconciledDocument {
        canonical_content,
        contradictions_resolved: resolved_contradictions,
        has_unresolved_conflicts: false,
    }
}

/// Normalizes a sentence for duplicate and similarity matching.
fn normalize_sentence(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extracts sentences by splitting on punctuation.
fn extract_clean_sentences(content: &str) -> Vec<String> {
    content
        .split(['.', '!', '?', '\n'])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Heuristic contradiction check: detects if two statements discuss the same configuration/key
/// but state conflicting values (e.g. different numbers/ports/boolean values).
fn is_contradictory_topic(a: &str, b: &str) -> bool {
    let norm_a = normalize_sentence(a);
    let norm_b = normalize_sentence(b);

    let words_a: HashSet<&str> = norm_a.split_whitespace().collect();
    let words_b: HashSet<&str> = norm_b.split_whitespace().collect();

    let intersection = words_a.intersection(&words_b).count();
    let min_len = words_a.len().min(words_b.len());

    // High lexical overlap (>60% of smaller sentence) but different numeric/value tokens
    if min_len >= 3 && (intersection as f32 / min_len as f32) > 0.6 {
        let nums_a: Vec<&str> = words_a
            .iter()
            .filter(|w| w.chars().any(|c| c.is_ascii_digit()))
            .copied()
            .collect();
        let nums_b: Vec<&str> = words_b
            .iter()
            .filter(|w| w.chars().any(|c| c.is_ascii_digit()))
            .copied()
            .collect();

        if !nums_a.is_empty() && !nums_b.is_empty() && nums_a != nums_b {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::manager::{ManagedMemory, MemoryQuality};
    use crate::memory::qmd_memory::MemoryDocument;

    #[test]
    fn test_reconcile_contradictory_port_numbers() {
        let doc1 = MemoryDocument {
            id: Some("mem_1".to_string()),
            path: "mem_1.md".to_string(),
            content: "The Xavier daemon listens on port 8006".to_string(),
            ..Default::default()
        };
        let quality1 = MemoryQuality {
            overall: 0.95, // Higher quality / newer
            ..Default::default()
        };

        let mem1 = ManagedMemory {
            doc: doc1,
            priority: crate::memory::manager::MemoryPriority::High,
            quality: quality1,
            access_count: 10,
            last_access: Some(Timestamp::now()),
            created_at: Some(Timestamp::now()),
            size_bytes: 0,
        };

        let doc2 = MemoryDocument {
            id: Some("mem_2".to_string()),
            path: "mem_2.md".to_string(),
            content: "The Xavier daemon listens on port 8000".to_string(),
            ..Default::default()
        };
        let quality2 = MemoryQuality {
            overall: 0.70, // Older/lower quality
            ..Default::default()
        };

        let mem2 = ManagedMemory {
            doc: doc2,
            priority: crate::memory::manager::MemoryPriority::Low,
            quality: quality2,
            access_count: 2,
            last_access: Some(Timestamp::now()),
            created_at: Some(Timestamp::now()),
            size_bytes: 0,
        };

        let result = reconcile_memories_deterministically(&[mem1, mem2]);

        assert!(result.canonical_content.contains("8006"));
        assert!(!result.canonical_content.contains("8000"));
        assert_eq!(result.contradictions_resolved.len(), 1);
        assert!(result.contradictions_resolved[0]
            .old_statement
            .contains("8000"));
        assert!(result.contradictions_resolved[0]
            .new_statement
            .contains("8006"));
    }
}
