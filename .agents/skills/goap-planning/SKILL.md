---
name: goap-planning
description: Build ordered, executable action plans from current state to target state using explicit preconditions, effects, and costs.
---

# GOAP Planning

1. Load state from `plans/GOAP_STATE.md` if present.
2. Define measurable target state.
3. Model actions via `references/action-model.md`.
4. Compute minimal path via `references/planner-pattern.md`.
5. Persist next action and update state after execution.
