# Diagram Template

- Components: `framework`, `hyperdim`, `reservoir`, `singularity`, `turso`, `wasm`.
- Runtime boundaries: native async runtime, wasm runtime.
- Persistence paths: Turso/libSQL path, object-store checkpoint path.
- Data flow arrows: inject -> singularity -> retrieve, persist/restore -> turso.
- Keep labels identical to Rust type names where possible.
