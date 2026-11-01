# Intra-workspace version pins: why they drift and what actually detects it

Issue #964, measured on `cae70539` (2026-11-01).

## The state that produced it

`chaotic_semantic_memory` is one root package plus 11 workspace members. Every dependency
edge between them carries an explicit `version = "x.y.z"` next to its `path = "…"`, so the
workspace has 26 such edges — and **18 of them declared `0.3.8`** (and `fuzz/Cargo.toml`
declared `0.3.7`, two releases behind) while every package's own `version =` said `0.3.9`.

Nothing broke. That is the entire reason it survived three releases:

- `version = "0.3.8"` means `^0.3.8`, and `0.3.9` satisfies it — Cargo resolves, builds,
  tests, and publishes.
- `Cargo.lock` records **resolved package versions**, not requirement strings, so rewriting
  a pin causes no lock churn. Every lock-based gate stayed green by construction.
- The cost lands on the first `0.4.x`: `csm-duckdb@0.4.0` would publish a manifest asserting
  compatibility with a root `^0.3.8` that no longer exists in that form. 7 of the 9 crates.io
  artifacts ship that metadata today.

## What the two existing tools actually covered

| tool | its claim | the blind spot |
|---|---|---|
| `sync-version.sh` | "updates workspace member versions" | rewrote `^version =` in `crates/*/Cargo.toml` (own versions) plus a root-only sed for `path = "crates/csm-…", version = "…"` — i.e. **one direction of one file**. Member → root, member → sibling, `[dev-dependencies]`, `[target.'cfg(..)'.dependencies]` were never touched. |
| `verify-version-sync.sh` | "✓ All local versions synchronized" | compared root version against `wasm/package.json`, `cli-npm/package.json`, `VERSION`, and `find crates -name Cargo.toml`'s own `version =`. It never read a **dependency** pin in either direction, and its glob could not see `benchmarks/`. |

So the verifier printed a success sentence about a different fact — the pattern in
`shell-script-quality`'s "Run the control, don't hope for it".

## Re-measuring the issue's own number

The issue said 16. It was 18, and the widening is instructive because each step was a
different enumeration method, not a different bug:

| count | method | what it missed |
|---|---|---|
| 16 | single-line `{ path = …, version = … }` grep | `fuzz/Cargo.toml`, whose pin is a multi-line `[dependencies.chaotic_semantic_memory]` table |
| 17 | tomllib over `crates/*` + root | `benchmarks/Cargo.toml` — a declared `[workspace]` **member** that no `crates/*` pattern reaches |
| 18 | a pruned filesystem walk of every `Cargo.toml`, cross-checked against `[workspace] members` | (nothing; this is the enumerator) |

## The tool that replaced both

`scripts/workspace-version-edges.py` answers `--list`, `--check` and `--sync <ver>` from
**one parser**, so the writer and the gate cannot disagree about what a pin is. Design
points worth keeping in mind when you extend it:

- **Line-oriented on purpose.** tomllib answers "what does this mean" but not "which line,
  which column", and `--sync` must rewrite one token without reformatting anyone else's
  manifest. tomllib is still consulted: `cross_check()` compares the edge set **and each
  edge's version value** against it, and any disagreement is exit 2. `version = '0.3.8'`
  (single quotes) is valid TOML that a `"` regex cannot see — an edge-set-only cross-check
  would call that "no pin to sync" and report clean.
- **`workspace = true` is not drift** — there is no literal to compare. A path edge with no
  `version` key at all is also not drift, but it is still counted and cross-checked.
- **A pin to a non-member is not the workspace's business.** A manifest the walk reaches
  that declares no member dependency is parsed and excluded from the edge count by name.
- **Exit 2 is not a verdict.** No manifests, no tomllib, a non-semver `--sync` argument, a
  malformed table, an escaping manifest, an undiscoverable declared member, or a parser
  disagreement all exit 2. `refuse()` exists because
  `SystemExit("msg")` exits **1**, which would have reported "could not audit" as drift.
- **Discovery is cross-checked, not announced.** It walks the filesystem — reaching a member
  before anyone `git add`s it, and needing no git at all — and prunes `target`, `.git`,
  `.qoder`, `node_modules` and `vendor`, because `cargo package` unpacks stale copies into
  `target/` and dispatched worktrees under `.qoder/` would otherwise make the gate red about
  bytes nobody can edit. The prune is not trusted: `check_member_coverage()` asks the root
  manifest which members *must* exist and exits 2 if the walk missed one. Printing
  "warning: I only looked at half the tree" and returning 0 was the previous design, and it
  was still a sensor narrowing its own scope while keeping its old verdict.
- **Containment is decided once, at the boundary.** `resolve_root()` realpath's the caller's
  path and every manifest is derived from it, so `keep_inside()`/`escapes()` can refuse a
  symlinked file *or a symlinked directory* that resolves outside the root — refuse, not
  skip, because skipping prints "every pin matches" about bytes it never opened. Symlinked
  directories are followed (each realpath entered once) and manifests are keyed by realpath,
  so a file reachable along two routes is one edge.

## Numbers to quote when someone says the gate is over-engineering

- Tool fixture: 77 cases, 0 failures; 9/9 original source mutations turn a direction red,
  and the discovery redesign's sweep caught 4 of 5 — the survivor is documented in
  `shell-script-quality`'s sensor-sharpness §5 as the redundant-guard case (a guard whose
  failure mode is wasted work has no exit code that can see it).
- `verify-version-sync.sh` fixture: extended 4 → 9 tests; the first run failed on the
  pre-existing mock because a copied-alone verifier cannot find its sibling tool. Second
  neuter sweep: 6/6 mutations caught (the first sweep let 2 through, which is what tests 7
  and 8 were written for).
- The fix itself: 18 pins rewritten across 11 manifests, `benchmarks`' own version
  `0.3.6 → 0.3.9`, one `Cargo.lock` line, `cargo metadata --locked` clean at the root and in
  `fuzz/`.

## Related

- #982 — the docs/tests/examples sweep has the same shape of blindness: it substitutes
  `$CURRENT_VERSION`, so literals older than the previous release survive it (seven
  occurrences across five files sit in `tests/`+`examples/` right now, and the staleness
  probe's hardcoded `0\.2\.[0-4]` pattern matches zero of them).
- `shell-script-quality/references/sensor-sharpness.md` — the general doctrine.
