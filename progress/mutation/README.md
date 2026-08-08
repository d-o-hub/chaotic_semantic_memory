# Mutation Testing Artifacts

Use the wrapper script:

```bash
scripts/mutation_test.sh fast
scripts/mutation_test.sh full
```

Reports and logs:
- `fast-<timestamp>.log` / `full-<timestamp>.log`: raw cargo-mutants logs
- `fast-latest.md` / `full-latest.md`: latest report summaries

Machine-readable inventory (deterministic directory, refreshed per profile):

```bash
scripts/mutation_test.sh --ci
# writes: target/mutation-artifacts/<profile>/
#   summary.txt — key=value inventory incl. the CI-parsable MUTATION_SUMMARY line
#   caught.txt, missed.txt, timeout.txt, unviable.txt — mutant name lists
#   equivalent.txt — documented proven-equivalent set (see scripts/mutation-equivalence.md)
#   candidate-count.txt, in-diff-production-files.txt
```

CI semantics: timeouts are UNRESOLVED (counted toward missed), unviable
mutants are excluded from the denominator, and an in-diff scan with no changed
production files (src/**/*.rs, crates/**/*.rs) fails closed with exit 1 rather
than silently passing. `scripts/mutation_test.sh --self-test` runs an embedded
test battery for the classification helpers.