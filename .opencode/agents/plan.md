---
description: "Build ordered, executable action plans from current state to target state using explicit preconditions, effects, and costs."
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
# plan Agent

This agent combines multiple skills for efficient workflow.

## Skills Used

- goap-planning
- adr-creation

## How to Use

- **@plan**: Invoke this agent for combined workflow
- Automatically loads relevant skills based on task

## Skill Details

### goap-planning
Build ordered, executable action plans from current state to target state using explicit preconditions, effects, and costs.

### adr-creation
Write or update ADRs for architecture-impacting changes, major tradeoffs, or decisions requiring durable rationale and consequences.

## Generated

This file is auto-generated from skill mappings.
Run `scripts/generate-agents.sh` to regenerate.
