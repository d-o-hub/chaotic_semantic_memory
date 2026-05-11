**Task:** Fix false positive clippy failures in `scripts/validate.sh`
**Context:** Reference the Validation section in `CONTRIBUTING.md`. The current `scripts/validate.sh` script reports "Error: clippy found new warnings/errors" even when `cargo clippy --all-targets --all-features -- -D warnings` successfully completes with no actual warnings or errors (e.g., when the output is just `Finished dev profile [unoptimized + debuginfo] target(s)...`). This blocks the CI and developer workflow.
**Constraint:** Ensure the bash script correctly distinguishes between actual clippy warnings/errors and standard cargo compilation/finish messages. The fix must run natively on both Linux and macOS bash environments without requiring additional dependencies.
**Success Criteria:**
- `./scripts/validate.sh` passes successfully when the codebase has no clippy warnings/errors.
- `./scripts/validate.sh` correctly fails and outputs the exact clippy errors when actual code violations exist.
- Does not break the existing `--save-baseline` functionality.
