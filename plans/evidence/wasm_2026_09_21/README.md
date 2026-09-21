# npm artifact evidence — 2026-09-21

Tier-3 evidence for the package `release.yml` publishes
(`@d-o-hub/chaotic_semantic_memory`), produced by `scripts/wasm-evidence.sh`
through the same build CI validates (`scripts/build-wasm.sh release-web`).

## What is here

| File | Content |
|---|---|
| `evidence.json` | Manifest: commit, dirty state, build mode/target, package version, `.wasm`/`.js`/`.d.ts` byte sizes and SHA-256s, smoke-test command + exit code, toolchain, hardware |
| `smoke.log` | Node runtime transcript of `node wasm/test.js` against the built package |

The built package itself (`pkg/`) is **not** committed: `*.wasm` is git-ignored
and the artifact is reproducible — three independent `release-web` builds on the
reference runner produced the same
`chaotic_semantic_memory_bg.wasm` (`sha256 6804814a4aea2885a10dd3ab11255a592d73aff3185cf4eab0df6e74443f5d27`,
656 657 bytes). CI uploads the exact package it smoke-tests as the
`wasm-pkg-release-web` artifact.

## Measured

| Field | Value |
|---|---|
| Version | 0.3.8 |
| `.wasm` | 656 657 B, `sha256:6804814a…` |
| JS glue | 38 984 B |
| TypeScript declarations | 11 517 B |
| Size gate | `scripts/wasm_size_gate.sh` (threshold 800 000 B, measured on this artifact) |
| Runtime smoke | `node wasm/test.js` → exit 0, including the `importFromBytes` round trip |

## Reproduce

```bash
scripts/wasm-evidence.sh plans/evidence/wasm_$(date -u +%Y%m%d)
```

The script builds the package, runs the smoke test, records the manifest and
fails the command when the smoke test exits non-zero.
