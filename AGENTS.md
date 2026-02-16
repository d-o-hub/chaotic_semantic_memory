# AGENTS Instructions

- Keep each Rust source file under 500 LOC.
- Always decompose tasks before implementation in `plans/`.
- Use `gh` CLI for issue and sub-issue workflows when GitHub is configured.
- Prefer draw.io diagrams for architecture artifacts.
- Continuously update this file with validated best practices learned from user input and web research.
- Ensure all new features are independently operable with no external framework coupling.
- No hard-coded configuration values: expose tunables via constants and/or builder configuration.
- Avoid magic numbers in production logic; centralize defaults as named constants.

## Reference Files
- `plans/README.md`: planning workflow anchor.
- `plans/fix_unsatisfied.md`: active remediation decomposition.
- `plans/implementation_loop_round9.md`: latest implementation loop decomposition.
- `.agents/skills/drawio-architecture/references/diagram-template.md`: architecture notation.
- `.agents/skills/rust-verification-loop/references/commands.md`: final verification loop.
- `.agents/skills/goap-orchestrator/references/goap-actions.md`: GOAP action catalog.
- `.agents/skills/goap-orchestrator/references/adr-template.md`: ADR decision template.
- `.agents/skills/goap-orchestrator/scripts/build_goap_plan.py`: deterministic GOAP plan generator.
- `.agents/skills/goap-orchestrator/scripts/create_adr.py`: deterministic ADR file generator.
- `.agents/skills/goap-orchestrator/scripts/orchestrate.py`: deterministic specialist assignment board generator.
- `README.md`: repository usage and verification guide.

## Agent Skills
- `drawio-architecture`: architecture diagram creation and maintenance.
- `rust-verification-loop`: end-to-end Rust verification and runnable example execution.
- `goap-orchestrator`: GOAP-first orchestration that delegates missing tasks to specialist agents and records ADRs.
