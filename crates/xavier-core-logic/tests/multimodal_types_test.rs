use xavier_core_logic::types::{MemoryDocument, ModalityType, SpatialRegion, TemporalRange};

#[test]
fn test_default_serialization() {
    let doc = MemoryDocument::default();

    // Convert to JSON
    let json = serde_json::to_string(&doc).unwrap();

    // Read back
    let parsed: MemoryDocument = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.modality, ModalityType::Text);
    assert_eq!(parsed.spatial, None);
    assert_eq!(parsed.temporal, None);
}

#[test]
fn test_multimodal_serialization() {
    let mut doc = MemoryDocument::default();
    doc.modality = ModalityType::VideoScene;
    doc.spatial = Some(SpatialRegion {
        x: 0.1,
        y: 0.2,
        width: 0.3,
        height: 0.4,
    });
    doc.temporal = Some(TemporalRange {
        start_ms: 1000,
        end_ms: 5000,
    });

    let json = serde_json::to_string(&doc).unwrap();
    let parsed: MemoryDocument = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.modality, ModalityType::VideoScene);
    assert!(parsed.spatial.is_some());

    let spatial = parsed.spatial.unwrap();
    assert_eq!(spatial.x, 0.1);
    assert_eq!(spatial.y, 0.2);
    assert_eq!(spatial.width, 0.3);
    assert_eq!(spatial.height, 0.4);

    assert!(parsed.temporal.is_some());
    let temporal = parsed.temporal.unwrap();
    assert_eq!(temporal.start_ms, 1000);
    assert_eq!(temporal.end_ms, 5000);
}

#[test]
fn test_legacy_backward_compatibility() {
    // A legacy JSON payload missing modality, spatial, and temporal fields
    let legacy_json = r#"{
        "id": "doc_123",
        "path": "/data/file.txt",
        "content": "Hello World",
        "metadata": {},
        "embedding": [0.1, 0.2, 0.3]
    }"#;

    let parsed: MemoryDocument = serde_json::from_str(legacy_json).unwrap();

    // The new fields should gracefully default
    assert_eq!(parsed.modality, ModalityType::Text);
    assert_eq!(parsed.spatial, None);
    assert_eq!(parsed.temporal, None);
}

#[test]
fn test_spatial_region_validation() {
    let valid_region = SpatialRegion {
        x: 10.0,
        y: 10.0,
        width: 50.0,
        height: 50.0,
    };
    assert!(valid_region.is_valid());

    let invalid_width = SpatialRegion {
        x: 10.0,
        y: 10.0,
        width: -5.0,
        height: 50.0,
    };
    assert!(!invalid_width.is_valid());

    let invalid_height = SpatialRegion {
        x: 10.0,
        y: 10.0,
        width: 50.0,
        height: -5.0,
    };
    assert!(!invalid_height.is_valid());
}

#[test]
fn test_temporal_range_validation() {
    let valid_range = TemporalRange {
        start_ms: 1000,
        end_ms: 5000,
    };
    assert!(valid_range.is_valid());

    let valid_point = TemporalRange {
        start_ms: 2000,
        end_ms: 2000,
    };
    assert!(valid_point.is_valid());

    let invalid_range = TemporalRange {
        start_ms: 5000,
        end_ms: 1000,
    };
    assert!(!invalid_range.is_valid());
}
