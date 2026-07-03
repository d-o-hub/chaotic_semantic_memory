#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
use csm_core::hyperdim::HVec10240;
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
        let n = candidates.len();
        let k = top_k.min(n);
        if k == 0 {
            return Vec::new();
        }

        let mut selected: Vec<RerankCandidate> = Vec::with_capacity(k);

        // Pre-calculate query similarities to avoid redundant calculations: O(N)
        let mut query_sims: Vec<f32> = candidates
            .iter()
            .map(|c| query.cosine_similarity(&c.vector))
            .collect();

        // Track max similarity to any already selected candidate to avoid O(K^2 * N) complexity
        let mut max_sim_to_selected = vec![0.0f32; n];

        // Greedily select candidates: O(K * N) total complexity
        for _ in 0..k {
            let mut best_idx = 0;
            let mut max_mmr = f32::NEG_INFINITY;

            for i in 0..candidates.len() {
                // MMR Formula: lambda * sim(query, cand) - (1 - lambda) * max_sim(cand, selected)
                let mmr_score =
                    self.lambda * query_sims[i] - (1.0 - self.lambda) * max_sim_to_selected[i];

                if mmr_score > max_mmr {
                    max_mmr = mmr_score;
                    best_idx = i;
                }
            }

            // O(1) removal by swapping with the last element
            let mut best_cand = candidates.swap_remove(best_idx);
            best_cand.score = max_mmr;
            let best_vec = best_cand.vector.clone();
            selected.push(best_cand);

            // Synchronize auxiliary vectors
            query_sims.swap_remove(best_idx);
            max_sim_to_selected.swap_remove(best_idx);

            // Update max_sim_to_selected for remaining candidates: O(N)
            for i in 0..candidates.len() {
                let sim = candidates[i].vector.cosine_similarity(&best_vec);
                max_sim_to_selected[i] = max_sim_to_selected[i].max(sim);
            }
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
        let now = csm_memory::unix_now_secs();
        let half_life_secs = self.half_life_days * 86400.0;

        for cand in &mut candidates {
            let age_secs = now.saturating_sub(cand.created_at_unix) as f32;
            let recency = 0.5f32.powf(age_secs / half_life_secs);

            // blended_score = blend * original_score + (1 - blend) * recency
            cand.score = self.blend * cand.score + (1.0 - self.blend) * recency;
        }

        candidates.sort_by(|a, b| b.score.total_cmp(&a.score));
        candidates.truncate(top_k);
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
pub fn parse_rerankers(s: &str) -> csm_core::error::Result<Vec<Box<dyn Reranker>>> {
    let mut rerankers: Vec<Box<dyn Reranker>> = Vec::new();
    for part in s.split(',') {
        if part.is_empty() {
            continue;
        }
        let (name, value) = part.split_once(':').unwrap_or((part, ""));

        match name {
            "mmr" => {
                let lambda = value.parse::<f32>().map_err(|_| {
                    csm_core::error::MemoryError::InvalidInput {
                        field: "rerank".to_string(),
                        reason: format!("invalid MMR lambda: {value}"),
                    }
                })?;

                if !(0.0..=1.0).contains(&lambda) {
                    return Err(csm_core::error::MemoryError::InvalidInput {
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
                    csm_core::error::MemoryError::InvalidInput {
                        field: "rerank".to_string(),
                        reason: format!("invalid recency half-life: {half_life_str}"),
                    }
                })?;

                if half_life <= 0.0 {
                    return Err(csm_core::error::MemoryError::InvalidInput {
                        field: "rerank".to_string(),
                        reason: format!("recency half-life must be positive: {half_life}"),
                    });
                }

                let blend = if let Some(blend_str) = recency_split.next() {
                    let b = blend_str.parse::<f32>().map_err(|_| {
                        csm_core::error::MemoryError::InvalidInput {
                            field: "rerank".to_string(),
                            reason: format!("invalid recency blend: {blend_str}"),
                        }
                    })?;
                    if !(0.0..=1.0).contains(&b) {
                        return Err(csm_core::error::MemoryError::InvalidInput {
                            field: "rerank".to_string(),
                            reason: format!("recency blend must be between 0.0 and 1.0: {b}"),
                        });
                    }
                    b
                } else {
                    0.5
                };

                if recency_split.next().is_some() {
                    return Err(csm_core::error::MemoryError::InvalidInput {
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
                    csm_core::error::MemoryError::InvalidInput {
                        field: "rerank".to_string(),
                        reason: format!("failed to load ONNX model {}: {}", value, e),
                    }
                })?;
                rerankers.push(Box::new(CrossEncoderReranker {
                    model: Arc::new(model),
                    model_path: value.to_string(),
                }));
            }
            _ => {
                return Err(csm_core::error::MemoryError::InvalidInput {
                    field: "rerank".to_string(),
                    reason: format!("unknown reranker: {name}"),
                });
            }
        }
    }
    Ok(rerankers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_candidate(id: &str, score: f32, age_days: f32) -> RerankCandidate {
        let now = csm_memory::unix_now_secs();
        let created_at_unix = now - (age_days * 86400.0) as u64;
        RerankCandidate {
            id: id.to_string(),
            vector: Arc::new(HVec10240::random()),
            metadata: HashMap::new(),
            score,
            created_at_unix,
        }
    }

    #[test]
    fn test_mmr_reranker() {
        // Use a seeded vector as query to ensure non-zero similarities
        let query = HVec10240::new_seeded(42);
        // Use seeded vectors for deterministic similarity
        // v1 will be the anchor
        let v1 = Arc::new(HVec10240::new_seeded(1));
        // v2 is identical to v1
        let v2 = Arc::new(HVec10240::new_seeded(1));
        // v3 is different
        let v3 = Arc::new(HVec10240::new_seeded(2));

        let c1 = RerankCandidate {
            id: "c1".into(),
            vector: v1,
            metadata: HashMap::new(),
            score: 0.9, // Higher initial score
            created_at_unix: 0,
        };
        let c2 = RerankCandidate {
            id: "c2".into(),
            vector: v2,
            metadata: HashMap::new(),
            score: 0.85,
            created_at_unix: 0,
        };
        let c3 = RerankCandidate {
            id: "c3".into(),
            vector: v3,
            metadata: HashMap::new(),
            score: 0.7,
            created_at_unix: 0,
        };

        // If lambda is 1.0, it should be pure similarity: c1, c2
        let reranker_sim = MmrReranker { lambda: 1.0 };
        let results_sim = reranker_sim.rerank(&query, vec![c1.clone(), c2.clone(), c3.clone()], 2);
        assert_eq!(results_sim[0].id, "c1");
        assert_eq!(results_sim[1].id, "c2");

        // If lambda is 0.5, diversity should kick in.
        let lambda = 0.5;
        let reranker = MmrReranker { lambda };

        // Pre-calculate similarities for verification before candidates are moved
        let sim_q_c1 = query.cosine_similarity(&c1.vector);
        let sim_q_c3 = query.cosine_similarity(&c3.vector);
        let sim_c3_c1 = c3.vector.cosine_similarity(&c1.vector);

        let results = reranker.rerank(&query, vec![c1, c2, c3], 2);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "c1");
        assert_eq!(results[1].id, "c3");

        // Verify score calculation: lambda * sim(query, c1) - (1-lambda) * 0
        let expected_score_0 = lambda * sim_q_c1;
        assert!((results[0].score - expected_score_0).abs() < 1e-6);

        // Verify score calculation: lambda * sim(query, c3) - (1-lambda) * sim(c3, c1)
        let expected_score_1 = lambda * sim_q_c3 - (1.0 - lambda) * sim_c3_c1;
        assert!((results[1].score - expected_score_1).abs() < 1e-6);
    }

    #[test]
    fn test_recency_reranker() {
        let query = HVec10240::zero();
        let c1 = create_candidate("old", 0.9, 10.0); // 10 days old
        let c2 = create_candidate("new", 0.8, 0.0); // 0 days old

        let reranker = RecencyDecayReranker {
            half_life_days: 5.0,
            blend: 0.5,
        };

        let results = reranker.rerank(&query, vec![c1, c2], 2);
        assert_eq!(results[0].id, "new");
    }

    #[test]
    fn test_parse_rerankers() {
        let rers = parse_rerankers("mmr:0.7,recency:30d:0.8").unwrap();
        assert_eq!(rers.len(), 2);
        assert_eq!(rers[0].name(), "mmr");
        assert_eq!(rers[1].name(), "recency");
    }

    #[test]
    #[cfg(feature = "rerank-cross")]
    fn test_parse_rerankers_windows_path() {
        let err = parse_rerankers(r"cross:C:\nonexistent\model.onnx").unwrap_err();
        if let csm_core::error::MemoryError::InvalidInput { reason, .. } = err {
            assert!(reason.contains(r"C:\nonexistent\model.onnx"));
        } else {
            panic!("Expected InvalidInput error with the full path");
        }
    }

    #[test]
    fn test_parse_rerankers_invalid_blend() {
        let err = parse_rerankers("recency:30d:not-a-number").unwrap_err();
        assert!(format!("{err}").contains("invalid recency blend"));
    }
}
