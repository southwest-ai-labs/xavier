//! Multimodal vector primitives for SigLIP/CLAP RAG scoring.
//!
//! Pure, I/O-free, WASM-compatible functions for cross-modal similarity,
//! temporal frame pooling, and dimension validation.
//!
//! Dimension conventions:
//! - `ModalityType::Image`      → 768-d (SigLIP / CLIP ViT-B/16)
//! - `ModalityType::VideoScene` → 768-d (mean-pooled keyframes)
//! - `ModalityType::Audio`      → 512-d (CLAP)
//! - `ModalityType::Text`       → any dimension (flexible)

use crate::types::ModalityType;

/// Expected embedding dimension for each concrete modality.
/// Returns `None` for `Text`, which has no fixed dimension.
pub fn expected_dim(modality: ModalityType) -> Option<usize> {
    match modality {
        ModalityType::Image => Some(768),
        ModalityType::VideoScene => Some(768),
        ModalityType::Audio => Some(512),
        ModalityType::Text => None,
    }
}

/// Dimension validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimError {
    pub modality: ModalityType,
    pub expected: usize,
    pub got: usize,
}

impl std::fmt::Display for DimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "dimension mismatch for {:?}: expected {}, got {}",
            self.modality, self.expected, self.got
        )
    }
}

/// Validate that a vector has the canonical dimension for its modality.
///
/// Returns `Ok(())` for `ModalityType::Text` (no fixed dimension).
pub fn validate_modality_dim(v: &[f32], modality: ModalityType) -> Result<(), DimError> {
    if let Some(expected) = expected_dim(modality) {
        if v.len() != expected {
            return Err(DimError {
                modality,
                expected,
                got: v.len(),
            });
        }
    }
    Ok(())
}

/// L2-normalise a vector in place, returning a new `Vec<f32>`.
///
/// If the input is the zero vector, returns the zero vector unchanged.
#[inline]
pub fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}

/// Cosine similarity between two vectors.
///
/// Both vectors are L2-normalised before computing the dot product, so the
/// result is in `[-1.0, 1.0]`.  Returns `0.0` if either vector is zero.
#[inline]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "cosine_similarity: dimension mismatch");
    let na = l2_normalize(a);
    let nb = l2_normalize(b);
    na.iter().zip(nb.iter()).map(|(x, y)| x * y).sum()
}

/// Euclidean distance between two equal-length vectors.
#[inline]
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "euclidean_distance: dimension mismatch");
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

/// Aggregate a sequence of per-frame embeddings (e.g. SigLIP keyframes from a
/// video clip) into a single L2-normalised pooled vector.
///
/// Algorithm: element-wise mean followed by L2 normalisation.
/// Returns `None` if `frames` is empty or all frames are empty.
pub fn pool_frames_l2(frames: &[Vec<f32>]) -> Option<Vec<f32>> {
    let dim = frames.first()?.len();
    if dim == 0 {
        return None;
    }
    let mut sum = vec![0.0_f32; dim];
    let mut count = 0usize;
    for frame in frames {
        if frame.len() != dim {
            continue; // skip malformed frames rather than panic
        }
        for (s, v) in sum.iter_mut().zip(frame.iter()) {
            *s += v;
        }
        count += 1;
    }
    if count == 0 {
        return None;
    }
    let mean: Vec<f32> = sum.iter().map(|s| s / count as f32).collect();
    Some(l2_normalize(&mean))
}

/// Per-modality weight applied to the cosine similarity when computing a
/// cross-modal retrieval score.
///
/// These are soft priors reflecting how well a text query typically aligns
/// with each media embedding space.
fn cross_modal_weight(modality: ModalityType) -> f32 {
    match modality {
        ModalityType::Text => 1.0,
        ModalityType::Image => 0.90,
        ModalityType::VideoScene => 0.85,
        ModalityType::Audio => 0.80,
    }
}

