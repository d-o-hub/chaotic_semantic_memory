# Local Gates

Run before pushing:

```bash
cargo check
cargo test --all-features
cargo fmt --check
cargo clippy -- -D warnings
cargo deny check                    # Supply chain audit (advisories, bans, licenses)
```

## LOC Gate (workspace-wide)

```bash
find src crates -name '*.rs' -not -path '*/target/*' -exec wc -l {} + | sort -rn | head -20
# Every file must be ≤ 500 LOC — applies to BOTH src/ and crates/
```

## Commitlint

When adding new workspace crates or package scopes, update `commitlint.config.cjs`:
- Add the scope name to `scope-enum` array
- Valid scopes: singularity, reservoir, framework, persistence, cli, cli-npm, wasm,
  retrieval, embedding, mcp, observability, bridge, duckdb, chaos, memory, core,
  traits, deps, ci, codacy, docs, release, clippy, lints, build, loc-gate, workspace

**Always validate the full PR range** (CI does this; last-commit-only is insufficient):

```bash
npx commitlint --from origin/main --to HEAD --verbose
```

Do not invent scopes (`ops`, `plans`, `goap` unless added to the enum first).
