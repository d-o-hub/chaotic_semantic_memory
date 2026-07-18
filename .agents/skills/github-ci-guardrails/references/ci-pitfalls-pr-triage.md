# CI pitfalls — PR triage & mutation (2026-07)

Compact map from failing job → likely cause → fix. Use after open-PR sweeps
or Jules bot PRs. Broader Wave 32 wasm/fuzz notes may live in agent docs if present.

| Failure | Likely cause | Fix |
|---------|--------------|-----|
| **commitlint** | Scope not in `scope-enum` (`ops`, `plans`, …); bad early commit in range | Squash/reword to allowed scope; `npx commitlint --from origin/main --to HEAD` |
| **lint / clippy** | `duplicated_attributes`: `#![cfg(test)]` in file also loaded via `#[cfg(test)] mod` | Drop inner `#![cfg(test)]`; keep mod-level gate in `lib.rs` |
| **mutation-test** score &lt; threshold | Unrelated functions in git diff; `Ok(1)` survives N=1 import test; `>` vs `>=` on top-k | Restore unrelated lines to main; assert multi/empty counts; boundary test `len == top_k`; exclude unkillable CLI entry (`run_query`) |
| **mutation-test** timeouts | Pathological mutants or heavy modules in surface | Tighten excludes with rationale; do not blank-path-exclude production code |
| **Empty Jules PR** | Research-sim task pushed 0 files | Close as no-op; optional issue for research notes |
| **Regressed after “green” fix** | Jules force-push rewrote branch | Re-diff `origin/main...HEAD`; restore sibling merges; force-with-lease clean tip |

## Allowed commitlint scopes (canonical)

See `commitlint.config.cjs` `scope-enum`. Common valid: `framework`, `retrieval`,
`ci`, `docs`, `workspace`. Prefer bare `docs:` over inventing `docs(plans)`.

## Merge order

Green independent first → CI fixes → dependent features. One squash-merge at a
time; never multi-PR `--auto` when base must be current.
