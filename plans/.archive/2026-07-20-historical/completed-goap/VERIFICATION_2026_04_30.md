# Verification Report — 2026-04-30

End-to-end real-usage verification of v0.3.5 (crates.io + npm CLI + npm WASM all aligned), benchmark refresh, and clippy lint best-practice audit.

## Method

- Loaded skills: `release-management`, `testing-validation`, `dist-channel-selection`, `turso-memory-verification`, `memory-lifecycle-verification`, `benchmarking-perf`.
- Used the **installed** `csm 0.3.5` CLI (not a fresh local build) to mirror end-user experience.
- Executed lifecycle save → probe → export → import → re-probe contract.
- Refreshed three Criterion benchmark suites in `--quick` mode.
- Audited `Cargo.toml` `[lints]` + `.clippy.toml` against pedantic/nursery surface.

## Toolchain

| Component | Value |
|---|---|
| Rust | 1.93.0 (254b59607 2026-01-19) |
| Cargo | 1.93.0 (083ac5135 2025-12-15) |
| MSRV (Cargo.toml) | 1.85 |
| Edition | 2024 |
| Repo version | 0.3.5 |
| crates.io | `chaotic_semantic_memory = 0.3.5` ✅ |
| npm CLI | `@d-o-hub/csm@0.3.5` ✅ |
| npm WASM | `@d-o-hub/chaotic_semantic_memory@0.3.5` ✅ |

All three distribution channels aligned.

## Real-Usage Lifecycle (skill: memory-lifecycle-verification)

```bash
DB1=.tmp/lifecycle-2026-04-30.db
DB2=.tmp/lifecycle-roundtrip-2026-04-30.db
ART=.tmp/lifecycle-export-2026-04-30.json

csm --database "$DB1" inject "test::lifecycle::alpha" --metadata '{"phase":"save"}'
csm --database "$DB1" inject "test::lifecycle::beta"  --metadata '{"phase":"save"}'
csm --database "$DB1" associate "test::lifecycle::alpha" "test::lifecycle::beta" -s 0.9
csm --database "$DB1" probe "test::lifecycle::alpha" -k 3 --output-format json
csm --database "$DB1" export -o "$ART"
csm --database "$DB2" import "$ART"
csm --database "$DB2" probe "test::lifecycle::alpha" -k 3 --output-format json
```

| Phase | Result | Notes |
|---|---|---|
| inject (×2) | ✅ | JSON output: `status=created` |
| associate | ✅ | strength=0.9 persisted |
| probe | ✅ | beta returned (similarity=0.0055 — random HVs, expected low) |
| export | ✅ | 4128 bytes JSON, 2 concepts captured |
| import (roundtrip) | ✅ | "existing state cleared" warning, 2 concepts re-imported |
| re-probe | ✅ | identical similarity score → roundtrip preserves vectors |
| **archive** | ⚠ skipped | no native CLI; gap analysis F1 |
| **delete** | ⚠ skipped | no native CLI; ADR-0066 will add `csm delete` |

**Outcome: all available lifecycle phases pass; missing phases are tracked in ADR-0066.**

## Benchmark Refresh (skill: benchmarking-perf)

### bm25_benchmark (`--quick`)

| Bench | Result | Target | Δ vs 2026-04-29 |
|---|---|---|---|
| `bm25_search_1000` | 47.1 µs | < 5 ms | -27% (was 64.4 µs) ✅ |
| `bm25_replace_doc` | 472 ns | — | +2% (was 463 ns) — noise |

### benchmark.rs (full suite, abbreviated)

| Bench (representative) | Time |
|---|---|
| `reservoir_step` (small) | 6.46 ns |
| `cosine_similarity` | 561 ns |
| `hvec_random` | 5.03 µs |
| `hvec_bind` | 1.30 µs |
| `singularity_probe_50000` | 3.73 ms ✅ (target < 10 ms) |
| `batch_similarity_1000` | 280-300 µs ✅ (target < 500 µs) |
| `inject_concept_metadata` | 1.40 ms |
| Full sequence (10 inputs) | 22.2 ms / 27.5 ms (β=0 / β=0.15) |

All within or below targets.

