//! Retrieval scoring and ranking algorithms
//!
//! Provides pure, I/O-free scoring logic for working, episodic, and semantic memories.

use crate::types::{ContextZone, EntityRecord, MemoryDocument, ScoredResult, SessionSummary};
use jiff::Timestamp;

pub const EXACT_PHRASE_MATCH_BONUS: f32 = 0.5;
pub const TERM_MATCH_BONUS: f32 = 0.1;
pub const TERM_OCCURRENCE_BONUS: f32 = 0.05;
pub const MAX_TERM_OCCURRENCE_BONUS: f32 = 0.3;
pub const EVENT_PHRASE_MATCH_BONUS: f32 = 0.3;
pub const EVENT_TERM_MATCH_BONUS: f32 = 0.05;

pub const EXACT_ENTITY_MATCH_SCORE: f32 = 1.0;
pub const PARTIAL_ENTITY_MATCH_SCORE: f32 = 0.7;
pub const ENTITY_DESCRIPTION_MATCH_SCORE: f32 = 0.4;
pub const ENTITY_DESCRIPTION_TERM_BONUS: f32 = 0.1;
pub const ENTITY_ALIAS_MATCH_SCORE: f32 = 0.6;
pub const SEMANTIC_CONFIDENCE_MULTIPLIER: f32 = 0.5;

pub struct WorkingScoringParams<'a> {
    pub query_lower: &'a str,
    pub query_terms: &'a [&'a str],
    pub active_zones: Option<&'a Vec<ContextZone>>,
    pub zone_boost_multiplier: f32,
    pub zone_penalty_multiplier: f32,
    pub now: Timestamp,
    pub recency_weight: f32,
    pub half_life_hours: f32,
}

/// Extract temporal anchor from string representation.
pub fn extract_temporal_anchor(s: &str) -> Option<Timestamp> {
    s.parse::<Timestamp>().ok()
}

/// Calculate recency decay using half life in days.
pub fn recency_decay(updated_at_ms: i64, now: Timestamp, half_life_days: f32) -> f32 {
    let updated_at = Timestamp::from_millisecond(updated_at_ms).unwrap_or(now);
    let elapsed_days = (now.duration_since(updated_at).as_secs_f64() / 86400.0).max(0.0) as f32;
    if half_life_days <= 0.0 {
        return 1.0;
    }
    (-elapsed_days / half_life_days).exp()
}

/// Calculate recency boost factor.
pub fn calculate_recency_boost_factor(
    updated_at_ms: Option<i64>,
    now: Timestamp,
    recency_weight: f32,
    half_life_hours: f32,
) -> f32 {
    if recency_weight <= 0.0 {
        return 1.0;
    }
    let Some(updated_at_ms) = updated_at_ms else {
        return 1.0;
    };
    let updated_at = Timestamp::from_millisecond(updated_at_ms).unwrap_or(now);
    let age_hours = (now.duration_since(updated_at).as_secs_f64() / 3600.0).max(0.0) as f32;
    if half_life_hours <= 0.0 {
        return 1.0 + recency_weight;
    }
    1.0 + (recency_weight * (-age_hours / half_life_hours).exp())
}

/// Score single working document.
pub fn score_single_working(
    doc: &MemoryDocument,
    params: &WorkingScoringParams<'_>,
) -> Option<ScoredResult> {
    let content_lower = doc.content.to_lowercase();
    let mut score = 0.0_f32;
    if content_lower.contains(params.query_lower) {
        score += EXACT_PHRASE_MATCH_BONUS;
    }
    for term in params.query_terms {
        if content_lower.contains(term) {
            score += TERM_MATCH_BONUS;
            let count = content_lower.matches(term).count() as f32;
            score += (count * TERM_OCCURRENCE_BONUS).min(MAX_TERM_OCCURRENCE_BONUS);
        }
    }
    if score > 0.0 {
        let doc_zone = doc
            .metadata
            .get("zone")
            .and_then(|v| v.as_str())
            .map(ContextZone::parse)
            .unwrap_or(ContextZone::Atomic);
        let mut final_score = score.min(1.0);
        if let Some(active) = params.active_zones {
            if active.contains(&doc_zone) {
                final_score *= params.zone_boost_multiplier;
            } else {
                final_score *= params.zone_penalty_multiplier;
            }
        }
        let updated_at_ms = doc
            .metadata
            .get("updated_at")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<Timestamp>().ok())
            .map(|dt| dt.as_millisecond());
        let recency = calculate_recency_boost_factor(
            updated_at_ms,
            params.now,
            params.recency_weight,
            params.half_life_hours,
        );
        final_score *= recency;
        Some(ScoredResult {
            id: doc.id.clone().unwrap_or_default(),
            content: doc.content.clone(),
            score: final_score,
            source: "working".to_string(),
            path: doc.path.clone(),
            updated_at: updated_at_ms,
            zone: Some(doc_zone.as_str().to_string()),
        })
    } else {
        None
    }
}

