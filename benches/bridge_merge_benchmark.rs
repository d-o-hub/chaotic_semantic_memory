use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ScoreBreakdown {
    pub deterministic: f32,
    pub concept: f32,
    pub semantic: f32,
    pub final_score: f32,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BridgeHit {
    pub id: String,
    pub text_preview: Option<String>,
    pub scores: ScoreBreakdown,
}

fn merge_with_breakdown_original(
    primary: &[(String, f32)],
    expanded: &[(String, f32)],
) -> Vec<BridgeHit> {
    let mut hit_map: HashMap<String, BridgeHit> = HashMap::new();

    for (id, score) in primary {
        hit_map.insert(
            id.clone(),
            BridgeHit {
                id: id.clone(),
                text_preview: None,
                scores: ScoreBreakdown {
                    deterministic: *score,
                    concept: 0.0,
                    semantic: 0.0,
                    final_score: 0.0,
                    evidence: vec!["deterministic_recall".to_string()],
                },
            },
        );
    }

    for (id, score) in expanded {
        if let Some(hit) = hit_map.get_mut(id) {
            hit.scores.concept = hit.scores.concept.max(*score);
            hit.scores.evidence.push("concept_expansion".to_string());
        } else {
            hit_map.insert(
                id.clone(),
                BridgeHit {
                    id: id.clone(),
                    text_preview: None,
                    scores: ScoreBreakdown {
                        deterministic: 0.0,
                        concept: *score,
                        semantic: 0.0,
                        final_score: 0.0,
                        evidence: vec!["concept_expansion".to_string()],
                    },
                },
            );
        }
    }

    hit_map.into_values().collect()
}

fn merge_with_breakdown_optimized<'a>(
    primary: &'a [(String, f32)],
    expanded: &'a [(String, f32)],
) -> Vec<BridgeHit> {
    // We use &str as the key to avoid cloning the id strings for the map
    let mut hit_map: HashMap<&'a str, BridgeHit> = HashMap::with_capacity(primary.len());

    for (id, score) in primary {
        hit_map.insert(
            id.as_str(),
            BridgeHit {
                id: id.clone(),
                text_preview: None,
                scores: ScoreBreakdown {
                    deterministic: *score,
                    concept: 0.0,
                    semantic: 0.0,
                    final_score: 0.0,
                    evidence: vec!["deterministic_recall".to_string()],
                },
            },
        );
    }

    for (id, score) in expanded {
        if let Some(hit) = hit_map.get_mut(id.as_str()) {
            hit.scores.concept = hit.scores.concept.max(*score);
            hit.scores.evidence.push("concept_expansion".to_string());
        } else {
            hit_map.insert(
                id.as_str(),
                BridgeHit {
                    id: id.clone(), // We only clone here, when placing into the struct
                    text_preview: None,
                    scores: ScoreBreakdown {
                        deterministic: 0.0,
                        concept: *score,
                        semantic: 0.0,
                        final_score: 0.0,
                        evidence: vec!["concept_expansion".to_string()],
                    },
                },
            );
        }
    }

    hit_map.into_values().collect()
}

fn bench_merge(c: &mut Criterion) {
    let mut primary = Vec::new();
    let mut expanded = Vec::new();
    // Simulate realistic sizes
    for i in 0..100 {
        primary.push((format!("concept_{}", i), 0.5));
    }
    // Most expansions will likely hit new, but some might hit existing. Let's do 50 overlaps and 50 new.
    for i in 50..150 {
        expanded.push((format!("concept_{}", i), 0.5));
    }

    let mut group = c.benchmark_group("merge_with_breakdown");
    group.bench_function("original", |b| {
        b.iter(|| merge_with_breakdown_original(black_box(&primary), black_box(&expanded)))
    });
    group.bench_function("optimized", |b| {
        b.iter(|| merge_with_breakdown_optimized(black_box(&primary), black_box(&expanded)))
    });
    group.finish();
}

criterion_group!(benches, bench_merge);
criterion_main!(benches);
