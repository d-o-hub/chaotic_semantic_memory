---
name: skill-memory
description: Use the csm CLI to persist skill learning and context. Skills store operations, recall patterns, and build knowledge graphs via the chaotic_semantic_memory CLI - validating the framework through actual usage (dogfooding).
---

# Skill Memory via CLI (Dogfooding CSM)

**Core Principle:** Skills use the `csm` CLI binary to store their own learning, validating the framework through real-world usage.

## Architecture

```
Skill (bash/rust/python)
    ↓ invokes
    csm inject "skill::rust-dev::refactor::abc123" --metadata '{...}'
    csm probe "similar refactoring" -k 5
    csm associate "error::xyz" "solution::abc" -s 0.9
    ↓ stores in
    .agents/memory/skill-memory.db (libsql)
```

## Quick Start

```bash
# Initialize memory database for a skill
export CSM_MEMORY_DB=".agents/memory/skill-memory.db"

# Remember an operation
csm --database "$CSM_MEMORY_DB" inject "skill::adr-creation::decision::0043" \
  --metadata '{"operation":"architectural_decision","context":"CSM Integration","result":"approved","timestamp":1708432000}'

# Recall similar operations
SIMILAR=$(csm --database "$CSM_MEMORY_DB" probe "CSM integration pattern" -k 5 --output-format json)
echo "$SIMILAR" | jq '.[] | select(.similarity > 0.7)'

# Create association
csm --database "$CSM_MEMORY_DB" associate "error::E0495::abc123" "solution::lifetime::def456" -s 0.95
```

## Why CLI-Based?

1. **Dogfooding**: We validate our own CLI by using it extensively
2. **Language Agnostic**: Any skill (bash, python, node) can use memory
3. **Real Testing**: CLI edge cases get tested through skill usage
4. **Simpler**: No Rust dependencies in skills - just shell out to `csm`
5. **Audit Trail**: All skill memory operations are logged via CLI

## CLI Commands for Skills

### 1. Remember (inject with metadata)

```bash
#!/bin/bash

remember_operation() {
    local skill_name="$1"
    local operation="$2"
    local context="$3"
    local result="$4"
    local db="${CSM_MEMORY_DB:-.agents/memory/skill-memory.db}"
    
    local concept_id="skill::${skill_name}::${operation}::$(date +%s)"
    local metadata=$(jq -n \
        --arg op "$operation" \
        --arg ctx "$context" \
        --arg res "$result" \
        --arg skill "$skill_name" \
        --arg ts "$(date -Iseconds)" \
        '{operation: $op, context: $ctx, result: $res, skill: $skill, timestamp: $ts}')
    
    csm --database "$db" inject "$concept_id" --metadata "$metadata"
    echo "$concept_id"
}

# Usage
remember_operation "adr-creation" "architectural_decision" \
    "ADR-0043: CSM Integration" "approved: High feasibility"
```

### 2. Recall (probe with filtering)

```bash
#!/bin/bash

recall_similar() {
    local query="$1"
    local threshold="${2:-0.7}"
    local top_k="${3:-5}"
    local db="${CSM_MEMORY_DB:-.agents/memory/skill-memory.db}"
    
    # Query and filter by similarity
    csm --database "$db" probe "$query" -k "$top_k" --output-format json | \
        jq --arg thresh "$threshold" '[.[] | select(.similarity >= ($thresh | tonumber))]'
}

# Usage
recall_similar "CSM integration pattern" 0.75 5
```

### 3. Associate (associate command)

```bash
#!/bin/bash

create_association() {
    local concept1="$1"
    local concept2="$2"
    local strength="${3:-0.8}"
    local db="${CSM_MEMORY_DB:-.agents/memory/skill-memory.db}"
    
    csm --database "$db" associate "$concept1" "$concept2" -s "$strength"
}

# Usage  
create_association "error::E0495::abc123" "solution::lifetime::def456" 0.95
```

### 4. Get Related (probe with concept)

```bash
#!/bin/bash

get_related() {
    local concept_id="$1"
    local min_strength="${2:-0.7}"
    local db="${CSM_MEMORY_DB:-.agents/memory/skill-memory.db}"
    
    # Export and filter by associations
    local export_data=$(csm --database "$db" export -o - --output-format json)
    echo "$export_data" | jq --arg concept "$concept_id" --arg strength "$min_strength" '
        .associations[] | select(.from == $concept and .strength >= ($strength | tonumber))'
}

# Usage
get_related "error::E0495::abc123" 0.8
```

## Configuration

Add to `AGENTS.md`:

```yaml
## Memory Configuration (CSM CLI)

memory:
  enabled: true
  database: ".agents/memory/skill-memory.db"  # libsql database path
  namespace_prefix: "skill"                    # All skill concepts prefixed
  
  # Auto-remember settings
  auto_remember:
    enabled: true
    operations:                  # Which operations to auto-remember
      - "architectural_decision"
      - "code_refactoring"
      - "error_resolution"
      - "test_failure"
    
  # Recall settings  
  auto_recall:
    enabled: true
    threshold: 0.7              # Min similarity for suggestions
    top_k: 5                    # Max suggestions
```

