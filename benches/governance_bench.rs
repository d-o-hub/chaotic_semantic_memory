use chaotic_semantic_memory::memory_governance::{DecayPolicy, GovernancePolicy};
use chaotic_semantic_memory::singularity::ConceptBuilder;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_governance(c: &mut Criterion) {
    let policy = GovernancePolicy {
        decay: Some(DecayPolicy::Exponential {
            half_life_secs: 100,
        }),
        ..Default::default()
    };

    let concept = ConceptBuilder::new("c1").build().unwrap();

    c.bench_function("apply_exponential_decay", |b| {
        b.iter(|| black_box(policy.apply(concept.clone(), concept.modified_at + 50)))
    });
}

criterion_group!(benches, bench_governance);
criterion_main!(benches);
