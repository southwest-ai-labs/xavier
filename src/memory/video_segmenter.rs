//! Video Scene Segmentation & Uniform Frame Keyframing
//!
//! Provides `VideoSceneSegmenter` for chunking continuous video timelines
//! into discrete semantic scenes or uniform millisecond windows, generating
//! keyframe timestamps for multi-frame feature pooling (e.g. SigLIP / CLIP).

use serde::{Deserialize, Serialize};

/// Represents a discrete scene or temporal window in a video.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoSceneChunk {
    /// 0-indexed scene number
    pub scene_index: usize,
    /// Start timestamp of scene in milliseconds
    pub start_ms: u64,
    /// End timestamp of scene in milliseconds
    pub end_ms: u64,
    /// Sampled keyframe timestamps (in ms) within [start_ms, end_ms]
    pub keyframe_timestamps_ms: Vec<u64>,
    /// Optional semantic or chapter label
    pub label: Option<String>,
}

impl VideoSceneChunk {
    /// Returns the duration of the scene in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.end_ms.saturating_sub(self.start_ms)
    }

    /// Checks if a given timestamp falls within this scene window.
    pub fn contains_timestamp(&self, timestamp_ms: u64) -> bool {
        timestamp_ms >= self.start_ms && timestamp_ms <= self.end_ms
    }
}

/// Configuration parameters for video segmentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSegmenterConfig {
    /// Target duration in milliseconds for uniform chunks (default: 5000 ms = 5s)
    pub target_duration_ms: u64,
    /// Minimum allowed scene duration to prevent micro-cuts (default: 1000 ms = 1s)
    pub min_scene_duration_ms: u64,
    /// Maximum allowed scene duration before forcing a cut (default: 15000 ms = 15s)
    pub max_scene_duration_ms: u64,
    /// Number of keyframe samples to extract per scene (default: 3)
    pub frames_per_scene: usize,
}

impl Default for VideoSegmenterConfig {
    fn default() -> Self {
        Self {
            target_duration_ms: 5000,
            min_scene_duration_ms: 1000,
            max_scene_duration_ms: 15000,
            frames_per_scene: 3,
        }
    }
}

/// Engine for segmenting video timelines into chunks and sampling keyframe timestamps.
#[derive(Debug, Clone)]
pub struct VideoSceneSegmenter {
    config: VideoSegmenterConfig,
}

impl Default for VideoSceneSegmenter {
    fn default() -> Self {
        Self::new(VideoSegmenterConfig::default())
    }
}

impl VideoSceneSegmenter {
    /// Create a new segmenter with custom configuration.
    pub fn new(config: VideoSegmenterConfig) -> Self {
        Self { config }
    }

    /// Access segmenter configuration.
    pub fn config(&self) -> &VideoSegmenterConfig {
        &self.config
    }

    /// Compute keyframe sample timestamps evenly spaced across `[start_ms, end_ms]`.
    pub fn sample_keyframes(&self, start_ms: u64, end_ms: u64) -> Vec<u64> {
        let n = self.config.frames_per_scene;
        if n == 0 || end_ms <= start_ms {
            return vec![start_ms];
        }

        if n == 1 {
            return vec![start_ms + (end_ms - start_ms) / 2];
        }

        let step = (end_ms - start_ms) as f64 / (n + 1) as f64;
        (1..=n)
            .map(|i| start_ms + (step * i as f64).round() as u64)
            .collect()
    }

    /// Segment a video of `total_duration_ms` into uniform scene chunks.
    pub fn segment_uniform(&self, total_duration_ms: u64) -> Vec<VideoSceneChunk> {
        if total_duration_ms == 0 {
            return Vec::new();
        }

        let target = self.config.target_duration_ms.max(1);
        let mut chunks = Vec::new();
        let mut current_start = 0u64;
        let mut scene_idx = 0;

        while current_start < total_duration_ms {
            let potential_end = current_start + target;
            let end_ms = if potential_end >= total_duration_ms {
                total_duration_ms
            } else if total_duration_ms - potential_end < self.config.min_scene_duration_ms {
                // If remaining tail is smaller than min duration, merge it into current chunk
                total_duration_ms
            } else {
                potential_end
            };

            let keyframes = self.sample_keyframes(current_start, end_ms);

            chunks.push(VideoSceneChunk {
                scene_index: scene_idx,
                start_ms: current_start,
                end_ms,
                keyframe_timestamps_ms: keyframes,
                label: Some(format!("Scene {}", scene_idx + 1)),
            });

            scene_idx += 1;
            current_start = end_ms;
        }

        chunks
    }

