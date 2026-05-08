//! Document chunk storage for Retrieval-Augmented Generation (RAG)

use std::collections::HashMap;

use chaotic_semantic_memory::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("📄 Document RAG Storage\n");

    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await?;

    // Simulate document chunks from 3 documents
    let chunks: Vec<(&str, &str, usize, &str)> = vec![
        (
            "doc-rust",
            "rust-c0",
            0,
            "Rust is a systems programming language focused on safety.",
        ),
        (
            "doc-rust",
            "rust-c1",
            1,
            "The borrow checker enforces memory safety at compile time.",
        ),
        (
            "doc-rust",
            "rust-c2",
            2,
            "Cargo is the Rust package manager and build system.",
        ),
        (
            "doc-python",
            "py-c0",
            0,
            "Python is a high-level interpreted language.",
        ),
        (
            "doc-python",
            "py-c1",
            1,
            "Python uses garbage collection for memory management.",
        ),
        (
            "doc-ai",
            "ai-c0",
            0,
            "Neural networks learn patterns from training data.",
        ),
        (
            "doc-ai",
            "ai-c1",
            1,
            "Transformers use self-attention for sequence modeling.",
        ),
        (
            "doc-ai",
            "ai-c2",
            2,
            "RAG combines retrieval with generation for grounded answers.",
        ),
    ];

    for (doc_id, chunk_id, chunk_idx, text) in &chunks {
        let mut metadata = HashMap::new();
        metadata.insert("doc_id".to_string(), serde_json::json!(doc_id));
        metadata.insert("chunk_index".to_string(), serde_json::json!(chunk_idx));
        metadata.insert("text".to_string(), serde_json::json!(text));

        framework
            .inject_concept_with_metadata(*chunk_id, HVec10240::random(), metadata)
            .await?;
    }
    println!("  ✅ Ingested {} chunks from 3 documents\n", chunks.len());

    // Simulate a user query
    let query_vec = HVec10240::random();
    let all_hits = framework.probe(query_vec, 8).await?;

    println!("🔍 All results ranked by similarity:");
    for (id, score) in &all_hits {
        let concept = framework.get_concept(id).await?.unwrap();
        let doc_id = concept
            .metadata
            .get("doc_id")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let text = concept
            .metadata
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        println!("   [{doc_id}] {id} ({score:.4}): \"{text}\"");
    }

    // Filter results by doc_id (simulating scoped RAG)
    let target_doc = "doc-ai";
    println!("\n📎 Filtered to '{target_doc}' only:");
    for (id, score) in &all_hits {
        let concept = framework.get_concept(id).await?.unwrap();
        let doc_id = concept
            .metadata
            .get("doc_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if doc_id == target_doc {
            let text = concept
                .metadata
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            println!("   {id} ({score:.4}): \"{text}\"");
        }
    }

    let stats = framework.stats().await?;
    println!("\n📊 Store has {} chunks indexed", stats.concept_count);
    println!("✅ Document RAG demo complete!");
    Ok(())
}
