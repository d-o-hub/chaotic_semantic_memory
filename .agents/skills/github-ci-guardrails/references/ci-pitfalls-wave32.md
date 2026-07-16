# CI pitfalls (Wave 32 / PR #518)

Use this checklist when CI fails or before claiming a PR is green.

## Pre-push (local)

```bash
npx commitlint --from origin/main --to HEAD --verbose
./scripts/generate-skill-catalog.sh --check
cargo clippy -- -D warnings
cargo check --manifest-path fuzz/Cargo.toml --all-targets --locked
```

## Failure → fix map

### commitlint `scope-enum`

- Scope must be in `commitlint.config.cjs`.
- Planning commits: use `docs`, `plans`, `goap`, or `agents` — not ad-hoc names.
- Or omit scope: `docs: update goap state`.

### commitlint `subject-case`

- Subject must not be UpperCase / Start-Case.
- Bad: `feat(framework): TTL lifecycle ...`
- Good: `feat(framework): add ttl cleanup ownership`

### wasm job / missing `chaotic_semantic_memory.js`

`wasm-pack build crates/csm-wasm --out-dir <path>` resolves **relative** paths
from `crates/csm-wasm/`, not the monorepo root.

Always pass absolute out-dir in CI:

```bash
--out-dir "${GITHUB_WORKSPACE}/wasm/pkg-ci-node"
```

### aarch64 `unreachable_code` (CLI matrix / macOS arm64)

After:

```rust
#[cfg(target_arch = "aarch64")]
{
    /* neon path */
    return results;
}
// fallback here is unreachable on aarch64 under -D warnings
```

Gate the fallback:

```rust
#[cfg(not(target_arch = "aarch64"))]
{ /* fallback */ }
```

### mutation timeouts / score 0%

Feature-disabled persistence stubs and Drop/TTL helpers that only compile under
special cfgs can yield **build timeouts** for every mutant. Timeouts do **not**
count as caught (ADR-0095). Exclude proven stubs in `scripts/mutation_test.sh`
or add tests that exercise the mutant symbols under default features.

### skill catalog stale

```bash
./scripts/generate-skill-catalog.sh
./scripts/generate-skill-catalog.sh --check
```

Generator must use `LC_ALL=C sort` and strip `wc -l` whitespace.

### cargo-fuzz short runs (PR fuzz job)

- **Required gate** is `cargo check --manifest-path fuzz/Cargo.toml --all-targets --locked`.
- Versions/timeouts/targets: **only** `.github/ci-settings.env` (workflows `source` it).
- Short runs need nightly + rust-src and
  `cargo install cargo-fuzz --version "${CARGO_FUZZ_VERSION}"`.
- Do **not** install via `taiki-e/install-action` `cargo-fuzz@x.y` (unsupported → binstall musl).
- Prefer `cargo +nightly fuzz run TARGET --sanitizer none` on PR runners.
- Optional escape: `FUZZ_SHORT_SECONDS=0` in ci-settings (or env) skips short runs.

### TTL cleanup lifecycle

- `timeout(handle).await` that fails **detaches** the task — call `handle.abort()`.
- Long `interval.tick()` blocks cooperative cancel — poll cancel between sleeps.

### BM25 absence short-circuit

- Do not skip BM25 for `--keyword-only` alone.
- Require concurrent HDC empty when short-circuiting.
- Invalidate absences on inject (`clear_all_absences` / `invalidate_absence_short_circuit`).
