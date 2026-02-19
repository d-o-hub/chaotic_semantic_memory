# [ADR-0008] WASM Rayon Gating Strategy

## Status
Accepted (supersedes ADR-0003)

## Context and Problem Statement
Rayon is used in three modules (`hyperdim.rs`, `reservoir.rs`, `singularity.rs`) for CPU parallelism. However, `wasm32-unknown-unknown` does not support threads, and importing `rayon::prelude::*` will fail to compile or panic at runtime on WASM targets.

ADR-0003 established the conditional compilation strategy but did not define the specific gating pattern for Rayon.

## Decision Drivers
* WASM target must compile without Rayon
* Native target must retain full Rayon parallelism
* Code duplication between native and WASM paths must be minimal
* Each source file must remain under 500 LOC

## Considered Options
1. **`cfg` guards with inline sequential fallbacks**
2. **Feature-gated Rayon behind `parallel` feature flag**
3. **Wrapper trait abstracting par_iter/iter**

## Decision Outcome
Chosen option: **`cfg` guards with inline sequential fallbacks**, because:
- Simplest approach with minimal abstraction
- Each call site gets a 3-4 line `cfg` block
- No new traits, features, or modules needed
- Consistent with existing `cfg(target_arch = "wasm32")` pattern in lib.rs

### Pattern
```rust
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

// In function body:
#[cfg(not(target_arch = "wasm32"))]
let results: Vec<_> = items.par_iter().map(|x| compute(x)).collect();

#[cfg(target_arch = "wasm32")]
let results: Vec<_> = items.iter().map(|x| compute(x)).collect();
```

### Affected Files
| File | Rayon usage | LOC impact |
|---|---|---|
| hyperdim.rs | `bundle()`, `batch_cosine_similarity()` | +6 LOC |
| reservoir.rs | `step()` tanh activation | +4 LOC |
| singularity.rs | `find_similar()` (after ADR-0007) | +4 LOC |

### Positive Consequences
* WASM target compiles cleanly
* No runtime feature detection needed
* Zero overhead — cfg is compile-time
* No new abstractions

### Negative Consequences
* Minor code duplication at each Rayon call site
* Must remember to add guards when adding new Rayon usage

## Pros and Cons of the Options

### cfg guards (chosen)
* Good: Zero abstraction overhead, compile-time
* Good: Explicit and visible
* Bad: ~4 LOC duplication per call site

### Feature flag
* Good: Single toggle
* Bad: Users must opt-in to parallelism
* Bad: Complicates dependency management

### Wrapper trait
* Good: Single code path
* Bad: Adds abstraction layer
* Bad: Trait overhead, harder to reason about
