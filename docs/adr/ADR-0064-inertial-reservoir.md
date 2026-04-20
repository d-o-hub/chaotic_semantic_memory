# ADR-0064: Inertial Reservoir Dynamics (InertialESN)

## Status

Accepted

## Context

The current Reservoir uses a first-order leaky integrator:

```
state[t] = (1-α)·state[t-1] + α·tanh(W·state[t-1] + W_in·input)
```

This limits long-range temporal memory because the decay coefficient α rapidly attenuates past inputs.

Paper: Zhao et al., "Inertial ESN", Neurocomputing Apr 2026, doi:10.1016/j.neucom.2026.133675

The paper proposes a second-order momentum term:

```
state[t] = (1-α)·state[t-1] + α·tanh(W·state[t-1] + W_in·input) + β·(state[t-1] - state[t-2])
```

Where β ∈ [0.0, 0.5] is the inertial coefficient. This "inertial" term captures momentum from previous state changes, improving memory retention.

## Decision

Add optional second-order inertial dynamics to `Reservoir::step()`:
- Add `prev_state: Vec<f32>` field storing state at t-1
- Add `beta: f32` field for inertial coefficient (default 0.0)
- Modify step() inner loop to include inertial term
- Provide `with_beta()` builder method for configuration

## Implementation

1. **Struct changes** (reservoir.rs):
   - `prev_state: Vec<f32>` — state at t-1 for momentum calculation
   - `beta: f32` — inertial coefficient, default 0.0 (backward-compatible)

2. **Initialization** (new_seeded):
   - `prev_state: vec![0.0; size]`
   - `beta: 0.0`

3. **Builder method**:
   - `pub fn with_beta(mut self, beta: f32) -> Result<Self>`
   - Validate β ∈ [0.0, 0.5], error if outside range

4. **Step modification** (line ~254):
   ```rust
   let inertial = self.beta * (state[i] - self.prev_state[i]);
   self.scratch[i] = state[i] * one_minus_alpha + activated * self.alpha + inertial;
   ```

5. **Before swap**: Copy current state to prev_state

6. **Reset**: Zero prev_state

## Consequences

### Benefits
- Improved temporal memory retention for β > 0
- Backward-compatible: β=0.0 recovers original behavior exactly
- Spectral radius constraint still applies (validated in paper)

### Costs
- +200KB memory at default 50K reservoir size (prev_state Vec)
- ~3-5% throughput overhead (one Vec copy + one multiply-add per active node)

### LOC Impact
- reservoir.rs at 496 LOC; implementation adds ~18 lines
- Extract to `reservoir_inertial.rs` extension file to stay under 500 LOC gate

## Phase 2 (Future ADR)

Deterministic topology via cyclic-shift mixing operator replacing random sparse matrix construction. Uses low-discrepancy permutations (number-theoretic sequences) for reproducible reservoir dynamics.

## References

- Zhao et al., "Inertial ESN", Neurocomputing Apr 2026, doi:10.1016/j.neucom.2026.133675
- GOAP_ACTION: write_adr_inertial_reservoir, implement_inertial_reservoir