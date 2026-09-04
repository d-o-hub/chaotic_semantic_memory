#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
use csm_core_lib::hyperdim::HVec10240;
use std::collections::HashMap;
use std::sync::Arc;

/// A candidate for reranking.
#[derive(Debug, Clone)]
pub struct RerankCandidate {
    /// Unique identifier for the concept.
    pub id: String,
    /// Hypervector representation.
    pub vector: Arc<HVec10240>,
    /// Associated metadata.
    pub metadata: HashMap<String, serde_json::Value>,
    /// Retrieval score (initially cosine similarity).
    pub score: f32,
    /// Creation timestamp in Unix seconds.
    pub created_at_unix: u64,
}

/// Trait for reranking retrieval results.
pub trait Reranker: Send + Sync + std::fmt::Debug {
    /// Returns the name of the reranker.
    fn name(&self) -> &str;

    /// Reranks the candidates based on the query and existing scores.
    fn rerank(
        &self,
        query: &HVec10240,
        candidates: Vec<RerankCandidate>,
        top_k: usize,
    ) -> Vec<RerankCandidate>;
}

/// Maximal Marginal Relevance (MMR) reranker for diversity.
#[derive(Debug)]
pub struct MmrReranker {
    /// Diversity vs similarity trade-off (0.0 = full diversity, 1.0 = pure similarity).
    pub lambda: f32,
}

impl Reranker for MmrReranker {
    fn name(&self) -> &str {
        "mmr"
    }

    fn rerank(
        &self,
        query: &HVec10240,
        mut candidates: Vec<RerankCandidate>,
        top_k: usize,
    ) -> Vec<RerankCandidate> {
        if candidates.is_empty() || top_k == 0 {
            return Vec::new();
        }

        if self.lambda >= 1.0 {
            // Fast-path: lambda >= 1.0 is pure similarity (no diversity penalty).
            // Avoids O(N * K) selection loop and expensive cosine_similarity penalty updates.
            for cand in &mut candidates {
                cand.score = query.cosine_similarity(&cand.vector);
            }
            if candidates.len() > top_k {
                let nth = top_k - 1;
                candidates.select_nth_unstable_by(nth, |a, b| b.score.total_cmp(&a.score));
                candidates.truncate(top_k);
            }
            candidates.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
            return candidates;
        }

        let mut selected: Vec<RerankCandidate> = Vec::with_capacity(top_k);

        // Precompute cosine similarity between query and all candidates.
        // This avoids recalculating query similarities (which are static across steps) O(N * K) times.
        let mut query_similarities: Vec<f32> = candidates
            .iter()
            .map(|cand| query.cosine_similarity(&cand.vector))
            .collect();

        // Cache the maximum similarity of each candidate to the already selected set.
        // Initially, the selected set is empty, so maximum similarity is 0.0.
        let mut max_sim_to_selected = vec![0.0f32; candidates.len()];

        let lambda = self.lambda;
        let one_minus_lambda = 1.0 - lambda;

        // Greedily select candidates
        while selected.len() < top_k && !candidates.is_empty() {
            let mut best_idx = 0;
            let mut max_mmr = f32::NEG_INFINITY;

            // Zip similarities to avoid manual indexing and bounds checks.
            for (idx, (&similarity, &max_sim)) in query_similarities
                .iter()
                .zip(max_sim_to_selected.iter())
                .enumerate()
            {
                // MMR Formula: lambda * sim(query, cand) - (1 - lambda) * max_sim(cand, selected)
                let mmr_score = lambda * similarity - one_minus_lambda * max_sim;

                if mmr_score > max_mmr {
                    max_mmr = mmr_score;
                    best_idx = idx;
                }
            }

            // O(1) swap_remove instead of O(N) remove to avoid shifting subsequent elements.
            // Alignment with query_similarities and max_sim_to_selected is preserved by
            // performing swap_remove on them as well.
            let mut best_cand = candidates.swap_remove(best_idx);
            best_cand.score = max_mmr;
            query_similarities.swap_remove(best_idx);
            max_sim_to_selected.swap_remove(best_idx);

            // Incrementally update the maximum similarity cache using the newly selected candidate.
            // This reduces the complexity of similarity-to-selected tracking from O(N * K^2) to O(N * K).
            // Using zip avoids bounds checks and indexing overhead.
            for (cand, max_sim) in candidates.iter().zip(max_sim_to_selected.iter_mut()) {
                let sim = cand.vector.cosine_similarity(&best_cand.vector);
                if sim > *max_sim {
                    *max_sim = sim;
                }
            }

            selected.push(best_cand);
        }

        selected
    }
}

/// Recency decay reranker to favor newer concepts.
#[derive(Debug)]
pub struct RecencyDecayReranker {
    /// Time period after which weight is halved (in days).
    pub half_life_days: f32,
    /// Balance between similarity and recency (0.0 = pure recency, 1.0 = pure similarity).
    pub blend: f32,
}

impl Reranker for RecencyDecayReranker {
    fn name(&self) -> &str {
        "recency"
    }

