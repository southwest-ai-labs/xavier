xavier::isolate_test_process!();

use xavier::embedding::selector::{ContentModality, EmbeddingSelector};
use xavier::server::alerts::SYSTEM_ALERTS;

#[tokio::test]
async fn test_modality_selection() {
    // Tests mapping extension and logic
    assert_eq!(
        EmbeddingSelector::select_modality("some python code", Some("py")),
        ContentModality::Code
    );
    assert_eq!(
        EmbeddingSelector::select_modality("some rust code", Some("rs")),
        ContentModality::Code
    );
    assert_eq!(
        EmbeddingSelector::select_modality("function example() {}", Some("js")),
        ContentModality::Code
    );
    assert_eq!(
        EmbeddingSelector::select_modality("# Title", Some("md")),
        ContentModality::Text
    );

    // Test code heuristics
    assert_eq!(
        EmbeddingSelector::select_modality("fn main() {}", None),
        ContentModality::Code
    );
    assert_eq!(
        EmbeddingSelector::select_modality("def hello_world():", None),
        ContentModality::Code
    );
    assert_eq!(
        EmbeddingSelector::select_modality("plain english text", None),
        ContentModality::Text
    );
}

#[tokio::test]
async fn test_selector_fallback() {
    std::env::set_var(
        "_XAVIER_TEST_OLLAMA_PROBE_URL",
        "http://127.0.0.1:59999/offline",
    );
    // Create new embedding selector
    SYSTEM_ALERTS.clear();
    let selector = EmbeddingSelector::new().await.unwrap();

    let text_embedder = selector.get_embedder(ContentModality::Text).await;
    assert_eq!(text_embedder.dimension(), 768);

    // Default configuration uses offline Qwen for test, it should fallback to 768d text embedder
    let code_embedder = selector.get_embedder(ContentModality::Code).await;
    assert_eq!(code_embedder.dimension(), 768);

    let alerts = SYSTEM_ALERTS.get_alerts();
    let embedding_alerts: Vec<String> = alerts
        .into_iter()
        .filter(|a| a.component == "embedding")
        .map(|a| format!("[{}] {}", a.level, a.message))
        .collect();

    assert!(embedding_alerts
        .iter()
        .any(|msg| msg.contains("Qwen 1024d is offline, falling back to Text 768d")));
    std::env::remove_var("_XAVIER_TEST_OLLAMA_PROBE_URL");
}

#[tokio::test]
async fn test_dimension_mismatch_rejection() {
    use rusqlite::Connection;
    use xavier::memory::multimodal_vec_store::{DataTypeKind, MultimodalVecManager};

    let manager = MultimodalVecManager::new();
    let conn = Connection::open_in_memory().unwrap();

    manager
        .ensure_modality_table(&conn, DataTypeKind::Text, 768)
        .unwrap();
    manager
        .ensure_modality_table(&conn, DataTypeKind::Code, 1024)
        .unwrap();

    let text_vec_bad = vec![0.0f32; 1024]; // Wrong dim for Text
    let code_vec_bad = vec![0.0f32; 512]; // Unhandled bad dim for Code

    let res_text =
        manager.insert_modality_embedding(&conn, DataTypeKind::Text, "text1", &text_vec_bad, "{}");
    assert!(res_text.is_err());
    assert!(res_text
        .unwrap_err()
        .to_string()
        .contains("expected 768 dimensions"));

    let res_code =
        manager.insert_modality_embedding(&conn, DataTypeKind::Code, "code1", &code_vec_bad, "{}");
    assert!(res_code.is_err());
    assert!(res_code
        .unwrap_err()
        .to_string()
        .contains("expected 1024 dimensions"));
}

#[tokio::test]
async fn test_dimension_fallback_routing() {
    use rusqlite::Connection;
    use xavier::memory::multimodal_vec_store::{DataTypeKind, MultimodalVecManager};

    let manager = MultimodalVecManager::new();
    let conn = Connection::open_in_memory().unwrap();

    manager
        .ensure_modality_table(&conn, DataTypeKind::Text, 768)
        .unwrap();
    manager
        .ensure_modality_table(&conn, DataTypeKind::Code, 1024)
        .unwrap();

    let fallback_code_vec = vec![0.0f32; 768];

    // Simulating inserting a Code snippet but with the fallback 768d vector
    let res = manager.insert_modality_embedding(
        &conn,
        DataTypeKind::Code,
        "code_snippet_fallback",
        &fallback_code_vec,
        "{}",
    );
    assert!(
        res.is_ok(),
        "Fallback 768d vector for Code modality should be stored successfully"
    );

    // It should route to the 768d (Text) table
    let search_res = manager
        .search_modality(&conn, DataTypeKind::Text, &fallback_code_vec, 1)
        .unwrap();
    assert_eq!(search_res.len(), 1);
    assert_eq!(search_res[0].item_id, "code_snippet_fallback");
}

