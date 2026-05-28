# Codacy PR Analysis Output Format

When running `codacy pull-request ... --output json`, the response contains structured quality data.

## Schema Highlights

- `newIssues`: Findings introduced by the PR.
- `fixedIssues`: Issues resolved by the PR.
- `qualityGateStatus`: Overall status (`Passed`, `Warning`, `Failed`).

## Issue Schema

```json
{
  "resultDataId": 987654321,
  "hash": "d41d8cd98f00b204e9800998ecf8427e",
  "message": "Double quote to prevent globbing and word splitting.",
  "file": "scripts/setup.sh",
  "line": 12,
  "tool": "ShellCheck",
  "severity": "Info"
}
```

### Identification: Hash vs. ID

| Field | Type | Purpose |
|-------|------|---------|
| `hash` | String | Used for matching issues in the UI and between commits. |
| `resultDataId` | Number | **MANDATORY** for CLI operations like `--ignore-issue`. |
