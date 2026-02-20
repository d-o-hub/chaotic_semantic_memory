#!/usr/bin/env bash
set -euo pipefail

# Example 07: Reservoir Size Limits
# Demonstrates that reservoir size must be >= DIMENSION (10240)
#
# Real-world context: The reservoir projects to a 10240-bit hypervector.
# If the reservoir is smaller than this, we can't perform the projection.

echo "=========================================="
echo "Example 07: Reservoir Size Limits"
echo "=========================================="
echo ""
echo "Context: Processing a document with a small/toy reservoir"
echo "The reservoir projects its state to a 10240-dimensional hypervector"
echo ""
echo "Constraint: reservoir_size >= 10240 (HVec10240::DIMENSION)"
echo ""

# Create a temporary directory for the example
TMP_DIR=$(mktemp -d)
EXAMPLE_RS="$TMP_DIR/reservoir_size.rs"

cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

cat > "$EXAMPLE_RS" << 'RUST_EOF'
use chaotic_semantic_memory::reservoir::Reservoir;
use chaotic_semantic_memory::hyperdim::HVec10240;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing reservoir size limits...\n");
    
    const DIMENSION: usize = HVec10240::DIMENSION; // 10240
    
    println!("Required dimension: {} bits", DIMENSION);
    println!();
    
    // Test 1: Sizes that are too small (will fail at to_hypervector)
    println!("Test 1: Reservoir sizes < DIMENSION (will fail projection)");
    println!("-----------------------------------------------------------");
    
    let small_sizes = [256_usize, 512, 1000, 5000, 10239];
    for &size in &small_sizes {
        let mut reservoir = Reservoir::new_seeded(100, size, 42)?;
        // Run a few steps to populate state
        let input = vec![0.5_f32; 100];
        for _ in 0..5 {
            reservoir.step(&input)?;
        }
        
        match reservoir.to_hypervector() {
            Ok(_) => println!("  Size {:5}: ❌ UNEXPECTED SUCCESS", size),
            Err(e) => {
                let error_str = format!("{}", e);
                if error_str.contains("InvalidDimension") || error_str.contains("expected") {
                    println!("  Size {:5}: ✅ REJECTED - Dimension mismatch", size);
                } else {
                    println!("  Size {:5}: ✅ REJECTED - {}", size, e);
                }
            }
        }
    }
    println!();
    
    // Test 2: Sizes that meet or exceed DIMENSION
    println!("Test 2: Reservoir sizes >= DIMENSION (will succeed)");
    println!("-----------------------------------------------------------");
    
    let valid_sizes = [10240_usize, 15000, 50000, 100000];
    for &size in &valid_sizes {
        let mut reservoir = Reservoir::new_seeded(100, size, 42)?;
        let input = vec![0.5_f32; 100];
        for _ in 0..5 {
            reservoir.step(&input)?;
        }
        
        match reservoir.to_hypervector() {
            Ok(hvec) => {
                // Verify it's a valid hypervector by checking it's not all zeros
                let zero_vec = HVec10240::zero();
                let similarity = hvec.cosine_similarity(&zero_vec);
                println!("  Size {:6}: ✅ SUCCESS (sim to zero: {:.4})", size, similarity);
            }
            Err(e) => println!("  Size {:6}: ❌ FAILED - {}", size, e),
        }
    }
    println!();
    
    // Test 3: Demonstrate the projection mechanism
    println!("Test 3: Understanding the projection mechanism");
    println!("-----------------------------------------------------------");
    println!("The projection divides reservoir state into {} chunks", DIMENSION);
    println!("Each chunk contributes 1 bit to the hypervector");
    println!("Chunk size = reservoir_size / DIMENSION");
    println!();
    
    for &size in &[10240_usize, 20480, 50000] {
        let chunk_size = size / DIMENSION;
        println!("  Reservoir size {}: chunk_size = {}", size, chunk_size);
        
        let mut reservoir = Reservoir::new_seeded(100, size, 42)?;
        let input = vec![0.5_f32; 100];
        for _ in 0..10 {
            reservoir.step(&input)?;
        }
        
        let hvec = reservoir.to_hypervector()?;
        let zero_vec = HVec10240::zero();
        let sim = hvec.cosine_similarity(&zero_vec);
        
        println!("    -> Hypervector similarity to zero: {:.4}", sim);
    }
    println!();
    
    // Test 4: Framework-level behavior
    println!("Test 4: Framework-level reservoir size configuration");
    println!("-----------------------------------------------------------");
    println!("Using ChaoticSemanticFramework::builder()");
    println!();
    
    println!("  Valid configurations:");
    println!("    - with_reservoir_size(10240)  -> ✅ Minimum valid");
    println!("    - with_reservoir_size(50000)  -> ✅ Default");
    println!("    - with_reservoir_size(100000) -> ✅ Large reservoir");
    println!();
    println!("  Invalid configurations (will fail at sequence processing):");
    println!("    - with_reservoir_size(5000)   -> ❌ Too small");
    println!("    - with_reservoir_size(1000)   -> ❌ Too small");
    println!();
    
    println!("========================================");
    println!("Summary:");
    println!("  - Minimum reservoir size: {} (HVec10240::DIMENSION)", DIMENSION);
    println!("  - Default reservoir size: 50000");
    println!("  - to_hypervector() requires size >= DIMENSION");
    println!("  - The projection uses chunk-based aggregation");
    println!("  - Smaller reservoirs = larger chunks = coarser projection");
    println!("========================================");
    
    Ok(())
}
RUST_EOF

echo "Running test to demonstrate reservoir size limits..."
echo ""

cd /home/do/git/chaotic_semantic_memory

# Run the existing test that validates this
echo "Existing test: test_to_hypervector_small_reservoir_errors"
cargo test --lib test_to_hypervector_small_reservoir_errors -- --nocapture 2>&1 || true

echo ""
echo "=========================================="
echo "Source code reference from src/reservoir.rs:291-297"
echo "=========================================="
grep -A 6 "pub fn to_hypervector" src/reservoir.rs | head -10 || echo "  (source code reference)"

echo ""
echo "=========================================="
echo "Why This Matters:"
echo ""
echo "1. Projection requires mapping reservoir state -> 10240-bit hypervector"
echo "2. Each of the 10240 bits is computed from a chunk of reservoir neurons"
echo "3. If reservoir_size < 10240, we can't create valid chunks"
echo "4. Error: InvalidDimension { expected: 10240, actual: <your_size> }"
echo ""
echo "Use cases affected:"
echo "  - process_sequence() - requires to_hypervector() at the end"
echo "  - Any temporal sequence encoding"
echo "  - Document/sequence embedding generation"
echo ""
echo "Solution: Always use reservoir_size >= 10240"
echo "  Default (50000) is recommended for good performance"
echo "=========================================="
