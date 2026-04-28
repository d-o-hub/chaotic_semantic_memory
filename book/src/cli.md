# CLI Usage

The `csm` binary provides command-line access to the memory system.

## Installation

```bash
cargo install chaotic_semantic_memory --features cli
```

## Commands

### inject

Inject a new concept:

```bash
# Random vector
csm inject my-concept --database csm_memory.db

# Vector from file
csm inject my-concept --from-file vector.bin --database memory.db

# Vector from stdin
echo "vector data" | csm inject my-concept --vector-source stdin --database memory.db

# With metadata
csm inject my-concept --metadata '{"key":"value"}' --database memory.db
```

### probe

Find similar concepts:

```bash
# By concept ID
csm probe my-concept -k 10 --database csm_memory.db

# Output as JSON
csm probe my-concept -k 10 --output-format json --database memory.db
```

### query

Text-based similarity search (encodes text and finds similar concepts):

```bash
# Hybrid search (keyword + semantic)
csm query "machine learning neural networks" -k 10 --database csm_memory.db

# Semantic-only HDC search
csm query "machine learning" --semantic-only -k 5 --database memory.db

# Keyword-only BM25 search
csm query "exact term match" --keyword-only -k 5 --database memory.db

# With minimum score threshold
csm query "AI concepts" -k 10 --min-score 0.5 --database memory.db

# Compact output (trim long text)
csm query "long document content" --compact -k 5 --database memory.db

# Code-aware encoding for source code
csm query "fn process_data" --code-aware -k 5 --database memory.db
```

### index-dir

Index Markdown files from a directory into memory:

```bash
# Index all markdown files in a directory
csm index-dir -g "docs/**/*.md" --database csm_memory.db

# Index with multiple glob patterns
csm index-dir -g "src/**/*.md" -g "book/**/*.md" --database memory.db

# Index code files with code-aware encoding
csm index-dir -g "src/**/*.rs" --code-aware --database memory.db

# Control heading chunking level (default: 2 for ## sections)
csm index-dir -g "docs/**/*.md" --heading-level 3 --database memory.db
```

**Important**: `index-dir` requires `--glob <PATTERN>` (not a direct path).
Concept IDs follow format: `md:<path>:<heading_level>:<chunk_index>`

### index-jsonl

Index JSONL file content into memory:

```bash
# Index from JSONL file
csm index-jsonl data.jsonl --database csm_memory.db

# Each line must be valid JSON with 'id' and optional 'text'/'metadata' fields
```

### associate

Create associations:

```bash
csm associate source target --strength 0.8 --database csm_memory.db
```

### export

Export memory state:

```bash
# JSON format
csm export --output backup.json --database csm_memory.db

# Binary format (smaller)
csm export --output backup.bin --format binary --database csm_memory.db
```

### import

Import memory state:

```bash
# Replace existing
csm import backup.json --database csm_memory.db

# Merge with existing
csm import backup.json --merge --database memory.db
```

### completions

Generate shell completions:

```bash
# Bash
csm completions bash > ~/.local/share/bash-completion/completions/csm

# Zsh
csm completions zsh > ~/.zsh/completions/_csm

# Fish
csm completions fish > ~/.config/fish/completions/csm.fish

# PowerShell
csm completions powershell > $HOME\.config\powershell\completions\csm.ps1
```

## Output Formats

| Format | Flag | Description |
|--------|------|-------------|
| Table | `--output-format table` | Human-readable table (default) |
| JSON | `--output-format json` | Machine-parseable JSON |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Configuration error |
| 2 | Database error |
| 3 | Input error |
| 4 | Output error |
| 5 | Validation error |
| 6 | Memory error |
| 7 | I/O error |
| 255 | Unknown error |
