use crate::types::{CaseResult, QueryCase};

pub fn hit_at_k(case: &QueryCase, result: &CaseResult, k: usize) -> bool {
    result
        .retrieved
        .iter()
        .take(k)
        .any(|r| case.gold_evidence_ids.iter().any(|id| id == &r.memory_id))
}

pub fn reciprocal_rank(case: &QueryCase, result: &CaseResult) -> f32 {
    for item in &result.retrieved {
        if case.gold_evidence_ids.iter().any(|id| id == &item.memory_id) {
            return 1.0 / item.rank as f32;
        }
    }
    0.0
}