    /// Segment a video by detected visual shot/scene boundary timestamps.
    /// Boundaries shorter than `min_scene_duration_ms` are merged.
    /// Boundaries longer than `max_scene_duration_ms` are subdivided.
    pub fn segment_by_boundaries(
        &self,
        boundaries_ms: &[u64],
        total_duration_ms: u64,
    ) -> Vec<VideoSceneChunk> {
        if total_duration_ms == 0 {
            return Vec::new();
        }

        let mut sorted_boundaries: Vec<u64> = boundaries_ms
            .iter()
            .copied()
            .filter(|&t| t > 0 && t < total_duration_ms)
            .collect();
        sorted_boundaries.sort_unstable();
        sorted_boundaries.dedup();

        let mut raw_ranges = Vec::new();
        let mut prev = 0u64;

        for b in sorted_boundaries {
            if b.saturating_sub(prev) >= self.config.min_scene_duration_ms {
                raw_ranges.push((prev, b));
                prev = b;
            }
        }
        if prev < total_duration_ms {
            raw_ranges.push((prev, total_duration_ms));
        }

        // Subdivide ranges that exceed max_scene_duration_ms
        let mut final_chunks = Vec::new();
        let mut scene_idx = 0;

        for (start, end) in raw_ranges {
            let duration = end.saturating_sub(start);
            if duration > self.config.max_scene_duration_ms {
                let sub_segmenter = VideoSceneSegmenter::new(VideoSegmenterConfig {
                    target_duration_ms: self.config.target_duration_ms,
                    min_scene_duration_ms: self.config.min_scene_duration_ms,
                    max_scene_duration_ms: self.config.max_scene_duration_ms,
                    frames_per_scene: self.config.frames_per_scene,
                });
                let sub_chunks = sub_segmenter.segment_uniform(duration);
                for sub in sub_chunks {
                    let chunk_start = start + sub.start_ms;
                    let chunk_end = start + sub.end_ms;
                    let keyframes = self.sample_keyframes(chunk_start, chunk_end);
                    final_chunks.push(VideoSceneChunk {
                        scene_index: scene_idx,
                        start_ms: chunk_start,
                        end_ms: chunk_end,
                        keyframe_timestamps_ms: keyframes,
                        label: Some(format!("Scene {}", scene_idx + 1)),
                    });
                    scene_idx += 1;
                }
            } else {
                let keyframes = self.sample_keyframes(start, end);
                final_chunks.push(VideoSceneChunk {
                    scene_index: scene_idx,
                    start_ms: start,
                    end_ms: end,
                    keyframe_timestamps_ms: keyframes,
                    label: Some(format!("Scene {}", scene_idx + 1)),
                });
                scene_idx += 1;
            }
        }

        final_chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_duration_returns_empty() {
        let segmenter = VideoSceneSegmenter::default();
        let chunks = segmenter.segment_uniform(0);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_short_video_single_chunk() {
        let segmenter = VideoSceneSegmenter::default();
        let chunks = segmenter.segment_uniform(3000);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start_ms, 0);
        assert_eq!(chunks[0].end_ms, 3000);
        assert_eq!(chunks[0].keyframe_timestamps_ms.len(), 3);
    }

    #[test]
    fn test_uniform_segmentation_standard() {
        let segmenter = VideoSceneSegmenter::new(VideoSegmenterConfig {
            target_duration_ms: 5000,
            min_scene_duration_ms: 1000,
            max_scene_duration_ms: 10000,
            frames_per_scene: 3,
        });

        let chunks = segmenter.segment_uniform(15000);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].start_ms, 0);
        assert_eq!(chunks[0].end_ms, 5000);
        assert_eq!(chunks[1].start_ms, 5000);
        assert_eq!(chunks[1].end_ms, 10000);
        assert_eq!(chunks[2].start_ms, 10000);
        assert_eq!(chunks[2].end_ms, 15000);
    }

    #[test]
    fn test_uniform_tail_merging() {
        let segmenter = VideoSceneSegmenter::new(VideoSegmenterConfig {
            target_duration_ms: 5000,
            min_scene_duration_ms: 2000,
            max_scene_duration_ms: 10000,
            frames_per_scene: 3,
        });

        // 5500 ms: first chunk would end at 5000, leaving 500 ms (< min 2000ms)
        // It must merge the tail so there is 1 chunk of 5500 ms.
        let chunks = segmenter.segment_uniform(5500);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].end_ms, 5500);
    }

    #[test]
    fn test_sample_keyframes_distribution() {
        let segmenter = VideoSceneSegmenter::new(VideoSegmenterConfig {
            target_duration_ms: 5000,
            min_scene_duration_ms: 1000,
            max_scene_duration_ms: 10000,
            frames_per_scene: 3,
        });

        let kf = segmenter.sample_keyframes(1000, 5000);
        assert_eq!(kf.len(), 3);
        assert!(kf[0] > 1000);
        assert!(kf[1] > kf[0]);
        assert!(kf[2] > kf[1]);
        assert!(kf[2] < 5000);
    }

    #[test]
    fn test_boundary_segmentation_with_subdivision() {
        let segmenter = VideoSceneSegmenter::new(VideoSegmenterConfig {
            target_duration_ms: 5000,
            min_scene_duration_ms: 1000,
            max_scene_duration_ms: 8000,
            frames_per_scene: 2,
        });

        // Scene 1: 0..3000 (3s), Scene 2: 3000..15000 (12s > max 8s -> subdivides)
        let boundaries = vec![3000];
        let chunks = segmenter.segment_by_boundaries(&boundaries, 15000);
        assert!(chunks.len() >= 3);
        assert_eq!(chunks[0].start_ms, 0);
        assert_eq!(chunks[0].end_ms, 3000);
        assert_eq!(chunks[chunks.len() - 1].end_ms, 15000);
    }
}