/// Score single episodic session summary.
pub fn score_single_episodic(
    session: &SessionSummary,
    query_lower: &str,
    query_terms: &[&str],
    now: Timestamp,
    recency_weight: f32,
    half_life_hours: f32,
) -> Option<ScoredResult> {
    let summary_lower = session.summary.to_lowercase();
    let mut score = 0.0_f32;
    if summary_lower.contains(query_lower) {
        score += EXACT_PHRASE_MATCH_BONUS;
    }
    for term in query_terms {
        if summary_lower.contains(term) {
            score += TERM_MATCH_BONUS;
            let count = summary_lower.matches(term).count() as f32;
            score += (count * TERM_OCCURRENCE_BONUS).min(MAX_TERM_OCCURRENCE_BONUS);
        }
    }
    for event in &session.key_events {
        let event_lower = event.description.to_lowercase();
        if event_lower.contains(query_lower) {
            score += EVENT_PHRASE_MATCH_BONUS;
        }
        for term in query_terms {
            if event_lower.contains(term) {
                score += EVENT_TERM_MATCH_BONUS;
            }
        }
    }
    if score > 0.0 {
        let mut final_score = score.min(1.0);
        let updated_at_ms = Some(session.start_time.as_millisecond());
        let recency =
            calculate_recency_boost_factor(updated_at_ms, now, recency_weight, half_life_hours);
        final_score *= recency;
        Some(ScoredResult {
            id: session.session_id.clone(),
            content: session.summary.clone(),
            score: final_score,
            source: "episodic".to_string(),
            path: format!("sessions/{}", session.session_id),
            updated_at: updated_at_ms,
            zone: Some("episodic".to_string()),
        })
    } else {
        None
    }
}

/// Score single semantic entity record.
pub fn score_single_semantic(
    entity: &EntityRecord,
    query_lower: &str,
    now: Timestamp,
    recency_weight: f32,
    half_life_hours: f32,
) -> Option<ScoredResult> {
    let name_lower = entity.name.to_lowercase();
    let normalized_lower = entity.normalized_name.to_lowercase();
    let mut score = 0.0_f32;
    if name_lower == query_lower || normalized_lower == query_lower {
        score = EXACT_ENTITY_MATCH_SCORE;
    } else if name_lower.contains(query_lower) || query_lower.contains(&name_lower) {
        score = PARTIAL_ENTITY_MATCH_SCORE;
    } else if let Some(desc) = &entity.description {
        let desc_lower = desc.to_lowercase();
        if desc_lower.contains(query_lower) {
            score = ENTITY_DESCRIPTION_MATCH_SCORE;
        }
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
        for term in &query_terms {
            if desc_lower.contains(term) {
                score += ENTITY_DESCRIPTION_TERM_BONUS;
            }
        }
    } else {
        for alias in &entity.aliases {
            if alias.to_lowercase().contains(query_lower) {
                score = ENTITY_ALIAS_MATCH_SCORE;
                break;
            }
        }
    }
    let mut final_score = (score * SEMANTIC_CONFIDENCE_MULTIPLIER).min(1.0);
    if final_score > 0.0 {
        let updated_at_ms = Some(entity.last_seen.as_millisecond());
        let recency =
            calculate_recency_boost_factor(updated_at_ms, now, recency_weight, half_life_hours);
        final_score *= recency;
        Some(ScoredResult {
            id: entity.id.clone(),
            content: entity.name.clone(),
            score: final_score,
            source: "semantic".to_string(),
            path: format!("entities/{}", entity.id),
            updated_at: updated_at_ms,
            zone: Some("semantic".to_string()),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::Span;

    #[test]
    fn test_recency_boost_zero_weight_returns_one() {
        let now = Timestamp::now();
        let result = calculate_recency_boost_factor(Some(now.as_millisecond()), now, 0.0, 24.0);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn test_recency_boost_no_timestamp_returns_one() {
        let now = Timestamp::now();
        let result = calculate_recency_boost_factor(None, now, 0.5, 24.0);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn test_recency_boost_fresh_returns_boosted() {
        let now = Timestamp::now();
        let result = calculate_recency_boost_factor(Some(now.as_millisecond()), now, 0.5, 24.0);
        assert!((result - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_recency_boost_old_decays() {
        let now = Timestamp::now();
        let hours_ago = now.checked_sub(Span::new().hours(48)).unwrap();
        let result =
            calculate_recency_boost_factor(Some(hours_ago.as_millisecond()), now, 0.5, 24.0);
        assert!(result > 1.0);
        assert!(result < 1.5);
    }

    #[test]
    fn test_recency_boost_negative_half_life() {
        let now = Timestamp::now();
        let result = calculate_recency_boost_factor(Some(now.as_millisecond()), now, 0.5, -1.0);
        assert!((result - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_recency_boost_future_timestamp_uses_now() {
        let now = Timestamp::now();
        let future = now.checked_add(Span::new().hours(24)).unwrap();
        let result = calculate_recency_boost_factor(Some(future.as_millisecond()), now, 0.5, 24.0);
        assert!((result - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_recency_decay_and_temporal_anchor() {
        let now = Timestamp::now();
        let parsed = extract_temporal_anchor("2026-11-19T12:00:00Z");
        assert!(parsed.is_some());

        let decay = recency_decay(now.as_millisecond(), now, 7.0);
        assert!((decay - 1.0).abs() < 0.001);

        let past = now.checked_sub(Span::new().hours(7 * 24)).unwrap();
        let decay_past = recency_decay(past.as_millisecond(), now, 7.0);
        assert!((decay_past - (-1.0_f32).exp()).abs() < 0.01);
    }
}
