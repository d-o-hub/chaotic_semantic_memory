# Jules Orchestration Evaluation Criteria

| Criterion | Pass | Fail |
|-----------|------|------|
| **Session Precision** | Prompt includes specific issue ID and/or ADR path. | Vague prompt without context. |
| **Concurrency Management** | Uses separate `new` commands or loops for different tasks; uses `--parallel` only for alternative solutions on a single task. | Misuses `--parallel` to attempt dispatching multiple different tasks in one command. |
| **Verification Loop** | Session is reviewed in TUI or locally before commit. | Pulling and committing without verification. |
| **Error Recovery** | Test failures are fed back to a new Jules session. | Manual patching of AI-generated code. |
| **Artifact Traceability** | Commit messages link to the Jules session ID. | No record of which session generated the code. |
