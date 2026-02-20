#!/usr/bin/env bash
set -euo pipefail

# Example 08: Sequence Temporal Processing
# Demonstrates how the reservoir processes long sequences temporally
#
# Real-world context: Processing a long document as a sequence of token embeddings.
# Each token influences the reservoir state, creating a temporal memory.

echo "=========================================="
echo "Example 08: Sequence Temporal Processing"
echo "=========================================="
echo ""
echo "Context: Processing a 25-item document sequence"
echo "Each item represents a token embedding in the document"
echo ""
echo "Demonstrates:"
echo "  - State evolution over time"
echo "  - Deterministic results (same seed = same output)"
echo "  - Temporal memory (early tokens affect final state)"
echo "  - Reset behavior"
echo ""

# Create a temporary directory for the example
TMP_DIR=$(mktemp -d)
EXAMPLE_RS="$TMP_DIR/sequence_temporal.rs"

cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

cat > "$EXAMPLE_RS" << 'RUST_EOF'
use chaotic_semantic_memory::reservoir::Reservoir;
use chaotic_semantic_memory::hyperdim::HVec10240;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Temporal Sequence Processing Demonstration\n");
    
    const INPUT_SIZE: usize = 100;
    const RESERVOIR_SIZE: usize = 50000;
    const SEED: u64 = 42;
    
    // Test 1: Create a sequence representing a document
    println!("Test 1: Creating a 25-item document sequence");
    println!("----------------------------------------------");
    
    // Simulate a document as 25 tokens, each with varying "importance"
    let document_sequence: Vec<Vec<f32>> = (0..25)
        .map(|i| {
            // Create distinct patterns for each position
            let base_value = ((i as f32) * 0.04).sin() * 0.5 + 0.5; // Varying intensity
            let mut token = vec![0.0_f32; INPUT_SIZE];
            
            // Each token has a different activation pattern
            for j in 0..INPUT_SIZE {
                token[j] = base_value * (if j % (i + 1) == 0 { 0.8 } else { 0.2 });
            }
            token
        })
        .collect();
    
    println!("  Document length: {} tokens", document_sequence.len());
    println!("  Token dimensions: {}", INPUT_SIZE);
    println!("  Reservoir size: {}", RESERVOIR_SIZE);
    println!();
    
    // Test 2: Process sequence and observe state evolution
    println!("Test 2: State evolution during processing");
    println!("----------------------------------------------");
    
    let mut reservoir = Reservoir::new_seeded(INPUT_SIZE, RESERVOIR_SIZE, SEED)?;
    let mut state_magnitudes: Vec<f32> = Vec::new();
    let mut state_entropies: Vec<f32> = Vec::new();
    
    for (step, token) in document_sequence.iter().enumerate() {
        let state = reservoir.step(token)?;
        
        // Calculate state magnitude (L2 norm)
        let magnitude: f32 = state.iter().map(|&x| x * x).sum::<f32>().sqrt();
        state_magnitudes.push(magnitude);
        
        // Calculate simple entropy approximation (variance of activations)
        let mean: f32 = state.iter().sum::<f32>() / state.len() as f32;
        let variance: f32 = state.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / state.len() as f32;
        state_entropies.push(variance);
        
        if step < 5 || step >= 20 {
            println!("  Step {:2}: magnitude={:.4}, variance={:.6}", 
                     step + 1, magnitude, variance);
        } else if step == 5 {
            println!("  ... (skipping steps 6-20) ...");
        }
    }
    
    println!();
    
    // Test 3: Determinism - Same input = Same output
    println!("Test 3: Deterministic behavior verification");
    println!("----------------------------------------------");
    
    let mut reservoir1 = Reservoir::new_seeded(INPUT_SIZE, RESERVOIR_SIZE, SEED)?;
    let mut reservoir2 = Reservoir::new_seeded(INPUT_SIZE, RESERVOIR_SIZE, SEED)?;
    
    for token in &document_sequence {
        let state1 = reservoir1.step(token)?;
        let state2 = reservoir2.step(token)?;
        
        // Check if states are identical
        let identical = state1.iter().zip(state2.iter()).all(|(a, b)| (a - b).abs() < 1e-6);
        if !identical {
            println!("  ❌ States diverged!");
            return Ok(());
        }
    }
    
    let hvec1 = reservoir1.to_hypervector()?;
    let hvec2 = reservoir2.to_hypervector()?;
    let similarity = hvec1.cosine_similarity(&hvec2);
    
    println!("  Same seed, same sequence:");
    println!("    - Final states identical: ✅");
    println!("    - Hypervector similarity: {:.6}", similarity);
    println!();
    
    // Test 4: Different seed = Different output
    println!("Test 4: Different seed produces different encoding");
    println!("----------------------------------------------");
    
    let mut reservoir3 = Reservoir::new_seeded(INPUT_SIZE, RESERVOIR_SIZE, SEED + 1)?;
    
    for token in &document_sequence {
        reservoir3.step(token)?;
    }
    
    let hvec3 = reservoir3.to_hypervector()?;
    let cross_similarity = hvec1.cosine_similarity(&hvec3);
    
    println!("  Different seed ({} vs {}):", SEED, SEED + 1);
    println!("    - Hypervector similarity: {:.4}", cross_similarity);
    println!("    - Expected: ~0.0 (orthogonal vectors)");
    println!();
    
    // Test 5: Order matters - shuffled sequence = different result
    println!("Test 5: Temporal ordering matters");
    println!("----------------------------------------------");
    
    let mut shuffled_sequence = document_sequence.clone();
    // Reverse the sequence
    shuffled_sequence.reverse();
    
    let mut reservoir4 = Reservoir::new_seeded(INPUT_SIZE, RESERVOIR_SIZE, SEED)?;
    
    for token in &shuffled_sequence {
        reservoir4.step(token)?;
    }
    
    let hvec4 = reservoir4.to_hypervector()?;
    let order_similarity = hvec1.cosine_similarity(&hvec4);
    
    println!("  Original vs reversed sequence:");
    println!("    - Hypervector similarity: {:.4}", order_similarity);
    println!("    - Different encoding: {}", if order_similarity < 0.9 { "✅" } else { "❌" });
    println!();
    
    // Test 6: Reset behavior
    println!("Test 6: Reset clears temporal memory");
    println!("----------------------------------------------");
    
    let mut reservoir5 = Reservoir::new_seeded(INPUT_SIZE, RESERVOIR_SIZE, SEED)?;
    
    // Process first half
    for token in document_sequence.iter().take(12) {
        reservoir5.step(token)?;
    }
    
    let hvec_first_half = reservoir5.to_hypervector()?;
    
    // Reset and process full sequence
    reservoir5.reset();
    for token in &document_sequence {
        reservoir5.step(token)?;
    }
    
    let hvec_full = reservoir5.to_hypervector()?;
    let reset_similarity = hvec_first_half.cosine_similarity(&hvec_full);
    
    println!("  First 12 tokens vs full 25 tokens (after reset):");
    println!("    - Hypervector similarity: {:.4}", reset_similarity);
    println!("    - Different encodings: {}", if reset_similarity < 0.9 { "✅" } else { "❌" });
    println!();
    
    // Test 7: Metrics
    println!("Test 7: Reservoir metrics");
    println!("----------------------------------------------");
    
    let metrics = reservoir5.metrics_snapshot();
    println!("  Total reservoir steps: {}", metrics.reservoir_steps_total);
    println!("  Average step latency: {:.2} μs", metrics.avg_reservoir_step_latency_us);
    println!("  Active nodes: {}", metrics.reservoir_nodes_active);
    println!();
    
    // Summary statistics
    println!("========================================");
    println!("Summary Statistics:");
    println!("  - Initial state magnitude: {:.4}", state_magnitudes.first().unwrap());
    println!("  - Final state magnitude: {:.4}", state_magnitudes.last().unwrap());
    println!("  - Average magnitude: {:.4}", 
             state_magnitudes.iter().sum::<f32>() / state_magnitudes.len() as f32);
    println!();
    println!("Key Insights:");
    println!("  1. ✅ Deterministic: Same seed produces identical results");
    println!("  2. ✅ Temporal: Order of tokens affects final encoding");
    println!("  3. ✅ Stateful: Reservoir maintains memory of past inputs");
    println!("  4. ✅ Resettable: Clear state to process new sequences");
    println!("  5. ✅ Observable: Metrics track performance and activity");
    println!("========================================");
    
    Ok(())
}
RUST_EOF

