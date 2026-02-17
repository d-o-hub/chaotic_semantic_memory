# W5 A -> B Handoff: Turso Latency Profile

## Action
- `benchmark_turso_roundtrip`

## Required Inputs
- Turso endpoint profile used for benchmark
- Query mix and payload size assumptions
- Concurrency level and connection pool settings

## Output Contract
- Report p50/p95 roundtrip latency in ms
- Explicit pass/fail against target: `p50 < 20ms`
- Note environmental caveats affecting reproducibility

## Consumption by Group B
- Reuse query/payload assumptions in memory-footprint scaling model
- Keep benchmark environment metadata aligned across Wave 5 reports
