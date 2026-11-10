//! Pure type definitions for core logic (BM25, RRF, scoring, snippet extraction)
//!
//! Free of I/O, database dependencies, or async runtimes. WASM compatible.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Clearance levels for security gating.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum ClearanceLevel {
    #[serde(
        rename = "UNCLASSIFIED",
        alias = "unclassified",
        alias = "Unclassified"
    )]
    #[default]
    Unclassified = 0,
    #[serde(rename = "INTERNAL", alias = "internal", alias = "Internal")]
    Internal = 1,
    #[serde(rename = "RESTRICTED", alias = "restricted", alias = "Restricted")]
    Restricted = 2,
    #[serde(
        rename = "CONFIDENTIAL",
        alias = "confidential",
        alias = "Confidential"
    )]
    Confidential = 3,
    #[serde(rename = "SECRET", alias = "secret", alias = "Secret")]
    Secret = 4,
    #[serde(
        rename = "TOPSECRET",
        alias = "top_secret",
        alias = "TopSecret",
        alias = "topsecret"
    )]
    TopSecret = 5,
}

impl From<u8> for ClearanceLevel {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Unclassified,
            1 => Self::Internal,
            2 => Self::Restricted,
            3 => Self::Confidential,
            4 => Self::Secret,
            5 => Self::TopSecret,
            _ => Self::Unclassified,
        }
    }
}

impl From<ClearanceLevel> for u8 {
    fn from(level: ClearanceLevel) -> Self {
        level as u8
    }
}

impl From<&str> for ClearanceLevel {
    fn from(value: &str) -> Self {
        match value.to_ascii_uppercase().as_str() {
            "UNCLASSIFIED" => Self::Unclassified,
            "INTERNAL" => Self::Internal,
            "RESTRICTED" => Self::Restricted,
            "CONFIDENTIAL" => Self::Confidential,
            "SECRET" => Self::Secret,
            "TOPSECRET" | "TOP_SECRET" => Self::TopSecret,
            _ => Self::Unclassified,
        }
    }
}

impl ClearanceLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unclassified => "unclassified",
            Self::Internal => "internal",
            Self::Restricted => "restricted",
            Self::Confidential => "confidential",
            Self::Secret => "secret",
            Self::TopSecret => "top_secret",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "unclassified" => Self::Unclassified,
            "internal" => Self::Internal,
            "restricted" => Self::Restricted,
            "confidential" => Self::Confidential,
            "secret" => Self::Secret,
            _ => Self::TopSecret,
        }
    }
}

/// Context zones for retrieval weighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContextZone {
    #[default]
    Atomic,
    Cluster,
    Global,
    Relational,
}

impl ContextZone {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Atomic => "atomic",
            Self::Cluster => "cluster",
            Self::Global => "global",
            Self::Relational => "relational",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cluster" => Self::Cluster,
            "global" => Self::Global,
            "relational" => Self::Relational,
            _ => Self::Atomic,
        }
    }
}

/// Memory processing levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryLevel {
    #[default]
    Raw,
    Processed,
    Extracted,
    Belief,
}

impl MemoryLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Processed => "processed",
            Self::Extracted => "extracted",
            Self::Belief => "belief",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "processed" => Self::Processed,
            "extracted" => Self::Extracted,
            "belief" => Self::Belief,
            _ => Self::Raw,
        }
    }
}

/// Relation kind descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelationKind {
    pub name: String,
    pub inverse: Option<String>,
}

impl RelationKind {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            inverse: None,
        }
    }
}

/// Modality type classification for multimodal RAG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ModalityType {
    #[default]
    #[serde(rename = "text", alias = "Text")]
    Text,
    #[serde(rename = "image", alias = "Image")]
    Image,
    #[serde(rename = "video_scene", alias = "VideoScene", alias = "video")]
    VideoScene,
    #[serde(rename = "audio", alias = "Audio")]
    Audio,
}

impl ModalityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::VideoScene => "video_scene",
            Self::Audio => "audio",
        }
    }
}

/// Normalized spatial region bounding box (0.0 to 1.0 coordinates).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SpatialRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl SpatialRegion {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Checks if coordinates are finite, non-negative, and fit inside normalized [0.0, 1.0] unit bounds.
    pub fn is_valid(&self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
            && self.x >= 0.0
            && self.y >= 0.0
            && self.width >= 0.0
            && self.height >= 0.0
            && (self.x + self.width) <= 1.0 + f32::EPSILON
            && (self.y + self.height) <= 1.0 + f32::EPSILON
    }
}

/// Millisecond temporal range for video scenes or audio segments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TemporalRange {
    pub start_ms: u64,
    pub end_ms: u64,
}

impl TemporalRange {
    pub fn new(start_ms: u64, end_ms: u64) -> Self {
        Self { start_ms, end_ms }
    }

    /// Validates that start_ms is less than or equal to end_ms.
    pub fn is_valid(&self) -> bool {
        self.start_ms <= self.end_ms
    }

