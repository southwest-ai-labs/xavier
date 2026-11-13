//! Xavier Core Logic — Pure domain crate for BM25, RRF, retrieval scoring, snippet processing, and token compression.
//!
//! Free of I/O, database dependencies, or async runtimes. WASM-ready.

pub mod bm25;
pub mod multimodal;
pub mod rrf;
pub mod scoring;
pub mod snippet;
pub mod token_counter;
pub mod types;

pub use bm25::{score_documents, Bm25Params};
pub use multimodal::{
    cosine_similarity, cross_modal_score, euclidean_distance, expected_dim, l2_normalize,
    pool_frames_l2, validate_modality_dim, DimError,
};
pub use rrf::{reciprocal_rank_fusion, reciprocal_rank_fusion_weighted};
pub use scoring::{
    calculate_recency_boost_factor, score_single_episodic, score_single_semantic,
    score_single_working, WorkingScoringParams,
};
pub use snippet::{clip_chars, extract, Excerpt, SnippetBudget};
pub use token_counter::{
    collapse_repeated_ast_snippets, compress_and_calculate_stats, estimate_tokens_utf8,
    CompressionStats,
};
pub use types::*;
