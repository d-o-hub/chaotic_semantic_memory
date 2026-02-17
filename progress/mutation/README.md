# Mutation Testing Artifacts

Use the wrapper script:

```bash
scripts/mutation_test.sh fast
scripts/mutation_test.sh full
```

Artifacts:
- `fast-<timestamp>.log` / `full-<timestamp>.log`: raw cargo-mutants logs
- `fast-latest.md` / `full-latest.md`: latest report summaries
