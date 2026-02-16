# chaotic_semantic_memory

Production-oriented Rust crate for chaotic semantic memory with:
- singularity mode concept injection/probe,
- Turso-style persistence surface,
- object-store checkpoint hooks,
- wasm-compatible bindings,
- GOAP-based agent orchestration skills under `.agents/skills`.

## Verify

```bash
cargo check
cargo test
cargo run --example singularity
TURSO_DATABASE_URL='file:/tmp/chaotic_semantic_memory_demo.db' TURSO_AUTH_TOKEN='local-token' cargo run --example turso_agent
cargo bench --bench benchmark
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
```

## GOAP Orchestration Utilities

```bash
python .agents/skills/goap-orchestrator/scripts/build_goap_plan.py "release_ready" plans/goap_plan_round6.json
python .agents/skills/goap-orchestrator/scripts/create_adr.py --slug goap-orchestration --title "Adopt GOAP-first specialist orchestration" --dir plans/adrs
python .agents/skills/goap-orchestrator/scripts/orchestrate.py --goal release_ready --plan plans/goap_plan_round6.json --tasks plans/missing_tasks_round7.json --out-json plans/goap_orchestration_round7.json --out-md plans/goap_orchestration_round7.md
```
