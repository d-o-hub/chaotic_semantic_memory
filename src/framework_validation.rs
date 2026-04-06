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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_concept_id_dangerous_chars() {
        // Null byte
        assert!(ChaoticSemanticFramework::validate_concept_id("test\0id").is_err());
        // Newline
        assert!(ChaoticSemanticFramework::validate_concept_id("test\nid").is_err());
        // Carriage return
        assert!(ChaoticSemanticFramework::validate_concept_id("test\rid").is_err());
        // Tab
        assert!(ChaoticSemanticFramework::validate_concept_id("test\tid").is_err());
        // ESC
        assert!(ChaoticSemanticFramework::validate_concept_id("test\x1bid").is_err());
        // DEL
        assert!(ChaoticSemanticFramework::validate_concept_id("test\x7fid").is_err());

        // Valid IDs
        assert!(ChaoticSemanticFramework::validate_concept_id("valid-id_123").is_ok());
        assert!(ChaoticSemanticFramework::validate_concept_id("id:with:colons").is_ok());
        assert!(ChaoticSemanticFramework::validate_concept_id("path/to/resource").is_ok());
    }

    #[test]
    fn test_validate_concept_id_empty() {
        assert!(ChaoticSemanticFramework::validate_concept_id("").is_err());
    }

    #[test]
    fn test_validate_concept_id_too_long() {
        let long_id = "a".repeat(257);
        assert!(ChaoticSemanticFramework::validate_concept_id(&long_id).is_err());
        let edge_id = "a".repeat(256);
        assert!(ChaoticSemanticFramework::validate_concept_id(&edge_id).is_ok());
    }
}
