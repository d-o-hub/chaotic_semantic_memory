# CLI Module Design Document

**Author:** Agent W9  
**Date:** 2026-02-18  
**Status:** Design Phase

## Overview

Design for `chaotic_semantic_memory` CLI crate using 2026 Rust best practices with clap 4.x derive macros.

## Module Layout

```
src/
├── cli/
│   ├── mod.rs           # Exports, root types, run() entry point
│   ├── args.rs          # CliArgs with global flags
│   ├── error.rs         # CliError enum, exit codes
│   ├── output.rs        # Output formatting (json/table/quiet)
│   └── commands/
│       ├── mod.rs       # Commands enum with Subcommand derive
│       ├── inject.rs    # inject subcommand handler
│       ├── probe.rs     # probe subcommand handler
│       ├── associate.rs # associate subcommand handler
│       ├── export.rs    # export subcommand handler
│       ├── import.rs    # import subcommand handler
│       └── version.rs   # version subcommand handler
└── main.rs              # Binary entry point
```

## Global Options

### CliArgs Struct (args.rs)

```
#[derive(Parser, Debug)]
#[command(name = "csm")]
#[command(about = "Chaotic Semantic Memory CLI", long_about = None)]
#[command(version)]
struct CliArgs {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, global = true, action = ArgAction::Count)]
    verbose: u8,

    #[arg(short, long, global = true, value_name = "FILE")]
    config: Option<PathBuf>,

    #[arg(short, long, global = true, value_name = "PATH")]
    database: Option<PathBuf>,

    #[arg(long, global = true, value_enum, default_value = "table")]
    output_format: OutputFormat,
}
```

### Verbose Levels

| Level | Flag | Behavior |
|-------|------|----------|
| 0 | (default) | Errors only |
| 1 | -v | Warnings + errors |
| 2 | -vv | Info + warnings + errors |
| 3 | -vvv | Debug + all above |
| 4+ | -vvvv | Trace + all above |

### Output Format Enum (output.rs)

```
#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Json,
    Table,
    Quiet,
}
```

## Subcommand Definitions

### Commands Enum (commands/mod.rs)

```
#[derive(Subcommand, Debug)]
enum Commands {
    Inject(commands::InjectArgs),
    Probe(commands::ProbeArgs),
    Associate(commands::AssociateArgs),
    Export(commands::ExportArgs),
    Import(commands::ImportArgs),
    Version(commands::VersionArgs),
}
```

### Inject Command (commands/inject.rs)

```
#[derive(Args, Debug)]
struct InjectArgs {
    #[arg(required = true)]
    concept_id: String,

    #[arg(short, long)]
    from_file: Option<PathBuf>,

    #[arg(short, long, default_value = "random")]
    vector_source: VectorSource,
}

enum VectorSource {
    Random,
    File,
    Stdin,
}
```

**Usage:**
```bash
csm inject my-concept                    # Random vector
csm inject my-concept --from-file vec.bin
csm inject my-concept -f vec.json
```

### Probe Command (commands/probe.rs)

```
#[derive(Args, Debug)]
struct ProbeArgs {
    #[arg(required = true)]
    concept_id: String,

    #[arg(short = 'k', long, default_value = "10")]
    top_k: usize,

    #[arg(short, long)]
    threshold: Option<f64>,
}
```

**Usage:**
```bash
csm probe my-concept                # Top 10 similar
csm probe my-concept -k 5           # Top 5
csm probe my-concept -k 20 -t 0.5   # Top 20, min score 0.5
```

### Associate Command (commands/associate.rs)

```
#[derive(Args, Debug)]
struct AssociateArgs {
    #[arg(required = true)]
    source_id: String,

    #[arg(required = true)]
    target_id: String,

    #[arg(short, long, default_value = "1.0")]
    strength: f64,
}
```

**Usage:**
```bash
csm associate concept-a concept-b
csm associate concept-a concept-b -s 0.75
```

### Export Command (commands/export.rs)

```
#[derive(Args, Debug)]
struct ExportArgs {
    #[arg(short, long, default_value = "export.json")]
    output: PathBuf,

    #[arg(long)]
    include_vectors: bool,

    #[arg(long)]
    include_associations: bool,
}
```

