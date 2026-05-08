//! Hybrid retrieval example using Semantic Bridge Layer (ADR-0061).
//!
//! Demonstrates probe_bridge_text for combining deterministic HDC recall
//! with concept graph expansion for improved semantic retrieval.

use chaotic_semantic_memory::bridge_retrieval::BridgeRetrieval;
use chaotic_semantic_memory::encoder::TextEncoder;
use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::semantic_bridge::{CanonicalConcept, ConceptGraph};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Bridge Hybrid Retrieval via Semantic Bridge Layer\n");

    // Build framework with persistence
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await?;

    // Create encoder for text-to-hypervector
    let encoder = TextEncoder::new();

    // Build concept graph with semantic relationships
    let mut graph = ConceptGraph::new();

    // Add canonical concepts with labels and relationships
    graph.add_concept(
        CanonicalConcept::new("concept.ai_memory")
            .with_label("agent memory")
            .with_label("cross-session context")
            .with_label("persistent memory")
            .with_related("concept.semantic_search"),
    );
    graph.add_concept(
        CanonicalConcept::new("concept.semantic_search")
            .with_label("similarity search")
            .with_label("vector retrieval")
            .with_related("concept.rag"),
    );
    graph.add_concept(
        CanonicalConcept::new("concept.rag")
            .with_label("retrieval augmented generation")
            .with_label("knowledge retrieval"),
    );

    println!("Concept graph: {} concepts loaded", graph.concept_count());

    // Inject memories with various IDs
    let memories = vec![
        (
            "mem-001",
            "The AI agent maintains cross-session context for coherent responses.",
        ),
        (
            "mem-002",
            "Semantic search finds similar vectors using cosine distance.",
        ),
        (
            "mem-003",
            "RAG combines retrieval with generation for factual responses.",
        ),
        (
            "mem-004",
            "Persistent memory enables long-term agent learning.",
        ),
        (
            "mem-005",
            "Vector retrieval uses hypervector encoding for semantic matching.",
        ),
    ];

    for (id, text) in &memories {
        framework.inject_text(id, text).await?;
        println!("Injected: {id} -> {text}");
    }

    // Create bridge retrieval pipeline
    let bridge = BridgeRetrieval::with_defaults(encoder, graph);

    // Query 1: Direct term match
    println!("\n--- Query 1: 'agent memory' ---");
    let results = framework
        .probe_bridge_text("agent memory", 3, &bridge)
        .await?;
    println!("Results: {} hits", results.len());
    for hit in &results {
        println!(
            "  {} | det={:.3} concept={:.3} final={:.3}",
            hit.id, hit.scores.deterministic, hit.scores.concept, hit.scores.semantic
        );
    }

    // Query 2: Semantic expansion (term not in index but related)
    println!("\n--- Query 2: 'AI context' ---");
    let results = framework
        .probe_bridge_text("AI context", 3, &bridge)
        .await?;
    println!("Results: {} hits", results.len());
    for hit in &results {
        println!(
            "  {} | det={:.3} concept={:.3} final={:.3}",
            hit.id, hit.scores.deterministic, hit.scores.concept, hit.scores.semantic
        );
    }

    // Query 3: Full semantic chain
    println!("\n--- Query 3: 'knowledge retrieval for generation' ---");
    let results = framework
        .probe_bridge_text("knowledge retrieval for generation", 5, &bridge)
        .await?;
    println!("Results: {} hits", results.len());
    for hit in &results {
        println!(
            "  {} | det={:.3} concept={:.3} final={:.3}",
            hit.id, hit.scores.deterministic, hit.scores.concept, hit.scores.semantic
        );
    }

    println!("\nHybrid retrieval complete");
    Ok(())
}
