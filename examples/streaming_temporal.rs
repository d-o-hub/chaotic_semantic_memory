//! Temporal sequence processing with the chaotic reservoir

// Casts are intentional for demonstration math
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use chaotic_semantic_memory::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("⏱️  Streaming Temporal Processing\n");

    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_reservoir_size(10240)
        .with_reservoir_input_size(64)
        .with_chaos_strength(0.05)
        .build()
        .await?;

    // Generate 3 synthetic time-series sequences (5 steps each, 64-dim input)
    let sequences: Vec<(&str, Vec<Vec<f32>>)> = vec![
        (
            "sine-wave",
            (0..5)
                .map(|t| {
                    (0..64)
                        .map(|i| ((t as f32 + i as f32) * 0.1).sin())
                        .collect()
                })
                .collect(),
        ),
        (
            "cosine-wave",
            (0..5)
                .map(|t| {
                    (0..64)
                        .map(|i| ((t as f32 + i as f32) * 0.1).cos())
                        .collect()
                })
                .collect(),
        ),
        (
            "sawtooth",
            (0..5)
                .map(|t| {
                    (0..64)
                        .map(|i| ((t as f32 + i as f32) * 0.05) % 1.0)
                        .collect()
                })
                .collect(),
        ),
    ];

    // Process each sequence through the reservoir and inject as a concept
    for (name, sequence) in &sequences {
        let hvec = framework.process_sequence(sequence).await?;
        framework.inject_concept(*name, hvec).await?;
        println!(
            "  ✅ Processed '{name}' ({} steps × {} dims)",
            sequence.len(),
            sequence[0].len()
        );
    }

    // Generate a query sequence similar to sine-wave (shifted slightly)
    let query_sequence: Vec<Vec<f32>> = (0..5)
        .map(|t| {
            (0..64)
                .map(|i| ((t as f32 + i as f32) * 0.1 + 0.01).sin())
                .collect()
        })
        .collect();

    let query_hvec = framework.process_sequence(&query_sequence).await?;
    println!("\n🔍 Query: slightly shifted sine wave");

    // Probe for similar temporal patterns
    let hits = framework.probe(query_hvec, 3).await?;
    println!("\n📊 Temporal similarity results:");
    for (id, score) in &hits {
        println!("   {id}: {score:.4}");
    }

    // Show metrics
    let snapshot = framework.metrics_snapshot().await;
    println!("\n📈 Metrics:");
    println!("   Concepts injected: {}", snapshot.concepts_injected_total);
    println!("   Reservoir steps: {}", snapshot.reservoir_steps_total);
    println!("   Probes executed: {}", snapshot.probes_total);

    println!("\n✅ Temporal processing demo complete!");
    Ok(())
}
