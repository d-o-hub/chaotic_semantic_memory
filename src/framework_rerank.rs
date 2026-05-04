//! Framework extensions for reranking retrieval results.

use crate::error::Result;
use crate::framework::ChaoticSemanticFramework;
use crate::hyperdim::HVec10240;
use crate::retrieval::rerank::{RerankCandidate, Reranker};
use std::sync::Arc;
use tracing::instrument;

impl ChaoticSemanticFramework {
    /// Query for similar concepts and apply a pipeline of rerankers.
    #[instrument(err, skip(self, query, rerankers))]
    pub async fn probe_with_rerankers(
        &self,
        query: HVec10240,
        initial_top_k: usize,
        rerankers: &[Box<dyn Reranker>],
        final_top_k: usize,
    ) -> Result<Vec<(String, f32)>> {
        self.validate_top_k(initial_top_k)?;
        self.validate_top_k(final_top_k)?;

        // 1. Initial probe
        let initial_results = self.probe(query, initial_top_k).await?;

        if rerankers.is_empty() {
            let mut results = initial_results;
            results.truncate(final_top_k);
            return Ok(results);
        }

        // 2. Fetch full concept data for reranking
        let mut candidates = Vec::with_capacity(initial_results.len());
        {
            let sing = self.singularity.read().await;
            for (id, score) in initial_results {
                if let Some(concept) = sing.get(&id) {
                    candidates.push(RerankCandidate {
                        id: concept.id.clone(),
                        vector: Arc::new(concept.vector),
                        metadata: concept.metadata.clone(),
                        score,
                        created_at_unix: concept.created_at,
                    });
                }
            }
        }

        // 3. Apply rerankers in sequence
        for reranker in rerankers {
            candidates = reranker.rerank(&query, candidates, initial_top_k);
        }

        // 4. Truncate and format final results
        candidates.truncate(final_top_k);
        Ok(candidates.into_iter().map(|c| (c.id, c.score)).collect())
    }
}
