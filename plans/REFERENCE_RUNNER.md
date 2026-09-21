# Reference runner (ADR-0095 Tier 3)

Release performance claims must name the machine and toolchain they were measured
on, so a reader can tell whether a number is comparable to their own setup.

## Named runner: `csm-ref-01`

| Field | Value |
|---|---|
| CPU | Intel(R) Core(TM) i5-8350U (4 cores / 8 threads, 1.70 GHz base) |
| RAM | 16 GB (`MemTotal: 16 247 740 kB`) |
| Disk | NVMe (`/dev/nvme0n1p2`), ext4 |
| OS | Linux 7.0.0-31-generic, x86_64 |
| Toolchain | rustc 1.88.0 (pinned by `rust-toolchain.toml`) |
| Build profile | `release`; `CARGO_BUILD_JOBS=2`, `-j 2` (this machine OOMs during parallel linking of the full test matrix) |

`csm-ref-01` is the laptop this repository was developed on; it is named so that
the artifacts produced on it can be identified as such rather than presented as
universal. CI runners (`ubuntu-latest`, `ubuntu-24.04-arm`) are a *different*
profile: numbers from them are only comparable to other CI runs.

## Rules for claims

1. **Every published performance number names its runner** — `csm-ref-01` or the
   CI runner, with the toolchain. The criterion baseline artifact records both
   (`plans/evidence/bench/canonical.json`: `cpu`, `os`, `toolchain`, `commit`,
   `dirty`).
2. **Cross-runner comparisons need a ratio, not an absolute.** A claim like
   "37.7 ns idle / 54.5 ns loaded" is meaningful only with the load and the
   runner stated (`progress/LEARNINGS.md`, "Benchmark under load").
3. **Release claims additionally need one of**: a committed canonical baseline
   measured on this runner, or a CI job that measures the same benchmark with a
   documented ceiling (for example the graph-candidates 600 µs gate).
4. **Wall-clock claims are re-measurable**: every artifact records the exact
   command, warm-up, measurement time and sample count so a reviewer can rerun
   it and compare.

## How to produce/verify a baseline

```bash
scripts/bench-baseline.sh save                 # record plans/evidence/bench/canonical.json
scripts/bench-baseline.sh compare              # same bench set, ±20 % tolerance
scripts/bench-baseline.sh compare --tolerance 0.10 --set all
```

`pre-release-validate.sh` section 8 runs the compare mode, so a release cannot
ship on a machine that regresses the canonical set beyond tolerance. The
artifact stores Criterion's median *with its confidence interval*
(`ci_lower_ns`/`ci_upper_ns`), which is what the ADR asks for.

## Machine-specific notes

- `cargo clean` removes `target/` (116 GiB after a week of `--all-features`
  matrices) but never the committed baseline — that is the point of the artifact.
- The full `--all-features` test suite takes ~20 minutes warm on `csm-ref-01`;
  a Criterion baseline run of the `core` set takes a few minutes plus build.
