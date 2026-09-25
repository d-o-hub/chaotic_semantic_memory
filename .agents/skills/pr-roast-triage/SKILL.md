---
name: pr-roast-triage
description: Triage all open PRs — detect duplicates, verify CI truth, roast, and emit manual merge order. Use when reviewing multiple open PRs or clearing a bot-generated PR backlog.
---

# PR Roast Triage

Review every open PR, close what has no independent impact, roast the rest
with fix recommendations. The human merges manually in the emitted order —
never auto-merge, never merge from this skill.

## When this gate applies (MANDATORY)

Roast **before implementing or merging** any GitHub PR or issue — never take
one at face value, however green its CI or plausible its description. Bound in
`AGENTS.md` (Phase 1 step 5, Phase 5 step 17, Core Rule 9):

1. **Before implementing an issue/PR** — verify the premise, the affected code
   path (which copy production actually executes), and the evidence bar *before*
   writing code. A no-impact premise → close as no-op with the roast comment.
2. **Before merging** — CI green is necessary, not sufficient. Emit a verdict
   and record it (Step 6). No verdict, no merge.
3. **On a no-impact PR** — roast comment first, then close, then update
   `progress/PROGRESS.md` + `progress/LEARNINGS.md`, then distill the reusable
   lesson into a skill. Do **not** quietly re-implement the closed idea; a
   resubmission must carry the evidence the roast demanded.
4. **Distill, don't duplicate** — the lesson goes into an existing skill
   (compact it) unless it opens a genuinely new domain; a near-duplicate skill
   is itself debt. `scripts/validate-skill-format.sh` gates skill files.

## Step 1 — Triage baseline (run first, before any opinion)

```bash
./scripts/pr-triage.sh   # conflict status + CI status + suggested order
git status --short       # scope out unrelated local changes first
git diff HEAD --stat
```

## Step 2 — CI truth (annotations, not badges)

A red badge is not a verdict. Pull the failing step's annotation:

```bash
gh api repos/<owner>/<repo>/check-runs/<job_id>/annotations \
  --jq '.[].message | .[0:200]'
```

Known infra flakes (re-run, do not "fix code"):
- `Unable to authenticate to FlakeHub` → Cargo Deny nix setup broken, pre-existing.
- `hosted runner lost communication` → CodeQL/runner loss, pre-existing.
- External-only red (Codacy/Sonar) with all GitHub checks green → code issue, real.

## Step 3 — Duplicate detection (same base blob + same hunk)

Bot swarms file near-identical PRs. Compare, don't assume:

```bash
gh pr diff <a> --name-only
gh pr diff <b> 2>/dev/null | sed -n '1,80p'   # same base hash? same function?
```

Same file + same base blob + same effect = duplicates. Keep exactly one:
1. Green CI beats mergeable-but-red.
2. Fewest unrelated files (no `export.json` timestamp, no lockfile churn).
3. Backward-compat wrapper kept beats private-fn signature break.
4. Newest branch (rebases cleanest onto latest main).

Close the losers with reason `superseded by #<keeper>`.

## Step 4 — Roast rubric (every keeper PR)

- **Atomicity**: one logical change. Mega-PRs (>5 files across concerns) must split.
- **Title honesty**: title must describe the diff (`re-export` hiding a BFS cap = reject/retitle).
- **commitlint scope**: must exist in `commitlint.config.cjs` `scope-enum`
  (`perf(hyperdim)` fails; only listed scopes pass).