## Integration Examples

### adr-creation Skill

```bash
#!/bin/bash
# File: .opencode/agents/adr-creation-memory.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DB="${CSM_MEMORY_DB:-.agents/memory/skill-memory.db}"

# Ensure database exists
if [ ! -f "$DB" ]; then
    mkdir -p "$(dirname "$DB")"
fi

remember_adr() {
    local adr_number="$1"
    local title="$2"
    local context="$3"
    local decision="$4"
    shift 4
    local related_adrs=("$@")
    
    local concept_id="skill::adr-creation::adr::${adr_number}"
    local metadata=$(jq -n \
        --arg num "$adr_number" \
        --arg title "$title" \
        --arg ctx "$context" \
        --arg dec "$decision" \
        --argjson related "$(printf '%s\n' "${related_adrs[@]}" | jq -R . | jq -s .)" \
        --arg ts "$(date -Iseconds)" \
        '{
            operation: "architectural_decision",
            adr_number: $num,
            title: $title,
            context: $ctx,
            decision: $dec,
            related_adrs: $related,
            timestamp: $ts,
            status: "proposed"
        }')
    
    echo "[skill-memory] Remembering ADR-${adr_number}: $title"
    csm --database "$DB" inject "$concept_id" --metadata "$metadata"
    
    # Create associations to related ADRs
    for related in "${related_adrs[@]}"; do
        local related_id="skill::adr-creation::adr::${related}"
        echo "[skill-memory] Associating with ADR-${related}"
        csm --database "$DB" associate "$concept_id" "$related_id" -s 0.8 || true
    done
    
    echo "$concept_id"
}

find_related_decisions() {
    local topic="$1"
    local threshold="${2:-0.75}"
    local top_k="${3:-5}"
    
    echo "[skill-memory] Finding related decisions for: $topic"
    
    local results=$(csm --database "$DB" probe "$topic" -k "$top_k" --output-format json)
    
    # Filter by threshold and format output
    echo "$results" | jq --arg thresh "$threshold" '
        [.[] | select(.similarity >= ($thresh | tonumber))] |
        map({
            adr_number: (.id | split("::")[3]),
            title: .metadata.title,
            similarity: .similarity,
            decision: .metadata.decision,
            timestamp: .metadata.timestamp
        })'
}

suggest_precedents() {
    local context="$1"
    
    echo "[skill-memory] Analyzing context for precedents..."
    local precedents=$(find_related_decisions "$context" 0.7 3)
    
    local count=$(echo "$precedents" | jq 'length')
    if [ "$count" -gt 0 ]; then
        echo ""
        echo "Related architectural decisions found:"
        echo "$precedents" | jq -r '.[] | 
            "  • ADR-\(.adr_number): \(.title) (similarity: \(.similarity | tostring | .[0:4]))"'
        echo ""
    fi
}

# Main execution
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    case "${1:-}" in
        remember)
            shift
            remember_adr "$@"
            ;;
        find)
            shift
            find_related_decisions "$@"
            ;;
        suggest)
            shift
            suggest_precedents "$@"
            ;;
        *)
            echo "Usage: $0 {remember|find|suggest} [args...]"
            exit 1
            ;;
    esac
fi
```

### debugging-reservoir Skill

```bash
#!/bin/bash
# File: .opencode/agents/debug-reservoir-memory.sh

set -euo pipefail

DB="${CSM_MEMORY_DB:-.agents/memory/skill-memory.db}"

remember_error() {
    local symptom="$1"
    local cause="$2"
    local solution="$3"
    local effectiveness="${4:-0.8}"
    
    local error_id="skill::debugging-reservoir::error::$(echo "$symptom" | sha256sum | head -c 8)"
    local solution_id="skill::debugging-reservoir::solution::$(echo "$solution" | sha256sum | head -c 8)"
    
    # Store error
    local error_metadata=$(jq -n \
        --arg symptom "$symptom" \
        --arg cause "$cause" \
        --arg ts "$(date -Iseconds)" \
        '{
            operation: "reservoir_error",
            symptom: $symptom,
            cause: $cause,
            timestamp: $ts,
            resolved: true
        }')
    
    echo "[skill-memory] Remembering error pattern"
    csm --database "$DB" inject "$error_id" --metadata "$error_metadata"
    
    # Store solution
    local solution_metadata=$(jq -n \
        --arg solution "$solution" \
        --arg effectiveness "$effectiveness" \
        --arg ts "$(date -Iseconds)" \
        '{
            operation: "reservoir_solution",
            solution: $solution,
            effectiveness: $effectiveness,
            timestamp: $ts
        }')
    
    csm --database "$DB" inject "$solution_id" --metadata "$solution_metadata"
    
    # Create associations
    echo "[skill-memory] Linking error to solution (strength: $effectiveness)"
    csm --database "$DB" associate "$error_id" "$solution_id" -s "$effectiveness"
    
    echo "Error stored: $error_id"
    echo "Solution stored: $solution_id"
}

find_solutions() {
    local symptom="$1"
    local threshold="${2:-0.6}"
    
    echo "[skill-memory] Finding solutions for: $symptom"
    
    # Find similar errors
    local similar_errors=$(csm --database "$DB" probe "$symptom" -k 10 --output-format json)
    
    # For each error, get associated solutions
    echo "$similar_errors" | jq --arg thresh "$threshold" '
        [.[] | select(.similarity >= ($thresh | tonumber) and .metadata.operation == "reservoir_error")] |
        map({
            error_id: .id,
            symptom: .metadata.symptom,
            similarity: .similarity
        })'
}

# Export functions if sourced
if [ "${BASH_SOURCE[0]}" != "${0}" ]; then
    export -f remember_error find_solutions
fi
```

