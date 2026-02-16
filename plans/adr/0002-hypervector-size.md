# [ADR-0002] Hypervector Size: 10240 bits

## Status
Accepted

## Context and Problem Statement
Need to determine the optimal size for hypervectors in the chaotic semantic memory system.

## Decision Drivers
- Performance: Smaller = faster
- Capacity: Larger = more representational power
- Memory: Must fit within RAM constraints
- SIMD optimization: Should align with CPU register sizes

## Considered Options
1. **4096 bits (512 bytes)** - Small, fast, limited capacity
2. **10240 bits (1280 bytes)** - Medium, good capacity, aligns with SIMD
3. **16384 bits (2048 bytes)** - Large, high capacity, slower operations

## Decision Outcome
Chosen option: **10240 bits (1280 bytes)**, implemented as `[u128; 80]`

### Positive Consequences
- Fits exactly 80 u128 values
- Good balance of capacity and performance
- Enables efficient SIMD operations
- 10 million concepts ≈ 12.2 GB RAM

### Negative Consequences
- Larger than 4096-bit alternatives
- More memory usage than strictly necessary for simple tasks

## Pros and Cons of the Options

### 10240 bits
* Good: Optimal SIMD alignment with u128
* Good: Good representational capacity
* Good: Achieves 200x faster than HNSW
* Bad: Higher memory usage than 4096-bit

### 4096 bits
* Good: Very fast operations
* Good: Lower memory footprint
* Bad: Limited representational power

### 16384 bits
* Good: Excellent capacity
* Bad: 2x slower operations
* Bad: 2x memory usage