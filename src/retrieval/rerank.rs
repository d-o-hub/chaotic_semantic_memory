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
            candidates.sort_by(|a, b| b.score.total_cmp(&a.score));
            candidates.truncate(top_k);
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
        let now = crate::singularity::unix_now_secs();
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
                        reason: format!("failed to load ONNX model {}: {}", value, e),
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
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use std::collections::HashMap;

    fn create_candidate(id: &str, score: f32, age_days: f32) -> RerankCandidate {
        let now = crate::singularity::unix_now_secs();
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
        let query = HVec10240::zero();
        let v1 = Arc::new(HVec10240::new_seeded(1));
        let v2 = Arc::new(HVec10240::new_seeded(1));
        let v3 = Arc::new(HVec10240::new_seeded(2));

        let c1 = RerankCandidate {
            id: "c1".into(),
            vector: v1,
            metadata: HashMap::new(),
            score: 0.9,
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

        let results_sim =
            MmrReranker { lambda: 1.0 }.rerank(&query, vec![c1.clone(), c2.clone(), c3.clone()], 2);
        assert_eq!(results_sim[0].id, "c1");
        assert_eq!(results_sim[1].id, "c2");
        // Assert exact scores under lambda = 1.0 (kills comparison operator mutations on the fast-path condition)
        assert!((results_sim[0].score - query.cosine_similarity(&c1.vector)).abs() < 1e-6);
        assert!((results_sim[1].score - query.cosine_similarity(&c2.vector)).abs() < 1e-6);

        let results = MmrReranker { lambda: 0.5 }.rerank(&query, vec![c1, c2, c3], 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "c1");
        assert_eq!(results[1].id, "c3");
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
        assert_eq!(results[1].id, "old");
        // Assert exact scores to prevent cargo-mutants bypass
        assert!((results[0].score - 0.9).abs() < 1e-6);
        assert!((results[1].score - 0.575).abs() < 1e-6);
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
        if let csm_core_lib::error::MemoryError::InvalidInput { reason, .. } = err {
            assert!(reason.contains(r"C:\nonexistent\model.onnx"));
        } else {
            panic!("Expected InvalidInput error with the full path");
        }
    }

    #[test]
    #[cfg(feature = "rerank-cross")]
    fn test_parse_rerankers_cross_unrecognized_path_errors() {
        // The "cross" arm should be reachable; a non-existent path must error
        // (not fall through to "unknown reranker")
        let err = parse_rerankers("cross:/tmp/nonexistent_model.onnx").unwrap_err();
        if let csm_core_lib::error::MemoryError::InvalidInput { reason, .. } = err {
            assert!(
                reason.contains("failed to load ONNX model"),
                "expected ONNX load error, got: {reason}"
            );
        } else {
            panic!("Expected InvalidInput, got: {err:?}");
        }
    }

    #[test]
    fn test_parse_rerankers_invalid_blend() {
        let err = parse_rerankers("recency:30d:not-a-number").unwrap_err();
        assert!(format!("{err}").contains("invalid recency blend"));
    }

    /// Kills the `|| → &&` mutation on the early-return guard.
    /// With `&&`, a non-empty candidates list + top_k=0 would NOT return early,
    /// causing a loop that never terminates (or panics). Must return empty vec.
    #[test]
    fn test_mmr_top_k_zero_returns_empty() {
        let query = HVec10240::zero();
        let c1 = RerankCandidate {
            id: "c1".into(),
            vector: Arc::new(HVec10240::new_seeded(1)),
            metadata: HashMap::new(),
            score: 0.9,
            created_at_unix: 0,
        };
        let reranker = MmrReranker { lambda: 0.5 };
        let results = reranker.rerank(&query, vec![c1], 0);
        assert!(
            results.is_empty(),
            "top_k=0 with non-empty candidates must return empty vec"
        );
    }

    /// Kills `* → +` mutation: with lambda=0.0, MMR score must be ≤ 0.
    #[test]
    fn test_mmr_lambda_zero_score_is_negative_after_first_selection() {
        let query = HVec10240::zero();
        // v1 and v2 are different seeded vectors (non-zero similarity to each other)
        let v1 = Arc::new(HVec10240::new_seeded(1));
        let v2 = Arc::new(HVec10240::new_seeded(2));

        // Sanity: v1 and v2 have non-trivial similarity to each other
        let sim_v1_v2 = v1.cosine_similarity(&v2);
        assert!(
            sim_v1_v2 > 0.0,
            "seeded vectors must have positive mutual similarity (got {sim_v1_v2})"
        );

        let c1 = RerankCandidate {
            id: "c1".into(),
            vector: v1,
            metadata: HashMap::new(),
            score: 0.9,
            created_at_unix: 0,
        };
        let c2 = RerankCandidate {
            id: "c2".into(),
            vector: v2,
            metadata: HashMap::new(),
            score: 0.8,
            created_at_unix: 0,
        };

        // lambda=0.0: pure diversity — score = 0 * sim(q,c) - 1 * max_sim_to_selected
        let reranker = MmrReranker { lambda: 0.0 };
        let results = reranker.rerank(&query, vec![c1, c2], 2);
        assert_eq!(results.len(), 2);

        // First selection has no prior selected set, so max_sim_to_selected = 0,
        // score = 0.0 * sim - 1.0 * 0.0 = 0.0.
        assert!(
            results[0].score <= 0.0,
            "lambda=0 first pick score must be <= 0, got {}",
            results[0].score
        );
        // Second selection has one prior, so max_sim_to_selected > 0,
        // score = -max_sim_to_selected < 0. Mutated formula gives > 1, catching the bug.
        assert!(
            results[1].score < 0.0,
            "lambda=0 second pick score must be < 0 (penalty for similarity to selected), got {}",
            results[1].score
        );
    }
}