- **Rationale comments**: never deleted to "shorten" (`Never delete rationale comments`). Under 500 LOC gate pressure, bot PRs may attempt to strip docstrings/comments to make room for new code. Reject/roast this behavior; require extracting child submodules (e.g. `hyperdim_binary_serde.rs`) instead.
  - **Verify before accusing** (PR #767, 2026-09-25). A 21-line doc removal
    *looked* like comment-stripping, but `grep -c "ADR-0094"` returned 3 → 3:
    the rationale was reworded, not lost. Count the knowledge that survives
    (`grep -c` the ADR/issue tag on both sides), then name the exact contract
    text that did not — here the `#[cfg(feature = "persistence")]` prose
    ("Only available when…", "no-op since persistence is unavailable") on ~7
    methods. Rewording is a nit; losing a documented contract is the
    violation. Never report the former as the latter.
  - A PR can violate this rule and still be **right on code** — #767's
    `with_chaos_strength` leaked `NaN`/`±∞` into the reservoir. Say both, and
    name which one blocks the merge.
  - **Flipped test expectations are contract changes.** A test asserting
    "negative strength fails" becoming "negative clamps" belongs in the body
    as a deliberate contract change, not filed as a test fix.
  - **`const fn` → `fn` is a public API change**; say so even when in-repo
    callers are all runtime.
- **`unsafe` bounds in generic code (PR #769, 2026-09-25)**: a `get_unchecked`
  SAFETY comment that hardcodes a dimension while the fn is generic over a
  trait constant is a **future out-of-bounds read**, not a perf tradeoff — all
  current impls matching proves nothing about the next one. Verify the
  invariant, then require the bound be type-enforced (`H::DIMENSION`) or the
  trait document it. Also check the *other* shift in the same loop: `hash_bits`
  is capped at 64 upstream precisely so `1u64 << i` cannot overflow.
- **Self-contradicting evidence tables (PR #768, 2026-09-25)**: a "measured"
  table with a headline number beside rows marked "Not measured" is a close.
  Also reject research scaffolding committed to the repo root (arXiv/Crossref
  search helpers, ad-hoc runners) and self-marked `[FALLBACK]` citations on a
  new public API. Close **as submitted, not as rejected** — name the clean
  diff that would be reviewable.
- **deny.toml**: must only ADD ignores with advisory ID + reason. Deleting or
  commenting out existing ignores re-breaks `cargo deny` — reject.
- **Perf claims**: no `criterion` output or flamegraph = not review-ready.
- **Correctness and impact are separate verdicts** (PR #763, 2026-09-24). Never
  let a correct diff pass on a bad claim, nor a good claim excuse a wrong one.
  Adjudicate each independently:
  - *Correctness* — provable by reading plus a throwaway harness. For an
    algebraic/monotonicity argument, brute-force it: 186 (N,k) cases with ties,
    negatives and duplicates, comparing subset **and returned values**
    (`worst_abs_diff` must be 0). Zero mismatches → "correct, bit-identical".
  - *Impact* — only by measurement on `csm-ref-01`. A FLOP-count reduction
    inside a function dominated by allocation/partitioning is **not** a
    speedup: check what the dominant term actually is before accepting the
    claim. Interleave A/B/A/B and demand a consistent sign; shared hosts
    sign-flip at ±25 % (same class as "Benchmark under load").
  - A **correct-but-unmeasured** PR is not a no-op: request the evidence
    (`## Performance Evidence`, enforced by `scripts/check-perf-pr-evidence.py`)
    and the regression test that locks the new correctness property, and keep
    the PR open. Reserve *close as no-op* for zero-delta or wrong diffs.
  - Demand the parity test whenever the diff's justification is a correctness
    argument — that test is worth more than the benchmark.
  - Look for the degenerate case: when `k == N` the partition is skipped, so
    deferring per-element work is a pure regression. Guard it.
- **`export.json` / `Cargo.lock` noise**: timestamp-only or resolver-churn hunks
  must be dropped before merge.
- **Bot comments** (Jules hello, Sonar/Codacy pass notes) are noise, not reviews.
  Zero human comments = no hidden requirements, but also no approval.
- **GitHub CLI GraphQL deprecation**: `gh pr edit` and `gh issue view` fail if querying
  deprecated `projectCards`. Query specific `--json` fields for reads, or use
  `gh api -X PATCH repos/<owner>/<repo>/pulls/<id> -f title="..."` for editing PR titles.
- **Multi-issue parent linkage**: When a parent PR implements multiple child issues,
  ensure all child issues (`Fixes #A`, `Fixes #B`) are declared in the PR body so all
  issues close cleanly upon merge.

## Step 5 — Manual merge order (emit, do not execute)

1. Trivial green first (dependabot 1-liners).
2. Security/clamp fixes (small, high value).
3. Foundation before dependents (commitlint scope additions, owner/facade
   migrations before perf touches on the same files).
4. One keeper per duplicate cluster; closes reference the keeper.
5. Mega-PRs and SIMD/`unsafe` last (need evidence + rebase after everything).
6. After each merge: rebase next, re-run CI, re-check `mergeable`.

```bash
# per PR, in order:
git fetch origin main
gh pr checks <n>                       # all green?
git diff --stat origin/main...<branch> # no surprise reverts?
gh pr merge <n> --squash --delete-branch
```

## Step 6 — Record in codebase

- Full roast → `plans/PR_ROAST_<YYYY_MM_DD>.md` (per-PR verdict table + order + commands).
- Pre-existing flakes/learnings → append `progress/LEARNINGS.md`.
- `plans/GOAP_STATE.md`: update counters in place, set `action_last_completed`
  (exactly once, last key) to the triage action.
