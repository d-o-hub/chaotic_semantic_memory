//! Proof of concept: Build working binary demonstrating all features

// Casts are intentional for demonstration math
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use std::io::Write;

use chaotic_semantic_memory::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧠 Chaotic Semantic Memory - Proof of Concept\n");

    // 1. Initialize with libSQL
    let framework = ChaoticSemanticFramework::builder()
        .with_reservoir_size(50000)
        .with_local_db("csm_memory.db")
        .build()
        .await?;

    println!("✅ Framework initialized");

    // 2. Inject concepts
    let concept_vec = HVec10240::random();
    framework
        .inject_concept("rust-programming", concept_vec)
        .await?;
    println!("✅ Concept injected: rust-programming");

    // Inject a few more
    framework
        .inject_concept("python-programming", HVec10240::random())
        .await?;
    framework
        .inject_concept("javascript-programming", HVec10240::random())
        .await?;
    framework
        .inject_concept("memory-systems", HVec10240::random())
        .await?;
    println!("✅ Additional concepts injected");

    // 3. Query
    let results = framework.probe(concept_vec, 3).await?;
    println!("✅ Query completed:");
    for (id, score) in &results {
        println!("   - {}: {:.4}", id, score);
    }

    // 4. Create associations
    framework
        .associate("rust-programming", "memory-systems", 0.9)
        .await?;
    framework
        .associate("python-programming", "rust-programming", 0.7)
        .await?;
    println!("✅ Associations created");

    // 5. Get associations
    let associations = framework.get_associations("rust-programming").await?;
    println!("✅ Associations for 'rust-programming':");
    for (id, strength) in associations {
        println!("   - {}: {:.2}", id, strength);
    }

    // 6. Persist
    framework.persist().await?;
    println!("✅ Persisted to database");

    // 7. Get stats
    let stats = framework.stats().await?;
    println!("\n📊 Framework Statistics:");
    println!("   - Concepts: {}", stats.concept_count);
    if let Some(size) = stats.db_size_bytes {
        println!("   - Database size: {} bytes", size);
    } else {
        println!("   - Database size: N/A (persistence disabled)");
    }

    // 8. Benchmark
    println!("\n⚡ Running benchmark...");
    let start = std::time::Instant::now();
    for i in 0..1000 {
        framework.probe(HVec10240::random(), 5).await?;
        if i % 100 == 0 {
            print!(".");
            std::io::stdout().flush()?;
        }
    }
    let elapsed = start.elapsed();
    println!(" Done!");
    println!(
        "✅ 1000 queries in {:?} ({:.2}μs avg)",
        elapsed,
        elapsed.as_micros() as f64 / 1000.0
    );

    println!("\n🎉 All systems operational!");
    Ok(())
}