    /// Returns the temporal duration in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.end_ms.saturating_sub(self.start_ms)
    }
}

/// Document representation for memory retrieval and BM25 scoring.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryDocument {
    pub id: Option<String>,
    pub path: String,
    pub content: String,
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub content_vector: Option<Vec<f32>>,
    pub embedding: Vec<f32>,
    #[serde(default)]
    pub cluster_id: Option<String>,
    pub parent_id: Option<String>,
    #[serde(default)]
    pub level: MemoryLevel,
    #[serde(default)]
    pub relation: Option<RelationKind>,
    #[serde(default)]
    pub clearance: ClearanceLevel,
    #[serde(default)]
    pub minhash: Option<Vec<u64>>,
    #[serde(default)]
    pub score: f32,
    #[serde(default)]
    pub source_node_id: Option<String>,
    #[serde(default)]
    pub source_db_id: Option<String>,
    #[serde(default)]
    pub modality: ModalityType,
    #[serde(default)]
    pub spatial: Option<SpatialRegion>,
    #[serde(default)]
    pub temporal: Option<TemporalRange>,
}

impl Default for MemoryDocument {
    fn default() -> Self {
        Self {
            id: None,
            path: String::new(),
            content: String::new(),
            metadata: serde_json::json!({}),
            content_vector: None,
            embedding: Vec::new(),
            cluster_id: None,
            parent_id: None,
            level: MemoryLevel::Raw,
            relation: None,
            clearance: ClearanceLevel::Unclassified,
            minhash: None,
            score: 0.0,
            source_node_id: None,
            source_db_id: None,
            modality: ModalityType::default(),
            spatial: None,
            temporal: None,
        }
    }
}

impl MemoryDocument {
    pub fn estimated_bytes(&self) -> u64 {
        self.id
            .as_ref()
            .map(|value| value.len())
            .unwrap_or_default() as u64
            + self.path.len() as u64
            + self.content.len() as u64
            + self.metadata.to_string().len() as u64
            + self
                .content_vector
                .as_ref()
                .map(|value| value.len() * std::mem::size_of::<f32>())
                .unwrap_or_default() as u64
            + (self.embedding.len() * std::mem::size_of::<f32>()) as u64
            + self.cluster_id.as_ref().map(|s| s.len()).unwrap_or(0) as u64
            + self.parent_id.as_ref().map(|s| s.len()).unwrap_or(0) as u64
            + 1
            + self.relation.as_ref().map(|r| r.name.len()).unwrap_or(0) as u64
            + self
                .minhash
                .as_ref()
                .map(|m| m.len() * std::mem::size_of::<u64>())
                .unwrap_or(0) as u64
            + 4
            + self.source_node_id.as_ref().map(|s| s.len()).unwrap_or(0) as u64
            + self.source_db_id.as_ref().map(|s| s.len()).unwrap_or(0) as u64
            + 1
            + self
                .spatial
                .as_ref()
                .map(|_| std::mem::size_of::<SpatialRegion>() as u64)
                .unwrap_or(0)
            + self
                .temporal
                .as_ref()
                .map(|_| std::mem::size_of::<TemporalRange>() as u64)
                .unwrap_or(0)
    }
}

/// Key event within an episodic session summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub timestamp: DateTime<Utc>,
    pub description: String,
    pub event_type: String,
}

/// Session summary structure for episodic retrieval scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub start_time: DateTime<Utc>,
    pub summary: String,
    pub key_events: Vec<Event>,
    #[serde(default)]
    pub sentiment_timeline: Vec<f32>,
}

/// Entity types in knowledge graph.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Person,
    Organization,
    Location,
    Product,
    Concept,
    Unknown,
}

impl EntityType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Organization => "organization",
            Self::Location => "location",
            Self::Product => "product",
            Self::Concept => "concept",
            Self::Unknown => "unknown",
        }
    }
}

/// Entity record in knowledge graph for semantic scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRecord {
    pub id: String,
    pub name: String,
    pub normalized_name: String,
    pub entity_type: EntityType,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub description: Option<String>,
    pub occurrence_count: usize,
    pub memory_count: usize,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    #[serde(default)]
    pub merged_from: Vec<String>,
    #[serde(default)]
    pub trust_score: f32,
    #[serde(default)]
    pub trust_rank: usize,
}

