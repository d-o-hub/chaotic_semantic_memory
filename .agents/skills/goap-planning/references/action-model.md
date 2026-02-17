# Action Model

Represent each action with:
- Preconditions: state facts that must already hold.
- Effects: state facts created/removed by the action.
- Cost: relative effort or risk weight.

Prefer low-cost, reversible actions early. Encode hard constraints explicitly (for example, file LOC caps, required test gates, and no hardcoded runtime settings/magic-number tunables).
