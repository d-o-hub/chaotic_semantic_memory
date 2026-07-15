# ADR-0025: Weighted Forgetting (Decay)

## Status

Implemented (2026-06-23)

## Context

Association strength could decay over time:
- Biological memory: associations weaken with disuse
- Problem: All associations have fixed strength
- Use case: Biological modeling, attention modeling

## Decision

Weighted forgetting is implemented as opt-in association decay with reinforcement and pruning APIs. The fixed-strength path remains available when decay is not configured.

Implementation ownership is `crates/csm-memory/src/singularity_decay.rs`, exposed through framework methods in `src/framework.rs`. Regression coverage is in `tests/association_decay.rs` and advanced TTL/decay tests.

## Consequences

### Positive
- Supports biological/recency-oriented forgetting when configured
- Reinforcement resets decay age
- Pruning removes associations below the configured threshold

### Negative
- Time-dependent scores require deterministic clock-aware tests
- Callers must understand whether raw or decayed association strength is returned

## Implementation

- Module: `crates/csm-memory/src/singularity_decay.rs`
- Framework: `reinforce_association` and `prune_decayed_associations`
- Tests: `tests/association_decay.rs`

## Sources

- ACTIONS.md lines 1170-1180 (deferred_association_decay action)
- ADR_REGISTRY.md: Deferred ADRs section
- Activation condition documented