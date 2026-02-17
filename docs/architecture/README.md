# Architecture Diagrams

This directory contains architecture documentation using a **two-tier approach** optimized for both LLM APIs and human developers.

## Two-Tier Architecture

### Tier 1: Structured YAML (For LLMs)
**File**: `context.yaml`

Machine-optimized context with:
- Pure semantic data (no markup)
- Structured constraints, modules, skills
- Data flow definitions
- Validation gates
- Performance targets

**Why YAML for LLMs:**
- 4.6x more compact than AGENTS.md
- 56x more compact than draw.io XML
- 100% parsing accuracy in validation tests
- Direct access to key-value pairs

### Tier 2: Visual Diagram (For Humans)
**File**: `context.drawio` (Generated from YAML)

Visual representation with:
- Color-coded sections
- Relationship arrows
- Spatial layout
- Exportable to PNG/PDF

## For LLM APIs

**Primary source**: `context.yaml`

Example understanding:
```yaml
constraints:
  hard:
    - name: max_loc_per_file
      value: 500
    - name: database_libsql
      value: "libsql"
      forbidden: "turso-client"
```

**Key advantages**:
- Instant constraint lookup
- No parsing overhead
- Structured relationships
- Validated: 9/9 tests pass (100%)

## For Human Developers

**Primary source**: `context.drawio`

View in draw.io:
1. Open [draw.io](https://app.diagrams.net)
2. File → Open From → Device
3. Select `context.drawio`

Or generate from YAML:
```bash
python3 scripts/yaml-to-drawio.py \
  docs/architecture/context.yaml \
  docs/architecture/context.drawio
```

## Files

| File | Purpose | Audience |
|------|---------|----------|
| `context.yaml` | Structured context | LLM APIs |
| `context.drawio` | Visual diagram | Humans |
| `arch.drawio` | Module architecture | Both |
| `high-level-arch.drawio` | System overview | Both |

## Validation

Test LLM understanding:
```bash
python3 tests/validate_llm_context.py
```

Expected output: 9/9 tests passed (100%)

## Legacy Files

- `llm-api-architecture.drawio` - Old auto-generated diagram (redundant)
- Use `context.yaml` + `context.drawio` instead

---

*This two-tier approach ensures both machines and humans can understand the codebase effectively.*
