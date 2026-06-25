#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! Dynamic query modification for binary LSH (arXiv:2605.23807).
//!
//! Implements a 3-phase search strategy:
//! 1. **Pre-screen** — standard LSH retrieval with the original query.
//! 2. **Center estimation** — compute an optimal center from candidate neighbors.
//! 3. **Re-retrieve** — search again using the modified (centered) query for higher recall.

use std::collections::HashMap;

use csm_core::hyperdim::HVec10240;

/// Estimate an optimal query center from candidate neighbor vectors.
///
/// Uses majority-vote bundling: for each bit position, set the output bit
/// if the majority of candidates have that bit set.  When `candidates` is
/// empty the original query is returned unchanged.
///
/// # Arguments
/// * `query` — the original query vector (fallback when candidates are empty).
/// * `candidates` — iterator over candidate neighbor vectors.
#[inline]
pub fn estimate_query_center<'a>(
    query: &HVec10240,
    candidates: impl Iterator<Item = &'a HVec10240>,
) -> HVec10240 {
    let (vecs, _): (Vec<&HVec10240>, _) =
        candidates.fold((Vec::new(), 0usize), |(mut acc, count), v| {
            acc.push(v);
            (acc, count + 1)
        });

    if vecs.is_empty() {
        return *query;
    }

    // Majority-vote bundling across all candidate words.
    let mut center = HVec10240::zero();
    let half = vecs.len() / 2;

    for word_idx in 0..HVec10240::WORDS {
        let mut acc = [0u32; 128];
        for v in &vecs {
            let bits = v.data[word_idx];
            for (bit, count) in acc.iter_mut().enumerate() {
                if (bits >> bit) & 1 == 1 {
                    *count += 1;
                }
            }
        }
        let mut word = 0u128;
        for (bit, &count) in acc.iter().enumerate() {
            if count > half as u32 {
                word |= 1u128 << bit;
            }
        }
        center.data[word_idx] = word;
    }

    center
}

/// Compute a u64 hash from bit-sampling projections (standalone, inlined).
///
/// Mirrors `LshIndex::compute_hash` but takes explicit parameters for use
/// outside the index struct.
///
/// # Arguments
/// * `vec` — the hypervector to hash.
/// * `projections` — bit positions to sample.
#[inline]
pub fn compute_hash_bits(vec: &HVec10240, projections: &[usize]) -> u64 {
    let bytes = vec.to_bytes();
    let mut hash = 0u64;
    for (i, &bit_pos) in projections.iter().enumerate() {
        let byte_idx = bit_pos / 8;
        let bit_idx = bit_pos % 8;
        if byte_idx < bytes.len() && (bytes[byte_idx] & (1 << bit_idx)) != 0 {
            hash |= 1u64 << i;
        }
    }
    hash
}