/// Compute a retrieval relevance score between a text query embedding and a
/// media (image / audio / video) embedding.
///
/// The score is the cosine similarity scaled by a per-modality discount weight
/// so that the final value is still in `[-1.0, 1.0]`.
pub fn cross_modal_score(text_vec: &[f32], media_vec: &[f32], modality: ModalityType) -> f32 {
    cosine_similarity(text_vec, media_vec) * cross_modal_weight(modality)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── l2_normalize ────────────────────────────────────────────────────────

    #[test]
    fn test_l2_normalize_unit_norm() {
        let v = vec![3.0_f32, 4.0];
        let n = l2_normalize(&v);
        let norm: f32 = n.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6, "norm = {norm}");
    }

    #[test]
    fn test_l2_normalize_zero_vector() {
        let v = vec![0.0_f32, 0.0, 0.0];
        let n = l2_normalize(&v);
        assert_eq!(n, v, "zero vector must be returned unchanged");
    }

    // ── cosine_similarity ───────────────────────────────────────────────────

    #[test]
    fn test_cosine_identity() {
        let v = vec![1.0_f32, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6, "sim = {sim}");
    }

    #[test]
    fn test_cosine_orthogonal() {
        let a = vec![1.0_f32, 0.0];
        let b = vec![0.0_f32, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6, "sim = {sim}");
    }

    #[test]
    fn test_cosine_opposite() {
        let a = vec![1.0_f32, 0.0];
        let b = vec![-1.0_f32, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-6, "sim = {sim}");
    }

    #[test]
    fn test_cosine_zero_vector_returns_zero() {
        let zero = vec![0.0_f32, 0.0, 0.0];
        let v = vec![1.0_f32, 2.0, 3.0];
        assert_eq!(cosine_similarity(&zero, &v), 0.0);
    }

    // ── euclidean_distance ──────────────────────────────────────────────────

    #[test]
    fn test_euclidean_distance_same_point() {
        let v = vec![1.0_f32, 2.0, 3.0];
        assert_eq!(euclidean_distance(&v, &v), 0.0);
    }

    #[test]
    fn test_euclidean_distance_known_value() {
        let a = vec![0.0_f32, 0.0];
        let b = vec![3.0_f32, 4.0];
        let d = euclidean_distance(&a, &b);
        assert!((d - 5.0).abs() < 1e-6, "d = {d}");
    }

    // ── pool_frames_l2 ──────────────────────────────────────────────────────

    #[test]
    fn test_pool_frames_l2_empty_returns_none() {
        assert_eq!(pool_frames_l2(&[]), None);
    }

    #[test]
    fn test_pool_frames_l2_single_frame() {
        let frame = vec![3.0_f32, 4.0];
        let result = pool_frames_l2(&[frame]).unwrap();
        // A single frame → normalised version
        let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_pool_frames_l2_two_frames_mean() {
        // Two identical frames → result should be the normalised version of that frame
        let f = vec![3.0_f32, 4.0];
        let result = pool_frames_l2(&[f.clone(), f.clone()]).unwrap();
        let expected = l2_normalize(&[3.0_f32, 4.0]);
        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6);
        }
    }

    #[test]
    fn test_pool_frames_l2_skips_malformed_frame() {
        let good = vec![3.0_f32, 4.0];
        let bad = vec![1.0_f32]; // wrong dim
                                 // Should process only 'good', return Some(normalised good)
        let result = pool_frames_l2(&[good.clone(), bad, good]).unwrap();
        let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    // ── validate_modality_dim ───────────────────────────────────────────────

    #[test]
    fn test_validate_dim_image_ok() {
        let v = vec![0.0_f32; 768];
        assert!(validate_modality_dim(&v, ModalityType::Image).is_ok());
    }

    #[test]
    fn test_validate_dim_audio_ok() {
        let v = vec![0.0_f32; 512];
        assert!(validate_modality_dim(&v, ModalityType::Audio).is_ok());
    }

    #[test]
    fn test_validate_dim_video_ok() {
        let v = vec![0.0_f32; 768];
        assert!(validate_modality_dim(&v, ModalityType::VideoScene).is_ok());
    }

    #[test]
    fn test_validate_dim_text_any_dim_ok() {
        // Text has no fixed dimension — any length is valid.
        let v = vec![0.0_f32; 42];
        assert!(validate_modality_dim(&v, ModalityType::Text).is_ok());
    }

    #[test]
    fn test_validate_dim_image_wrong_dim_err() {
        let v = vec![0.0_f32; 512]; // wrong for Image
        let err = validate_modality_dim(&v, ModalityType::Image).unwrap_err();
        assert_eq!(err.expected, 768);
        assert_eq!(err.got, 512);
        assert_eq!(err.modality, ModalityType::Image);
    }

    #[test]
    fn test_validate_dim_audio_wrong_dim_err() {
        let v = vec![0.0_f32; 768]; // wrong for Audio
        let err = validate_modality_dim(&v, ModalityType::Audio).unwrap_err();
        assert_eq!(err.expected, 512);
        assert_eq!(err.got, 768);
    }

    // ── cross_modal_score ───────────────────────────────────────────────────

    #[test]
    fn test_cross_modal_score_text_text_is_cosine() {
        let v = vec![1.0_f32, 0.0, 0.0];
        let score = cross_modal_score(&v, &v, ModalityType::Text);
        assert!((score - 1.0).abs() < 1e-6, "score = {score}");
    }

    #[test]
    fn test_cross_modal_score_image_positive() {
        let text = vec![1.0_f32, 1.0, 0.0];
        let image = vec![1.0_f32, 0.5, 0.0];
        let score = cross_modal_score(&text, &image, ModalityType::Image);
        assert!(score > 0.0, "score = {score}");
        assert!(score <= 0.90, "capped by modality weight: {score}");
    }

    #[test]
    fn test_cross_modal_weight_ordering() {
        // Text ≥ Image ≥ VideoScene ≥ Audio
        let w_text = cross_modal_weight(ModalityType::Text);
        let w_img = cross_modal_weight(ModalityType::Image);
        let w_vid = cross_modal_weight(ModalityType::VideoScene);
        let w_aud = cross_modal_weight(ModalityType::Audio);
        assert!(w_text >= w_img);
        assert!(w_img >= w_vid);
        assert!(w_vid >= w_aud);
    }
}
