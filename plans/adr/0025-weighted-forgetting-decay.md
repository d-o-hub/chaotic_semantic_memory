# ADR-0025: Weighted Forgetting (Decay)

## Status

Deferred (backfilled 2026-05-01)

## Context

Association strength could decay over time:
- Biological memory: associations weaken with disuse
- Problem: All associations have fixed strength
- Use case: Biological modeling, attention modeling

## Decision

**Deferred** - Weighted forgetting not implemented for 1.0.

**Rationale:**
- No current user request for decay
- Fixed strength simplifies debugging
- Biological modeling is niche use case
- Complexity not justified for initial release

**Activation Trigger:**
- Biological memory modeling requested by users
- Research application with decay requirement
- Attention modeling use case emerges

## Consequences

### Positive (Deferred)
- Simplified mental model (fixed strength)
- No decay computation overhead
- Easier to reason about associations

### Negative (Deferred)
- Cannot model biological forgetting
- All associations persist equally
- No "use it or lose it" behavior

## Future Implementation

- Module: `src/singularity.rs`
- Pattern: decay factor per association
- Trigger: access count updates strength
- Cost: ~6 (estimated)

## Sources

- ACTIONS.md lines 1170-1180 (deferred_association_decay action)
- ADR_REGISTRY.md: Deferred ADRs section
- Activation condition documented