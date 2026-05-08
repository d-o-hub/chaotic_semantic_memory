//! Knowledge graph with entity associations and multi-hop traversal

use chaotic_semantic_memory::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🕸️  Knowledge Graph\n");

    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await?;

    // Inject entities as concepts
    let entities = ["rust", "programming", "memory", "ai", "python", "safety"];
    for entity in &entities {
        framework
            .inject_concept(*entity, HVec10240::random())
            .await?;
    }
    println!("  ✅ Injected {} entities\n", entities.len());

    // Create typed associations with varying strengths
    let edges: Vec<(&str, &str, f32)> = vec![
        ("rust", "programming", 0.95),   // Rust is a programming language
        ("rust", "safety", 0.90),        // Rust emphasizes safety
        ("rust", "memory", 0.85),        // Rust manages memory
        ("python", "programming", 0.95), // Python is a programming language
        ("python", "ai", 0.80),          // Python is used in AI
        ("ai", "memory", 0.70),          // AI uses memory systems
        ("safety", "memory", 0.75),      // Safety relates to memory
    ];

    for (from, to, strength) in &edges {
        framework.associate(from, to, *strength).await?;
        println!("  🔗 {from} → {to} ({strength:.2})");
    }

    // Print full graph structure
    println!("\n📊 Graph Structure:");
    for entity in &entities {
        let neighbors = framework.get_associations(entity).await?;
        if neighbors.is_empty() {
            println!("  {entity}: (no outbound edges)");
        } else {
            let neighbor_str: Vec<String> = neighbors
                .iter()
                .map(|(id, s)| format!("{id}({s:.2})"))
                .collect();
            println!("  {entity}: {}", neighbor_str.join(", "));
        }
    }

    // Multi-hop traversal: neighbors of neighbors (2-hop)
    let start = "rust";
    println!("\n🚀 2-hop traversal from '{start}':");
    let hop1 = framework.get_associations(start).await?;
    println!(
        "  Hop 1: {:?}",
        hop1.iter().map(|(id, _)| id).collect::<Vec<_>>()
    );

    let mut hop2_set: Vec<(String, String, f32)> = Vec::new();
    for (neighbor, _) in &hop1 {
        let hop2 = framework.get_associations(neighbor).await?;
        for (target, strength) in &hop2 {
            if target != start && !hop1.iter().any(|(n, _)| n == target) {
                hop2_set.push((neighbor.clone(), target.clone(), *strength));
            }
        }
    }

    if hop2_set.is_empty() {
        println!("  Hop 2: (no new entities reached)");
    } else {
        for (via, target, strength) in &hop2_set {
            println!("  Hop 2: {start} → {via} → {target} ({strength:.2})");
        }
    }

    println!("\n✅ Knowledge graph demo complete!");
    Ok(())
}
