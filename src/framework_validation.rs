use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::{MemoryError, Result};
use crate::framework::ChaoticSemanticFramework;
use crate::hyperdim::Hypervector;
use crate::metadata_filter::{MAX_FILTER_DEPTH, MetadataFilter};
use crate::singularity::Concept;
use crate::singularity_retrieval::RetrievalConfig;

const MAX_CONCEPT_ID_BYTES: usize = 256;
const MAX_BUCKET_PROBE_WIDTH: usize = 16;
const MAX_TRAVERSAL_DEPTH: usize = 32;
const MAX_TRAVERSAL_RESULTS: usize = 10_000;
pub(crate) const MAX_PATH_LENGTH: usize = 4096;

pub(crate) fn validate_path(path: &str) -> Result<PathBuf> {
    if path.len() > MAX_PATH_LENGTH {
        return Err(MemoryError::InvalidInput {
            field: "path".to_string(),
            reason: format!("path exceeds maximum length of {MAX_PATH_LENGTH} characters"),
        });
    }

    let path = PathBuf::from(path);

    if path
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(MemoryError::InvalidInput {
            field: "path".to_string(),
            reason: "path traversal '..' components are not allowed".to_string(),
        });
    }

    if path.is_absolute() {
        let normalized = if path.exists() {
            path.canonicalize().map_err(|_| MemoryError::InvalidInput {
                field: "path".to_string(),
                reason: "absolute path cannot be accessed".to_string(),
            })?
        } else {
            let parent = path.parent().ok_or_else(|| MemoryError::InvalidInput {
                field: "path".to_string(),
                reason: "absolute path has no parent directory".to_string(),
            })?;
            let file_name = path.file_name().ok_or_else(|| MemoryError::InvalidInput {
                field: "path".to_string(),
                reason: "absolute path must include a file name".to_string(),
            })?;
            let parent_normalized =
                parent
                    .canonicalize()
                    .map_err(|_| MemoryError::InvalidInput {
                        field: "path".to_string(),
                        reason: "absolute path parent does not exist or cannot be accessed"
                            .to_string(),
                    })?;
            parent_normalized.join(file_name)
        };

        let current_dir = std::env::current_dir().map_err(|e| MemoryError::InvalidInput {
            field: "path".to_string(),
            reason: format!("cannot determine current working directory: {e}"),
        })?;

        if !normalized.starts_with(&current_dir) && !normalized.starts_with("/tmp") {
            return Err(MemoryError::InvalidInput {
                field: "path".to_string(),
                reason: "absolute paths must be within current working directory or /tmp"
                    .to_string(),
            });
        }
    }

    Ok(path)
}

impl<H: Hypervector> ChaoticSemanticFramework<H> {
    pub(crate) fn validate_retrieval_config(config: &RetrievalConfig) -> Result<()> {
        if config.bucket_probe_width > MAX_BUCKET_PROBE_WIDTH {
            return Err(MemoryError::InvalidInput {
                field: "bucket_probe_width".to_string(),
                reason: format!("bucket_probe_width exceeds {MAX_BUCKET_PROBE_WIDTH}"),
            });
        }
        Ok(())
    }

    pub(crate) fn validate_concept_id(id: &str) -> Result<()> {
        if id.is_empty() {
            return Err(MemoryError::InvalidInput {
                field: "id".to_string(),
                reason: "concept ID must not be empty".to_string(),
            });
        }
        if id.len() > MAX_CONCEPT_ID_BYTES {
            return Err(MemoryError::InvalidInput {
                field: "id".to_string(),
                reason: format!(
                    "concept ID exceeds {} bytes (got {})",
                    MAX_CONCEPT_ID_BYTES,
                    id.len()
                ),
            });
        }
        if id.chars().any(|c| c.is_control()) {
            return Err(MemoryError::InvalidInput {
                field: "id".to_string(),
                reason: "concept ID must not contain control characters".to_string(),
            });
        }
        Ok(())
    }

    pub(crate) fn validate_association_strength(strength: f32) -> Result<()> {
        if !strength.is_finite() {
            return Err(MemoryError::InvalidInput {
                field: "strength".to_string(),
                reason: "association strength must be finite".to_string(),
            });
        }
        if !(0.0..=1.0).contains(&strength) {
            return Err(MemoryError::InvalidInput {
                field: "strength".to_string(),
                reason: format!("association strength must be in [0.0, 1.0], got {strength}"),
            });
        }
        Ok(())
    }

    pub(crate) fn validate_metadata_bytes(
        metadata: &HashMap<String, serde_json::Value>,
        max_metadata_bytes: Option<usize>,
    ) -> Result<()> {
        let Some(limit) = max_metadata_bytes else {
            return Ok(());
        };
        let size = serde_json::to_vec(metadata)?.len();
        if size > limit {
            return Err(MemoryError::InvalidInput {
                field: "metadata".to_string(),
                reason: format!("metadata exceeds {limit} bytes (got {size})"),
            });
        }
        Ok(())
    }

    pub(crate) fn validate_concept(&self, concept: &Concept<H>) -> Result<()> {
        Self::validate_concept_id(&concept.id)?;
        Self::validate_metadata_bytes(&concept.metadata, self.config.max_metadata_bytes)
    }

