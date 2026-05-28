# Supported Codacy Tools

This repository primarily relies on the following tools integrated into Codacy.

## Core Tools

| Tool | Focus | Local Availability |
|------|-------|-------------------|
| **Opengrep** | Rust Security / Patterns | ✅ Yes |
| **ShellCheck** | Shell Script Quality | ✅ Yes |
| **markdownlint** | Documentation Consistency | ✅ Yes |
| **Trivy** | Vulnerability / Secret Scanning | ✅ Yes |
| **Checkov** | Infrastructure as Code (YAML) | ✅ Yes |

## Rust-Specific Notes

While Codacy runs **Opengrep** for Rust, the primary source for Rust linting in this repository is `cargo clippy`. Codacy findings for Rust should be cross-referenced with `scripts/validate.sh` outputs.

## Local Analysis Limitations

The local `codacy-analysis` CLI may fail to run certain tools if the required runtimes (e.g., Ruby for SQLint, Java for PMD) are not available in the local environment. Always check the Cloud CLI (`codacy pull-request`) for the definitive list of issues.