#[tokio::test]
async fn test_multimodal_modality_selection() {
    assert_eq!(
        EmbeddingSelector::select_modality("binary data", Some("png")),
        ContentModality::Image
    );
    assert_eq!(
        EmbeddingSelector::select_modality("binary data", Some("jpg")),
        ContentModality::Image
    );
    assert_eq!(
        EmbeddingSelector::select_modality("binary data", Some("webp")),
        ContentModality::Image
    );
    assert_eq!(
        EmbeddingSelector::select_modality("waveform data", Some("mp3")),
        ContentModality::Audio
    );
    assert_eq!(
        EmbeddingSelector::select_modality("waveform data", Some("wav")),
        ContentModality::Audio
    );
    assert_eq!(
        EmbeddingSelector::select_modality("video stream", Some("mp4")),
        ContentModality::Video
    );
    assert_eq!(
        EmbeddingSelector::select_modality("video stream", Some("webm")),
        ContentModality::Video
    );
}

#[tokio::test]
async fn test_multimodal_embedder_routing() {
    let selector = EmbeddingSelector::new().await.unwrap();

    let image_embedder = selector.get_embedder(ContentModality::Image).await;
    assert_eq!(image_embedder.dimension(), 768);

    let video_embedder = selector.get_embedder(ContentModality::Video).await;
    assert_eq!(video_embedder.dimension(), 768);

    let audio_embedder = selector.get_embedder(ContentModality::Audio).await;
    assert_eq!(audio_embedder.dimension(), 768);
}

#[tokio::test]
async fn test_multimodal_storage_tables() {
    use rusqlite::Connection;
    use xavier::memory::multimodal_vec_store::{DataTypeKind, MultimodalVecManager};

    let manager = MultimodalVecManager::new();
    let conn = Connection::open_in_memory().unwrap();

    manager
        .ensure_modality_table(&conn, DataTypeKind::Image, 768)
        .unwrap();
    manager
        .ensure_modality_table(&conn, DataTypeKind::Audio, 512)
        .unwrap();
    manager
        .ensure_modality_table(&conn, DataTypeKind::Video, 768)
        .unwrap();

    let image_vec = vec![0.036_084_39_f32; 768];
    let audio_vec = vec![0.044_194_17_f32; 512];

    manager
        .insert_modality_embedding(
            &conn,
            DataTypeKind::Image,
            "img_diagram_1",
            &image_vec,
            "{}",
        )
        .unwrap();
    manager
        .insert_modality_embedding(&conn, DataTypeKind::Audio, "audio_clip_1", &audio_vec, "{}")
        .unwrap();

    let image_results = manager
        .search_modality(&conn, DataTypeKind::Image, &image_vec, 1)
        .unwrap();
    assert_eq!(image_results.len(), 1);
    assert_eq!(image_results[0].item_id, "img_diagram_1");

    let audio_results = manager
        .search_modality(&conn, DataTypeKind::Audio, &audio_vec, 1)
        .unwrap();
    assert_eq!(audio_results.len(), 1);
    assert_eq!(audio_results[0].item_id, "audio_clip_1");
}

