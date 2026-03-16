# Accuracy Guardrails

- Do not assume crate existence/version; verify.
- When uncertain on modern Rust practice, verify with web research.
- If a decision changes architecture, write/update ADR in `plans/adr/`.
- Prefer exact, testable instructions over high-level advice.
- **Never create unused code**: Before adding proc-macros, traits, or convenience APIs, verify at least one real usage site exists in examples, tests, or docs.
- **Before adding or recommending any GitHub-hosted crate**, run:
```bash
gh repo view <owner>/<repo> --json isArchived,pushedAt
```
Confirm the repo is **not archived** and was pushed to within the last 12 months.
If archived: find an active alternative on crates.io, or fork into `d-o-hub/` and maintain it.
