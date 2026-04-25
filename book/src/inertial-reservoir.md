# Inertial Reservoir

The standard Echo State Network uses a first-order leaky integrator that attenuates past inputs quickly. The **inertial reservoir** adds a second-order momentum term that improves long-range temporal memory retention.

## Why It Matters

The default reservoir update rule:

```
state[t] = (1-α)·state[t-1] + α·tanh(W·state[t-1] + W_in·input)
```

decays past inputs at rate α per step, limiting how far back the reservoir "remembers." The inertial extension adds momentum from previous state changes:

```
state[t] = (1-α)·state[t-1] + α·tanh(W·state[t-1] + W_in·input) + β·(state[t-1] - state[t-2])
```

The β term carries forward the direction of recent state change, allowing the reservoir to retain temporal context over longer input sequences.

Based on: Zhao et al., "Inertial ESN", Neurocomputing Apr 2026, doi:10.1016/j.neucom.2026.133675

## The Beta Parameter

| Property | Value |
|---|---|
| Valid range | `[0.0, 0.5]` |
| Default | `0.0` (no inertia) |
| Recommended | `0.1–0.2` for most use cases |

- **β = 0.0** — Original reservoir behavior, fully backward-compatible.
- **β = 0.1–0.2** — Good default range; improves temporal retention without instability.
- **β > 0.3** — Aggressive momentum; may amplify noise in short-context workloads.
- **β > 0.5** — Rejected by `with_beta()` to prevent divergence.

## Enabling Inertia

Use the `with_beta()` builder method on `Reservoir`:

```rust,no_run
use chaotic_semantic_memory::reservoir::Reservoir;

let reservoir = Reservoir::new_seeded(768, 50_000, 42)?
    .with_beta(0.15)?;
```

Or via `ChaoticReservoir` by constructing the base reservoir first.

The spectral radius constraint still applies — `with_beta()` does not alter weight scaling.

## Backward Compatibility

When `beta = 0.0` (the default), the inertial term evaluates to zero and the reservoir behaves identically to the original first-order dynamics. No API changes are required for existing code.

## Trade-offs

| Cost | Impact |
|---|---|
| Memory | +1 `Vec<f32>` (`prev_state`) — ~200 KB at default 50K reservoir size |
| Throughput | One extra multiply-add per active node per step; <10% overhead in benchmarks |
| Complexity | 34 LOC in `reservoir_inertial.rs`; step loop adds 2 lines |

The `prev_state` vector is allocated at construction time regardless of beta, keeping the hot path branch-free.

## Tuning Guidance

1. **Start with β = 0.15** — a safe middle ground.
2. **Increase β** if retrieval quality degrades on long documents or multi-turn conversations.
3. **Decrease β** (or set to 0.0) if you observe amplified noise on very short inputs.
4. **Monitor** via `reservoir.metrics_snapshot()` — step latency should remain within 10% of baseline.

## Further Reading

- [ADR-0064: Inertial Reservoir Dynamics](https://github.com/d-o-hub/chaotic_semantic_memory/blob/main/docs/adr/ADR-0064-inertial-reservoir.md)
- [Architecture](./architecture.md) — overall system design
- [Performance](./performance.md) — benchmarking methodology
