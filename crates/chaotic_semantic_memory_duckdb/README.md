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

## Features

- **DuckDB Integration**: Run SQL queries over your semantic memory.
- **Parquet Export**: Export concepts and associations to Apache Parquet for external OLAP processing (Polars, Spark, BI).
- **Analytic Views**: Pre-defined SQL views for centrality, connectivity, and pattern analysis.

### Parquet Export Example (Polars + Python)

Once you've exported your memory to Parquet:

```rust
let opts = ParquetExportOptions::default();
analytics.export_all_parquet("./export_dir", &opts)?;
```

You can read it in Python using Polars:

```python
import polars as pl

# Read concepts and join with associations
concepts = pl.read_parquet("export_dir/concepts.parquet")
associations = pl.read_parquet("export_dir/associations.parquet")

# Example: find top-10 most connected concepts
top_connected = (
    associations
    .group_by("src_id")
    .count()
    .sort("count", descending=True)
    .limit(10)
    .join(concepts, left_on="src_id", right_on="id")
)
print(top_connected)
```

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
chaotic_semantic_memory_duckdb = { version = "0.1" }
```