/// 3-phase dynamic query modification search.
///
/// Phase 1 — **Pre-screen**: retrieve candidates with the original query.
/// Phase 2 — **Center estimation**: compute the majority-vote center of the
///            top candidates from phase 1.
/// Phase 3 — **Re-retrieve**: search all tables using the centered query to
///            improve recall.
///
/// Returns up to `k` results sorted by descending similarity score.
///
/// # Arguments
/// * `query` — the original query vector.
/// * `tables` — the LSH hash tables (table_idx → hash → bucket ids).
/// * `projections` — bit-sampling projections per table.
/// * `concepts` — id → hypervector mapping.
/// * `k` — number of results to return.
#[inline]
pub fn modified_query_search(
    query: &HVec10240,
    tables: &[HashMap<u64, Vec<String>>],
    projections: &[Vec<usize>],
    concepts: &HashMap<String, HVec10240>,
    k: usize,
) -> Vec<(String, f32)> {
    if k == 0 || concepts.is_empty() || tables.is_empty() {
        return Vec::new();
    }

    let num_tables = tables.len();

    // ── Phase 1: pre-screen ──────────────────────────────────────────
    let mut pre_candidates: HashMap<&str, ()> = HashMap::new();
    for i in 0..num_tables {
        let hash = compute_hash_bits(query, &projections[i]);
        if let Some(bucket) = tables[i].get(&hash) {
            for id in bucket {
                pre_candidates.insert(id, ());
            }
        }
    }

    if pre_candidates.is_empty() {
        // No LSH hits; fall through to a direct scan of all concepts.
        return fallback_full_scan(query, concepts, k);
    }

    // Rank pre-screen candidates by Hamming distance and take the top
    // `pre_k` neighbours to build the center estimate.
    let pre_k = (k * 2).max(8);
    let mut ranked: Vec<(&str, u32)> = pre_candidates
        .keys()
        .filter_map(|id| concepts.get(*id).map(|v| (*id, query.hamming_distance(v))))
        .collect();
    ranked.sort_unstable_by_key(|&(_, dist)| dist);
    ranked.truncate(pre_k);

    // ── Phase 2: center estimation ───────────────────────────────────
    let neighbor_vecs: Vec<&HVec10240> = ranked
        .iter()
        .filter_map(|(id, _)| concepts.get(*id))
        .collect();
    let centered = estimate_query_center(query, neighbor_vecs.into_iter());

    // ── Phase 3: re-retrieve with centered query ─────────────────────
    let mut candidates: HashMap<&str, ()> = HashMap::new();
    for i in 0..num_tables {
        let hash = compute_hash_bits(&centered, &projections[i]);
        if let Some(bucket) = tables[i].get(&hash) {
            for id in bucket {
                candidates.entry(id).or_insert(());
            }
        }
    }

    // Also include the original pre-screen hits so we never lose recall.
    for &id in pre_candidates.keys() {
        candidates.entry(id).or_insert(());
    }

    let mut scores: Vec<(&str, u32)> = candidates
        .keys()
        .filter_map(|id| concepts.get(*id).map(|v| (*id, query.hamming_distance(v))))
        .collect();

    // Partial select to avoid full sort on large candidate sets.
    if scores.len() <= k {
        scores.sort_unstable_by_key(|&(_, dist)| dist);
    } else {
        scores.select_nth_unstable_by(k - 1, |a, b| a.1.cmp(&b.1));
        scores.truncate(k);
        scores.sort_unstable_by_key(|&(_, dist)| dist);
    }

    scores
        .into_iter()
        .map(|(id, dist)| (id.to_string(), 1.0 - (dist as f32 / 5120.0)))
        .collect()
}

