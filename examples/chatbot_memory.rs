//! Chatbot session memory: store and retrieve conversation messages

use std::collections::HashMap;

use chaotic_semantic_memory::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("💬 Chatbot Session Memory\n");

    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await?;

    // Simulate a conversation with role-tagged messages
    let messages = [
        ("msg-1", "user", "How does Rust handle memory safety?"),
        (
            "msg-2",
            "assistant",
            "Rust uses ownership and borrowing to guarantee memory safety at compile time.",
        ),
        ("msg-3", "user", "What about concurrency?"),
        (
            "msg-4",
            "assistant",
            "Rust prevents data races through its type system using Send and Sync traits.",
        ),
        ("msg-5", "user", "Can I use async/await in Rust?"),
        (
            "msg-6",
            "assistant",
            "Yes, Rust supports async/await with runtimes like Tokio for concurrent I/O.",
        ),
    ];

    let session_id = "session-abc123";

    for (i, (id, role, text)) in messages.iter().enumerate() {
        let mut metadata = HashMap::new();
        metadata.insert("role".to_string(), serde_json::json!(role));
        metadata.insert("session_id".to_string(), serde_json::json!(session_id));
        metadata.insert("turn".to_string(), serde_json::json!(i + 1));
        metadata.insert("text".to_string(), serde_json::json!(text));

        // Each message gets a unique random vector (in production, use an embedding model)
        framework
            .inject_concept_with_metadata(*id, HVec10240::random(), metadata)
            .await?;
        println!("  ✅ Stored {id} [{role}]: \"{text}\"");
    }

    // Associate question-answer pairs
    framework.associate("msg-1", "msg-2", 0.95).await?;
    framework.associate("msg-3", "msg-4", 0.95).await?;
    framework.associate("msg-5", "msg-6", 0.95).await?;
    // Topic chain: memory safety → concurrency → async
    framework.associate("msg-2", "msg-3", 0.7).await?;
    framework.associate("msg-4", "msg-5", 0.7).await?;
    println!("\n🔗 Associations created between Q&A pairs\n");

    // Query for similar messages
    let query = HVec10240::random();
    let hits = framework.probe(&query, 3).await?;
    println!("🔍 Top 3 messages by similarity:");
    for (id, score) in &hits {
        let concept = framework.get_concept(id).await?.unwrap();
        let role = concept
            .metadata
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let text = concept
            .metadata
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        println!("   {id} [{role}] ({score:.4}): \"{text}\"");
    }

    // Show associations for the top hit
    if let Some((top_id, _)) = hits.first() {
        let assoc = framework.get_associations(top_id).await?;
        println!("\n🔗 Associations for '{top_id}':");
        for (to_id, strength) in &assoc {
            println!("   → {to_id} (strength: {strength:.2})");
        }
    }

    println!("\n✅ Chatbot memory demo complete!");
    Ok(())
}
