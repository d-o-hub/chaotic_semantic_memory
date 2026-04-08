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