/// Fallback full-scan when LSH buckets yield no candidates.
///
/// Scores every concept by Hamming distance and returns the top `k`.
#[inline]
fn fallback_full_scan(
    query: &HVec10240,
    concepts: &HashMap<String, HVec10240>,
    k: usize,
) -> Vec<(String, f32)> {
    let mut scores: Vec<(&str, u32)> = concepts
        .iter()
        .map(|(id, v)| (id.as_str(), query.hamming_distance(v)))
        .collect();

    if scores.len() <= k {
        scores.sort_unstable_by_key(|&(_, dist)| dist);
    } else {
        scores.select_nth_unstable_by(k - 1, |a, b| a.1.cmp(&b.1));
        scores.truncate(k);
        scores.sort_unstable_by_key(|&(_, dist)| dist);
    }

    scores
        .into_iter()
        .map(|(id, dist)| (id.to_string(), 1.0 - (dist as f32 / 5120.0)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a minimal in-memory LSH store (tables + projections
    /// + concepts) without a full `LshIndex` for unit testing.
    fn mini_store(
        n: usize,
        hash_bits: usize,
    ) -> (
        Vec<HashMap<u64, Vec<String>>>,
        Vec<Vec<usize>>,
        HashMap<String, HVec10240>,
    ) {
        use rand::RngExt;
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let mut projections = Vec::with_capacity(n);
        let mut tables: Vec<HashMap<u64, Vec<String>>> = Vec::with_capacity(n);
        let mut concepts = HashMap::new();

        for _ in 0..n {
            let bits: Vec<usize> = (0..hash_bits)
                .map(|_| rng.random_range(0..HVec10240::DIMENSION))
                .collect();
            projections.push(bits);
            tables.push(HashMap::new());
        }

        // Insert deterministic vectors so results are reproducible.
        for idx in 0..20 {
            let v = HVec10240::new_seeded(idx as u64);
            let id = format!("c{idx}");
            for (t_idx, table) in tables.iter_mut().enumerate() {
                let hash = compute_hash_bits(&v, &projections[t_idx]);
                table.entry(hash).or_default().push(id.clone());
            }
            concepts.insert(id, v);
        }

        (tables, projections, concepts)
    }

    #[test]
    fn estimate_query_center_single_candidate_returns_that_candidate() {
        let query = HVec10240::zero();
        let v = HVec10240::new_seeded(99);
        let center = estimate_query_center(&query, std::iter::once(&v));
        assert_eq!(center, v, "single candidate center should equal candidate");
    }

    #[test]
    fn estimate_query_center_empty_candidates_returns_query() {
        let query = HVec10240::new_seeded(7);
        let center = estimate_query_center(&query, std::iter::empty());
        assert_eq!(
            center, query,
            "empty candidates should return original query"
        );
    }

    #[test]
    fn estimate_query_center_two_identical_candidates_returns_that_vector() {
        let query = HVec10240::zero();
        let v = HVec10240::new_seeded(42);
        let center = estimate_query_center(&query, vec![&v, &v].into_iter());
        assert_eq!(center, v);
    }

    #[test]
    fn compute_hash_bits_deterministic() {
        let v = HVec10240::new_seeded(10);
        let proj = vec![0, 5, 100, 10239];
        let h1 = compute_hash_bits(&v, &proj);
        let h2 = compute_hash_bits(&v, &proj);
        assert_eq!(h1, h2, "same input must produce same hash");
    }

    #[test]
    fn compute_hash_bits_empty_projections_returns_zero() {
        let v = HVec10240::new_seeded(3);
        let h = compute_hash_bits(&v, &[]);
        assert_eq!(h, 0, "empty projections must produce zero hash");
    }

    #[test]
    fn modified_query_search_returns_up_to_k_results() {
        let (tables, projections, concepts) = mini_store(4, 12);
        let query = HVec10240::new_seeded(5);
        let results = modified_query_search(&query, &tables, &projections, &concepts, 5);
        assert!(
            results.len() <= 5,
            "must return at most k results, got {}",
            results.len()
        );
    }

    #[test]
    fn modified_query_search_scores_are_in_range() {
        let (tables, projections, concepts) = mini_store(4, 12);
        let query = HVec10240::new_seeded(5);
        let results = modified_query_search(&query, &tables, &projections, &concepts, 10);
        for (id, score) in &results {
            assert!(
                (-0.01..=1.01).contains(score),
                "score for {id} out of range: {score}"
            );
        }
    }

    #[test]
    fn modified_query_search_empty_index_returns_empty() {
        let tables: Vec<HashMap<u64, Vec<String>>> = vec![HashMap::new()];
        let projections = vec![vec![0, 1]];
        let concepts = HashMap::new();
        let query = HVec10240::new_seeded(1);
        let results = modified_query_search(&query, &tables, &projections, &concepts, 5);
        assert!(results.is_empty(), "empty index must return no results");
    }

    #[test]
    fn modified_query_search_scores_descending() {
        let (tables, projections, concepts) = mini_store(4, 12);
        let query = HVec10240::new_seeded(5);
        let results = modified_query_search(&query, &tables, &projections, &concepts, 10);
        for pair in results.windows(2) {
            assert!(
                pair[0].1 >= pair[1].1,
                "results must be descending by score"
            );
        }
    }
}