**Usage:**
```bash
csm export -o backup.json
csm export -o full.json --include-vectors --include-associations
```

### Import Command (commands/import.rs)

```
#[derive(Args, Debug)]
struct ImportArgs {
    #[arg(required = true)]
    input: PathBuf,

    #[arg(long)]
    merge: bool,
}
```

**Usage:**
```bash
csm import backup.json
csm import backup.json --merge  # Don't overwrite existing
```

### Version Command (commands/version.rs)

```
#[derive(Args, Debug)]
struct VersionArgs {
    #[arg(short, long)]
    detailed: bool,
}
```

**Usage:**
```bash
csm version
csm version --detailed  # Shows deps, build info
```

## Error Handling

### CliError Enum (error.rs)

```
#[derive(Debug)]
pub enum CliError {
    Memory(MemoryError),
    Io(std::io::Error),
    Config(String),
    InvalidInput { field: String, reason: String },
    NotFound { resource: String },
    DatabaseConnection { path: String, source: String },
}

impl CliError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            CliError::Memory(_) => ExitCode::from(1),
            CliError::Io(_) => ExitCode::from(2),
            CliError::Config(_) => ExitCode::from(3),
            CliError::InvalidInput { .. } => ExitCode::from(4),
            CliError::NotFound { .. } => ExitCode::from(5),
            CliError::DatabaseConnection { .. } => ExitCode::from(6),
        }
    }
}
```

### Exit Code Mapping

| Code | Error Type | Description |
|------|------------|-------------|
| 0 | Success | Command completed |
| 1 | Memory | Framework error |
| 2 | Io | Filesystem error |
| 3 | Config | Configuration error |
| 4 | InvalidInput | Bad user input |
| 5 | NotFound | Resource not found |
| 6 | DatabaseConnection | DB connection failed |

## Output Formatting

### JSON Output

```json
{
  "status": "success",
  "data": {
    "concept_id": "my-concept",
    "similar": [
      {"id": "other", "score": 0.85}
    ]
  }
}
```

### Table Output

```
┌─────────────┬───────┐
│ Concept     │ Score │
├─────────────┼───────┤
│ concept-b   │ 0.92  │
│ concept-c   │ 0.78  │
│ concept-d   │ 0.65  │
└─────────────┴───────┘
```

### Quiet Mode

- Success: Exit 0, no output
- Failure: Exit non-zero, error message to stderr

## Dependency Additions

Add to Cargo.toml:

```toml
[dependencies]
clap = { version = "4.5", features = ["derive", "env"] }
exitcode = "1.1"
tabled = "0.17"  # Optional: for table formatting

[[bin]]
name = "csm"
path = "src/main.rs"
```

## Entry Point Flow

```
main.rs
├── parse_args() → CliArgs
├── init_tracing(verbose_level)
├── load_config(config_path) → Config
├── run(args) → Result<(), CliError>
│   ├── match command:
│   │   ├── Commands::Inject → handle_inject()
│   │   ├── Commands::Probe → handle_probe()
│   │   └── ...
│   └── format_output(result, output_format)
└── exit_with_code(result)
```

## File Size Estimates

| File | Est. LOC |
|------|----------|
| mod.rs | ~80 |
| args.rs | ~40 |
| error.rs | ~60 |
| output.rs | ~80 |
| commands/mod.rs | ~30 |
| inject.rs | ~70 |
| probe.rs | ~70 |
| associate.rs | ~50 |
| export.rs | ~60 |
| import.rs | ~70 |
| version.rs | ~40 |
| main.rs | ~40 |
| **Total** | **~690** |

All files well under 500 LOC limit.

## Testing Strategy

1. **Unit tests**: Each command handler in its module
2. **Integration tests**: `tests/cli_integration.rs`
3. **Snapshot tests**: Output format verification

## Future Extensions

- `csm config` - View/edit configuration
- `csm stats` - Show framework statistics
- `csm migrate` - Database migrations
- `csm serve` - Run as HTTP server (optional feature)
