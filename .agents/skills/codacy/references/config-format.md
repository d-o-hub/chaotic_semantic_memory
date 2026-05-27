# Codacy Configuration Format

The repository uses `.codacy.yml` or `.codacy.yaml` for advanced configuration.

## Basic Structure

```yaml
---
exclude_paths:
  - "target/**"
  - "node_modules/**"
  - "benches/fixtures/**"

languages:
  rust:
    enabled: true
  shell:
    enabled: true

engines:
  duplication:
    enabled: true
    exclude_paths:
      - "tests/**"
```

## Tool-Specific Configuration

You can tune specific engines under the `engines` key:

```yaml
engines:
  shellcheck:
    exclude_paths:
      - "scripts/legacy/**"
  metric:
    # Cyclomatic complexity thresholds
    config:
      languages:
        - "rust"
```

## Validation

Validate your configuration locally:
```bash
codacy-analysis-cli validate-configuration --directory .
```
