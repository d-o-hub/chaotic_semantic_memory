# Hard Constraints

- Source files: `<= 500 LOC` each.
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
