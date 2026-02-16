---
name: drawio-architecture
description: Create and maintain draw.io architecture diagrams for system design changes, especially when updating memory, persistence, and runtime boundaries.
---

# Draw.io Architecture Skill

Use this skill when a change impacts architecture, data flow, persistence, or deployment topology.

## Workflow
1. Identify changed boundaries: API surface, storage, runtime, or async execution model.
2. Open `references/diagram-template.md` and map nodes/edges from code.
3. Produce or update a `.drawio` diagram in `plans/`.
4. Ensure names in the diagram match Rust modules and public API names.

## References
- Use `references/diagram-template.md` for notation and naming rules.
