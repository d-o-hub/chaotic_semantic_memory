# PR/Issue Roast — 2026-11-08 open issue queue

Scope: the six open issues at session start (992, 1002, 1003, 1004, 1005, 1014).
Each premise was re-measured against this tree before any code was written
(`AGENTS.md` Phase 1 step 5). Verdicts are per issue, not inherited from the
issue text.

| Issue | Verdict | Evidence |
|---|---|---|
| #992 path selection | **keep — valid, implement** | `mcp` regex is `^(src/mcp/\|Cargo\.(toml\|lock)$\|ci\.yml$)`: `src/cli/mcp.rs` and `tests/mcp*.rs` do not match. `cli_cross` does match `src/cli/`, so only the dedicated MCP job is skipped. No contract fixture exists. |
| #1002 SHA pin validator | **keep — valid and live** | `validate.sh` only runs it when `CSM_VALIDATE_GITHUB_ACTIONS_SHAS=true`, passes `--offline`, and no workflow sets the variable. Online run today: 118 actions checked, 0 errors. `cf430e03` is a valid *upload-artifact* commit (v7.0.2) and is invalid for *download-artifact* (HTTP 422) — the exact cross-repo misuse the gate must catch. |
| #1003 catalog fixture detector | **keep — valid, implement** | `fixture_for()` probes only `test-{stem}`; `scripts/mutation_union.py` has `scripts/test-mutation-union.sh` and the catalog prints `none found`. |
| #1004 DuckDB cache pollution | **keep — valid, and it is the current `main` blocker** | `main` tip `1210f732` run 42090681886 is red in `lint` Clippy with `libduckdb-sys v1.20000.1` → `DirectoryNotEmpty` at `build_bundled_cc.rs:65`. `libduckdb-sys` publishes `duckdb-sources/<hash>` by `fs::rename`; a cached tree without `.complete` makes the rename fail before any test. `lint`, `test`, `test-duckdb` and one `test-workspace-crates` leg all build it through shared caches. **Stale acceptance item:** `ci_all_checks_passed` was deliberately deleted from `GOAP_STATE.md` (derived CI truth); the green-run id is recorded in `progress/`, never restored as a stored key. |
| #1005 mutation build scope | **keep — valid, implement** | `TEST_ARGS+=(--lib)` reaches only the `--` test invocation. Probed cargo-mutants 27.1.0 on a throwaway crate: `--cargo-arg --lib` puts `--lib` on the `cargo test --no-run` build command too (`mutants.out/debug.log`), so the build no longer compiles all integration targets. |
| #1014 crates.io 0/9 | **keep open — operator-only; repo half is PR #1015** | The nine trusted-publisher configs, the v0.3.9 recovery dispatch and the token revocation require crates.io account access; no repository change can perform them. `publish-crates.yml` + ADR-0104 exist only on the unmerged #1015 branch, so #1014 cannot be satisfied until #1015 lands and the operator performs steps 1–4. |

## Ordered merge plan

1. #1004 — unblock `main` (`lint` is a required ruleset context).
2. #992 — path classifier (touches the same workflow; rebase after #1004).
3. #1003 — catalog fixture resolver (scripts only).
4. #1002 — SHA pin gate (workflow + validator; rebase after #992).
5. #1005 — mutation build scope (`mutation_test.sh` + fixture).
6. #1015 (existing PR) / #1014 — land the repo half, then leave the operator steps open.

One PR at a time, each rebased onto the latest `main` with CI re-run on that
head (`AGENTS.md` Phase 5 step 18). No `--auto`.
