# Internal Memory Operations

Use these commands for quick internal memory loops.

## Setup

```bash
export CSM_MEMORY_DB=".agents/csm-memory/skill-memory.db"
mkdir -p "$(dirname "$CSM_MEMORY_DB")"
```

## Save

```bash
csm --database "$CSM_MEMORY_DB" inject \
  "skill::impl::decision::$(date +%s)" \
  --metadata '{"operation":"decision","result":"accepted"}'
```

## Load

```bash
csm --database "$CSM_MEMORY_DB" probe "decision accepted" -k 5 --output-format json
```

## Archive (soft)

```bash
csm --database "$CSM_MEMORY_DB" inject \
  "skill::archive::pointer::$(date +%s)" \
  --metadata '{"status":"archived","target":"skill::impl::decision::..."}'
```

## Delete

If a hard delete command exists in your current `csm` build, use it. Otherwise use archive markers and retention rules.