    pub(crate) fn validate_top_k(&self, top_k: usize) -> Result<()> {
        if top_k == 0 {
            return Err(MemoryError::InvalidInput {
                field: "top_k".to_string(),
                reason: "top_k must be greater than 0".to_string(),
            });
        }
        if top_k > self.config.max_probe_top_k {
            return Err(MemoryError::InvalidInput {
                field: "top_k".to_string(),
                reason: format!(
                    "top_k exceeds configured limit {} (got {})",
                    self.config.max_probe_top_k, top_k
                ),
            });
        }
        Ok(())
    }

    pub(crate) fn validate_batch_size(&self, batch_size: usize) -> Result<()> {
        if batch_size > self.config.max_batch_size {
            return Err(MemoryError::InvalidInput {
                field: "batch_size".to_string(),
                reason: format!(
                    "batch size exceeds configured limit {} (got {})",
                    self.config.max_batch_size, batch_size
                ),
            });
        }
        Ok(())
    }

    pub(crate) fn validate_traversal_config(
        config: &crate::graph_traversal::TraversalConfig,
    ) -> Result<()> {
        if config.max_depth > MAX_TRAVERSAL_DEPTH {
            return Err(MemoryError::InvalidInput {
                field: "max_depth".to_string(),
                reason: format!(
                    "traversal depth exceeds {} (got {})",
                    MAX_TRAVERSAL_DEPTH, config.max_depth
                ),
            });
        }
        if config.max_results > MAX_TRAVERSAL_RESULTS {
            return Err(MemoryError::InvalidInput {
                field: "max_results".to_string(),
                reason: format!(
                    "traversal results exceed {} (got {})",
                    MAX_TRAVERSAL_RESULTS, config.max_results
                ),
            });
        }
        Ok(())
    }

    pub(crate) fn validate_sequence_length(&self, length: usize) -> Result<()> {
        if length > self.config.max_sequence_length {
            return Err(MemoryError::InvalidInput {
                field: "sequence_length".to_string(),
                reason: format!(
                    "sequence length exceeds configured limit {} (got {})",
                    self.config.max_sequence_length, length
                ),
            });
        }
        Ok(())
    }

    pub(crate) fn validate_metadata_filter(filter: &MetadataFilter) -> Result<()> {
        let depth = filter.depth();
        if depth > MAX_FILTER_DEPTH {
            return Err(MemoryError::InvalidInput {
                field: "filter".to_string(),
                reason: format!(
                    "metadata filter depth exceeds maximum allowed {MAX_FILTER_DEPTH} (got {depth})"
                ),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyperdim::HVec10240;

    #[test]
    fn test_validate_concept_id_dangerous_chars() {
        // Null byte
        assert!(ChaoticSemanticFramework::<HVec10240>::validate_concept_id("test\0id").is_err());
        // Newline
        assert!(ChaoticSemanticFramework::<HVec10240>::validate_concept_id("test\nid").is_err());
        // Carriage return
        assert!(ChaoticSemanticFramework::<HVec10240>::validate_concept_id("test\rid").is_err());
        // Tab
        assert!(ChaoticSemanticFramework::<HVec10240>::validate_concept_id("test\tid").is_err());
        // ESC
        assert!(ChaoticSemanticFramework::<HVec10240>::validate_concept_id("test\x1bid").is_err());
        // DEL
        assert!(ChaoticSemanticFramework::<HVec10240>::validate_concept_id("test\x7fid").is_err());

        // Valid IDs
        assert!(ChaoticSemanticFramework::<HVec10240>::validate_concept_id("valid-id_123").is_ok());
        assert!(
            ChaoticSemanticFramework::<HVec10240>::validate_concept_id("id:with:colons").is_ok()
        );
        assert!(
            ChaoticSemanticFramework::<HVec10240>::validate_concept_id("path/to/resource").is_ok()
        );
    }

    #[test]
    fn test_validate_concept_id_empty() {
        assert!(ChaoticSemanticFramework::<HVec10240>::validate_concept_id("").is_err());
    }

    #[test]
    fn test_validate_concept_id_too_long() {
        let long_id = "a".repeat(257);
        assert!(ChaoticSemanticFramework::<HVec10240>::validate_concept_id(&long_id).is_err());
        let edge_id = "a".repeat(256);
        assert!(ChaoticSemanticFramework::<HVec10240>::validate_concept_id(&edge_id).is_ok());
    }

    #[test]
    fn test_validate_retrieval_config_bucket_width() {
        let config = RetrievalConfig {
            bucket_probe_width: 16,
            ..RetrievalConfig::default()
        };
        assert!(ChaoticSemanticFramework::<HVec10240>::validate_retrieval_config(&config).is_ok());

        let config = RetrievalConfig {
            bucket_probe_width: 17,
            ..RetrievalConfig::default()
        };
        assert!(ChaoticSemanticFramework::<HVec10240>::validate_retrieval_config(&config).is_err());
    }

    #[test]
    fn path_traversal_blocked() {
        assert!(validate_path("../etc/passwd").is_err());
    }

    #[test]
    fn path_too_long() {
        let long = "a".repeat(5000);
        assert!(validate_path(&long).is_err());
    }

    #[test]
    fn path_relative_ok() {
        assert!(validate_path("test.json").is_ok());
    }
}
