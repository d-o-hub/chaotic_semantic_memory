#!/bin/bash
# Generate .opencode/agents/ from combined skill mappings
# Creates agents that use multiple skills for common workflows

set -e

SKILLS_DIR=".agents/skills"
AGENTS_DIR=".opencode/agents"

# Check if regeneration is needed
NEEDS_REGEN=0

if [ ! -d "$AGENTS_DIR" ] || [ -z "$(ls -A "$AGENTS_DIR" 2>/dev/null)" ]; then
  NEEDS_REGEN=1
  REASON="agents directory missing or empty"
else
  # Check if any skill file is newer than the oldest agent
  OLDEST_AGENT=$(find "$AGENTS_DIR" -name "*.md" -printf '%T@\t%p\n' 2>/dev/null | sort -n | head -1 | cut -f2-)
  if [ -n "$OLDEST_AGENT" ]; then
    for skill_file in "$SKILLS_DIR"/*/SKILL.md; do
      if [ -f "$skill_file" ] && [ "$skill_file" -nt "$OLDEST_AGENT" ]; then
        NEEDS_REGEN=1
        REASON="skill $(basename $(dirname "$skill_file")) modified"
        break
      fi
    done
  fi
fi

if [ "$NEEDS_REGEN" -eq 0 ]; then
  echo "=== Agents up to date, skipping regeneration ==="
  exit 0
fi

echo "=== Generating OpenCode Agents from Skill Mappings ==="
echo "Reason: $REASON"

mkdir -p "$AGENTS_DIR"

# Clear existing generated agents
rm -f "$AGENTS_DIR"/*.md 2>/dev/null || true

# Define agent mappings (multiple skills per agent)
declare -A AGENT_MAPPINGS=(
  ["impl"]="rust-development testing-validation"
  ["fix"]="rust-development testing-validation debugging-reservoir"
  ["perf"]="benchmarking-perf debugging-reservoir swarm-performance"
  ["test"]="testing-validation swarm-testing-quality"
  ["plan"]="goap-planning adr-creation"
  ["ci"]="github-ci-git-workflow"
  ["swarm"]="swarm-testing-quality swarm-performance swarm-observability swarm-advanced-features"
)

# Helper function to get skill description
get_skill_desc() {
  local skill_file="$SKILLS_DIR/$1/SKILL.md"
  if [ -f "$skill_file" ]; then
    grep -m1 "^description:" "$skill_file" | sed 's/^description: *//' | sed 's/^"//;s/"$//' || echo "$1 skill"
  else
    echo "$1 skill"
  fi
}

# Generate each agent
for agent_name in "${!AGENT_MAPPINGS[@]}"; do
  skills="${AGENT_MAPPINGS[$agent_name]}"
  skill_list=$(echo "$skills" | tr ' ' ', ')
  
  # Build description from first skill
  first_skill=$(echo "$skills" | awk '{print $1}')
  description=$(get_skill_desc "$first_skill")
  
  agent_file="${AGENTS_DIR}/${agent_name}.md"
  
  cat > "$agent_file" << EOFAGENT
---
description: "$description"
mode: subagent
tools:
  write: true
  edit: true
  bash: true
  glob: true
  grep: true
  read: true
  skill: true
---
# $agent_name Agent

This agent combines multiple skills for efficient workflow.

## Skills Used

$(for s in $skills; do echo "- $s"; done)

## How to Use

- **@$agent_name**: Invoke this agent for combined workflow
- Automatically loads relevant skills based on task

## Skill Details

$(for s in $skills; do
  desc=$(get_skill_desc "$s")
  echo "### $s"
  echo "$desc"
  echo ""
done)

## Generated

This file is auto-generated from skill mappings.
Run \`scripts/generate-agents.sh\` to regenerate.
EOFAGENT

  echo "  + Created: $agent_name (${skills})"
done

echo ""
echo "=== Summary ==="
echo "Generated: ${#AGENT_MAPPINGS[@]} agents from skill combinations"
echo ""
echo "Agent -> Skills mapping:"
for agent_name in "${!AGENT_MAPPINGS[@]}"; do
  echo "  $agent_name: ${AGENT_MAPPINGS[$agent_name]}"
done
