---
name: pr-roast-triage
description: Triage all open PRs — detect duplicates, verify CI truth, roast, and emit manual merge order. Use when reviewing multiple open PRs or clearing a bot-generated PR backlog.
---

# PR Roast Triage

Review every open PR, close what has no independent impact, roast the rest
with fix recommendations. The roast emits a merge **order**; merging is an
explicit owner decision. **Never `--auto`**, and when told to merge, follow
Step 5 discipline: one PR at a time, each rebased onto the latest `main`
with CI re-verified on that head before it lands.

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

**A live duplicate can hold coverage the keeper lacks — fold, then dominate.** Diff the
rival's *test* files, not just its source: #834 proved the SIGTERM path on
`csm mcp serve --transport streamable-http` while the keeper #836 covered only
`watch`. Take that one dimension into the keeper's own harness and say in the
roast that the competitor supplied it. Two agents reaching the same design
independently is also the cheapest confirmation the design is the obvious one.

**Closed predecessors are duplicates too — and `gh pr list` cannot see them.**
A bot re-files the same finding under a new task id, so grep the roast records
for the *symbol* before writing a fresh verdict:

```bash
grep -rn '<symbol_or_function>' plans/PR_ROAST_*.md
```

If the idea was already measured and closed (this happened to
`HVec10240::permute`'s zero-shift branch: #800 measured → closed no-impact, #814
re-filed the same 3 lines with no evidence), the resubmission inherits the old
verdict. Close it citing the recorded numbers; do not re-litigate. Rule 17: a
resubmission must bring the evidence the earlier roast demanded, not restate the
claim — and check the claim's *mechanism* sentence against the code, since both
PRs asserted a zero-copy win that a by-value `-> Self` return rules out.
A predecessor closed **without** a valid measurement passes on the *demand*, not a verdict — and a predecessor closed **with** one still does not export its *framing*: #956 inherited #900's `csm-chaos is invisible to CI` headline long after `ci.yml` grew a `cargo test -p csm-chaos` job. Re-measure every environment claim before reusing it (grep the workflow, the profile, the default-members).
- **An empty branch is a resubmission to catch by content (2026-10-09, #860).** `gh pr diff` + `changed_files: 0` + empty `git diff <merge-base>..<head> --stat` = the PR merges nothing.

## Step 4 — Roast rubric (every keeper PR)

- **Atomicity**: one logical change. Mega-PRs (>5 files across concerns) must split.
- **Title honesty**: the title must describe the diff (`re-export` hiding a BFS cap = reject/retitle), and a body or rationale comment must describe a *measured* one: a prediction reads as a prediction (#953's race has never refused a release — nothing reached the step), and every quoted number is re-measured (my first "slowest gate pass 5m38s" was a *failure* run).
- **commitlint scope**: must exist in `commitlint.config.cjs` `scope-enum`
  (`perf(hyperdim)` fails; only listed scopes pass).
- **Rationale comments**: never deleted to "shorten" (`Never delete rationale comments`). Under 500 LOC gate pressure, bot PRs may attempt to strip docstrings/comments to make room for new code. Reject/roast this behavior; require extracting child submodules (e.g. `hyperdim_binary_serde.rs`) instead.
  - Detail, with the #767 `grep -c ADR-0094` 3→3 verification, the surviving-tag
    rule, restore-for-symmetry, flipped test expectations, `const fn` → `fn`, and
    #816's dedup-deletes-prose case: `references/rationale-comments.md`.
- **"Not breaking" is a semver claim, not a compile claim** (2026-10-03, #816 self-roast): a workspace
  that still builds says nothing about a removed public derive or impl. Use the repo's
  `**Breaking (scope)**` changelog label (count `!`-header precedent in `git log --format=%s` first), and
  re-measure every count an implementer agent reports — its summary is a lead, not evidence.
  **Positive-control a probe before believing a zero**: one path segment short returns 0 for every input,
  and the zero agrees with the story the title already told (#955 read `versions[*].attestations` instead
  of `versions[*].dist.attestations` and "proved" 0 of 11 npm versions unattested; the registry says 19
  of 21 were attested). Run the query on one input that must be non-zero before trusting its verdict.
- **`unsafe` bounds in generic code (PR #769, 2026-09-25)**: a `get_unchecked` SAFETY comment that
  hardcodes a dimension while the fn is generic over a trait constant is a **future out-of-bounds read**,
  not a perf tradeoff — all current impls matching proves nothing about the next one. Verify the
  invariant, then require the bound be type-enforced (`H::DIMENSION`) or the trait document it. Also check
  the *other* shift in the same loop: `hash_bits` is capped at 64 upstream precisely so `1u64 << i` cannot
  overflow.
- **Self-contradicting evidence tables (PR #768, 2026-09-25)**: a "measured" table with a headline number
  beside rows marked "Not measured" is a close. Also reject research scaffolding committed to the repo root
  (arXiv/Crossref search helpers, ad-hoc runners) and self-marked `[FALLBACK]` citations on a new public
  API. Close **as submitted, not as rejected** — name the clean diff that would be reviewable.
- **deny.toml**: must only ADD ignores with advisory ID + reason. Deleting or
  commenting out existing ignores re-breaks `cargo deny` — reject.
- **Perf claims**: no `criterion` output or flamegraph = not review-ready.
  - **A green evidence gate proves shape, not coverage (PR #763, 2026-09-25).**
    `scripts/check-perf-pr-evidence.py` checks that the PR body *has* a
    `## Performance Evidence` section naming a baseline, an artifact and a
    before/after pair — it never runs the bench. So the reviewer must trace the
    cited benchmark to the changed lines: find the bench id in `benches/`, then
    follow the guards it hits. #763 defers scaling inside `merge_single_list`,
    but the cited `merge_results_N1000_K20` passes *both* lists non-empty, so
    `merge_results` takes its HashMap path and the changed helper is never
    called — the "after" number was slower (92.5 → 93.3 µs) because it timed
    unrelated code. Check the sign too: an "after" that is *worse* is evidence
    against the claim, not neutral.
  - State all three in the verdict: the benchmark reaches changed code, its input distribution exercises the affected case, and the measured direction/sign supports the claim. A `perf` → `refactor` title change never bypasses this evidence bar.
  - **A bench can be blind three ways (#800; #909/#891; #917).** By its *arguments*
    (`permute(321)` never reaches the word-aligned path); by the *callee's fast path*
    (`make_candidates` prefills `score = 1.0`, so `rerank`'s `>= 0.99` return starved every
    `mmr_rerank/*` id); and by a *flat corpus* — #917's `sort_unstable_by_key(entries.len())`
    ran on every bm25 bench and reordered nothing, because each feeds equal-length posting
    lists. Reachability shows at the call site; **blindness lives in the data**, so for an
    ordering or selection claim read the corpus generator's key distribution. Proof recipe:
    the `bench-blindness` reference.
  - **Separate zero-fill elimination from scheduling effects (PR #783, 2026-09-28).** LLVM
    already removes the overwritten zero-init — an old-only body disassembles to a bare
    `memcpy`, old/new `from_hvec`/`from_bytes` folding to the *same address* — so "no
    demonstrated zero-fill benefit" is not "zero codegen delta": a scheduling claim needs
    reproducible affected-path benchmarks. Disassembly recipe (`#[no_mangle]
    #[inline(never)]` in a linked binary — raw `--emit obj/asm` drops unreferenced `pub fn`s)
    and #893's initializer-*position* caveat: `references/codegen-and-measured-claims.md`.
  - **Evidence belongs in the PR body.** A `## Performance Evidence` section in a commit
    message satisfies neither the gate nor a reviewer. A passing gate prints what it accepted
    (`shape OK — benchmark ids: …; artifact: …`) plus a one-line scope warning; read that CI
    log line to see the claim without opening the body.
- **Correctness and impact are separate verdicts** (PR #763, 2026-09-24). Never
  let a correct diff pass on a bad claim, nor a good claim excuse a wrong one.
  Adjudicate each independently:
  - *Correctness* — provable by reading plus a throwaway harness. For an
    algebraic/monotonicity argument, brute-force it: 186 (N,k) cases with ties,
    negatives and duplicates, comparing subset **and returned values**
    (`worst_abs_diff` must be 0). Zero mismatches → "correct, bit-identical".
  - **Test the public input domain, not just internal callers (#790/#793).**
    Positive internal weights do not prove monotonicity for an arbitrary-weight
    public API: negative weights reverse raw-score ordering. Close an incorrect
    duplicate as submitted; do not call its separate unmeasured allocation idea
    a proven no-op. A resubmission needs corrected parity tests and path evidence.
  - *Impact* — only by measurement on `csm-ref-01`. A FLOP-count reduction
    inside a function dominated by allocation/partitioning is **not** a
    speedup: check what the dominant term actually is before accepting the
    claim. Interleave A/B/A/B and demand a consistent sign; shared hosts
    sign-flip at ±25 % (same class as "Benchmark under load"). Measure the
    PR's own base in a worktree: a claimed "after" can be slower than the head
    measures (#889) — `references/codegen-and-measured-claims.md`.
    **Toggling the source is not an interleave** (#917): legs straddling a rebuild read as
    a 25 % *win* for a change that is 8 % slower, because absolute times on this host drift
    ~50 % with desktop load. Build both arms, `cp` the two
    `target/release/deps/<bench>-<hash>` binaries apart (cargo reuses the hashed name per
    variant), run `A B A B` from `/tmp` with no rebuild between, print `/proc/loadavg` per
    leg, and pass `--bench` — without it criterion runs test mode and prints no timings.
  - A **correct-but-unmeasured** PR is not a no-op: request the evidence
    (`## Performance Evidence`, enforced by `scripts/check-perf-pr-evidence.py`) and the test
    locking the new correctness property, and keep the PR open. Reserve *close as no-op* for
    zero-delta or wrong diffs.
  - Demand the parity test whenever the justification is a correctness argument — it is worth
    more than the benchmark. And look for the degenerate case: at `k == N` the partition is
    skipped, so deferring per-element work is a pure regression. Guard it.
- **Five things a green CI cannot tell you about a test diff** (2026-10-05, #834):
  a subprocess test needs a deadline on every `child.wait()`, readiness synchronised on
  the child's own output rather than `sleep(200ms)`, and an explicit `--database` (with
  none `src/cli/args.rs:20` writes into the checkout); an `async` test that only
  *constructs* a future asserts nothing but compilation, and
  `check-test-attributes.sh` cannot see that; a mutation exclusion is not judgeable by
  eye — `--exclude-re` matches the mutant *description*, `--exclude` is a file glob, and
  `--exclude-re "mod::path"` silently matches nothing; and a new `pub mod` for an
  internal need is a public-API cost that `llms.txt` counts. Distrust an `--exclude` of a
  file whose only tests sit in a target the profile never runs — and apply that to your
  own branch first, because reachability is not sharpness: the fast profile is `--lib`
  only (`scripts/mutation_test.sh:142`), so `tests/**` cannot kill a mutant however
  precisely it pins the observable. #836 scored 0.0000% discovering this about itself.
  Detail + measured counts: `references/subprocess-and-claim-rubric.md`.
- **Green CI can be green about another package (2026-10-09, #866).** Root `--all-features` gates
  select only `workspace_default_members` (here the root package); build member features with `-p`.
- **`export.json` / `Cargo.lock` noise**: timestamp-only or resolver-churn hunks
  must be dropped before merge.
- **Bot comments** (Jules hello, Sonar/Codacy pass notes) are noise, not reviews.
  Zero human comments = no hidden requirements, but also no approval.
- **GitHub CLI shapes that fail quietly**: `gh pr edit`/`gh issue view` break on deprecated
  `projectCards` — query specific `--json` fields, or `gh api -X PATCH …/pulls/<id> -f title=…`. This
  build's `gh pr close` has **no `--comment-file`** and `--comment "$(cat f)"` command-substitutes the
  backticks a roast body is full of: `gh pr comment <n> --body-file f`, then close, then re-read `gh pr
  list --state open` — three "successful" closes here left three PRs open.
- **Multi-issue parent linkage**: a parent PR implementing several child issues declares each
  (`Fixes #A`, `Fixes #B`) in the body, so all issues close cleanly on merge.

## Step 5 — Manual merge order (emit, do not execute unless instructed)

1. Trivial green first (dependabot 1-liners).
2. Security/clamp fixes (small, high value).
3. Foundation before dependents (commitlint scope additions, owner/facade
   migrations before perf touches on the same files).
4. One keeper per duplicate cluster; closes reference the keeper.
5. Mega-PRs and SIMD/`unsafe` last (need evidence + rebase after everything).
6. **One PR at a time, and only from a current head.** Each merge moves `main`,
   so every subsequent PR is behind by construction. Never `--auto`, never
   batch. A green CI on a stale head proves nothing about the tree that lands.

```bash
# per PR, in order — the rebase is mandatory, not optional:
git fetch origin main
git rebase origin/main <branch>        # or: gh pr update-branch <n>
git push --force-with-lease
gh pr checks <n>                       # wait for GREEN on this new head
gh pr view <n> --json mergeable,mergeStateStatus   # expect MERGEABLE/CLEAN
git diff --stat origin/main...<branch> # no surprise reverts?
gh pr merge <n> --squash --delete-branch
```

Verified-safe shortcut: `gh api -X PUT repos/<owner>/<repo>/pulls/<n>/update-branch` then wait
for CI; confirm a run exists for the *post-rebase* SHA before trusting a green tick.

## Step 6 — Record in codebase

- Full roast → `plans/PR_ROAST_<YYYY_MM_DD>.md` (per-PR verdict table + order + commands).
- Pre-existing flakes/learnings → append `progress/LEARNINGS.md`.
- `plans/GOAP_STATE.md`: update counters in place, set `action_last_completed`
  (exactly once, last key) to the triage action.
