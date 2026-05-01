//! Singularity extension methods for API completeness.

use std::collections::HashMap;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use tracing::instrument;

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;
use crate::metadata_filter::MetadataFilter;
use crate::singularity::{Concept, Singularity, unix_now_secs};
pub use crate::singularity_cache::CacheMetricsSnapshot;
use crate::singularity_retrieval::{CandidateSource, FilterStrategy};

impl Singularity {
    /// Bundle multiple concepts into a single hypervector (strict version).
    /// Returns `NotFound` error if any concept ID is missing.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        instrument(skip(self), fields(ids_count = ids.len()))
    )]
    pub fn bundle_concepts_strict(&self, ids: &[String]) -> Result<HVec10240> {
        let mut vectors = Vec::with_capacity(ids.len());
        for id in ids {
            match self.concepts.get(id) {
                Some(concept) => vectors.push(concept.vector),
                None => {
                    return Err(MemoryError::NotFound {
                        entity: "Concept".to_string(),
                        id: id.clone(),
                    });
                }
            }
        }
        HVec10240::bundle(&vectors)
    }

    /// Remove an association between two concepts.
    /// Returns Ok(()) even if the association didn't exist.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        instrument(skip(self), fields(from_id = %from, to_id = %to))
    )]
    pub fn disassociate(&mut self, from: &str, to: &str) -> Result<()> {
        if !self.concepts.contains_key(from) {
            return Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: from.to_string(),
            });
        }
        if let Some(links) = self.associations.get_mut(from) {
            links.remove(to);
        }
        self.invalidate_cache();
        Ok(())
    }

    /// Clear all outbound associations for a concept.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        instrument(skip(self), fields(concept_id = %id))
    )]
    pub fn clear_associations(&mut self, id: &str) -> Result<()> {
        if !self.concepts.contains_key(id) {
            return Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: id.to_string(),
            });
        }
        self.associations.remove(id);
        self.invalidate_cache();
        Ok(())
    }

    /// Clear the similarity query cache.
    pub fn clear_similarity_cache(&self) {
        self.invalidate_cache();
    }

    /// Update concept metadata.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        instrument(skip(self), fields(concept_id = %id))
    )]
    pub fn update_metadata(
        &mut self,
        id: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        if let Some(concept) = self.concepts.get_mut(id) {
            concept.metadata = metadata;
            concept.modified_at = unix_now_secs();
            Ok(())
        } else {
            Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: id.to_string(),
            })
        }
    }

    /// Find similar concepts filtered by metadata predicate (ADR-0065: selectivity-aware).
    ///
    /// Routes to optimal strategy based on filter selectivity:
    /// - selectivity < 0.3: Pre-filter candidates, then score
    /// - selectivity 0.3-0.8: Bucket candidates, score, post-filter
    /// - selectivity >= 0.8: Full scan, score, post-filter
    #[cfg_attr(
        not(target_arch = "wasm32"),
        instrument(skip(self, query), fields(top_k = top_k))
    )]
    pub fn find_similar_filtered(
        &self,
        query: &HVec10240,
        top_k: usize,
        filter: &MetadataFilter,
    ) -> Arc<[(String, f32)]> {
        // Defensive depth check (CWE-674)
        if filter.depth() > crate::metadata_filter::MAX_FILTER_DEPTH {
            return Arc::from(Vec::new());
        }

        let start_ns = crate::singularity::unix_now_ns();
        if top_k == 0 || self.concepts.is_empty() {
            return Arc::from(Vec::new());
        }

        // ADR-0065: Compute selectivity ratio
        let total_count = self.concepts.len();
        let matching_count = self
            .concepts
            .values()
            .filter(|c| filter.matches(&c.metadata))
            .count();
        let selectivity = matching_count as f32 / total_count as f32;

        if matching_count == 0 {
            return Arc::from(Vec::new());
        }

        // ADR-0065: Route based on selectivity
        // For small datasets (<20 concepts), always use PreFilter for correctness
        let strategy = if total_count < 20 || selectivity < 0.3 {
            FilterStrategy::Pre
        } else if selectivity < 0.8 {
            FilterStrategy::BucketPost
        } else {
            FilterStrategy::ScanPost
        };

        match strategy {
            FilterStrategy::Pre => {
                let cand_start = crate::singularity::unix_now_ns();
                let candidates: Vec<usize> = self
                    .concepts
                    .iter()
                    .filter(|(_, concept)| filter.matches(&concept.metadata))
                    .filter_map(|(id, _)| self.id_to_index.get(id).copied())
                    .collect();
                let cand_ns = crate::singularity::unix_now_ns().saturating_sub(cand_start);

                self.scored_candidate_retrieval_with_stats(
                    crate::singularity_retrieval::ScoredCandidateParams {
                        query,
                        top_k,
                        candidates,
                        start_ns,
                        cand_ns,
                        source: CandidateSource::Metadata,
                        bypass_cache: true,
                    },
                    selectivity,
                    Some(strategy),
                )
            }
            FilterStrategy::BucketPost => {
                let cand_start = crate::singularity::unix_now_ns();
                let candidates = self.generate_bucket_candidates(query);
                let cand_ns = crate::singularity::unix_now_ns().saturating_sub(cand_start);

                let all_results = self.scored_candidate_retrieval_with_stats(
                    crate::singularity_retrieval::ScoredCandidateParams {
                        query,
                        top_k: top_k * 2,
                        candidates,
                        start_ns,
                        cand_ns,
                        source: CandidateSource::Bucket,
                        bypass_cache: true,
                    },
                    selectivity,
                    Some(strategy),
                );

                let filtered: Vec<(String, f32)> = all_results
                    .iter()
                    .filter(|(id, _)| {
                        self.concepts
                            .get(id)
                            .map(|c| filter.matches(&c.metadata))
                            .unwrap_or(false)
                    })
                    .take(top_k)
                    .map(|(id, score)| (id.clone(), *score))
                    .collect();
                Arc::from(filtered)
            }
            FilterStrategy::ScanPost => {
                let all_results = self.exact_similarity_scan(query, top_k * 2, start_ns, true);

                let filtered: Vec<(String, f32)> = all_results
                    .iter()
                    .filter(|(id, _)| {
                        self.concepts
                            .get(id)
                            .map(|c| filter.matches(&c.metadata))
                            .unwrap_or(false)
                    })
                    .take(top_k)
                    .map(|(id, score)| (id.clone(), *score))
                    .collect();

                // Update stats via direct call
                if let Ok(mut s) = self.last_retrieval_stats.write() {
                    *s = crate::singularity_retrieval::RetrievalStats {
                        candidate_count: matching_count,
                        scored_count: filtered.len(),
                        fell_back_to_exact_scan: true,
                        candidate_ns: 0,
                        scoring_ns: 0,
                        selectivity_ratio: selectivity,
                        filter_strategy: Some(strategy),
                    };
                }
                Arc::from(filtered)
            }
        }
    }

    pub fn concept_ids(&self) -> Vec<String> {
        self.concepts.keys().cloned().collect()
    }

    pub fn all_concepts(&self) -> Vec<Concept> {
        self.concepts.values().cloned().collect()
    }

    pub fn all_associations(&self) -> Vec<(String, String, f32)> {
        let mut output = Vec::new();
        for (from, links) in &self.associations {
            for (to, strength) in links {
                output.push((from.clone(), to.clone(), *strength));
            }
        }
        output
    }

    pub fn len(&self) -> usize {
        self.concepts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.concepts.is_empty()
    }

    pub fn cache_metrics_snapshot(&self) -> CacheMetricsSnapshot {
        self.cache_metrics.snapshot()
    }

    pub fn reset_cache_metrics(&self) {
        self.cache_metrics.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MemoryError;
    use crate::singularity::{ConceptBuilder, Singularity, SingularityConfig};
    use std::collections::HashMap;

    #[test]
    fn test_bundle_concepts_strict_success() {
        let mut singularity = Singularity::with_config(SingularityConfig::default());
        let vec1 = HVec10240::random();
        let vec2 = HVec10240::random();

        let c1 = ConceptBuilder::new("c1").with_vector(vec1).build().unwrap();
        let c2 = ConceptBuilder::new("c2").with_vector(vec2).build().unwrap();

        singularity.inject(c1).unwrap();
        singularity.inject(c2).unwrap();

        let result = singularity.bundle_concepts_strict(&["c1".to_string(), "c2".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bundle_concepts_strict_missing_id() {
        let mut singularity = Singularity::with_config(SingularityConfig::default());
        let vec1 = HVec10240::random();

        let c1 = ConceptBuilder::new("c1").with_vector(vec1).build().unwrap();
        singularity.inject(c1).unwrap();

        let result =
            singularity.bundle_concepts_strict(&["c1".to_string(), "missing_id".to_string()]);

        match result {
            Err(MemoryError::NotFound { entity, id }) => {
                assert_eq!(entity, "Concept");
                assert_eq!(id, "missing_id");
            }
            _ => panic!("Expected NotFound error, got {:?}", result),
        }
    }

    #[test]
    fn test_update_metadata_not_found() {
        let mut sing = Singularity::new();
        let metadata = HashMap::new();

        let result = sing.update_metadata("non-existent-id", metadata);

        match result {
            Err(MemoryError::NotFound { entity, id }) => {
                assert_eq!(entity, "Concept");
                assert_eq!(id, "non-existent-id");
            }
            _ => panic!("Expected MemoryError::NotFound, got {:?}", result),
        }
    }

    #[test]
    fn test_update_metadata_success() {
        let mut sing = Singularity::new();
        let concept = ConceptBuilder::new("test-id")
            .with_metadata("original", serde_json::Value::Bool(true))
            .build()
            .expect("Failed to build concept");

        sing.concepts.insert("test-id".to_string(), concept);

        let mut new_metadata = HashMap::new();
        new_metadata.insert("updated".to_string(), serde_json::Value::Bool(true));

        let time_before = crate::singularity::unix_now_secs();

        let result = sing.update_metadata("test-id", new_metadata.clone());
        assert!(result.is_ok());

        let updated_concept = sing.concepts.get("test-id").unwrap();
        assert_eq!(updated_concept.metadata, new_metadata);
        assert!(updated_concept.modified_at >= time_before);
    }
}
