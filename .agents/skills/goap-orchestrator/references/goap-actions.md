# GOAP Action Catalog

Use these normalized actions to build plans.

## analyze-repo
- Preconditions: repository accessible
- Effects: current-state snapshot produced

## define-acceptance
- Preconditions: user requirements known
- Effects: target acceptance criteria documented

## compute-gaps
- Preconditions: state snapshot and acceptance criteria available
- Effects: explicit missing-task list produced

## delegate-specialists
- Preconditions: missing-task list available
- Effects: specialist handoffs assigned

## implement-changes
- Preconditions: specialist implementation handoffs assigned
- Effects: code artifacts updated

## add-tests
- Preconditions: implementation changes present
- Effects: verification tests cover new behavior

## record-adr
- Preconditions: non-trivial architectural decisions identified
- Effects: ADR entries created or updated

## run-verification
- Preconditions: merged changes available
- Effects: verification command set completed

## run-example
- Preconditions: successful build and runnable example available
- Effects: demonstrable behavior output captured

## finalize-release
- Preconditions: all goals satisfied
- Effects: commit + PR metadata prepared
