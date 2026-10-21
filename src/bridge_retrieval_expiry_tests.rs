//! TTL coverage for the concept-expansion step of [`BridgeRetrieval::query`].
//!
//! Split out of `bridge_retrieval_tests.rs`, which sits at the 500-LOC gate.
//! The expansion step keeps a label only when its concept exists in the
//! singularity *and* `expires_at.is_none_or(|e| e > now)` — the comparison is
//! strict, so a concept expiring in the current second is already gone. Kept
//! scores are the observable: an empty `expanded_labels` short-circuits the
//! whole scoring step, leaving every `scores.concept` at zero.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::bridge_retrieval::BridgeRetrieval;
use crate::concept_builder::ConceptBuilder;
use crate::semantic_bridge::{CanonicalConcept, ConceptGraph};
use crate::singularity::{Singularity, SingularityConfig};
use csm_core_lib::encoder::TextEncoder;
use csm_core_lib::hyperdim::HVec10240;

const NS: &str = "_default";

struct Harness {
    bridge: BridgeRetrieval,
    singularity: Singularity<HVec10240>,
}

/// `hub` expands to `labels`; the singularity holds a primary anchor plus one
/// concept per label, each carrying `ttl_seconds` (0 expires in this second).
fn harness(labels: &[&str], ttl_seconds: u64) -> Harness {
    let encoder = TextEncoder::new();
    let mut graph = ConceptGraph::new();
    let mut canonical = CanonicalConcept::new("hub");
    for label in labels {
        canonical = canonical.with_label(*label);
    }
    graph.add_concept(canonical);

    let mut singularity = Singularity::<HVec10240>::new(SingularityConfig::default());
    singularity
        .inject(
            NS,
            ConceptBuilder::new("mem-1")
                .with_vector(encoder.encode("alpha gamma hub"))
                .build()
                .unwrap(),
        )
        .unwrap();
    for label in labels {
        singularity
            .inject(
                NS,
                ConceptBuilder::new(*label)
                    .with_vector(encoder.encode(label))
                    .with_ttl(ttl_seconds)
                    .build()
                    .unwrap(),
            )
            .unwrap();
    }

    Harness {
        bridge: BridgeRetrieval::with_defaults(encoder, graph),
        singularity,
    }
}

fn hits(harness: &Harness) -> Vec<(String, f32)> {
    harness
        .bridge
        .query(NS, &harness.singularity, "alpha gamma", 10, None)
        .unwrap()
        .iter()
        .map(|hit| (hit.id.clone(), hit.scores.concept))
        .collect()
}

#[test]
fn expansion_scores_labels_that_have_not_expired() {
    let harness = harness(&["alpha", "gamma"], 3600);
    let hits = hits(&harness);

    assert!(!hits.is_empty(), "the anchor must still be recalled");
    assert!(
        hits.iter().any(|(_, concept)| *concept > 0.0),
        "unexpired expansion labels must contribute a concept score, got {hits:?}"
    );
}

#[test]
fn expansion_drops_labels_expiring_in_the_current_second() {
    // Built back-to-back with the query inside `hits`, so `expires_at` resolves
    // to the unix second that the query reads as `now`.
    let harness = harness(&["beta"], 0);
    assert!(
        harness.singularity.get(NS, "beta").is_some(),
        "precondition: the expiring concept is still stored, so the recall below \
         must be decided by the expiry check alone"
    );

    let hits = hits(&harness);
    assert!(!hits.is_empty(), "the anchor must still be recalled");
    // An expired label never reaches the candidate list, so it cannot appear as
    // a hit at all: primary recall filters it independently of this step.
    assert!(
        hits.iter().all(|(id, _)| id != "beta"),
        "`expires > now` is strict: a label expiring in this second must not be \
         recalled even though its concept is still stored, got {hits:?}"
    );
    assert!(
        hits.iter().all(|(_, concept)| *concept == 0.0),
        "with every label dropped there is nothing to expand, got {hits:?}"
    );
}

/// The strict comparison only differs from `>=` during the single second where
/// `expires == now`, and a TTL built back-to-back with the query usually lands
/// *after* it. Aligning the build to a fresh second and then sampling every
/// query provably inside that second makes the boundary observable.
///
/// The sampled labels must be ones the anchor resembles: an expansion score is
/// min-max normalised, so a single unrelated label leaves every existing hit at
/// zero and a retained-label mutant would be invisible.
#[cfg(not(miri))]
fn aligned_harness(labels: &[&str]) -> Option<(Harness, u64)> {
    let clock = crate::singularity::unix_now_secs;
    for _ in 0..5 {
        let tick = clock();
        while clock() == tick {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let harness = harness(labels, 0);
        let second = clock();
        if labels.iter().all(|label| {
            harness
                .singularity
                .get(NS, label)
                .and_then(|concept| concept.expires_at)
                == Some(second)
        }) {
            return Some((harness, second));
        }
    }
    None
}

// Miri simulates the clock: `sleep` does not move `SystemTime::now()` the way
// wall time does, so `aligned_harness` can never land on the expiry second
// there. The case is about a real clock crossing a one-second boundary, so it
// is skipped rather than faked.
#[cfg(not(miri))]
#[test]
fn expansion_drops_labels_at_the_exact_expiry_second() {
    let Some((harness, expires)) = aligned_harness(&["alpha", "gamma"]) else {
        panic!("could not resolve every ttl-0 expiry onto the second the harness was built in");
    };

    let clock = crate::singularity::unix_now_secs;
    let mut sampled = 0_usize;
    while clock() == expires {
        let before = clock();
        let hits = hits(&harness);
        let after = clock();
        if before == expires && after == expires {
            assert!(
                !hits.is_empty(),
                "the anchor must still be recalled inside the expiry second"
            );
            assert!(
                hits.iter().all(|(_, concept)| *concept == 0.0),
                "`expires > now` is false when `expires == now`, so nothing may expand \
                 into a concept score, got {hits:?}"
            );
            sampled += 1;
        }
    }

    assert!(
        sampled > 0,
        "precondition: at least one query must run fully inside the expiry second"
    );
}
