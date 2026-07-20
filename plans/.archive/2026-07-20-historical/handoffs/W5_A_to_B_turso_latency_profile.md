# W5 A -> B Handoff: Turso Latency Profile

## Action
- `benchmark_turso_roundtrip`

## Implementation
- Added integration gate test: `tests/turso_roundtrip.rs`
- Added local fallback latency gate: `tests/performance_targets.rs`
- CI wiring: `.github/workflows/ci.yml` runs Turso test with secret-backed env vars

## Query Profile
- Operation: `load_concept(id)` roundtrip after one seeded `save_concept`
- Samples: configurable via `CSM_TURSO_ROUNDTRIP_SAMPLES` (default: `25`)
- Pool size: configurable via `CSM_TURSO_POOL_SIZE` (default: `4`)
- Pass threshold: configurable via `CSM_TURSO_ROUNDTRIP_MAX_P50_MS` (default: `20ms`)

## Latest Local Run
- `LOCAL_ROUNDTRIP_P50_MS=0.475` (`tests/performance_targets.rs`)
- Turso remote measurement: skipped locally because `TURSO_DATABASE_URL` and `TURSO_AUTH_TOKEN` are not set

## Repro Commands
- `cargo test --test performance_targets -- --nocapture`
- `TURSO_DATABASE_URL=... TURSO_AUTH_TOKEN=... cargo test --test turso_roundtrip -- --nocapture`

## Handoff to Group B
- Use same operation mix and sample size (25) when aligning workload assumptions in memory sizing notes.
