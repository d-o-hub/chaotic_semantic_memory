//! Framework extensions for reranking retrieval results.

use crate::error::Result;
use crate::hyperdim::Hypervector;
use crate::framework::ChaoticSemanticFramework;
use crate::retrieval::rerank::{RerankCandidate, Reranker};
use std::sync::Arc;
use tracing::instrument;

impl<H: Hypervector> ChaoticSemanticFramework<H> {
    /// Query for similar concepts and apply a pipeline of rerankers.
    #[instrument(err, skip(self, query, rerankers))]
    pub async fn probe_with_rerankers(
        &self,
        query: H,
        initial_top_k: usize,
        rerankers: &[Box<dyn Reranker>],
        final_top_k: usize,
    ) -> Result<Vec<(String, f32)>> {
        self.validate_top_k(initial_top_k)?;
        self.validate_top_k(final_top_k)?;

        // ADR-0071: initial_top_k must not be smaller than final_top_k to avoid under-fetching
        let actual_initial_k = initial_top_k.max(final_top_k);

        // 1. Initial probe
        let initial_results = self.probe(query, actual_initial_k).await?;

        if rerankers.is_empty() {
            let mut results = initial_results;
            results.truncate(final_top_k);
            return Ok(results);
        }

        // 2. Fetch full concept data for reranking
        let mut candidates = Vec::with_capacity(initial_results.len());
        {
            let sing = self.singularity.read().await;
            let ns = self.namespace.read().await;
            for (id, score) in initial_results {
                if let Some(concept) = sing.get(&ns, &id) {
                    candidates.push(RerankCandidate {
                        id: concept.id.clone(),
                        vector: Arc::new(concept.vector.to_hvec()),
                        metadata: concept.metadata.clone(),
                        score,
                        created_at_unix: concept.created_at,
                    });
                }
            }
        }

        // 3. Apply rerankers in sequence
        for reranker in rerankers {
            candidates = reranker.rerank(&query.to_hvec(), candidates, actual_initial_k);
        }

        // 4. Truncate and format final results
        candidates.truncate(final_top_k);
        Ok(candidates.into_iter().map(|c| (c.id, c.score)).collect())
    }
}
