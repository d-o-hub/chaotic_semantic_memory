# Mutation outcome classification & equivalence guidance

Companion to `scripts/mutation_test.sh`. Explains the exact semantics of the
inventory that the wrapper emits, and how to properly classify **equivalent**
versus **unviable** mutants.

## Outcome buckets (exact semantics)

cargo-mutants (the `mutants.rs` tool) assigns exactly one outcome to every
generated mutant:

| Bucket      | Meaning                                                                            | Score effect                                        |
|-------------|------------------------------------------------------------------------------------|-----------------------------------------------------|
| `caught`    | A test failed with the mutant applied.                                              | `+1` in the numerator                              |
| `missed`    | No test failed. Either a coverage gap **or** a behaviorally equivalent mutant.       | `+1` in the denominator, not in numerator           |
| `timeout`   | The test suite ran long and was killed by the timeout.                              | `+1` in the denominator, not in numerator (UNRESOLVED) |
| `unviable`  | The mutant did not compile.                                                         | excluded from the denominator (no signal)          |
| excluded    | Generated but not tested: filtered out by `--exclude*` and/or out of `--in-diff`.   | excluded from the denominator (no signal)          |

`score = caught / (total - unviable) * 100`.

Only **caught** mutants improve the score; `missed` and `timeout` both reduce
it. The wrapper never counts a timeout as caught — a timed-out mutant was
neither killed by a test nor proven equivalent, so it is **UNRESOLVED** and is
reported in its own bucket while counting toward the missed side of the
denominator.

## Equivalent vs unviable — the important distinction

The two are often confused, but they mean opposite things:

- **Unviable** — the mutation does not compile, so the test suite cannot tell us
  anything about it. Cargo-mutants removes these from the denominator. They
  are a quality signal about mutant generation (or a Rust version / feature
  flag mismatch), never about test coverage.
- **Equivalent** — the mutated code compiles and behaves identically to the
  original, so *no correct test can ever fail on it* (e.g. `i += 1` → `i += 1`
  after constant folding, or a change to a code path the function never uses in
  that build). cargo-mutants reports these as **missed**: from its perspective
  the mutant survived, which is indistinguishable from a coverage gap.

This wrapper therefore **never calls a mutant "equivalent" on its own**: that
label could be fabricated. A mutant is reported as `equivalent` only when it is
**proven** (documented) in `scripts/mutation-equivalent.txt` AND it actually
survived in the current run. Everything else in `missed` is reported as a
genuine survival:

- `missed` in `summary.txt` / `MUTATION_SUMMARY` = survived MINUS documented-proven equivalents
- `documented_equivalent` = count of documented mutants that also survived this
  run (from `scripts/mutation-equivalent.txt`, copied to
  `target/mutation-artifacts/<profile>/equivalent.txt`)
- `unviable` is always reported in its own bucket and excluded from the
  denominator — it is never "equivalent".

## How to prove a mutant equivalent (do not guess)

1. Look at the survived mutants in `target/mutation-artifacts/<profile>/missed.txt`.
2. For a candidate, apply exactly the mutation described to a scratch copy
   (`cargo mutants --file <path>` or `--Zmutate-file` can show the diff) and
   confirm the *behavior* is semantically identical to the original: e.g. the
   mutation is a no-op code transformation, or removes a side effect the
   surrounding code never exercises in that build/feature-gate combination.
3. Record the mutant name verbatim (one per line) in
   `scripts/mutation-equivalent.txt`, with the reason after `#`:
   ```
   src/foo.rs:42: replace i + 1 with i | adds 1 to a value never used; behavior identical
   ```
   Lines starting with `#` are comments. Only exact matches against that run's
   `missed.txt` are counted; a recorded mutant that gets genuinely killed in a
   future run simply stops being counted.
4. Re-run the wrapper: the mutant moves from the reported `missed` count into
   `documented_equivalent`. Note the `score` itself does NOT change: the
   wrapper reports the distinction honestly rather than silently inflating the
   score.

Only **proven** equivalences are recorded this way. Guessing "it's probably
equivalent" and moving mutants out of `missed` without proof is exactly what
the failure-closed semantics are meant to prevent.

## Artifacts & CI line

Every run rewrites a deterministic directory:

```
target/mutation-artifacts/<profile>/
  summary.txt            full key=value inventory + MUTATION_SUMMARY line
  caught.txt missed.txt timeout.txt unviable.txt   mutant name lists
  equivalent.txt         copy of scripts/mutation-equivalent.txt
  in-diff-production-files.txt  (fast profile: files used for --in-diff)
  candidate-count.txt    generated mutants before filters
```

The CI-parsable line (stdout + `summary.txt`) is stable:

```
MUTATION_SUMMARY: profile=… exit=… total=… caught=… viable=… missed=… timeout=… unviable=… excluded=… score=… threshold=… result=PASS
```