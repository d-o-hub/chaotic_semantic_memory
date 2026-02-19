---
description: "Implement or refactor Rust in this repository. Use when writing new modules, modifying existing source files, or adding features to the chaotic_semantic_memory crate."
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
# fix Agent

This agent combines multiple skills for efficient workflow.

## Skills Used

- rust-development
- testing-validation
- debugging-reservoir

## How to Use

- **@fix**: Invoke this agent for combined workflow
- Automatically loads relevant skills based on task

## Skill Details

### rust-development
Implement or refactor Rust in this repository. Use when writing new modules, modifying existing source files, or adding features to the chaotic_semantic_memory crate.

### testing-validation
Validate the chaotic_semantic_memory crate: compile, test, lint, LOC caps, and benchmarks. Use when asked to validate, check, or verify the build.

### debugging-reservoir
Debug and tune the echo state network reservoir. Use when diagnosing spectral radius issues, chaotic dynamics problems, sparse weight anomalies, or reservoir-to-hypervector projection failures.

## Generated

This file is auto-generated from skill mappings.
Run `scripts/generate-agents.sh` to regenerate.
