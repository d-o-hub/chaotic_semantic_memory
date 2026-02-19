---
description: "Validate the chaotic_semantic_memory crate: compile, test, lint, LOC caps, and benchmarks. Use when asked to validate, check, or verify the build."
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
# test Agent

This agent combines multiple skills for efficient workflow.

## Skills Used

- testing-validation
- swarm-testing-quality

## How to Use

- **@test**: Invoke this agent for combined workflow
- Automatically loads relevant skills based on task

## Skill Details

### testing-validation
Validate the chaotic_semantic_memory crate: compile, test, lint, LOC caps, and benchmarks. Use when asked to validate, check, or verify the build.

### swarm-testing-quality
Property-based testing, fuzzing, and edge case coverage. Use when adding comprehensive test coverage with proptest or cargo-fuzz.

## Generated

This file is auto-generated from skill mappings.
Run `scripts/generate-agents.sh` to regenerate.
