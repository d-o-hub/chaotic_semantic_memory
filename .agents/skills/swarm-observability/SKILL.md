---
name: swarm-observability
description: "Tracing, metrics, derive macros, and error context. Use when adding observability or improving developer experience."
---

# Swarm: Observability

## Workflow
1. Add tracing/metrics dependencies to `Cargo.toml`
2. Instrument async functions with `#[instrument]`
3. Add metric collection points (counters, histograms, gauges)
4. Create proc-macro crate if needed (chaotic_semantic_memory_derive)
5. Enhance error types with `#[source]` and context
6. Test with `tracing-subscriber` fmt or json output

## Tracing Setup

```rust
use tracing::{instrument, info, debug};

#[instrument(skip(self))]
pub async fn inject_concept(&self, id: String, vector: HVec10240) -> Result<()> {
    debug!(concept_id = %id, "injecting concept");
    // ... operation
    info!(concept_id = %id, "concept injected");
    Ok(())
}
```

## Metrics Collection

```rust
use metrics::{counter, histogram, gauge};

pub async fn probe(&self, query: HVec10240, top_k: usize) -> Result<Vec<(String, f32)>> {
    let start = Instant::now();
    counter!("probe_requests_total").increment(1);
    
    let results = // ... search
    
    histogram!("probe_latency_ms").record(start.elapsed().as_millis() as f64);
    gauge!("concept_count").set(self.singularity.len() as f64);
    
    Ok(results)
}
```

## Derive Macros (DEPRECATED)

Derive macros were removed after discovery of zero usage. Use `ConceptBuilder` directly:

```rust
// Instead of #[derive(Concept)]:
let concept = ConceptBuilder::new("id")
    .with_vector(HVec10240::random())
    .build()?;
```

## Error Context

```rust
#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Concept '{concept_id}' not found")]
    ConceptNotFound { concept_id: String },
    
    #[error("Database error: {message}")]
    Database { 
        message: String,
        #[source]
        source: Option<Box<dyn Error>>,
    },
}
```

## LOC Constraint
All files must remain ≤ 500 lines. Proc-macro crate counts separately.
