# Hard Constraints

- Source files: `<= 500 LOC` each (applies to BOTH `src/` AND `crates/`). Pre-check before every session:
  ```bash
  find src crates -name '*.rs' -not -path '*/target/*' -exec wc -l {} + | sort -rn | head -20
  ```
- `SKILL.md` (`.agents/skills/` folder): `<= 250 LOC`; detailed references in `reference/`, `scripts/`, or `assets/`.
- Use `libsql` (never `turso-client`).
- Use Tokio async/await for I/O.
- Use Rayon for CPU parallelism.
- All fallible public APIs return `Result<T, Error>`.
- Reservoir spectral radius must stay in `[0.9, 1.1]`.
- WASM threading paths must be gated with `#[cfg(not(target_arch = "wasm32"))]`.
- No hardcoded runtime settings or magic numbers in production paths; use named constants and configurable env/config values.
- Never render architecture diagrams as raw ASCII art in responses; use fenced ```mermaid``` blocks for all inline diagrams.
- **Never use an archived GitHub repository as a reference package or dependency.**
  Always either:
  1. Find an actively maintained alternative crate on crates.io, OR
  2. Fork the archived repo into `d-o-hub/` and maintain it yourself.
  Before adding any dependency, verify its repository is not archived via:
  ```bash
  gh repo view <owner>/<repo> --json isArchived,pushedAt
  ```
- **Supply chain advisories must pass** before any release. Run `cargo deny check`
  and address failures (upgrade deps or document ignores in `deny.toml`).
- **Commitlint scopes must be kept current.** When adding a new workspace crate
  or package scope, also add the scope to `commitlint.config.cjs` `scope-enum` list.
  Allowed scopes include package names (`framework`, `ci`, `wasm`, …) plus planning
  scopes `plans`, `goap`, `agents`. **Before every push**, validate the whole PR range:
  ```bash
  npx commitlint --from origin/main --to HEAD --verbose
  ```
  Subject must not start with UpperCase tokens (`TTL lifecycle` fails; use `ttl cleanup`).
  Scope `plans` is allowed; bare `docs:` (no scope) is also fine.

- **CI pitfalls (Wave 32 / PR #518 — do not reintroduce):**
  1. **wasm-pack out-dir is relative to the crate directory**, not the repo root.
     When building `crates/csm-wasm`, pass an **absolute** `--out-dir`
     (e.g. `${{ github.workspace }}/wasm/pkg`). Relative `wasm/pkg` lands under
     `crates/csm-wasm/wasm/pkg` and breaks smoke tests.
  2. **aarch64 + early `return` after NEON paths:** any fallback code after a
     `#[cfg(target_arch = "aarch64")] { ... return; }` block is unreachable under
     `-D warnings`. Gate fallbacks with `#[cfg(not(target_arch = "aarch64"))]`.
  3. **Mutation / feature-disabled stubs:** modules that only return
     `UnsupportedOperation` under `not(feature = "persistence")` produce build
     timeouts when mutated. Exclude proven-stub paths in `scripts/mutation_test.sh`
     or ensure unit tests exercise them under the feature matrix.
  4. **TTL background tasks:** `tokio::time::timeout` on a `JoinHandle` that expires
     **detaches** the task. Always `abort()` (or re-store the handle for `Drop`) on
     timeout. Poll cancel during long intervals; do not block solely on `interval.tick()`.
  5. **Absence short-circuit:** never skip BM25 for `--keyword-only` alone; require
     concurrent HDC-empty; **invalidate** absence rows on inject/corpus change.
  6. **Skill catalog:** generate with `LC_ALL=C sort` and strip `wc -l` whitespace so
     CI and local catalogs byte-match (`scripts/generate-skill-catalog.sh --check`).
- **Version numbers must be synchronized across all files before release.**
  Run `scripts/verify-version-sync.sh` to verify. Files checked:
  - `Cargo.toml` - `version = "X.Y.Z"`
  - `wasm/package.json` - `"version": "X.Y.Z"`
  - Test fixtures and examples with `"version":` literals
- **Lint policy**: `unwrap_used`/`expect_used`/`panic` are `warn` in workspace lints (errors in CI).
  Tests are exempt via `.clippy.toml` (`allow-unwrap-in-tests = true` etc.).
  Production allows require `#[allow(...)]` with a justification comment.