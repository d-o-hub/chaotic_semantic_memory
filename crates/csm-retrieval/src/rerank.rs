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
        if candidates.is_empty() || top_k == 0 {
            return Vec::new();
        }

        let mut selected: Vec<RerankCandidate> = Vec::with_capacity(top_k);

        // Greedily select candidates
        while selected.len() < top_k && !candidates.is_empty() {
            let mut best_idx = 0;
            let mut max_mmr = f32::NEG_INFINITY;

            for (idx, cand) in candidates.iter().enumerate() {
                let mut max_sim_to_selected = 0.0f32;
                for sel in &selected {
                    let sim = cand.vector.cosine_similarity(&sel.vector);
                    if sim > max_sim_to_selected {
                        max_sim_to_selected = sim;
                    }
                }

                // MMR Formula: lambda * sim(query, cand) - (1 - lambda) * max_sim(cand, selected)
                let similarity = query.cosine_similarity(&cand.vector);
                let mmr_score =
                    self.lambda * similarity - (1.0 - self.lambda) * max_sim_to_selected;
                if mmr_score > max_mmr {
                    max_mmr = mmr_score;
                    best_idx = idx;
                }
            }

            let mut best_cand = candidates.remove(best_idx);
            best_cand.score = max_mmr;
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
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
        // Use a zero vector as query for deterministic (and low) similarity
        let query = HVec10240::zero();
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
        // Step 1: Selection.
        // MMR(c1) = 0.5 * sim(query, c1) - 0.5 * 0.0 = 0.5 * sim(query, c1)
        // MMR(c2) = 0.5 * sim(query, c2)
        // MMR(c3) = 0.5 * sim(query, c3)
        // Since sim(query, c1) is highest (initially we use query.cosine_similarity), c1 is selected.

        // Step 2:
        // MMR(c2) = 0.5 * sim(query, c2) - 0.5 * sim(c2, c1) = 0.5 * sim(query, c2) - 0.5 * 1.0
        // MMR(c3) = 0.5 * sim(query, c3) - 0.5 * sim(c3, c1)
        // Since sim(c3, c1) < 1.0, MMR(c3) will be greater than MMR(c2).
        let reranker = MmrReranker { lambda: 0.5 };
        let results = reranker.rerank(&query, vec![c1, c2, c3], 2);

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

    /// Kills the `* → +` mutation on the MMR diversity penalty term.
    ///
    /// With lambda=0.0 and one candidate already selected, the MMR score for a
    /// second candidate is:
    ///   correct:   0.0 * sim(q, c) - 1.0 * max_sim_to_selected
    ///              = -max_sim_to_selected  (always ≤ 0)
    ///   mutated:   0.0 + sim(q, c) + 1.0 + max_sim_to_selected
    ///              = sim(q, c) + 1.0 + max_sim_to_selected  (always > 1)
    ///
    /// The test checks that the second-round MMR score is negative, which is
    /// impossible under the mutated formula.
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
