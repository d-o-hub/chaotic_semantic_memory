# ADR-0031: Two-Tier Architecture Documentation (YAML + DrawIO)

## Status
Accepted

## Context and Problem Statement

Architecture documentation serves two distinct audiences with different needs:
1. **LLMs/AI agents** need structured, machine-parseable data for context injection
2. **Human developers** need visual diagrams for understanding and communication

Maintaining both formats separately creates drift risk and duplicated effort.

## Decision Drivers

- LLM context windows benefit from compact, structured YAML
- Humans comprehend complex relationships faster with visual diagrams
- DrawIO files are XML-based and inefficient for LLM parsing
- Single source of truth reduces maintenance burden
- docs/architecture/context.yaml already exists as LLM-optimized format

## Considered Options

1. **YAML only**: Machine-optimized, no visual output
2. **DrawIO only**: Human-optimized, poor LLM context
3. **Two-tier with YAML canonical**: YAML is source, DrawIO generated/updated for visualization
4. **Two-tier with DrawIO canonical**: DrawIO is source, YAML generated from it

## Decision Outcome

Chosen option: "Two-tier with YAML canonical", because YAML is the primary LLM context source and already integrated into agent workflows. DrawIO serves as visualization layer.

### Positive Consequences

- LLMs get optimized context via context.yaml
- Humans get visual diagrams via .drawio files
- YAML as canonical source prevents drift
- Single maintenance point (YAML) with derived visualization

### Negative Consequences

- Must maintain synchronization between formats
- DrawIO updates require manual or scripted generation
- Two file formats to track in version control

## Implementation

| Element | Format | Audience | Location |
|---------|--------|----------|----------|
| Architecture data | YAML | LLM | docs/architecture/context.yaml |
| Visual diagrams | DrawIO | Human | docs/architecture/*.drawio |

### Sync Strategy

1. YAML is the canonical source of truth
2. DrawIO files are generated/updated when architecture changes
3. Both formats tracked in git for history
4. `drawio` skill handles diagram generation

## Links

- docs/architecture/context.yaml
- .agents/skills/drawio/SKILL.md
