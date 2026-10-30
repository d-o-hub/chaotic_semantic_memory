# CI pitfalls — PR triage & mutation (2026-07)

Compact map from failing job → likely cause → fix. Use after open-PR sweeps
or Jules bot PRs. Broader Wave 32 wasm/fuzz notes may live in agent docs if present.

| Failure | Likely cause | Fix |
|---------|--------------|-----|
| **commitlint** | Scope not in `scope-enum` (`ops`, `plans`, …); bad early commit in range | Squash/reword to allowed scope; `npx commitlint --from origin/main --to HEAD` |
| **lint / clippy** | `duplicated_attributes`: `#![cfg(test)]` in file also loaded via `#[cfg(test)] mod` | Drop inner `#![cfg(test)]`; keep mod-level gate in `lib.rs` |
| **mutation-test** score &lt; threshold | Unrelated functions in git diff; `Ok(1)` survives N=1 import test; `>` vs `>=` on top-k | Restore unrelated lines to main; assert multi/empty counts; boundary test `len == top_k`; exclude unkillable CLI entry (`run_query`) |
| **mutation-test** timeouts | Pathological mutants or heavy modules in surface | Tighten excludes with rationale; do not blank-path-exclude production code |
| **mutation-test** score low on a diff that cannot be reached | **phantom mutants**: cfg-blind lines (`#[cfg(target_arch = "wasm32")]`, an off feature) the profile never compiles, so no test can kill them — the score is measuring the denominator, not the tests | Separate `unviable`/cfg-off from real `missed` and name the count. Measured on #947: **62% → 94.0476%** once 19 wasm32 phantoms left the set. Never "fix" it by writing a test for an uncompilable line |
| **Release run with zero jobs** | an expression GitHub rejects **before scheduling** — double-quoted string literals inside `if:` (`if: matrix.target == aarch64-unknown-linux-gnu`). PyYAML loads the file happily, `gh pr checks` shows nothing, and no notification fires | `gh api …/runs/<id>/jobs` → `total_count: 0` with `updated_at == run_started_at` is the tell; `./scripts/check-actionlint.sh` (rc-gating, `-shellcheck ''`) is the local gate. Measured: 69 consecutive jobless runs, `0f77c0d0` 2026-10-22 → #947 2026-10-30 |
| **`release.yml` refuses a gate that passed** | `release.yml` and `pre-release-gate.yml` are **siblings on one CI completion** (`workflow_run: [CI] … branches: [main]`), so `validate`'s one-shot `gh run list --workflow=pre-release-gate.yml` samples a verdict ~1 min into a ~4½-min run (#953) | Bound a wait, keep it fail-closed and loud; or re-key the trigger to the gate — but that trades a visible refusal for *no run at all*, which is the failure this repo already lived through silently for eight days |
| **Empty Jules PR** | Research-sim task pushed 0 files | Close as no-op; optional issue for research notes |
| **Regressed after “green” fix** | Jules force-push rewrote branch | Re-diff `origin/main...HEAD`; restore sibling merges; force-with-lease clean tip |

## Allowed commitlint scopes (canonical)

See `commitlint.config.cjs` `scope-enum`. Common valid: `framework`, `retrieval`,
`ci`, `docs`, `workspace`. Prefer bare `docs:` over inventing `docs(plans)`.

## Merge order

Green independent first → CI fixes → dependent features. One squash-merge at a
time; never multi-PR `--auto` when base must be current.