/// Scored search result returned by retrieval and RRF algorithms.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ScoredResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub source: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub updated_at: Option<i64>,
    #[serde(default)]
    pub zone: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modality_type_defaults_and_serde() {
        assert_eq!(ModalityType::default(), ModalityType::Text);
        assert_eq!(ModalityType::Text.as_str(), "text");
        assert_eq!(ModalityType::Image.as_str(), "image");
        assert_eq!(ModalityType::VideoScene.as_str(), "video_scene");
        assert_eq!(ModalityType::Audio.as_str(), "audio");

        let text_json = serde_json::to_string(&ModalityType::Text).unwrap();
        assert_eq!(text_json, "\"text\"");

        let img: ModalityType = serde_json::from_str("\"Image\"").unwrap();
        assert_eq!(img, ModalityType::Image);

        let video1: ModalityType = serde_json::from_str("\"VideoScene\"").unwrap();
        assert_eq!(video1, ModalityType::VideoScene);

        let video2: ModalityType = serde_json::from_str("\"video\"").unwrap();
        assert_eq!(video2, ModalityType::VideoScene);

        let audio: ModalityType = serde_json::from_str("\"Audio\"").unwrap();
        assert_eq!(audio, ModalityType::Audio);
    }

    #[test]
    fn test_spatial_region_validation() {
        let valid = SpatialRegion::new(0.1, 0.2, 0.5, 0.6);
        assert!(valid.is_valid());

        let invalid_negative = SpatialRegion::new(-0.1, 0.0, 0.5, 0.5);
        assert!(!invalid_negative.is_valid());

        let invalid_overflow_x = SpatialRegion::new(0.8, 0.0, 0.5, 0.5);
        assert!(!invalid_overflow_x.is_valid());

        let invalid_overflow_y = SpatialRegion::new(0.0, 0.8, 0.5, 0.5);
        assert!(!invalid_overflow_y.is_valid());

        let invalid_nan = SpatialRegion::new(f32::NAN, 0.0, 0.5, 0.5);
        assert!(!invalid_nan.is_valid());

        let edge_full = SpatialRegion::new(0.0, 0.0, 1.0, 1.0);
        assert!(edge_full.is_valid());
    }

    #[test]
    fn test_temporal_range_validation_and_duration() {
        let valid = TemporalRange::new(1000, 5000);
        assert!(valid.is_valid());
        assert_eq!(valid.duration_ms(), 4000);

        let instantaneous = TemporalRange::new(500, 500);
        assert!(instantaneous.is_valid());
        assert_eq!(instantaneous.duration_ms(), 0);

        let invalid = TemporalRange::new(6000, 1000);
        assert!(!invalid.is_valid());
        assert_eq!(invalid.duration_ms(), 0);
    }

    #[test]
    fn test_memory_document_default_and_roundtrip() {
        let doc = MemoryDocument::default();
        assert_eq!(doc.modality, ModalityType::Text);
        assert_eq!(doc.spatial, None);
        assert_eq!(doc.temporal, None);

        let mut image_doc = MemoryDocument {
            id: Some("doc-img-1".to_string()),
            path: "images/cat.jpg".to_string(),
            content: "a cute cat on a couch".to_string(),
            modality: ModalityType::Image,
            spatial: Some(SpatialRegion::new(0.1, 0.2, 0.4, 0.5)),
            ..MemoryDocument::default()
        };
        image_doc.embedding = vec![0.1, 0.2, 0.3];

        let serialized = serde_json::to_string(&image_doc).unwrap();
        let deserialized: MemoryDocument = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.modality, ModalityType::Image);
        assert_eq!(
            deserialized.spatial,
            Some(SpatialRegion::new(0.1, 0.2, 0.4, 0.5))
        );
        assert_eq!(deserialized.temporal, None);
        assert_eq!(deserialized, image_doc);

        let mut video_doc = MemoryDocument {
            id: Some("doc-video-1".to_string()),
            path: "videos/lecture.mp4".to_string(),
            content: "lecture intro scene".to_string(),
            modality: ModalityType::VideoScene,
            temporal: Some(TemporalRange::new(0, 15000)),
            ..MemoryDocument::default()
        };
        video_doc.embedding = vec![0.4, 0.5];

        let serialized_video = serde_json::to_string(&video_doc).unwrap();
        let deserialized_video: MemoryDocument = serde_json::from_str(&serialized_video).unwrap();
        assert_eq!(deserialized_video.modality, ModalityType::VideoScene);
        assert_eq!(
            deserialized_video.temporal,
            Some(TemporalRange::new(0, 15000))
        );
        assert_eq!(deserialized_video.spatial, None);
        assert_eq!(deserialized_video, video_doc);
    }

    #[test]
    fn test_legacy_payload_deserialization_backward_compatibility() {
        let legacy_json = serde_json::json!({
            "id": "legacy-doc-123",
            "path": "notes/todo.md",
            "content": "buy milk and eggs",
            "metadata": {"tags": ["grocery"]},
            "embedding": [0.05, 0.12, 0.88]
        })
        .to_string();

        let doc: MemoryDocument = serde_json::from_str(&legacy_json).unwrap();
        assert_eq!(doc.id.as_deref(), Some("legacy-doc-123"));
        assert_eq!(doc.path, "notes/todo.md");
        assert_eq!(doc.content, "buy milk and eggs");
        assert_eq!(doc.modality, ModalityType::Text);
        assert_eq!(doc.spatial, None);
        assert_eq!(doc.temporal, None);
    }
}