### rust-development Skill

```bash
#!/bin/bash
# File: .opencode/agents/rust-dev-memory.sh

set -euo pipefail

DB="${CSM_MEMORY_DB:-.agents/memory/skill-memory.db}"

remember_refactoring() {
    local file_path="$1"
    local pattern_type="$2"  # e.g., "match_to_trait", "extract_method"
    local description="$3"
    local success="${4:-true}"
    
    local concept_id="skill::rust-development::refactor::$(date +%s)"
    local metadata=$(jq -n \
        --arg file "$file_path" \
        --arg pattern "$pattern_type" \
        --arg desc "$description" \
        --argjson success "$success" \
        --arg ts "$(date -Iseconds)" \
        '{
            operation: "code_refactoring",
            file_path: $file,
            pattern_type: $pattern,
            description: $desc,
            success: $success,
            timestamp: $ts
        }')
    
    echo "[skill-memory] Remembering refactoring: $pattern_type"
    csm --database "$DB" inject "$concept_id" --metadata "$metadata"
    
    # Associate with pattern type
    local pattern_concept="skill::rust-development::pattern::${pattern_type}"
    csm --database "$DB" associate "$concept_id" "$pattern_concept" -s 1.0
    
    echo "$concept_id"
}

find_similar_refactorings() {
    local code_context="$1"
    local pattern_type="${2:-}"
    
    local query="$code_context"
    if [ -n "$pattern_type" ]; then
        query="$pattern_type $code_context"
    fi
    
    echo "[skill-memory] Finding similar refactorings"
    csm --database "$DB" probe "$query" -k 5 --output-format json | \
        jq '[.[] | select(.metadata.operation == "code_refactoring")]'
}

get_pattern_success_rate() {
    local pattern_type="$1"
    
    echo "[skill-memory] Calculating success rate for: $pattern_type"
    
    # Export all and analyze
    local data=$(csm --database "$DB" export -o - --output-format json)
    echo "$data" | jq --arg pattern "$pattern_type" '
        .concepts[] | 
        select(.metadata.pattern_type == $pattern) |
        .metadata.success' | \
        jq -s 'group_by(.) | map({status: .[0], count: length})'
}
```

## Database Schema

Skills use these concept ID patterns:

```
skill::{skill_name}::{operation_type}::{unique_id}

Examples:
- skill::adr-creation::adr::0043
- skill::debugging-reservoir::error::a3f7b2d9
- skill::rust-development::refactor::1708432000
- skill::testing-validation::test_failure::test_name_hash
```

## Testing the Integration

```bash
# Test adr-creation memory
source .opencode/agents/adr-creation-memory.sh

remember_adr "0043" "CSM Integration" "Need memory system" "Use CLI" "0042"
find_related_decisions "memory system pattern"

# Test debugging-reservoir memory  
source .opencode/agents/debug-reservoir-memory.sh

remember_error "spectral radius > 1.1" "unstable dynamics" "clamp to 1.1" 0.95
find_solutions "reservoir unstable"

# Test rust-development memory
source .opencode/agents/rust-dev-memory.sh

remember_refactoring "src/lib.rs" "match_to_trait" "Converted match to polymorphic traits" true
find_similar_refactorings "convert match statement to trait"
```

## Benefits of CLI Approach

1. **Validates CLI**: Every skill memory operation tests CLI functionality
2. **Cross-Language**: Bash, Python, Node skills all use same memory
3. **Simple**: No Rust compilation in skills - just shell commands
4. **Observable**: All operations logged via CLI output
5. **Portable**: Works anywhere `csm` binary is available
6. **Realistic**: Tests actual user workflows

## Performance Considerations

- Each CLI call has ~50-100ms overhead
- Batch operations when possible
- Use JSON output format for programmatic parsing
- Cache frequent queries in skill-local variables

## Debugging

```bash
# Enable verbose mode
export CSM_VERBOSE=1

# Check database contents
csm --database .agents/memory/skill-memory.db export -o - | jq '.'

# Verify associations
csm --database .agents/memory/skill-memory.db export -o - | jq '.associations'
```
