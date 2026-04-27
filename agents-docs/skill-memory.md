# Skill Memory (Dogfooding CSM)

Skills use the `csm` CLI to persist learning and build knowledge graphs.

## Configuration

```yaml
memory:
  enabled: true
  database: ".agents/csm-memory/skill-memory.db"
  namespace_prefix: "skill"
```

## Quick Usage

```bash
source scripts/skill-memory/skill-memory.sh

# Remember operation
CONCEPT_ID=$(skill_remember "adr-creation" "decision" "ADR-0043" "approved")

# Recall similar
skill_recall "CSM integration" 0.7 5

# Create association
skill_associate "error::xyz" "solution::abc" 0.95
```

## Available Functions

- `skill_remember skill op context result` - Store operation
- `skill_recall query [threshold] [top_k]` - Find similar
- `skill_associate c1 c2 [strength]` - Link concepts
- `skill_related concept_id [min_strength]` - Get related
- `skill_suggest query [threshold]` - Show suggestions

## Dogfooding Principle

By using the `csm` CLI for skill memory, we validate:
- CLI reliability in real workflows
- libsql persistence durability
- Edge cases through actual usage
- Framework utility through self-use