### persistence_benchmark.rs

| Bench | Time |
|---|---|
| `persistence_cold_start` | 695-710 µs |
| `delete_concept` | 1.65 ms |
| `save_association` | 1.92 ms |
| `crud_roundtrip` | 2.25 ms |

Persistence layer is healthy.

## Clippy Best-Practice Audit

### Current state

```bash
cargo clippy --all-features --all-targets -- -D warnings
# → green ✅
```

`Cargo.toml` enables `clippy::all = warn` and `[lints.rust]` baseline.
`.clippy.toml` sets `cognitive-complexity-threshold = 30`, `too-many-lines-threshold = 100`.

### Probe with pedantic + nursery

```bash
cargo clippy --all-features --all-targets -- -W clippy::pedantic -W clippy::nursery
# → 936 warnings
```

### Top categories (correctness vs cosmetic)

| Lint | Count | Category | Worth promoting? |
|---|---|---|---|
| `uninlined_format_args` | 207 | cosmetic | no |
| `missing_errors_doc` | 121 | DX | yes (Phase C) |
| `doc_markdown` | 98 | cosmetic | no |
| `semicolon_if_nothing_returned` | 55 | cosmetic | no |
| `must_use_candidate` | 51 | DX/safety | yes (Phase C) |
| **`float_cmp`** | **44** | **correctness** | **yes (Phase B)** |
| `redundant_else` | 34 | cosmetic | no |
| `missing_const_for_fn` | 25 | perf | yes (Phase B) |
| **`significant_drop_tightening`** | **21** | **correctness/perf** | **yes (Phase B)** |
| `must_use_self` | 18 | DX | optional |
| **`cast_precision_loss`** | **13** | **correctness** | **yes (Phase B)** |
| **`cast_possible_truncation`** | **8** | **correctness** | **yes (Phase B)** |
| `redundant_clone` | 7 | perf | yes (Phase B) |

### Recommendation → ADR-0077

Promote 5 correctness/perf lints from blanket-allow to `warn`:

```toml
float_cmp = "warn"
significant_drop_tightening = "warn"
cast_precision_loss = "warn"
cast_possible_truncation = "warn"
redundant_clone = "warn"
missing_const_for_fn = "warn"
```

Estimated 110 sites to fix across 5 themed PRs (≤ 250 LOC each).

DX lints (`missing_errors_doc`, `must_use_candidate`) deferred to Phase C — total 172 doc additions.

## Findings → Plan Updates

| Finding | Action | Plan ref |
|---|---|---|
| CLI lifecycle missing delete/get/stats | Already in roadmap | ADR-0066 |
| Distribution channels v0.3.5 aligned | No action needed | — |
| Benchmarks within targets | No regression | — |
| `bm25_search_1000` improved 27% | Update perf table in skill | benchmarking-perf SKILL.md |
| Pedantic surface 936 / 110 actionable | New ADR | **ADR-0077** |
| Probe-by-id quirk: probing alpha returns beta | Document — `probe <id>` queries by neighbors of id, alpha itself not returned | book/src/cli-reference.md |

## GOAP Effects

Update `plans/GOAP_STATE.md`:

```yaml
verification_2026_04_30_completed: true
verification_2026_04_30_lifecycle_pass: true
verification_2026_04_30_benchmarks_pass: true
verification_2026_04_30_dist_channels_aligned: true   # crates.io + npm CLI + npm WASM all 0.3.5
verification_2026_04_30_clippy_audit: true
clippy_pedantic_surface_warnings: 936
clippy_actionable_warnings: 110
adr_0077_clippy_promotion_drafted: true
```

## References

- [plans/GAP_ANALYSIS_2026_04_30.md](GAP_ANALYSIS_2026_04_30.md) — feature roadmap
- [plans/adr/0066-cli-framework-api-parity.md](adr/0066-cli-framework-api-parity.md) — addresses missing CLI surface
- [plans/adr/0077-clippy-pedantic-selective-promotion.md](adr/0077-clippy-pedantic-selective-promotion.md) — clippy hardening plan
- `benchmarks/results/verify-2026-04-30/` — raw bench output (artifact dir)
