//! Multimodal Virtual Vector Manager for SQLite
//!
//! Provides isolated virtual vector tables per data modality (e.g., Legal Text 1536d,
//! CLIP Image 768d, Code 384d, Video 768d, Audio 512d) to prevent dimension mismatch
//! crashes in sqlite-vec.

use std::collections::HashMap;
use std::sync::RwLock;

use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::memory::sqlite_vec_store::vector;
pub use xavier_core_logic::types::{SpatialRegion, TemporalRange};

#[path = "video_segmenter.rs"]
pub mod video_segmenter;
pub use video_segmenter::{VideoSceneChunk, VideoSceneSegmenter, VideoSegmenterConfig};

/// Modality types for isolated vector tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataTypeKind {
    Text,
    Legal,
    Code,
    Image,
    Video,
    Audio,
}

impl DataTypeKind {
    /// Returns the isolated SQLite table name for this modality.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "vec_text",
            Self::Legal => "vec_legal",
            Self::Code => "vec_code",
            Self::Image => "vec_image",
            Self::Video => "vec_video",
            Self::Audio => "vec_audio",
        }
    }

    /// Default dimension for the given data type kind.
    pub fn default_dim(&self) -> usize {
        match self {
            Self::Text => 768,
            Self::Legal => 1536,
            Self::Code => 384,
            Self::Image => 768,
            Self::Video => 768,
            Self::Audio => 512,
        }
    }
}

/// Vector search result item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorSearchResult {
    pub item_id: String,
    pub distance: f32,
    pub payload_json: String,
}

/// Query filter for spatiotemporal attributes.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SpatiotemporalFilter {
    pub spatial: Option<SpatialRegion>,
    pub temporal: Option<TemporalRange>,
    /// Minimum Intersection-over-Union (IoU) overlap when spatial filter is set.
    /// Defaults to 0.0 (any overlap or containment).
    pub min_iou: Option<f32>,
}

/// Optional spatiotemporal metadata attributes for multimodal embeddings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SpatiotemporalMetadata {
    pub spatial: Option<SpatialRegion>,
    pub temporal: Option<TemporalRange>,
    pub extra: Option<serde_json::Value>,
}

impl SpatiotemporalMetadata {
    /// Creates metadata with a spatial region.
    pub fn spatial(region: SpatialRegion) -> Self {
        Self {
            spatial: Some(region),
            temporal: None,
            extra: None,
        }
    }

    /// Creates metadata with a temporal range.
    pub fn temporal(range: TemporalRange) -> Self {
        Self {
            spatial: None,
            temporal: Some(range),
            extra: None,
        }
    }

    /// Creates metadata with both spatial and temporal attributes.
    pub fn new(spatial: Option<SpatialRegion>, temporal: Option<TemporalRange>) -> Self {
        Self {
            spatial,
            temporal,
            extra: None,
        }
    }
}

impl SpatiotemporalFilter {
    /// Creates a filter with spatial region only.
    pub fn spatial(region: SpatialRegion) -> Self {
        Self {
            spatial: Some(region),
            temporal: None,
            min_iou: None,
        }
    }

    /// Creates a filter with temporal range only.
    pub fn temporal(range: TemporalRange) -> Self {
        Self {
            spatial: None,
            temporal: Some(range),
            min_iou: None,
        }
    }

    /// Calculates IoU between two bounding boxes.
    pub fn calculate_iou(a: &SpatialRegion, b: &SpatialRegion) -> f32 {
        let x1 = a.x.max(b.x);
        let y1 = a.y.max(b.y);
        let x2 = (a.x + a.width).min(b.x + b.width);
        let y2 = (a.y + a.height).min(b.y + b.height);

        let intersection_width = (x2 - x1).max(0.0);
        let intersection_height = (y2 - y1).max(0.0);
        let intersection_area = intersection_width * intersection_height;

        let area_a = a.width * a.height;
        let area_b = b.width * b.height;
        let union_area = area_a + area_b - intersection_area;

        if union_area <= 0.0 {
            0.0
        } else {
            intersection_area / union_area
        }
    }

