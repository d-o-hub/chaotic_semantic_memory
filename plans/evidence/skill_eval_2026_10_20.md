# Behavioral Skill Evaluation — 2026-10-20

## Scope and method

Evaluated the five critical skills selected by the 2026-10-19 audit: `rust-development`, `testing-validation`, `git-workflow`, `pr-roast-triage`, and `self-fix-loop`. Each skill has four deterministic cases in its `evals/evals.json`; 20 cases ran against the current `SKILL.md` content.

Each response was checked against every case assertion with `judge_batch` using `typesafe/jev-1.13.0`. A case passes when the judge's probability that **all** assertions are satisfied is at least 0.50. This is a functional pass rate, not a claim about time saved or repeat-run consistency.

## Results

| Skill | Case probabilities | Passed |
|---|---:|---:|
| `rust-development` | 0.87, 0.94, 0.93, 0.87 | 4/4 |
| `testing-validation` | 0.91, 0.91, 0.89, 0.89 | 4/4 |
| `git-workflow` | 0.79, 0.94, 0.94, 0.89 | 4/4 |
| `pr-roast-triage` | 0.86, 0.89, 0.90, 0.84 | 4/4 |
| `self-fix-loop` | 0.73, 0.58, 0.92, 0.93 | 4/4 |
| **Total** | **mean probability 0.871** | **20/20 (100%)** |

The batch completed 20/20 judgments with no judge failures. Earlier eval iterations missed required details in performance evidence and mismatched push confirmation; those skill rules and the corresponding cases were tightened, then the full suite was rerun. The final run clears the >90% success-rate target. The lowest-confidence case (self-fix-loop case 2, 0.58) still passes at the defined threshold and remains a useful follow-up case.

## What the failures taught

The first measured run scored 18/20 and exposed two rubric gaps that the skills did not state:

- `pr-roast-triage` case 3: the response rejected the unreachable benchmark but never said that
  retitling `perf` → `refactor` is gate-gaming. The skill now states that the verdict must cover
  reachability, workload, and measured sign, and that a retitle never bypasses the evidence bar.
- `self-fix-loop` case 4: the response refused the mismatched SHA but never mentioned hooks/CI.
  The skill now names a remote/branch/SHA mismatch as a hard stop that also forbids bypassing the
  refusal or skipping hooks/CI.

A third case (`git-workflow` case 2) was fragile: the response listed the gates but omitted the
full-range commitlint check. The skill's pre-commit checklist now states that commitlint applies to
the entire branch range on every change, and the case prompt asks for every remaining check.

The case prompts were rewritten to ask for the *decision plus the verification*, not only the
refusal — a refusal that omits the compensating check is the failure mode these skills must avoid.
