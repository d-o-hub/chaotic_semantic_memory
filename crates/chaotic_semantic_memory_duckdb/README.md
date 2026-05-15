# chaotic_semantic_memory_duckdb

Optional analytics, Parquet export, and SQL inspection for `chaotic_semantic_memory`.

## Usage

```rust
use chaotic_semantic_memory_duckdb::Analytics;

let mut analytics = Analytics::open_in_memory()?;
analytics.load_export_json("export.json")?;

let batch = analytics.query("SELECT count(*) FROM concepts")?;
println!("Concepts: {:?}", batch);
```

## Planned Features

- **Planned:** **DuckDB Integration**: Run SQL queries over your semantic memory.
- **Planned:** **Parquet Export**: Export concepts and associations to Apache Parquet for external OLAP processing.
- **Planned:** **Analytic Views**: Pre-defined SQL views for centrality, connectivity, and pattern analysis.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
chaotic_semantic_memory_duckdb = { version = "0.1" }
```
