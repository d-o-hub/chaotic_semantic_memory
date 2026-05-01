# CLI Reference

The `csm` command provides a full interface to the chaotic semantic memory framework.

## Global Options

- `--database <PATH>`: Path to libSQL/SQLite database file.
- `--git-local`: Use `.git/memory-index/csm.db` as storage.
- `--output-format <json|table|quiet>`: Control output formatting (default: `table`).
- `--verbose`: Increase logging verbosity (repeat for more detail).

## Commands

### Concept Lifecycle

#### `inject <ID> [--metadata JSON] [--from-file PATH]`
Inject a concept into memory. If ID exists, it is updated.

#### `get <ID>`
Retrieve full concept details including metadata and vector info.

#### `delete <ID>`
Remove a concept and all its associations from memory and persistence.

#### `update <ID> [--vector-from-text TEXT] [--metadata JSON]`
Update specific fields of an existing concept.

### Retrieval & Search

#### `query <TEXT> [-k N] [--min-score F]`
Search concepts using hybrid HDC + BM25 encoding.

#### `probe <ID> [-k N] [--threshold F]`
Find concepts similar to an existing concept's vector.

#### `probe-filtered <ID> -k N --filter JSON`
Search similar concepts while applying a metadata predicate.

#### `index-dir --glob PATTERN [--code-aware]`
Bulk index Markdown or source files.

#### `index-jsonl --file PATH [--field TEXT_FIELD]`
Bulk index concepts from a JSONL file.

### Graph Operations

#### `associate <FROM> <TO> [--strength F]`
Create a directed association between two concepts.

#### `disassociate <FROM> [<TO>]`
Remove an association between two concepts, or clear all outbound associations if TO is omitted.

#### `associations <ID> [--reverse]`
List all outbound (or inbound with `--reverse`) associations for a concept.

#### `traverse <START> [--depth N] [--min-strength F]`
Perform a breadth-first traversal of the association graph.

#### `path <FROM> <TO> [--weighted]`
Find the shortest path between two concepts. Use `--weighted` for Dijkstra (by association strength).

### System & Maintenance

#### `stats`
Show concept counts and database storage size.

#### `metrics [--reset]`
Show performance metrics (latencies, cache hits). Use `--reset` to clear counters.

#### `watch [--filter KIND]`
Stream memory events (injections, associations, deletes) as JSONL.

#### `export --output PATH [--format json|binary]`
Export entire memory state to a file.

#### `import <PATH> [--merge]`
Import memory state from a file.

#### `completions <SHELL>`
Generate shell completion scripts for Bash, Zsh, Fish, or PowerShell.
