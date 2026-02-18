use std::collections::HashMap;

use crate::error::{MemoryError, Result};
use crate::framework::ChaoticSemanticFramework;
use crate::singularity::Concept;

const MAX_CONCEPT_ID_BYTES: usize = 256;

impl ChaoticSemanticFramework {
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
        Ok(())
    }

    pub(crate) fn validate_association_strength(strength: f32) -> Result<()> {
        if !strength.is_finite() {
            return Err(MemoryError::InvalidInput {
                field: "strength".to_string(),
                reason: "association strength must be finite".to_string(),
            });
        }
        if strength < 0.0 {
            return Err(MemoryError::InvalidInput {
                field: "strength".to_string(),
                reason: "association strength must be non-negative".to_string(),
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
                reason: format!("metadata exceeds {} bytes (got {})", limit, size),
            });
        }
        Ok(())
    }

    pub(crate) fn validate_concept(&self, concept: &Concept) -> Result<()> {
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
}
