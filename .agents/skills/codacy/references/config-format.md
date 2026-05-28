# Codacy Configuration Format

The repository uses `.codacy/codacy.config.json` for advanced configuration.

## Basic Structure

```json
{
  "tools": [
    {
      "name": "eslint-9",
      "enabled": true
    }
  ],
  "exclude_paths": [
    "target/**",
    "node_modules/**"
  ]
}
```

## Local Initialization

Generate a default configuration based on repository discovery:
```bash
codacy-analysis init --default
```

## Validation

Current versions of the Analysis CLI perform implicit validation during `analyze` or `discover`. To check for configuration-related issues:
```bash
codacy-analysis discover .
```
