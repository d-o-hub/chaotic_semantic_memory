# Wave 21 P0 — Adoption Unblockers — Completion Note

**Date:** 2026-05-18
**Branch:** `feat/wave21-p0-complete`
**Author:** orchestrator agent
**Predecessor:** [GAP_ANALYSIS_2026_04_30.md](GAP_ANALYSIS_2026_04_30.md)

## Summary

Wave 21 P0 had three queued actions in [ACTIONS.md](ACTIONS.md):

| Action | ADR | Pre-session status | Post-session status |
|--------|-----|--------------------|---------------------|
| `implement_cli_framework_parity` | [0066](adr/0066-cli-framework-api-parity.md) | queued | **complete** ✅ |
| `implement_mcp_server` | [0067](adr/0067-mcp-server.md) | queued | **delegated → Jules #246** 🤖 |
| `backfill_missing_adrs` | [0076](adr/0076-adr-backfill.md) | queued | **complete** ✅ |

Investigation showed two of three actions were already complete in source
(GOAP state had drifted). One genuinely long-running task remained and was
delegated to a Jules-labeled GitHub issue per the standing workflow.

## Findings & evidence

### ✅ CLI parity (ADR-0066) — already complete in source

The installed `csm 0.3.5` binary on disk lacked the 11 promised
subcommands, which made the gap appear open. The local source tree on
`main` already wires all of them.

- [src/cli/args.rs#L46-L83](file:///home/do/git/chaotic_semantic_memory/src/cli/args.rs#L46-L83) — `Commands` enum has 22 variants
- [src/bin/csm.rs#L128-L177](file:///home/do/git/chaotic_semantic_memory/src/bin/csm.rs#L128-L177) — dispatch covers every variant
- [src/cli/commands/mod.rs](file:///home/do/git/chaotic_semantic_memory/src/cli/commands/mod.rs) — re-exports every `run_*` handler
- Verified locally:
  ```bash
  cargo build --bin csm --features cli
  ./target/debug/csm --help        # 22 commands listed
  ```

**Action taken:** added [tests/cli_parity.rs](file:///home/do/git/chaotic_semantic_memory/tests/cli_parity.rs)
with two smoke tests:

1. `cli_help_lists_every_expected_subcommand` — fails if any of the 22
   commands is missing from `--help` output. Locks the surface so a
   future refactor cannot silently drop a command.
2. `cli_each_subcommand_has_help` — verifies `csm <cmd> --help` exits
   zero for every command.

Result: `cargo test --test cli_parity --features cli` → **2 passed**.

### 🤖 MCP server (ADR-0067) — delegated to Jules

The MCP module exists but is mostly stubs:

- [src/mcp/tools.rs](file:///home/do/git/chaotic_semantic_memory/src/mcp/tools.rs) — 11 of 12 `handle_*` methods return `{"status": "ok", "... stub"}`; only `handle_associate` is wired.
- [src/mcp/resources.rs](file:///home/do/git/chaotic_semantic_memory/src/mcp/resources.rs) — all 3 resource handlers stubbed (`concept://`, `stats://`, `health://`).
- [src/mcp/server.rs#L50](file:///home/do/git/chaotic_semantic_memory/src/mcp/server.rs#L50) — `TODO: Wire up rmcp server with tools and resources`.
- No `csm mcp serve` subcommand.

This is a 16-cost task spanning protocol transports (stdio + SSE), 14
TODOs, integration tests, and a Claude Desktop smoke harness — well above
the threshold for a single interactive session.

**Delegated:** GitHub issue [#246](https://github.com/d-o-hub/chaotic_semantic_memory/issues/246)
with label `jules`. Will be picked up by jules.google.com per the
[jules-orchestration skill](file:///home/do/git/chaotic_semantic_memory/.agents/skills/jules-orchestration/SKILL.md).

### ✅ ADR backfill (ADR-0076) — already complete

`plans/ADR_REGISTRY.md` opens with:

> ADRs 0001-0056 were backfilled on 2026-05-01 from ACTIONS.md, git
> history, and handoff files. See ADR-0076 for the backfill process.

Parity check confirms only ADR-0003 is missing on disk and the registry
itself marks it `_Superseded by ADR-0008_, N/A`.

**Action taken:** added [scripts/check-adr-parity.sh](file:///home/do/git/chaotic_semantic_memory/scripts/check-adr-parity.sh)
which:

- Cross-references registry IDs against files in `plans/adr/` and `docs/adr/`.
- Exits non-zero on missing-with-backing files (excludes Superseded/N/A rows).
- Warns on orphan files present on disk but not yet in the registry.
- Output today: `ok: ADR parity satisfied (registry=78, disk=77)`.

The inline check in [scripts/validate.sh#L140-L170](file:///home/do/git/chaotic_semantic_memory/scripts/validate.sh#L140-L170)
already enforces the basic registry → disk direction. The new standalone
script extends to `docs/adr/` and reports orphans for ad-hoc use.

## Files changed

```
plans/ACTIONS.md                  # marked 3 Wave 21 P0 actions complete/delegated
plans/GOAP_STATE.md               # reconciled Wave 21 P0 booleans + action_last_completed
plans/WAVE_21_P0_COMPLETION.md    # this note
scripts/check-adr-parity.sh       # new — registry ↔ disk parity enforcer
tests/cli_parity.rs               # new — locks the 22-command CLI surface
```

## Next wave

Wave 21 P0 is done. The natural successor is **Wave 22 P1 — Capability
Ceiling Removal**:

- [ADR-0068](adr/0068-hnsw-ann-index.md) HNSW ANN index (cost 18, blocks Wave 24)
- [ADR-0069](adr/0069-embedding-model-bridge.md) Embedding bridge (cost 14, scaffolding present in [src/embedding/](file:///home/do/git/chaotic_semantic_memory/src/embedding/))
- [ADR-0070](adr/0070-graphrag-hybrid-retrieval.md) GraphRAG retrieval (cost 8, CLI surface already present as `probe-graph`)

Recommend cutting `v0.3.6` first to ship the merged-but-unreleased
DuckDB Phase 1–3 + hyperdim SIMD + framework events work before opening
Wave 22.
