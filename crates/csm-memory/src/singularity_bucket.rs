//! Bucketed candidate generation for reduced-candidate retrieval (ADR-0065).
//!
//! Split out of `singularity_retrieval.rs` at the 500-LOC gate (AGENTS.md:
//! child module extraction over comment stripping).

use crate::singularity::Singularity;
use crate::singularity_retrieval::MAX_BUCKET_PROBE_WIDTH;
use csm_core_lib::hyperdim::HVec10240;

#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
use rayon::prelude::*;

/// Extra candidates a caller will tolerate above the configured budget before
/// the bucketed path declines to produce a set at all.
pub(crate) const BUCKET_BUDGET_SLACK: usize = 2;

/// Probe width that keeps one bucket within `budget` candidates on a corpus of
/// `corpus_size`, never below the configured width and never above
/// [`MAX_BUCKET_PROBE_WIDTH`].
///
/// The configured `bucket_probe_width` is a floor: a fixed mask covers a
/// shrinking share of the space as the corpus grows, which is what made
/// recall@10 collapse from 0.604 at 10k to 0.208 at 200k with a fixed width of 8
/// (`plans/evidence/scale_release_2026_09_21/ann_scale.json`).
fn effective_bucket_probe_width(configured: usize, corpus_size: usize, budget: usize) -> usize {
    let configured = configured.min(MAX_BUCKET_PROBE_WIDTH);
    if budget == 0 || corpus_size <= budget {
        return configured;
    }
    // ceil(log2(corpus_size / budget)) in integer arithmetic: the number of bits
    // needed to represent (ratio - 1). Avoids float casts, which the previous
    // location suppressed file-wide.
    let ratio = corpus_size.div_ceil(budget).max(1);
    let needed = (usize::BITS - (ratio - 1).leading_zeros()) as usize;
    configured.max(needed).min(MAX_BUCKET_PROBE_WIDTH)
}

impl Singularity {
    /// Generate candidates by coarse bucketing (multi-probe).
    ///
    /// The probe width scales with the corpus so one bucket stays within
    /// `max_candidates`; candidates within one bit of the query bucket are
    /// accepted as well, which recovers the neighbours a single bucket loses
    /// (`effective_bucket_probe_width` documents the measurement). When the
    /// probe still exceeds `max_candidates * BUCKET_BUDGET_SLACK` the generator
    /// returns nothing: an arbitrary index-ordered slice of an oversized bucket
    /// is what dropped recall@10 to 0.016, so the caller's exact-scan fallback
    /// is the honest answer.
    pub(crate) fn generate_bucket_candidates(&self, ns: &str, query: &HVec10240) -> Vec<usize> {
        let Some(ns_state) = self.get_namespace(ns) else {
            return Vec::new();
        };
        debug_assert!(self._retrieval_config.bucket_probe_width <= 127);

        let budget = self._retrieval_config.max_candidates.max(1);
        let width = effective_bucket_probe_width(
            self._retrieval_config.bucket_probe_width,
            ns_state.concept_vectors.len(),
            budget,
        );
        let bucket_mask = (1u128 << width) - 1;
        let query_bucket = query.data[0] & bucket_mask;

        // Multi-probe: distance <= 1 inside the masked bits.
        let filter = |(idx, vec): (usize, &HVec10240)| {
            let distance = (vec.data[0] & bucket_mask ^ query_bucket).count_ones();
            if distance <= 1 { Some(idx) } else { None }
        };

        // Algorithmic Optimization: Parallelize O(N) candidate generation via Rayon.
        // Reduces latency from O(N) to O(N/P) where P is the number of execution units.
        #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
        let res: Vec<usize> = ns_state
            .concept_vectors
            .par_iter()
            .enumerate()
            .filter_map(filter)
            .collect();

        #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
        let res: Vec<usize> = ns_state
            .concept_vectors
            .iter()
            .enumerate()
            .filter_map(filter)
            .collect();

        if res.len() > budget.saturating_mul(BUCKET_BUDGET_SLACK) {
            // Too many matches to be a *candidate* set: let the caller scan exactly.
            return Vec::new();
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_probe_width_scales_with_corpus_and_stays_bounded() {
        assert_eq!(effective_bucket_probe_width(2, 500, 1000), 2);
        assert_eq!(effective_bucket_probe_width(8, 1000, 1000), 8);
        assert_eq!(effective_bucket_probe_width(2, 4_000, 1000), 2);
        assert_eq!(effective_bucket_probe_width(2, 8_000, 1000), 3);
        assert_eq!(effective_bucket_probe_width(2, 200_000, 1000), 8);
        assert_eq!(effective_bucket_probe_width(2, 10_000_000, 1000), 14);
        assert_eq!(effective_bucket_probe_width(12, 4_000, 1000), 12);
        assert_eq!(
            effective_bucket_probe_width(2, usize::MAX / 2, 1),
            MAX_BUCKET_PROBE_WIDTH
        );
        assert_eq!(effective_bucket_probe_width(4, 10_000, 0), 4);
    }
}
