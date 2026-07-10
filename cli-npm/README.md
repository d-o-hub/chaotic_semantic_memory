# csm CLI

Command-line interface for [chaotic_semantic_memory](https://github.com/d-o-hub/chaotic_semantic_memory) - AI memory systems with hyperdimensional vectors and chaotic reservoirs.

## Installation

**Use npx (recommended — no install needed, no permission issues):**
```bash
npx @d-o-hub/csm --version
npx @d-o-hub/csm inject my-concept --database csm_memory.db
```

`npx` downloads and runs the correct platform binary automatically. No global install required.

**Global install (optional):**
```bash
npm install -g @d-o-hub/csm
```

> **⚠ Permission errors (EACCES)?**
>
> Global installs write to a system directory that may require elevated permissions.
> **Do not use `sudo npm install -g`** — this creates root-owned files that cause further issues.
>
> Instead, configure a user-writable npm prefix:
> ```bash
> npm config set prefix ~/.npm-global
> export PATH=~/.npm-global/bin:$PATH  # Add to ~/.bashrc or ~/.zshrc to persist
> ```
>
> Or simply use `npx` above, which requires no global install and avoids permission issues entirely.

## Usage

```bash
# Inject a concept
csm inject my-concept --database csm_memory.db

# Find similar concepts
csm probe my-concept -k 10 --database csm_memory.db

# Create associations
csm associate source-concept target-concept --strength 0.8 --database csm_memory.db

# Export memory state
csm export --output backup.json

# Import memory state
csm import backup.json --merge

# Show version
csm version

# Generate shell completions
csm completions bash > ~/.local/share/bash-completion/completions/csm
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `inject` | Inject a new concept with a random or provided vector |
| `probe` | Find similar concepts by concept ID |
| `associate` | Create an association between two concepts |
| `export` | Export memory state to JSON or binary |
| `import` | Import memory state from file |
| `index-jsonl` | Index concepts from JSONL file |
| `index-dir` | Index documents from directory |
| `version` | Show version information |
| `completions` | Generate shell completions |

## Options

| Option | Description |
|--------|-------------|
| `--database <path>` | Path to libSQL database file |
| `--git-local` | Use git-local storage (.git/memory-index/csm.db) |
| `--index-path <path>` | Custom path for git-local index |
| `--output-format <format>` | Output format: table, json, compact |
| `--verbose` | Increase logging verbosity |
| `--help` | Show help for command |

## Platform Support

The CLI binary is downloaded automatically during install for:
- Linux x64
- Linux arm64
- macOS x64 (Intel)
- macOS arm64 (Apple Silicon)
- Windows x64

Binaries are bundled directly inside the npm package for the platforms above. The postinstall script
falls back to downloading from the corresponding GitHub Release asset if a platform-specific binary
is missing, so manual installs can recover gracefully.

## License

MIT
