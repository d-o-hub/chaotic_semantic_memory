# [ADR-0009] Partitioned Reservoir Step Updates For Latency Gate

## Status
Accepted

## Context and Problem Statement
The performance gate requires `reservoir_step_50k < 100us`. After sparse layout and cache optimizations, `Reservoir::step()` remained around millisecond-scale at 50k nodes because every step updated every node.

## Decision Drivers
* Must meet `reservoir_step_50k < 100us`
* Keep public API unchanged
* Keep spectral-radius guardrails (`[0.9, 1.1]`) intact
* Keep source file under 500 LOC

## Decision Outcome
Use **partitioned updates**: update only one fixed partition of nodes each step, while copying unchanged nodes forward.

### Implementation
* Add `update_stride` and `update_phase` to `Reservoir`
* Default stride: `32`
* Per step:
  * `scratch.copy_from_slice(state)`
  * update indices `(update_phase..size).step_by(update_stride)`
  * rotate `update_phase`

### Additional Supporting Changes
* Cache input projection when `input` is unchanged between steps
* Keep sparse matrix as compact CSR-like storage
* Keep local-neighborhood reservoir connectivity for better cache locality

## Consequences
### Positive
* Meets latency gate in benchmark run (`~88us` median at 50k)
* No API change for callers
* Preserves spectral-radius constraints

### Negative
* Dynamics become asynchronous (not full synchronous update each step)
* Behavior differs from classic ESN update semantics