    fn rerank(
        &self,
        _query: &HVec10240,
        mut candidates: Vec<RerankCandidate>,
        top_k: usize,
    ) -> Vec<RerankCandidate> {
        if candidates.is_empty() || top_k == 0 {
            return Vec::new();
        }

        let now = csm_memory::unix_now_secs();
        let half_life_secs = self.half_life_days * 86400.0;
        let inv_half_life = 1.0 / half_life_secs;
        let blend = self.blend;
        let one_minus_blend = 1.0 - blend;

        for cand in &mut candidates {
            let age_secs = now.saturating_sub(cand.created_at_unix) as f32;
            // CPU-native base-2 exponentiation (-age_secs * inv_half_life).exp2()
            // leveraged via identity 0.5^x = 2^-x to avoid powf overhead.
            let recency = (-age_secs * inv_half_life).exp2();

            // blended_score = blend * original_score + (1 - blend) * recency
            cand.score = blend * cand.score + one_minus_blend * recency;
        }

        if candidates.len() > top_k {
            let nth = top_k - 1;
            candidates.select_nth_unstable_by(nth, |a, b| b.score.total_cmp(&a.score));
            candidates.truncate(top_k);
        }
        candidates.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
        candidates
    }
}

/// Cross-encoder reranker using ONNX (opt-in).
#[cfg(feature = "rerank-cross")]
#[derive(Debug)]
pub struct CrossEncoderReranker {
    pub model: Arc<candle_onnx::onnx::ModelProto>,
    pub model_path: String,
}

#[cfg(feature = "rerank-cross")]
impl Reranker for CrossEncoderReranker {
    fn name(&self) -> &str {
        "cross-encoder"
    }

    fn rerank(
        &self,
        _query: &HVec10240,
        candidates: Vec<RerankCandidate>,
        top_k: usize,
    ) -> Vec<RerankCandidate> {
        // Implementation would load and run ONNX model
        // For now, it's a skeleton that returns candidates as-is
        let mut results = candidates;
        results.truncate(top_k);
        results
    }
}

/// Parses a list of rerankers from a string flag (e.g., "mmr:0.7,recency:30d").
pub fn parse_rerankers(s: &str) -> csm_core_lib::error::Result<Vec<Box<dyn Reranker>>> {
    let mut rerankers: Vec<Box<dyn Reranker>> = Vec::new();
    for part in s.split(',') {
        if part.is_empty() {
            continue;
        }
        let (name, value) = part.split_once(':').unwrap_or((part, ""));

        match name {
            "mmr" => {
                let lambda = value.parse::<f32>().map_err(|_| {
                    csm_core_lib::error::MemoryError::InvalidInput {
                        field: "rerank".to_string(),
                        reason: format!("invalid MMR lambda: {value}"),
                    }
                })?;

                if !(0.0..=1.0).contains(&lambda) {
                    return Err(csm_core_lib::error::MemoryError::InvalidInput {
                        field: "rerank".to_string(),
                        reason: format!("MMR lambda must be between 0.0 and 1.0: {lambda}"),
                    });
                }

                rerankers.push(Box::new(MmrReranker { lambda }));
            }
            "recency" => {
                let mut recency_split = value.split(':');
                let half_life_str = recency_split.next().unwrap_or("");
                let val_str = if let Some(stripped) = half_life_str.strip_suffix('d') {
                    stripped
                } else {
                    half_life_str
                };
                let half_life = val_str.parse::<f32>().map_err(|_| {
                    csm_core_lib::error::MemoryError::InvalidInput {
                        field: "rerank".to_string(),
                        reason: format!("invalid recency half-life: {half_life_str}"),
                    }
                })?;

                if half_life <= 0.0 {
                    return Err(csm_core_lib::error::MemoryError::InvalidInput {
                        field: "rerank".to_string(),
                        reason: format!("recency half-life must be positive: {half_life}"),
                    });
                }

                let blend = if let Some(blend_str) = recency_split.next() {
                    let b = blend_str.parse::<f32>().map_err(|_| {
                        csm_core_lib::error::MemoryError::InvalidInput {
                            field: "rerank".to_string(),
                            reason: format!("invalid recency blend: {blend_str}"),
                        }
                    })?;
                    if !(0.0..=1.0).contains(&b) {
                        return Err(csm_core_lib::error::MemoryError::InvalidInput {
                            field: "rerank".to_string(),
                            reason: format!("recency blend must be between 0.0 and 1.0: {b}"),
                        });
                    }
                    b
                } else {
                    0.5
                };

                if recency_split.next().is_some() {
                    return Err(csm_core_lib::error::MemoryError::InvalidInput {
                        field: "rerank".to_string(),
                        reason: format!("extra segments in recency reranker: {value}"),
                    });
                }

                rerankers.push(Box::new(RecencyDecayReranker {
                    half_life_days: half_life,
                    blend,
                }));
            }
            #[cfg(feature = "rerank-cross")]
            "cross" => {
                let model = candle_onnx::read_file(value).map_err(|e| {
                    csm_core_lib::error::MemoryError::InvalidInput {
                        field: "rerank".to_string(),
                        reason: format!("failed to load ONNX model {value}: {e}"),
                    }
                })?;
                rerankers.push(Box::new(CrossEncoderReranker {
                    model: Arc::new(model),
                    model_path: value.to_string(),
                }));
            }
            _ => {
                return Err(csm_core_lib::error::MemoryError::InvalidInput {
                    field: "rerank".to_string(),
                    reason: format!("unknown reranker: {name}"),
                });
            }
        }
    }
    Ok(rerankers)
}

#[cfg(test)]
#[path = "rerank_tests.rs"]
mod tests;