    /// Checks if a payload satisfies this filter.
    pub fn matches(&self, payload_val: &serde_json::Value) -> bool {
        if let Some(target_spatial) = &self.spatial {
            let spatial_obj = payload_val.get("spatial");
            let Some(spatial_val) = spatial_obj else {
                return false;
            };
            if let Ok(item_spatial) = serde_json::from_value::<SpatialRegion>(spatial_val.clone()) {
                let iou = Self::calculate_iou(target_spatial, &item_spatial);
                let min_threshold = self.min_iou.unwrap_or(0.0);
                if iou < min_threshold {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(target_temporal) = &self.temporal {
            let temporal_obj = payload_val.get("temporal");
            let Some(temporal_val) = temporal_obj else {
                return false;
            };
            if let Ok(item_temporal) = serde_json::from_value::<TemporalRange>(temporal_val.clone())
            {
                // Overlap check: start_a <= end_b && start_b <= end_a
                if target_temporal.start_ms > item_temporal.end_ms
                    || item_temporal.start_ms > target_temporal.end_ms
                {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

/// Manages multi-modal virtual vector tables isolated per modality dimension.
#[derive(Debug, Default)]
pub struct MultimodalVecManager {
    dimensions: RwLock<HashMap<DataTypeKind, usize>>,
}

impl MultimodalVecManager {
    /// Creates a new `MultimodalVecManager` instance.
    pub fn new() -> Self {
        Self {
            dimensions: RwLock::new(HashMap::new()),
        }
    }

    /// Registers the sqlite-vec extension if available.
    pub fn register_extension(&self) -> Result<()> {
        vector::register_sqlite_vec_extension()
    }

    /// Ensures that the virtual vector table for a specific modality exists with the specified dimension.
    pub fn ensure_modality_table(
        &self,
        conn: &Connection,
        modality: DataTypeKind,
        dim: usize,
    ) -> Result<()> {
        let table_name = modality.as_str();

        // Register sqlite-vec auto extension if possible
        let _ = self.register_extension();

        // 1. Create modality metadata tracking table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS multimodal_modality_meta (
                modality TEXT PRIMARY KEY,
                dim INTEGER NOT NULL
            );",
            [],
        )?;

        conn.execute(
            "INSERT OR REPLACE INTO multimodal_modality_meta (modality, dim) VALUES (?1, ?2);",
            params![table_name, dim as i64],
        )?;

        // Update internal dimension cache
        if let Ok(mut dims) = self.dimensions.write() {
            dims.insert(modality, dim);
        }

        // 2. Try creating sqlite-vec virtual table using vec0
        let vec0_sql = format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS {table_name} USING vec0(item_id text primary key, vector float[{dim}], payload_json text);"
        );

        if conn.execute(&vec0_sql, []).is_err() {
            // Fallback: create standard table for pure-Rust cosine distance matching
            let fallback_sql = format!(
                "CREATE TABLE IF NOT EXISTS {table_name} (
                    item_id TEXT PRIMARY KEY,
                    vector BLOB NOT NULL,
                    payload_json TEXT NOT NULL,
                    dim INTEGER NOT NULL
                );"
            );
            conn.execute(&fallback_sql, [])?;
        }

        Ok(())
    }

    /// Retrieves the registered dimension for a modality.
    pub fn get_modality_dim(&self, conn: &Connection, modality: DataTypeKind) -> Result<usize> {
        if let Ok(dims) = self.dimensions.read() {
            if let Some(&dim) = dims.get(&modality) {
                return Ok(dim);
            }
        }

        let table_name = modality.as_str();
        let dim_option: Option<i64> = conn
            .query_row(
                "SELECT dim FROM multimodal_modality_meta WHERE modality = ?1",
                params![table_name],
                |row| row.get(0),
            )
            .ok();

        if let Some(dim) = dim_option {
            let dim_usize = dim as usize;
            if let Ok(mut dims) = self.dimensions.write() {
                dims.insert(modality, dim_usize);
            }
            Ok(dim_usize)
        } else {
            Ok(modality.default_dim())
        }
    }

    /// Inserts or replaces a vector embedding for a given modality.
    pub fn insert_modality_embedding(
        &self,
        conn: &Connection,
        modality: DataTypeKind,
        item_id: &str,
        vector: &[f32],
        payload_json: &str,
    ) -> Result<()> {
        let expected_dim = self.get_modality_dim(conn, modality)?;

        // Fallback dimensional routing: if inserting Code with 768d (due to fallback to Text embedder), route to vec_text
        let target_modality =
            if modality == DataTypeKind::Code && vector.len() == 768 && expected_dim == 1024 {
                DataTypeKind::Text
            } else {
                modality
            };

        let target_expected_dim = self.get_modality_dim(conn, target_modality)?;
        let table_name = target_modality.as_str();

        if vector.len() != target_expected_dim {
            return Err(anyhow!(
                "Dimension mismatch for modality '{}': expected {} dimensions, got {}",
                table_name,
                target_expected_dim,
                vector.len()
            ));
        }

        // Try insert into virtual table vec0 (using DELETE + INSERT since vec0 virtual tables do not support INSERT OR REPLACE)
        let vec_json = serde_json::to_string(vector)?;
        let delete_virtual_sql = format!("DELETE FROM {table_name} WHERE item_id = ?1;");
        let insert_virtual_sql = format!(
            "INSERT INTO {table_name} (item_id, vector, payload_json) VALUES (?1, vec_f32(?2), ?3);"
        );

        let _ = conn.execute(&delete_virtual_sql, params![item_id]);
        if conn
            .execute(
                &insert_virtual_sql,
                params![item_id, vec_json, payload_json],
            )
            .is_err()
        {
            // Fallback insert for standard table
            let blob_data = super::sqlite_vec_store::vector::serialize_embedding(vector);
            let insert_fallback_sql = format!(
                "INSERT OR REPLACE INTO {table_name} (item_id, vector, payload_json, dim) VALUES (?1, ?2, ?3, ?4);"
            );
            conn.execute(
                &insert_fallback_sql,
                params![item_id, blob_data, payload_json, expected_dim as i64],
            )?;
        }

        Ok(())
    }

    /// Searches for top_k nearest vector neighbors within a specific modality isolated table.
    pub fn search_modality(
        &self,
        conn: &Connection,
        modality: DataTypeKind,
        query_vector: &[f32],
        top_k: usize,
    ) -> Result<Vec<VectorSearchResult>> {
        let table_name = modality.as_str();
        let expected_dim = self.get_modality_dim(conn, modality)?;

        if query_vector.len() != expected_dim {
            return Err(anyhow!(
                "Query dimension mismatch for modality '{}': expected {} dimensions, got {}",
                table_name,
                expected_dim,
                query_vector.len()
            ));
        }

        let query_json = serde_json::to_string(query_vector)?;
        let search_virtual_sql = format!(
            "SELECT item_id, distance, payload_json FROM {table_name} WHERE vector MATCH vec_f32(?1) AND k = ?2 ORDER BY distance;"
        );

        let virtual_res: Result<Vec<VectorSearchResult>> = (|| {
            let mut stmt = conn.prepare(&search_virtual_sql)?;
            let rows = stmt.query_map(params![query_json, top_k as i64], |row| {
                Ok(VectorSearchResult {
                    item_id: row.get(0)?,
                    distance: row.get(1)?,
                    payload_json: row.get(2)?,
                })
            })?;
            let mut results = Vec::new();
            for r in rows {
                results.push(r?);
            }
            Ok(results)
        })();

        if let Ok(results) = virtual_res {
            return Ok(results);
        }

        // Fallback: Pure Rust Cosine Distance search over standard table
        let fallback_sql = format!("SELECT item_id, vector, payload_json FROM {table_name};");
        let mut stmt = conn.prepare(&fallback_sql)?;
        let mut rows = stmt.query([])?;
        let mut scored_items = Vec::new();

        while let Some(row) = rows.next()? {
            let item_id: String = row.get(0)?;
            let blob_data: Vec<u8> = row.get(1)?;
            let payload_json: String = row.get(2)?;

            let vec = super::sqlite_vec_store::vector::deserialize_embedding(&blob_data);
            let distance = compute_cosine_distance(query_vector, &vec);

            scored_items.push(VectorSearchResult {
                item_id,
                distance,
                payload_json,
            });
        }

        scored_items.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored_items.truncate(top_k);

        Ok(scored_items)
    }

    /// Inserts an embedding along with spatial bounding box and/or temporal range into payload JSON.
    pub fn insert_spatiotemporal_embedding(
        &self,
        conn: &Connection,
        modality: DataTypeKind,
        item_id: &str,
        vector: &[f32],
        meta: SpatiotemporalMetadata,
    ) -> Result<()> {
        let mut map = serde_json::Map::new();
        if let Some(sp) = meta.spatial {
            map.insert("spatial".to_string(), serde_json::to_value(sp)?);
        }
        if let Some(tp) = meta.temporal {
            map.insert("temporal".to_string(), serde_json::to_value(tp)?);
        }
        if let Some(serde_json::Value::Object(extra)) = meta.extra {
            for (k, v) in extra {
                map.insert(k, v);
            }
        }
        let payload_json = serde_json::to_string(&serde_json::Value::Object(map))?;
        self.insert_modality_embedding(conn, modality, item_id, vector, &payload_json)
    }

    /// Searches for nearest vector neighbors that satisfy a spatiotemporal filter.
    pub fn search_spatiotemporal(
        &self,
        conn: &Connection,
        modality: DataTypeKind,
        query_vector: &[f32],
        filter: &SpatiotemporalFilter,
        top_k: usize,
    ) -> Result<Vec<VectorSearchResult>> {
        // Query an overfetched candidate pool to allow post-filtering
        let candidate_k = (top_k * 5).max(50);
        let candidates = self.search_modality(conn, modality, query_vector, candidate_k)?;

        let mut filtered = Vec::new();
        for item in candidates {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&item.payload_json) {
                if filter.matches(&val) {
                    filtered.push(item);
                    if filtered.len() == top_k {
                        break;
                    }
                }
            }
        }

        Ok(filtered)
    }
}

/// Computes Cosine Distance (1.0 - cosine_similarity) between two vectors.
pub fn compute_cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 1.0;
    }
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    if norm_a <= 0.0 || norm_b <= 0.0 {
        return 1.0;
    }

    let similarity = dot / (norm_a.sqrt() * norm_b.sqrt());
    (1.0 - similarity).clamp(0.0, 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iou_calculation_identical() {
        let r1 = SpatialRegion {
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 20.0,
        };
        let r2 = SpatialRegion {
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 20.0,
        };
        let iou = SpatiotemporalFilter::calculate_iou(&r1, &r2);
        assert!((iou - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_iou_calculation_disjoint() {
        let r1 = SpatialRegion {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        };
        let r2 = SpatialRegion {
            x: 20.0,
            y: 20.0,
            width: 10.0,
            height: 10.0,
        };
        let iou = SpatiotemporalFilter::calculate_iou(&r1, &r2);
        assert_eq!(iou, 0.0);
    }

    #[test]
    fn test_iou_calculation_partial_overlap() {
        // Box A: 0..10 x 0..10, Area = 100
        // Box B: 5..15 x 0..10, Area = 100
        // Intersection: 5..10 x 0..10 = 5 * 10 = 50
        // Union: 100 + 100 - 50 = 150
        // IoU: 50 / 150 = 1/3 ~ 0.33333
        let r1 = SpatialRegion {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        };
        let r2 = SpatialRegion {
            x: 5.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        };
        let iou = SpatiotemporalFilter::calculate_iou(&r1, &r2);
        assert!((iou - (1.0 / 3.0)).abs() < 1e-4);
    }

    #[test]
    fn test_spatiotemporal_filter_spatial_matching() {
        let target_box = SpatialRegion {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let mut filter = SpatiotemporalFilter::spatial(target_box);
        filter.min_iou = Some(0.2);

        // Matching box
        let item_payload = serde_json::json!({
            "spatial": { "x": 10.0, "y": 10.0, "width": 80.0, "height": 80.0 }
        });
        assert!(filter.matches(&item_payload));

        // Outside box
        let non_matching_payload = serde_json::json!({
            "spatial": { "x": 200.0, "y": 200.0, "width": 50.0, "height": 50.0 }
        });
        assert!(!filter.matches(&non_matching_payload));

        // Missing spatial field
        let empty_payload = serde_json::json!({});
        assert!(!filter.matches(&empty_payload));
    }

    #[test]
    fn test_spatiotemporal_filter_temporal_matching() {
        let filter = SpatiotemporalFilter::temporal(TemporalRange {
            start_ms: 1000,
            end_ms: 5000,
        });

        // Solapamiento total o parcial
        let overlapping_1 = serde_json::json!({ "temporal": { "start_ms": 2000, "end_ms": 3000 } });
        let overlapping_2 = serde_json::json!({ "temporal": { "start_ms": 500, "end_ms": 1500 } });
        let overlapping_3 = serde_json::json!({ "temporal": { "start_ms": 4500, "end_ms": 6000 } });
        assert!(filter.matches(&overlapping_1));
        assert!(filter.matches(&overlapping_2));
        assert!(filter.matches(&overlapping_3));

        // Sin solapamiento
        let disjoint_before = serde_json::json!({ "temporal": { "start_ms": 0, "end_ms": 999 } });
        let disjoint_after =
            serde_json::json!({ "temporal": { "start_ms": 5001, "end_ms": 8000 } });
        assert!(!filter.matches(&disjoint_before));
        assert!(!filter.matches(&disjoint_after));
    }

    #[test]
    fn test_insert_and_search_spatiotemporal() {
        let manager = MultimodalVecManager::new();
        let conn = Connection::open_in_memory().unwrap();

        manager
            .ensure_modality_table(&conn, DataTypeKind::Image, 1152)
            .unwrap();

        // 3 image embeddings: query vector is all 1.0s normalized
        let mut v1 = vec![0.0f32; 1152];
        v1[0] = 1.0;
        let mut v2 = vec![0.0f32; 1152];
        v2[1] = 1.0;

        let region_target = SpatialRegion {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let region_other = SpatialRegion {
            x: 500.0,
            y: 500.0,
            width: 50.0,
            height: 50.0,
        };

        manager
            .insert_spatiotemporal_embedding(
                &conn,
                DataTypeKind::Image,
                "img_target",
                &v1,
                SpatiotemporalMetadata::spatial(region_target.clone()),
            )
            .unwrap();

        manager
            .insert_spatiotemporal_embedding(
                &conn,
                DataTypeKind::Image,
                "img_other",
                &v1,
                SpatiotemporalMetadata::spatial(region_other),
            )
            .unwrap();

        // Búsqueda sin filtro retorna ambos
        let all_res = manager
            .search_modality(&conn, DataTypeKind::Image, &v1, 10)
            .unwrap();
        assert_eq!(all_res.len(), 2);

        // Búsqueda con filtro espacial IoU >= 0.5
        let mut filter = SpatiotemporalFilter::spatial(region_target);
        filter.min_iou = Some(0.5);

        let filtered_res = manager
            .search_spatiotemporal(&conn, DataTypeKind::Image, &v1, &filter, 10)
            .unwrap();
        assert_eq!(filtered_res.len(), 1);
        assert_eq!(filtered_res[0].item_id, "img_target");
    }
}
