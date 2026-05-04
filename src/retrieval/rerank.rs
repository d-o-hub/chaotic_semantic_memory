use crate::hyperdim::HVec10240;
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
pub trait Reranker: Send + Sync {
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
        _query: &HVec10240,
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
                let mmr_score =
                    self.lambda * cand.score - (1.0 - self.lambda) * max_sim_to_selected;
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

        for cand in &mut candidates {
            #[allow(clippy::cast_precision_loss)]
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
pub fn parse_rerankers(s: &str) -> crate::error::Result<Vec<Box<dyn Reranker>>> {
    let mut rerankers: Vec<Box<dyn Reranker>> = Vec::new();
    for part in s.split(',') {
        if part.is_empty() {
            continue;
        }
        let mut split = part.split(':');
        let name = split.next().unwrap_or("");
        let value = split.next().unwrap_or("");

        match name {
            "mmr" => {
                let lambda =
                    value
                        .parse::<f32>()
                        .map_err(|_| crate::error::MemoryError::InvalidInput {
                            field: "rerank".to_string(),
                            reason: format!("invalid MMR lambda: {}", value),
                        })?;
                rerankers.push(Box::new(MmrReranker { lambda }));
            }
            "recency" => {
                let val_str = if let Some(stripped) = value.strip_suffix('d') {
                    stripped
                } else {
                    value
                };
                let half_life = val_str.parse::<f32>().map_err(|_| {
                    crate::error::MemoryError::InvalidInput {
                        field: "rerank".to_string(),
                        reason: format!("invalid recency half-life: {}", value),
                    }
                })?;
                let blend = split.next().and_then(|b| b.parse::<f32>().ok()).unwrap_or(0.5);
                rerankers.push(Box::new(RecencyDecayReranker {
                    half_life_days: half_life,
                    blend,
                }));
            }
            #[cfg(feature = "rerank-cross")]
            "cross" => {
                let model = candle_onnx::read_file(value).map_err(|e| {
                    crate::error::MemoryError::InvalidInput {
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
                return Err(crate::error::MemoryError::InvalidInput {
                    field: "rerank".to_string(),
                    reason: format!("unknown reranker: {}", name),
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
        let now = crate::singularity::unix_now_secs();
        #[allow(clippy::cast_possible_truncation)]
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
        let query = HVec10240::random();
        let v1 = Arc::new(HVec10240::random());
        // v2 is very similar to v1
        let v2 = Arc::new(*v1.as_ref());

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
            vector: Arc::new(HVec10240::random()),
            metadata: HashMap::new(),
            score: 0.7,
            created_at_unix: 0,
        };

        let reranker = MmrReranker { lambda: 0.5 };
        let results = reranker.rerank(&query, vec![c1, c2, c3], 2);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "c1");
        // c2 should be penalized for being identical to c1, so c3 should be preferred
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
}
