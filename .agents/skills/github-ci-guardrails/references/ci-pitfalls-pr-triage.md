# CI pitfalls — PR triage & mutation (2026-07)

Compact map from failing job → likely cause → fix. Use after open-PR sweeps
or Jules bot PRs. Broader Wave 32 wasm/fuzz notes may live in agent docs if present.

| Failure | Likely cause | Fix |
|---------|--------------|-----|
| **commitlint** | Scope not in `scope-enum` (`ops`, `plans`, ...); bad early commit in range; a paragraph after the first blank line longer than 100 chars — `body-max-line-length` is disabled, but `footer-max-line-length` (default 100) still applies because commitlint classifies trailing paragraphs as footers | Squash/reword to allowed scope and wrap every paragraph at ≤100 chars; `npx commitlint --from origin/main --to HEAD` (read the whole output — only the failing input carries `✖`) |
| **lint / clippy** | `duplicated_attributes`: `#![cfg(test)]` in file also loaded via `#[cfg(test)] mod` | Drop inner `#![cfg(test)]`; keep mod-level gate in `lib.rs` |
| **mutation-test** score &lt; threshold | Unrelated functions in git diff; `Ok(1)` survives N=1 import test; `>` vs `>=` on top-k | Restore unrelated lines to main; assert multi/empty counts; boundary test `len == top_k`; exclude unkillable CLI entry (`run_query`) |
| **mutation-test** timeouts | Pathological mutants or heavy modules in surface | Tighten excludes with rationale; do not blank-path-exclude production code |
| **mutation-test** score low on a diff that cannot be reached | **phantom mutants**: cfg-blind lines (`#[cfg(target_arch = "wasm32")]`, an off feature) the profile never compiles, so no test can kill them — the score is measuring the denominator, not the tests | Separate `unviable`/cfg-off from real `missed` and name the count. Measured on #947: **62% → 94.0476%** once 19 wasm32 phantoms left the set. Never "fix" it by writing a test for an uncompilable line |
| **Release run with zero jobs** | an expression GitHub rejects **before scheduling** — double-quoted string literals inside `if:` (`if: matrix.target == aarch64-unknown-linux-gnu`). PyYAML loads the file happily, `gh pr checks` shows nothing, and no notification fires | `gh api …/runs/<id>/jobs` → `total_count: 0` with `updated_at == run_started_at` is the tell; `./scripts/check-actionlint.sh` (rc-gating, `-shellcheck ''`) is the local gate. Measured: 69 consecutive jobless runs, `0f77c0d0` 2026-10-22 → #947 2026-10-30 |
| **`release.yml` refuses a gate that passed** | `release.yml` and `pre-release-gate.yml` are **siblings on one CI completion** (`workflow_run: [CI] … branches: [main]`), so `validate`'s one-shot `gh run list --workflow=pre-release-gate.yml` samples a verdict ~1 min into a ~4½-min run (#953) | Bound a wait, keep it fail-closed and loud; or re-key the trigger to the gate — but that trades a visible refusal for *no run at all*, which is the failure this repo already lived through silently for eight days |
| **Empty Jules PR** | Research-sim task pushed 0 files | Close as no-op; optional issue for research notes |
| **Regressed after “green” fix** | Jules force-push rewrote branch | Re-diff `origin/main...HEAD`; restore sibling merges; force-with-lease clean tip |
| **A required check reads `cancel`** | The job never acquired a runner — possible causes, **none of them code**: a *superseded-run cancellation* (`ci.yml`'s concurrency group cancels in-flight runs for the same ref, PR runs included) or *runner capacity/queueing*. Observable: `conclusion=cancelled`, empty `runner_name`, zero steps, no retrievable log. `gh pr checks --json` buckets it `cancel` (`pass`/`fail`/`pending`/`skipping`/`cancel`), and `gh run view --json jobs` carries no runner field, so neither the tick nor that JSON can separate infra from a defect. **Do not assert a cause from a cancellation alone** | Take the cancelled job's `databaseId` from `gh run view <run> --json jobs`, then `gh api repos/{owner}/{repo}/actions/jobs/<databaseId> --jq '{runner_name,labels,steps}'` — an empty `runner_name` with an empty `steps` array is the tell that nothing ran. **First check whether the run was superseded**: `gh run list --branch <ref> --limit 3 --json databaseId,headSha,status,conclusion` — a newer run on the same head means the cancellation was routine, so wait for that run instead of diagnosing. Only for a *not*-superseded run is capacity intelligence relevant, and then GitHub's own `notice` annotation is the evidence — but a notice on some other, older run does not identify this job's cause. Otherwise re-run just that job: `gh run rerun --job <databaseId>` (add `--debug` for a full log) and treat the retry as a probe, not a diagnosis. Measured 2026-11-13: a cancelled `mutation-test` on head `7adfd5b5` was a superseded run (a newer run existed on the same head), and the replacement run passed; 2026-11-12: a cancelled `Build CLI (macos-arm64)` passed 2m55s when re-run, with the cause never established |

## Allowed commitlint scopes (canonical)

See `commitlint.config.cjs` `scope-enum`. Common valid: `framework`, `retrieval`,
`ci`, `docs`, `workspace`. Prefer bare `docs:` over inventing `docs(plans)`.

## Merge order

Green independent first → CI fixes → dependent features. One squash-merge at a
time; never multi-PR `--auto` when base must be current.
