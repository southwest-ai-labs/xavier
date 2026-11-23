use crate::embedding::{Embedder, EmbeddingError};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "embed-native")]
use xavier_embed_ort::OrtEmbedder;

pub struct NativeGemmaEmbedder {
    #[cfg(feature = "embed-native")]
    embedder: Option<OrtEmbedder>,
}

impl NativeGemmaEmbedder {
    pub const DIMENSION: usize = 768;

    pub fn new() -> Result<Self, EmbeddingError> {
        #[cfg(feature = "embed-native")]
        {
            let data_dir = std::env::var("XAVIER_DATA_DIR").unwrap_or_else(|_| ".data".to_string());
            let model_dir = PathBuf::from(data_dir)
                .join("models")
                .join("embeddinggemma-2");

            let embedder = OrtEmbedder::new(&model_dir).map_err(|e| {
                if let xavier_embed_ort::EmbedOrtError::ModelNotFound(_) = e {
                    EmbeddingError::Config("ModelUnavailable".to_string())
                } else {
                    EmbeddingError::Config(e.to_string())
                }
            })?;

            Ok(Self {
                embedder: Some(embedder),
            })
        }
        #[cfg(not(feature = "embed-native"))]
        {
            Err(EmbeddingError::Config(
                "Native embedding feature is not enabled".to_string(),
            ))
        }
    }
}

#[async_trait]
impl Embedder for NativeGemmaEmbedder {
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        #[cfg(feature = "embed-native")]
        {
            self.embedder
                .as_ref()
                .ok_or_else(|| EmbeddingError::Config("Native embedder uninitialized".to_string()))?
                .embed_query(text)
                .map_err(|e| EmbeddingError::Parse(e.to_string()))
        }
        #[cfg(not(feature = "embed-native"))]
        {
            Err(EmbeddingError::Config(
                "Native embedding feature is not enabled".to_string(),
            ))
        }
    }

    async fn embed_document(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        #[cfg(feature = "embed-native")]
        {
            self.embedder
                .as_ref()
                .ok_or_else(|| EmbeddingError::Config("Native embedder uninitialized".to_string()))?
                .embed_document(None, text)
                .map_err(|e| EmbeddingError::Parse(e.to_string()))
        }
        #[cfg(not(feature = "embed-native"))]
        {
            Err(EmbeddingError::Config(
                "Native embedding feature is not enabled".to_string(),
            ))
        }
    }

    fn dimension(&self) -> usize {
        Self::DIMENSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimension() {
        let embedder = NativeGemmaEmbedder {
            #[cfg(feature = "embed-native")]
            embedder: None,
        };
        assert_eq!(embedder.dimension(), 768);
    }
}
