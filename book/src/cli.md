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
csm inject my-concept --database memory.db

# Vector from hex
csm inject my-concept --vector 0xdeadbeef... --database memory.db

# With metadata
csm inject my-concept --metadata '{"key":"value"}' --database memory.db
```

### probe

Find similar concepts:

```bash
# By concept ID
csm probe my-concept -k 10 --database memory.db

# Output as JSON
csm probe my-concept -k 10 --output-format json --database memory.db
```

### associate

Create associations:

```bash
csm associate source target --strength 0.8 --database memory.db
```

### export

Export memory state:

```bash
# JSON format
csm export --output backup.json --database memory.db

# Binary format (smaller)
csm export --output backup.bin --format binary --database memory.db
```

### import

Import memory state:

```bash
# Replace existing
csm import backup.json --database memory.db

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
| 1 | Invalid arguments |
| 2 | Database error |
| 3 | Concept not found |
| 4 | Import/export error |
| 5 | I/O error |
| 6 | Serialization error |
| 7 | Unknown error |
