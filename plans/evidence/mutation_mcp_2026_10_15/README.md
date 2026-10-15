# `src/mcp` mutation baseline — 2026-10-15 (GOAP #833)

First mutation evidence for the feature-gated MCP module, and the profile change
that keeps it in scope. Written by the session that ran the numbers; every
figure below comes from a `cargo-mutants` run recorded in this directory.

## Why this exists

`src/mcp/**` was invisible to **every** mutation run, for two independent
reasons: the module is `#[cfg(feature = "mcp")]` and `mcp` is not a default
feature, and `scripts/mutation_test.sh` excluded the path outright
(`--exclude-re "mcp::"`, `--exclude-re "McpHandler::"`, `--exclude "src/mcp/*"`).
#817 fixed the first half (the fast profile now passes
`--features chaotic_semantic_memory/mcp`) while the *path* stayed excluded, so the
module had no baseline at all: `grep -c "src/mcp"` was 0 in every
`mutants.out/{caught,missed}.txt` and in `outcomes.json`. This change lifts the
three exclusions and lands the lib-target tests that make the lift meaningful.

## Commands

Toolchain: `rustc 1.88.0` (`rust-toolchain.toml`), `cargo-mutants 27.1.0`,
`--lib -p chaotic_semantic_memory`, feature `chaotic_semantic_memory/mcp`.
All runs are `--in-place --no-shuffle` (the CI profile's own shape; a scratch-tree
run needs `TMPDIR` that breaks this repo's six path-guarded tests, measured).

Baseline, before any test was added (34 missed / 18 caught / 8 unviable):

```bash
cargo mutants --file 'src/mcp/*.rs' --exclude 'src/mcp/*_tests.rs' \
  --features chaotic_semantic_memory/mcp \
  --timeout 120 --minimum-test-timeout 15 --build-timeout 900 \
  --in-place --no-shuffle --output /tmp/mcp-mutants \
  -- -p chaotic_semantic_memory --lib
```

Re-verification, after the tests and the two documented exclusions
(37 filtered mutants, 37 caught):

```bash
cargo mutants --file 'src/mcp/*.rs' --exclude 'src/mcp/*_tests.rs' \
  --exclude-re 'replace serve ->' \
  --exclude-re 'replace \| with \^ in parse_hvec_u64_halves' \
  --re 'map_error|get_info|initialize|list_tools|list_resources|tool_def|_schema ->|serve_with_shutdown|delete match arm|replace == with !=|execute_read_resource' \
  --features chaotic_semantic_memory/mcp \
  --timeout 120 --minimum-test-timeout 15 --build-timeout 900 \
  --in-place --no-shuffle --output /tmp/mcp-mutants-verified \
  -- -p chaotic_semantic_memory --lib
```

## Baseline (pre-change): 60 mutants, 34 missed / 18 caught / 8 unviable

| file | missed | caught | unviable |
|------|-------:|-------:|---------:|
| `src/mcp/handler.rs` | 6 | 1 | 5 |
| `src/mcp/schema.rs` | 13 | 0 | 0 |
| `src/mcp/server.rs` | 2 | 2 | 0 |
| `src/mcp/tools.rs` | 13 | 15 | 3 |
| **total** | **34** | **18** | **8** |

Machine-readable copy: `baseline_outcomes.json` (per-mutant name + verdict).

## Triage of all 34 survivors

| survivor (34) | disposition | mechanism |
|---|---|---|
| 13 × `schema.rs: ` `*_schema -> Default::default()` | **killed** | `handler_tests.rs::every_advertised_tool_schema_names_and_requires_its_arguments` — table of the 13 tools asserting declared + required properties; the mutant returns `Null`, which is not an object |
| 8 × `delete match arm "memory_*"` (`probe_text`, `probe_filtered`, `associate`, `traverse`, `shortest_path`, `stats`, `export`, `list_gaps`) | **killed** | `execute_tool_dispatches_every_advertised_tool` (positive shape per tool) + `each_tool_arm_rejects_its_own_missing_arguments` (a deleted arm answers `Tool not implemented`, not `Missing to_id`) |
| `handler.rs`: `map_error`, `get_info`, `tool_def` | **killed** | direct assertions on the returned `ErrorData` / `ServerConfig` / `Tool` fields |
| `handler.rs`: `initialize`, `list_tools`, `list_resources` | **killed** | `wire_tests.rs::wire_handshake_lists_every_tool_and_resource` — a real rmcp server over a `tokio::io::duplex` transport, spoken to with newline-delimited JSON-RPC (`initialize` → `notifications/initialized` → `tools/list` → `resources/list`); asserts the server identity/version/capabilities, all 13 tool names + a tool schema, and the three resource URIs |
| `server.rs`: `serve_with_shutdown -> Ok(())` | **killed** | `server_tests.rs::serve_with_shutdown_returns_when_the_signal_resolves` now requires the listener to *accept a connection* before the shutdown signal is sent; a body that returns `Ok(())` immediately never binds, so the readiness loop fails it |
| `tools.rs`: `execute_read_resource -> Ok(Default::default())`, 2 × `replace == with !=` | **killed** | `read_resource_serves_every_advertised_uri_and_rejects_unknown_ones` — `concept://` hit *and* miss, `stats://current`, `health://current`, unknown URI |
| `tools.rs`: `replace == with !=` in `execute_tool` (`format == "binary"`) | **killed** | the export loop asserts the written file's *content* per format (binary must not parse as JSON) |
| `server.rs`: `serve -> Ok(())` | **excluded**, documented | entry point whose only observable is a process receiving SIGINT/SIGTERM; the hop is pinned by `server_tests.rs::sse_exit_stops_the_ttl_cleanup_task` (reaper slot `Some` → signal → await → `None`) and by `tests/cli_shutdown_signal.rs::serve_sse_stops_cooperatively_on_sig{term,int}` (integration profile) |
| `tools.rs`: `replace | with ^ in parse_hvec_u64_halves` | **excluded**, documented | equivalent mutant: operands are `u128::from(high) << 64` (bits 64..128) and `u128::from(low)` (bits 0..64), disjoint by construction, so `|` and `^` agree on every input. The sibling `| -> &` and all seven shift/arithmetic mutants in that function are caught |
| 8 × `Default::default()` on types without `Default` (`framework` leak, `call_tool`, `read_resource`, `res_def`, `with_ttl_cleanup_interval`, 3 × `parse_hvec*`) | **unviable** | the mutant does not compile — not a test weakness |

## Verification

`37 mutants tested in 68m: 37 caught` (run `/tmp/mcp-mutants-verified`).

The filtered set is the 32 survivors triaged as killable **plus the five
pre-existing controls** (`memory_inject`, `memory_inject_text`, `memory_probe`,
`memory_get`, `memory_delete` arm deletions) that were already caught before this
change — they re-confirm the harness and prove the new tests did not disturb the
old kills. Adding tests cannot un-catch a mutant, so the other 13 baseline-caught
mutants (including `serve_with_handler` and `run_sse_server_with_shutdown`) were
not re-run.

Post-change arithmetic: **60 generated − 2 documented exclusions = 58 tested;
58 − 8 unviable = 50 killable; 50 caught** (18 from the baseline + 32 from the
new tests) = **100 % of killable mutants**, with no undocumented survivors.

Two mutants touch the export path and were re-verified after the test file's
last edit (a `Drop` guard that removes the artifact even when an assertion
fails, so a mutant cannot leave `export_<epoch>.json` in the checkout).

## Scope notes and residuals

- **Test modules produce no mutants here.** Control: `--file 'src/mcp/tools_tests.rs'`
  lists 0 while `--file 'src/mcp/schema.rs'` lists 13, so the baseline's
  `--exclude 'src/mcp/*_tests.rs'` matches the profile's scope exactly — the
  profile mutates `src/mcp/*.rs` only in practice.
- **The `mcp::` exclude was inert, not load-bearing.**
  `--exclude-re` matches the mutant *description*, never a module path; the
  `McpHandler::` label was the one that actually hid 12 → 8 mutants in
  `handler.rs`. Both are removed; only `replace serve ->` and the `| -> ^`
  equivalence remain documented.
- **CI budget, measured.** Unmutated baseline build 82 s + 20 s test; per-mutant
  rebuilds ran 86–108 s with `CARGO_BUILD_JOBS=2` and 3–6 s of tests. The PR job
  is `--in-diff` with `timeout-minutes: 45` and a pre-warm step capped at 25 min,
  so the mutant count follows the *changed lines* of a diff. A PR rewriting all of
  `src/mcp/tools.rs` (31 mutants) would need ~35–50 min of mutant time and could
  hit the job budget; the honest mitigation is a per-file mutant cap or a raised
  job timeout, neither of which this change does. It is a residual, not a
  regression: before this change those lines produced no results at all.
- **Not covered by any profile:** the `serve` entry-point body (excluded with its
  mechanism above) and the eight unviable mutants. Both are listed rather than
  glossed.