#[tokio::test]
async fn test_multimodal_spatiotemporal_filtering() {
    use rusqlite::Connection;
    use xavier::memory::multimodal_vec_store::{
        DataTypeKind, MultimodalVecManager, SpatialRegion, SpatiotemporalFilter,
        SpatiotemporalMetadata, TemporalRange,
    };

    let manager = MultimodalVecManager::new();
    let conn = Connection::open_in_memory().unwrap();

    manager
        .ensure_modality_table(&conn, DataTypeKind::Video, 768)
        .unwrap();

    let vec_clip = vec![0.036_084_39_f32; 768];

    // Clip 1: Scene at 10s - 25s (10,000ms - 25,000ms) with a face bounding box
    manager
        .insert_spatiotemporal_embedding(
            &conn,
            DataTypeKind::Video,
            "scene_interview",
            &vec_clip,
            SpatiotemporalMetadata::new(
                Some(SpatialRegion {
                    x: 100.0,
                    y: 100.0,
                    width: 200.0,
                    height: 200.0,
                }),
                Some(TemporalRange {
                    start_ms: 10_000,
                    end_ms: 25_000,
                }),
            ),
        )
        .unwrap();

    // Clip 2: Scene at 40s - 60s (40,000ms - 60,000ms) with bottom subtitle region
    manager
        .insert_spatiotemporal_embedding(
            &conn,
            DataTypeKind::Video,
            "scene_credits",
            &vec_clip,
            SpatiotemporalMetadata::new(
                Some(SpatialRegion {
                    x: 0.0,
                    y: 800.0,
                    width: 1920.0,
                    height: 200.0,
                }),
                Some(TemporalRange {
                    start_ms: 40_000,
                    end_ms: 60_000,
                }),
            ),
        )
        .unwrap();

    // Query 1: Filter temporal range around 15s - 20s (matches scene_interview)
    let temp_filter = SpatiotemporalFilter::temporal(TemporalRange {
        start_ms: 15_000,
        end_ms: 20_000,
    });
    let temp_results = manager
        .search_spatiotemporal(&conn, DataTypeKind::Video, &vec_clip, &temp_filter, 5)
        .unwrap();
    assert_eq!(temp_results.len(), 1);
    assert_eq!(temp_results[0].item_id, "scene_interview");

    // Query 2: Filter spatial region matching subtitle area
    let mut spatial_filter = SpatiotemporalFilter::spatial(SpatialRegion {
        x: 0.0,
        y: 800.0,
        width: 1920.0,
        height: 200.0,
    });
    spatial_filter.min_iou = Some(0.9);
    let spatial_results = manager
        .search_spatiotemporal(&conn, DataTypeKind::Video, &vec_clip, &spatial_filter, 5)
        .unwrap();
    assert_eq!(spatial_results.len(), 1);
    assert_eq!(spatial_results[0].item_id, "scene_credits");
}

#[tokio::test]
async fn test_multimodal_video_segmentation_retrieval() {
    use rusqlite::Connection;
    use xavier::memory::multimodal_vec_store::{
        DataTypeKind, MultimodalVecManager, SpatiotemporalFilter, SpatiotemporalMetadata,
        TemporalRange,
    };
    use xavier::memory::video_segmenter::{VideoSceneSegmenter, VideoSegmenterConfig};

    let segmenter = VideoSceneSegmenter::new(VideoSegmenterConfig {
        target_duration_ms: 5000,
        min_scene_duration_ms: 1000,
        max_scene_duration_ms: 10000,
        frames_per_scene: 3,
    });

    let manager = MultimodalVecManager::new();
    let conn = Connection::open_in_memory().unwrap();
    manager
        .ensure_modality_table(&conn, DataTypeKind::Video, 768)
        .unwrap();

    let chunks = segmenter.segment_uniform(15000);
    assert_eq!(chunks.len(), 3);

    // Insert embeddings for all 3 chunks
    let dummy_vec = vec![0.036_084_39_f32; 768];
    for chunk in &chunks {
        let item_id = format!("scene_{}", chunk.scene_index);
        let metadata = SpatiotemporalMetadata::new(
            None,
            Some(TemporalRange {
                start_ms: chunk.start_ms,
                end_ms: chunk.end_ms,
            }),
        );
        manager
            .insert_spatiotemporal_embedding(
                &conn,
                DataTypeKind::Video,
                &item_id,
                &dummy_vec,
                metadata,
            )
            .unwrap();
    }

    // Query for a moment at 7500 ms (belongs strictly to chunk 1: [5000, 10000])
    let query_filter = SpatiotemporalFilter::temporal(TemporalRange {
        start_ms: 7000,
        end_ms: 8000,
    });
    let results = manager
        .search_spatiotemporal(&conn, DataTypeKind::Video, &dummy_vec, &query_filter, 5)
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].item_id, "scene_1");
}
