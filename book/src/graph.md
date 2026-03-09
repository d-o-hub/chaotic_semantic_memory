# Graph Traversal

`Singularity` stores weighted directed associations and exposes graph traversal APIs.

## Available Traversals

- `neighbors(id, min_strength)` for one-hop expansion
- `bfs(start, config)` for bounded breadth-first traversal
- `shortest_path(from, to, config)` for weighted Dijkstra traversal
- `shortest_path_hops(from, to, config)` for hop-count pathing

## Example

```rust,no_run
use chaotic_semantic_memory::graph_traversal::TraversalConfig;
use chaotic_semantic_memory::prelude::{ConceptBuilder, Result, Singularity};

fn main() -> Result<()> {
    let mut sing = Singularity::new();
    sing.inject(ConceptBuilder::new("a").build()?)?;
    sing.inject(ConceptBuilder::new("b").build()?)?;
    sing.inject(ConceptBuilder::new("c").build()?)?;
    sing.associate("a", "b", 0.9)?;
    sing.associate("b", "c", 0.8)?;

    let cfg = TraversalConfig::default();
    let bfs = sing.bfs("a", &cfg)?;
    let path = sing.shortest_path("a", "c", &cfg)?;
    assert!(!bfs.is_empty());
    assert!(path.is_some());
    Ok(())
}
```

## Weighted Cost Model

`shortest_path` converts edge strengths to costs using `-ln(strength)`.

- Higher strength -> lower cost
- `strength <= 0.0` is treated as non-traversable
- This preserves intuitive preference for stronger semantic links

## WASM Support

WASM bindings expose:

- `neighbors(id, min_strength)`
- `bfs(start)`
- `shortest_path(from, to)`
