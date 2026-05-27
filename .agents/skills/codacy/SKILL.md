---
name: codacy
description: "Orchestrate static analysis using Codacy CLIs. Use when Codacy blocks a PR, for fixing quality gate failures, or suppressing false positives in this Rust/Shell codebase."
---

# Codacy Static Analysis

Use Codacy CLIs (Analysis CLI for local, Cloud CLI for remote) to maintain code quality and security standards.

## When to Use

- When a PR is blocked by a Codacy quality gate.
- To triage and fix static analysis findings (Rust, Shell, Markdown).
- To suppress false positives identified in the Codacy dashboard.
- To verify local changes before pushing (for supported tools).

## Do NOT Use

- For architectural changes or logic bugs not caught by static analysis.
- As the sole source of truth for Rust analysis (local `cargo clippy` is more precise).
- For local analysis of languages requiring complex runtimes not in the environment.

## Installation & Auth

```bash
# Requires Node.js
npm i -g @codacy/analysis-cli @codacy/codacy-cloud-cli
export CODACY_API_TOKEN=<your-api-token>
```

## Workflows

### PR Triage
1. **Fetch Analysis**:
   `codacy pull-request gh <org> <repo> <prNumber> --output json > /tmp/codacy-pr.json`
2. **Review Status & Issues**:
   Check `qualityGateStatus` for the overall status (Passed, Warning, Failed). Examine `newIssues` in the JSON and note the `resultDataId` for any false positives.
3. **Suppress False Positives**:
   `codacy pull-request gh <org> <repo> <prNumber> --ignore-issue <resultDataId> --ignore-reason FalsePositive`

### Local Verification
```bash
# Initialize if missing
codacy-analysis init --default

# Run local analysis on current branch
codacy-analysis analyze --pr --output-format json
```

## Tool Support Matrix

| Tool | Focus | Local Support |
|------|-------|---------------|
| **Opengrep** | Rust/Security | ✅ Partial |
| **ShellCheck** | Shell Scripts | ✅ Full |
| **markdownlint** | Documentation | ✅ Full |
| **Trivy** | Security/SBOM | ✅ Full |
| **Clippy** | Rust Lints | ❌ Cloud Only (via Codacy) |

## Key Constraints

- **resultDataId**: Always use the numeric `resultDataId` for CLI suppressions. The `hash` is NOT supported for this operation.
- **Cloud Source of Truth**: Local analysis is a subset. If the Cloud dashboard shows issues not found locally, the Cloud results are authoritative.
- **Ignore Reason**: Suppressions require a valid `--ignore-reason` (e.g., `FalsePositive`, `Won't Fix`).

## References

- `references/output-format.md`: JSON schema for PR analysis.
- `references/supported-tools.md`: Detailed tool availability.
- `references/config-format.md`: `.codacy.yml` configuration syntax.