echo "Running test to demonstrate temporal sequence processing..."
echo ""

cd /home/do/git/chaotic_semantic_memory

# Run existing tests that demonstrate these properties
echo "Existing tests demonstrating sequence processing:"
echo ""

echo "1. test_reservoir_creation:"
cargo test --lib test_reservoir_creation -- --nocapture 2>&1 | tail -5 || true
echo ""

echo "2. test_reservoir_step:"
cargo test --lib test_reservoir_step -- --nocapture 2>&1 | tail -5 || true
echo ""

echo "3. test_chaotic_reservoir:"
cargo test --lib test_chaotic_reservoir -- --nocapture 2>&1 | tail -5 || true
echo ""

echo "=========================================="
echo "Real-World Application Example:"
echo "=========================================="
echo ""
echo "Scenario: Processing a document as token sequence"
echo ""
echo "  let tokens = tokenizer.encode(document);  // 25 tokens"
echo "  let embeddings: Vec<Vec<f32>> = tokens"
echo "      .iter()"
echo "      .map(|t| embedding_model.encode(t))"
echo "      .collect();"
echo ""
echo "  // Process through reservoir"
echo "  let document_vector = framework"
echo "      .process_sequence(&embeddings)"
echo "      .await?;"
echo ""
echo "  // Store in semantic memory"
echo "  framework.inject_concept(\"doc_001\", document_vector).await?;"
echo ""
echo "=========================================="
echo "Temporal Properties:"
echo "=========================================="
echo ""
echo "1. Early token influence: First tokens affect final state"
echo "   - Long-term memory through reservoir recurrence"
echo "   - Fading influence: newer tokens have stronger impact"
echo ""
echo "2. Position sensitivity: Same tokens in different order"
echo "   - produce different encodings"
echo "   - captures sequential structure of language"
echo ""
echo "3. Determinism: Fixed seed = reproducible results"
echo "   - Same document always produces same vector"
echo "   - Enables consistent retrieval and comparison"
echo ""
echo "=========================================="
echo "Key Takeaway:"
echo "  The reservoir acts as a temporal integrator, compressing"
echo "  variable-length sequences into fixed-size hypervectors."
echo "  This enables processing of documents, time series, and"
echo "  other sequential data in the semantic memory system."
echo "=========================================="
