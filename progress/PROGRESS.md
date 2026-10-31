# PROGRESS

## 2026-10-30: drained the queue by roasting, and roasted my own PR before merging it

`main` at `3953e818` (#947, the audit-follow-up wave, squash-merged 12:55:38Z). Full verdicts in
`plans/PR_ROAST_2026_10_30.md`.

**The drain (#956/#957/#958) closed the queue to zero without a single label.** The owner's instruction
was explicit: do not label other people's PRs to route around `pr-drain-check`; roast each and let the
gate pass on its own. All three were closed with measured verdicts (comments
`6522738677`/`6522739077`/`6522739477`, 4,268/6,636/5,894 chars) and `gh pr list --state open` went
4 → 1 → 0. #956 was the only one carrying a real defect: `ChaoticLsh::new`/`VoronoiLsh::new` take an
unbounded `input_dim` and both hot paths return an **all-zero hypervector** on a length mismatch
(`crates/csm-chaos/src/hashing/chaotic_lsh.rs:44-46,88-90`, `voronoi_lsh.rs:46-47,78`), so the
`.min(MAX_INPUT_DIM)` clamp the PR proposed would convert a loud panic into silent wrong retrieval
embeddings. It is preserved as issue **#961** with the five requirements a re-created fix must carry —
reject, do not clamp; one validation point per constructor; the mismatch arms tested; a bucket test like
its sibling maps already have; and the 500-LOC child-module split done by extraction rather than by
deleting prose. #957 (`Svsc4d`) was the *second* filing of `experimental-svsc-4d` after #900 closed the
same idea on the same two blockers, so rule 17 applied and it inherited the verdict. #958 was the most
technically correct of the three and still closed: its SIMD `xor` adds no test for `xor` at all — `grep -n '^+.*[.]xor()' ` on the PR diff reads 0, and on `main` the same grep over `tests/` and `crates/*/tests/` is empty too, the cited `#define XOR_PSHUFFLE 1`
lines belong to Intel's ISA-L sample directory rather than to the shipped path, and — the honest half —
its 10 “deleted” tests are not deleted: they moved to `crates/csm-core-lib/src/hyperdim_simd_tests.rs`
behind `lib.rs:48-51` `include!`, which is this repo's prescribed answer to the 500-LOC cap. Three of my
own framing claims about #958 did not survive reading the diff.

**#947 itself was roasted before it merged** (it is the biggest PR of the year: 159 files). The evidence
gate that mattered was mutation, and it was measured, not asserted: **94.0476 %** on that head against a
62 % initial reading — the difference being 19 cfg-blind `wasm32` phantom mutants in the denominator.
The exclusions were audited rather than trusted: the surviving 7 mutants and one over-broad
observability glob are recorded as residuals, not as a green score.

**Then #962 fixed the last publication blocker** (`#953`: `release.yml` and `pre-release-gate.yml` are
siblings on one `workflow_run`, so `validate`'s one-shot gate read samples a verdict that cannot exist
yet — `validate` runs 62 s, the slowest gate pass is 4m37s). Two things about that PR are the point of
the round, not the fix: it is the first PR this session that got roasted *against itself* hard enough to
change. Four separate overstatements in my own writing were caught by re-deriving state — a failed gate
run quoted as the slowest pass, a “refused a release” failure that has never happened (the workflow was
unparsable, so nothing reached the step), a diffstat read off a stale commit, and an invented full SHA
that returned an empty run list. And the design objection I thought I had found independently — *why
poll when you can trigger on the event* — turned out to be standing repo policy since 2026-09-30
(`progress/LEARNINGS.md:85`), which my first verdict had re-derived and then ignored. The merged form
discloses the departure and its measured justification (69 consecutive jobless release runs from
`0f77c0d0` to #947: silence is the failure mode this repo has actually lived through), instead of
arguing it in a private note.

GOAP: 9 queued actions (#960 #954 #955 #961 #948 #949 #951 #952 #959), `action_last_completed:
repair_the_release_gate_race`, `release_pipeline_validated: false` until the tag and both npm channels
and crates.io verify. `CSM_GOAP_QUEUE_REQUIRED=true scripts/check-goap-queue-issues.sh` → 9/9/9, rc 0.

## 2026-10-19: a reorder claim that no benchmark in the repo could see

Six open PRs, empty GOAP queue, 0 issues, `main` at `0ab86f4d`. The round's work was the mandatory
pre-merge roast (`AGENTS.md` Phase 1 step 5), recorded in `plans/PR_ROAST_2026_10_19.md`.

**#917** (draft Jules `perf(retrieval)`, +6/-0 in `crates/csm-retrieval/src/bm25.rs`) was **closed as
no demonstrated impact** — the seventh member of the #737/#739/#740/#754/#783/#800/#915 class, with
`commitlint` red on `scripts/check-perf-pr-evidence.py` and a body that offered a cache-locality
story instead of numbers. What made this one different from a shape-check is that the claim was
*testable*, and testing it refuted it twice over:

1. **Every bm25 corpus in the repository has uniform posting lengths**, so sorting by
   `entries.len()` swaps nothing. `benches/benchmark.rs:711-719` indexes all four tokens into every
   document; `benches/bm25_benchmark.rs:19-29` gives doc `i` token `j` iff `(i + j) % 2 == 0`, so
   each of the ten tokens lands in exactly 500 documents; the `selective_100000` corpus
   `(7i + 13j) mod 1000` gives exactly 2 000 postings per term. The branch executes on all of them
   and is a semantic no-op on all of them — a third blindness cause alongside wrong arguments
   (#800) and the callee's fast path (#909/#891), and only discoverable by reading the corpus
   generator rather than the call site.
2. **In the one corpus where it does reorder — 20 000 docs, postings 20 000 / 2 000 / 200, query
   supplied descending — it is ~8 % slower.** Median 168.3 → 180.0 µs across four time-interleaved
   pairs (+5.1 / +9.0 / +10.1 / +7.2 %, consistent sign).

The first measurement pair was *wrong in the flattering direction*: 190 µs with the sort against
255 µs without, a fake 25 % win, because the legs straddled a rebuild while desktop load fell. Absolute
times on `csm-ref-01` drifted ~50 % across the session and the canonical `search_1000_docs`
A/B/A/B sign-flipped (11.59 / 10.87 / 11.81 / 14.82 µs). The fix that worked: compile each arm,
copy the two `target/release/deps/bm25_benchmark-*` binaries apart, then run `A B A B` with no
rebuild between, printing loadavg per leg. That method, the flat-corpus check and the honest
withdrawal of my own tie-break objection (bit-identical `to_bits()`, same top-5 ids on a corpus
built to expose it) are distilled into `skill://pr-roast-triage` and
`progress/LEARNINGS.md`; the stale `BM25 search 10k ~406 µs` baseline was re-measured at 113–116 µs.

The five dependabot PRs (#918 `taiki-e/install-action` 2.87.27 → 2.87.29; #919 root `serde_json`
1.0.151 → 1.0.154 + `tokio` 1.53.2 → 1.53.3; #920 root `proptest` 1.11.0 → 1.12.0; #921/#922 the
same two in `fuzz/Cargo.lock`) are keepers: all `MERGEABLE`, each head contains `origin/main`, each
diff minimal, and root vs `fuzz/` lockfiles are different workspaces so they are not duplicates of
each other. They were pending on `lint`/`test`/`mutation-test`/`Build CLI (linux-x64)` at triage, so
they land one at a time — rebase, CI on the new head, re-check `mergeStateStatus`, squash-merge —
per Phase 5 step 18.

## 2026-10-05 (round 8): the queue drifted again behind its own fix, and I read a pipe's exit code as the script's

### Three merges, one at a time, each re-based onto the moving `main`
**#846** merged as `77bd5f0` (tree verified byte-identical to the reviewed head `099ddab` after the
squash), **#852** as `83e9a6c` (#849: `quality-gates.sh` was handing `cargo nextest run --quiet`,
which dies in argument parsing with RC=2 before a test compiles), **#854** as `2984d3e` (#841: yamllint
severity split, `.yamllint` profile, 12-case fixture, apt-installed yamllint in the `lint` job because
`pip` hits PEP 668 on ubuntu-24.04). **#847** was re-based twice (onto `83e9a6c`, then onto `2984d3e`)
and its two fixture suites re-run on each new base — git auto-merged `validate.sh` and `ci.yml` without
conflict, so the merge was verified semantically rather than trusted: `check-skill-catalog.sh` at
`validate.sh:235` and `validate-workflows.sh --check` + its fixture at `:297-303` both survive, and
`ci.yml` carries the apt yamllint step *and* the two catalog steps.

### The drift recurred while I was writing the gate for it
Round 7 recorded `cover_sigterm_in_server_shutdown` as removed "queue 8 → 7". **Git says that never
happened**: `git show 2f26214:plans/ACTIONS.md | grep -c '^  - name:'` = **10**, and so are `f235874`,
`87fa734` and `6ec3cf6`. The count only dropped to 8 at `77bd5f0`, when #846's branch happened to carry
the removals. Measured per commit this round:

| commit | queue length | what landed |
|--------|--------------|-------------|
| `87fa734` | 10 | #837 (`derive_ci_crate_matrix_from_workspace`) — entry left alive |
| `f235874` | 10 | #838 (`validate_archive_manifest_completeness`) — left alive |
| `2f26214` | 10 | #836 (`cover_sigterm_in_server_shutdown`) — left alive, `in_progress` |
| `6ec3cf6` | 10 | #848 (`deduplicate_unreleased_changelog_headings`) — left alive |
| `77bd5f0` | 8 | #846 — removed three, added #851's entry |
| `83e9a6c` | 8 | #852 |
| this commit | 10 | removes #832, queues #840/#841/#842 |

So the `queued_actions_count` ledger in `GOAP_STATE.md` was describing removals that had not occurred,
and my own prose repeated the error. `deduplicate_unreleased_changelog_headings` (#832) was still
queued on `83e9a6c` — a fifth same-day instance, created *after* #846's branch was cut, which is the
strongest argument the reconciliation must be a gate and not a hand pass.

### The gate (#851) is implemented, and one of its checks was wrong until measurement
`scripts/check-goap-queue-issues.sh` (277 lines) + `scripts/test-goap-queue-issues.sh` (22 tests, 46
assertions, stubbed `gh`) wired into `validate.sh`. One `gh issue list --state all --limit 500` call
feeds STALE and UNQUEUED; `SKIP — NOT A PASS` when `gh` is absent or unauthenticated while UNDECLARED
and STATE still run; `CSM_GOAP_QUEUE_REQUIRED=true` makes an unreachable tracker fatal; missing plans
files and a zero-action parse both fail closed. **A check I specified was wrong**: I asked for "every
queued action's effect must be declared in `world_state`", and on `83e9a6c` that flagged 8 of the 10
effect keys — a pending effect is by definition not held yet, and pre-declaring each as `false` only
relocates the drift to "flag says false, reality says true". Replaced with the inverse, which is the
failure that actually occurred: a queued action whose effect `world_state` reports `true`, and an
`action_last_completed` naming an action still in the queue. Each of the five checks was shown red by
neutering its implementing line, and every restore verified `sha256`-identical.

### Two measurements I got wrong and then corrected
1. `bash scripts/validate-links.sh | tail -8; echo $?` reported **0**, which is `tail`'s status, not
   the validator's. Re-run without the pipe: `rc=1`. Both link validators are genuinely red on `main`
   (5 broken refs and 39 issues) and genuinely unwired — not fail-open, which is what the first reading
   implied and what would have gone into #840's note as a false defect.
2. My round-8 note on #847 claimed "three SC2016 findings removed by quoting the heredoc". The commit
   says seven: two removed honestly (`render_catalog` rewritten as one quoted heredoc, regeneration
   proven byte-identical), five given `# shellcheck disable=SC2016` with a reason. Corrected in
   `plans/ACTIONS.md` before the commit, from `git show --stat` rather than from recall.

### Roasting #855 found that its tracker half would never have run in CI
The rubric applies to my own PR, so I ran it: `grep -n "CSM_GOAP_QUEUE_REQUIRED" .github/workflows/`
returned **nothing** — no job set the flag, and the `lint` job inherits the workflow-level
`permissions: contents: read`, which does not include `issues: read`. So STALE and UNQUEUED — the two
checks that exist because four landed PRs left their queue entries alive — would have printed
`SKIP — NOT A PASS` on every CI run, permanently. A gate whose decisive branch never executes on the
machine that matters is #853's machine-asymmetry class in a new place. Three fixes, all measured:
`permissions: {contents: read, issues: read}` on the `lint` job (a job-level block **replaces** the
workflow-level one, so `contents` must be restated or `actions/checkout` loses its token),
`GITHUB_TOKEN` + `CSM_GOAP_QUEUE_REQUIRED=true` on the `validate.sh` step, and the deletion of the
`gh auth status` probe that sat in front of the real query. The probe was the worse of the two findings:
it is a second, independent path to "tracker unreachable", and it can fail on a runner whose token
answers `gh issue list` fine. The gate now decides from that one call, folds its stderr into the reason
(test 13b asserts the literal `HTTP 403` survives into the SKIP line), and treats rc=0 with a non-JSON
body as unreachable rather than parsing it as zero issues — a truncated payload read as an empty backlog
would have been a *pass*. Test 2 tightened from `gh calls <= 2` to `-eq 1` so re-adding a probe is caught.
Fixture: 46 → **50 assertions, 23 tests**, red demonstrated for the shape guard by replacing its
condition with `false` (3 failures, all in 13c). `yamllint -c .yamllint` unchanged at 16 findings and 0
errors; `shellcheck` clean at every level on both scripts.

### Bookkeeping
`plans/ACTIONS.md`: removed #832's entry, revised #829 to the parent claim `single_gate_graph`
(its two achieved effects moved to `GOAP_STATE.md` as `true` — a queued action advertising an effect
the state file already reports true is now a gate failure), added `reconcile_and_wire_the_two_link_validators`
(#840), `classify_yaml_findings_in_validate_workflows` (#841) and `consolidate_the_three_hook_installers`
(#842). `plans/GOAP_STATE.md`: `main_head: "83e9a6c"`, `queued_actions_count` with the measured ledger,
`action_last_completed` (exactly once, last key).
**The gate caught its first drift before this branch was committed.** #854 merged as `2984d3e` while the
docs were being written; `Fixes #841` closed the issue, and `check-goap-queue-issues.sh` run against the
uncommitted tree reported the entry I had *just added* as STALE — then, after removing it, reported
`queued_actions_count is 10 ... holds 9 action(s)`. Both were fixed in place; final state:
`ok: GOAP queue reconciled — 9 action(s), 9 tracked issue(s), 10 effect key(s), one gh call`, RC=0. Five
drift instances earlier the same day took a human to find; the sixth took two seconds and needed no one.

## 2026-10-05 (round 7): a merged PR left its queue entry alive, and my own roast published a false protection claim

### #836 merged as `2f26214` — and `Fixes #824` closed the issue while the plan file did not
Verified before merging, not after: head current with `main`, `mutation-test` **success** on that
head (the `src/shutdown_tests.rs` arms from round 4), `lint` and the crate matrix green. After the
squash, `src/shutdown.rs::operator_shutdown()` selects SIGINT *and* SIGTERM, both callers go through
it (`src/cli/commands/watch.rs:91`, `src/mcp/server.rs:79`), and `Fixes #824` auto-closed the issue.
What did **not** happen automatically: `cover_sigterm_in_server_shutdown` was still in
`plans/ACTIONS.md` on the new `main`, contradicting that file's own header rule ("when an action
completes, remove it here"). GitHub closes issues; nothing closes queue entries. Closed by hand this
round, queue 8 → 7 counted with `grep -c '^  - name:'`, and the gap is recorded as a learning rather
than trusted to memory.

### #845 was an independent implementation of #832 — roasted, closed as superseded, and I posted an erratum
Draft #845 (Jules) fixed the same duplicated `### Changed` heading as my local #832 work. Rather than
merge two copies, I compared them: #845 dedupes the heading, mine adds per-section duplicate scoring
plus a 10-case fixture and a `validate.sh` block, so mine strictly dominates and #848 is the one that
lands; #845's changelog dedupe is worth keeping and #832's own note says so. **But my roast comment
asserted "main is not protected"** — that came from one API call, `GET /repos/…/branches/main/protection`
→ 404, which is what a ruleset-protected branch returns: the classic endpoint has no record because
the protection is a **ruleset** (`GET /branches/main` → `protected: true`). I published the wrong claim
in good faith, checked it after filing #850, and posted a public correction on #845 instead of editing
it quietly. The real, measured finding is #850: ruleset `12897801` requires exactly three checks
(`Build CLI (linux-x64)`, `Codacy Static Code Analysis`, `commitlint`) with
`strict_required_status_checks_policy: true`, so `lint`, `test` and `mutation-test` are *not* merge
blockers — a PR can squash-merge with a red test job. Not fixed unilaterally: repo settings are not
mine to change silently.

### Two issues from the same audit trail, both measured before filing
`#849`: `scripts/quality-gates.sh:147` passes `--quiet` to `cargo nextest run` at the top level, and
nextest rejects it (`error: unexpected argument '--quiet'`, RC=2 measured) — so on any machine with
`cargo-nextest` installed the "quality gates" script takes a branch that cannot compile or run a test,
and a green-looking run proves nothing. Found while writing #832's fixture, because writing a fixture
means running the thing. `#850` is the ruleset finding above.

### Rebase arithmetic: three PRs editing the same three files
`#846`, `#847` and `#848` each add a block to `scripts/validate.sh` and each carry `plans/` + `progress/`
documentation, so every `main` move produces the same four-file conflict (`ACTIONS.md`, `GOAP_STATE.md`,
`LEARNINGS.md`, `PROGRESS.md`). Resolved this round as a union with an ordering rule: PROGRESS is
newest-first, so the incoming round-6/round-5 blocks go **above** main's round-4/3/2 blocks, not after
them. After resolving, the numbers that the conflict *touched* were re-measured rather than inherited —
`main_head` to `2f26214`, the actions↔issues mapping to 7/7 (`grep`-counted), `tests_count` kept at
main's fresher 1044 instead of my branch's stale 1037, and two landed gates (#827, #831) that never had
a world-state key got one so nobody re-queues them.

### Remote state as measured, not as hoped
`gh pr view --json statusCheckRollup` on the current heads: **#846** `883e074` DIRTY (that is the
conflict being fixed by this rebase), 23 success / 9 skipped; **#847** `dcf84a3` BEHIND, 31 success;
**#848** `9b5cab7` BLOCKED with `lint`, `test` and `Analyze (rust)` still at an empty conclusion. All
three show `Codacy Static Code Analysis` as `ACTION_REQUIRED`, which is the only thing #850 lists as
mandatory. Merge order stays #848 → #846 → #847, one rebase per `main` move.

### Dispatch accounting
No agents dispatched this round — the remaining work is docs conflict resolution and merge sequencing,
which is exactly the class of work an agent cannot be trusted to verify for me.

## 2026-10-05 (round 6): main moved twice under me, and my own monitor called a merged PR "UNKNOWN"

### PR #838 merged while this round was running — verified, not assumed
`validate_archive_manifest_completeness` landed as **`f235874`** (12:48:58Z) with a
dependabot bump (`792d951`, #843) in between, so `main` moved twice since round 5. I checked
that what merged is what I reviewed rather than trusting the title: `git diff b075a73 f235874
-- scripts plans agents-docs` is **empty**, and the gate is live on main at
`scripts/validate.sh:234-238` (131 archive files on disk = 131 enumerated, RC=0). That is why
`plan_archive_manifest_valid` changed from a dated claim to a gated fact, why
`validate_archive_manifest_completeness` left the queue (9 → 8 actions, counted with
`grep -c '^  - name:'`), and why `action_last_completed` is now that action.

### PR #836 rebased again, and the remote had moved
The remote branch had been advanced to `d407b38` — a *merge* of main into the branch — while my
local copy was a rebase onto `87fa734`. Before overwriting anything I compared the commit sets:
remote-only was the merge commit alone, local-only was that same 10-commit history plus the
`llms-full.txt` regeneration and one more docs commit (`git diff --stat d407b38 c7462d8` = 3
files). Rebase onto `f235874` was clean, `check-llms-sync.sh` RC=0 locally — the failure that
was blocking the PR is gone — then `--force-with-lease` push (the `pre-push` hook ran
`cargo fmt --check` + clippy `--all-targets --all-features -D warnings` to completion, RC=0).
PR head is `39ec716`, `MERGEABLE`, CI re-running on that head; `mutation-test` had already gone
**success** on the previous head.

### A duplicate track for `deduplicate_unreleased_changelog_headings` (#832)
Draft PR **#845** appeared on branch `dedup-changelog-headings-…` (26 pass / 9 skipping, files:
`CHANGELOG.md`, `commitlint.config.cjs`, `scripts/pre-commit.sh`,
`scripts/validate-changelog.sh`) — i.e. an outside implementation of the action my local agent
is still compiling. Roast-before-implement applies: #845 gets reviewed on its merits and the
local duplicate does not land behind it.

### Monitoring truth
My own monitor line read `PR838 SETTLED state=UNKNOWN checks_pending=0 failures=0`. The state
was not "settled green" — the PR had **merged** and GitHub had deleted the branch, so
`gh pr checks` returned nothing at all. `checks_pending=0` derived from an empty result set is
not evidence; the same "empty output is not a measurement" class as the truncated
`--list` probe, arriving from the opposite direction.

## 2026-10-05 (round 5): one PR merged, two verified, and a wired gate that validated a broken frontmatter block

Fourth/fifth GOAP round on this queue. `#837` merged; `#838`/`#836` verified and rebased;
`#829` implemented and split; the `#828` swarm agent came back with a generator whose red
direction I reproduced myself. Roast verdicts in `plans/PR_ROAST_2026_10_05.md`.

### `derive_ci_crate_matrix_from_workspace` — completed (PR #837, squashed as `87fa734`)
Merged only after the derived wiring was seen executing in real Actions: **8**
`Test Workspace Crates (csm-*)` jobs green on head `f921e81`, zero non-green checks. The
8-vs-10 crate question was answered by the gate itself, not by me: `csm-duckdb`/`csm-wasm`
are excluded by name and the script fails if an excluded crate is no longer a member
(`:188`) or has no dedicated CI job (`:193`). Verdict posted to the PR **before** merging.

### PR #838 (`validate_archive_manifest_completeness`) — rebased, CI in flight
`#837` moving `main` forced the rebase #838 was going to need anyway. Clean (different
`validate.sh` regions: `:36-42` vs `:234-238`), then re-run rather than re-trusted:
`ok: archive manifest complete (131 files under plans/.archive/, 131 enumerated)`, RC=0,
`shellcheck -S error` RC=0. New head `b075a73`.

### #829 — the gate graph, and a fail-open that only a fixture could find
Audited every claim against `main` before implementing. Five orphan suites confirmed
(RC table in the issue comment), and the audit found more than it expected:
- **`validate-skill-format.sh` failed open.** A `SKILL.md` opening `---` with no closing
  delimiter validated as healthy, because `extract_frontmatter` returns everything after
  the first delimiter when a second never arrives — so the `-z` branch that prints
  *"invalid frontmatter (missing closing ---)"* could not fire for the case it named, and
  actually reported an *empty* block. The gate is CI-wired; no skill on disk trips it,
  which is why it survived. Fixed + `scripts/test-validate-skill-format.sh` (15 cases),
  with the four neuter directions demonstrated on the mutated checker (cap 250→2500,
  `-gt`→`-ge`, `-lt 2`→`-lt 1`, empty-inventory `exit 1`→`exit 0`) — each caught by the
  one test designed to catch it, file restored `cmp`-identical after every mutation.
- **A never-invoked script has two untested properties.** `test-version-sync.sh` was mode
  **664** and resolved its subject as `cp scripts/verify-version-sync.sh` — CWD-relative —
  so from anywhere but the repo root it died with `cp: cannot stat` (RC=1 measured). Both
  would have been invisible in a green run from the root.
- **Three gates cannot be wired as-is**, so they are split out instead of "fixed": #840
  (two competing link validators; `validate-links.sh` RC=1 on 5 links that are all false
  positives — root-relative `@AGENTS.md` resolved inside the skill dir, plus example syntax
  quoted in prose; `check-docs-links.sh` RC=1/66 s with all 33 broken links inside the
  frozen `plans/.archive/**` that #838 now pins), #841 (`validate-workflows.sh` RC=1
  "Errors: 4" where the payload is yamllint lint output under a *"YAML syntax error"*
  heading and the repo has **no** `.yamllint` profile, so the noise is the gate's own
  missing configuration), #842 (three hook installers, three hook sets, `.githooks/`
  installed by none, `cmp` differs at byte 40, and `core.hooksPath` checked only in
  `--global` while this workstation carries it locally).
- `negative-fixtures.sh` **deleted**, not wired: it asserts rustc rejects
  `fn main() { invalid_syntax!` and rustfmt flags unformatted code — the toolchain, not the
  repo — and it wrote `/tmp/test_invalid.rs` under a fixed name with no `set -e`.

### #828 — swarm agent verified before any PR
`scripts/gen-skill-catalog.sh` (292) + `check-skill-catalog.sh` (76) +
`test-skill-catalog-gate.sh` (184); `CATALOG.md` regenerated to **33 skills** and now lists
`pr-roast-triage`, the entry its own roast rule depends on. I re-derived all of it from git
and reproduced the teeth personally: dropped the `pr-roast-triage` row → gate RC=1, tampered
with only the count line → gate RC=1, `git checkout` restore → RC=0 with a clean
`git status`; generator run twice → `cmp` byte-identical; `shellcheck -S error` RC=0;
rebased onto `87fa734` clean, `ci.yml` parses (`yaml.safe_load`).

### PR #836 — the mutation gate passed; a stale generated file did not
`mutation-test` went **success** on `a61dbf3` after `src/shutdown_tests.rs` put an assertion
in the target the `--lib` profile actually runs. The `lint` job still failed, for a
different reason than the last time: `check-llms-sync.sh` regenerates `llms-full.txt`, which
emits one `## <file>` section per source file **whether or not that file declares public
API** — so adding the fixture file itself made the committed copy stale by two lines.
Reproduced locally (RC=1, `llms-full.txt | 2 ++`), regenerated, committed, rebased onto
`87fa734` → head `c7462d8`. `cargo fmt --check` and `cargo test` were both green throughout:
the local gate I skipped was `validate.sh`, not the compiler.

### Swarm/dispatch accounting
Two agents dispatched this round (#828 landed and is verified; #832 still running with one
commit and an in-progress `validate-changelog.sh`). The #829 agent's worktree was re-derived
as **zero commits, clean tree** at `origin/main` with no processes → stopped and taken over
rather than waiting on a silent dispatch.

## 2026-10-05 (round 4): the mutation gate roasted my own coverage claim; two agent trees verified

**CI failed #836 at `mutation score 0.0000%`** (run 37276860430, head `0107c9c`): all three
in-diff mutants in `src/shutdown.rs` survived — `replace operator_shutdown with ()` (`:21:5`),
`replace sigint with ()` (`:40:5`), `replace sigterm with ()` (`:56:5`). Cause, read off the
script rather than guessed: the fast profile is `cargo mutants --lib -p csm-retrieval -p
chaotic_semantic_memory` (`scripts/mutation_test.sh:142`), so `tests/cli_shutdown_signal.rs`
is never built for the run. Every judgement I had made about this branch's coverage was made
from the subprocess tests, which are the right tests for the *observable* and useless for the
*gate*. I had also written that same `--lib` limitation up as one of #834's flaws.

`55c0437` adds `src/shutdown_tests.rs` — one pending-check per arm at `NO_SIGNAL = 100ms` —
chosen so a failure names the arm. Reproduced each mutant individually by deleting that
function's body and re-running: RC 101 for all three, failing sets `1 / 2 / 2`
(`operator_shutdown` alone; `sigint` and `sigterm` each take the composite with them).
Restored from `/tmp/shutdown.pristine` with `cmp` clean, then `3 passed; 0 failed` /
`GREEN2_RC=0`. The positive direction was deliberately left out of the lib target: `raise()`
in the shared libtest binary kills the whole process on a mutant that skips registration.

**Both swarm trees re-derived from git, not from their reports.**
- **#831** (`c7085c5f`, `21b7b25`, base `8e69a83`, `+319/-29`): `plans/ARCHIVE_MANIFEST.md`
  now enumerates **131** files, and `scripts/check-archive-manifest.sh` is wired into
  `scripts/validate.sh` (which `ci.yml:587` runs) with a fail-closed `else exit 1`. I proved
  red myself in three directions: unlisted file on disk → 1, fabricated row → 1, deleted real
  row → 1, then restore → 0 with `cmp` clean. The 131/131 green count and the disk count
  (`find plans/.archive -type f | wc -l` → 131) match. Spot-checked that archived "Why
  archived" reasons survived the rewrite (6/6 present) — my first attempt at that check was
  itself a broken extraction (`comm` against a bad column split reported *all 21 lost*), which
  I only noticed by reading the actual rows.
- **#827** (`f1dafe50`, `f921e81`, `+626/-12`): derived matrix equals the eight hand-written
  crates exactly — the failure mode that mattered was a silent drop, and `--check` prints
  `8 derived / 2 excluded (csm-duckdb, csm-wasm) / no hand-written entries`. Its 16-fixture
  test script passes, including empty-matrix-rejected and re-typed-list-rejected, and
  `--github-output` outside Actions exits 1 instead of emitting nothing. `shellcheck -S error`
  clean on all three scripts.

**Unlanded, and why**: #827 and #831 are verified but not yet pushed — the orchestrator owns
that step, and merging is one-at-a-time from a current head. #836 must clear CI on `55c0437`
first.

## 2026-10-05 (round 3): #824 implemented, and a draft duplicate forced the coverage to widen

`cover_sigterm_in_server_shutdown` (#824) shipped as **PR #836** on branch
`fix/mcp-sigterm-cooperative-shutdown`. The fix is one `pub(crate)`
`src/shutdown.rs::operator_shutdown()` that resolves on SIGINT **and** SIGTERM; it
replaces `ctrl_c_shutdown` in `mcp::serve` and the raw `tokio::signal::ctrl_c()` in
`run_watch`, so a long-running command cannot wire only half of its signals. No new
public API (`llms.txt` untouched), gated `all(not(target_arch = "wasm32"), feature =
"cli")` because `mcp` implies `cli` (`Cargo.toml:338`).

Then the queue grew a competitor: draft PR **#834** (Jules, also `Fixes #824`, CI green on
`09973d8`) implemented the same primitive in `src/cli/shutdown.rs` as a `pub fn`.
Re-derived from its own head rather than from its report: two binary-level tests with
**unbounded `child.wait()`** (a regression hangs CI instead of failing), fixed
**200/300 ms sleeps** in place of readiness synchronisation, both commands spawned with
**no `--database`** — so per `src/cli/args.rs:20` they write `.git/memory-index/csm.db`
into the checkout — a `#[tokio::test]` whose body only *constructs* the future (an
`async fn` is lazy, so it asserts compilation), `sig.recv().await` discarding the
`Option` (a closed stream reads as a shutdown request), `+6` lines of public API in
`llms.txt`, and two new `scripts/mutation_test.sh` exclusions. On the exclusions I wrote
the first verdict backwards and then corrected it by measurement: `--exclude-re
"replace shutdown_signal"` **does** match (`--exclude-re` is matched against the mutant
*description*, proven by counts on `cargo-mutants 27.1.0`: `src/shutdown.rs` baseline 3
mutants → 2 with `"replace operator_shutdown"`, 2 with `"sigterm"`, unchanged 3 with
`"shutdown::"`, control `"zzz_no_match"` 3), while my original "no path contains the word
replace, so it is inert" reading came from a `tail`-truncated `--list` I mistook for a
result. The real problem is one layer down: that line is redundant with the
`--exclude "src/cli/shutdown.rs"` under it, and the module needs excluding at all only
because its behavioural tests live in `tests/cli_integration.rs`, which the fast profile
never runs (`mutation_test.sh:142`) — so `replace shutdown_signal with ()` is unkillable
in-gate, and the exclusion files a placement mistake as policy.

But it covered `csm mcp serve --transport sse`, which #836 did not. That dimension was
folded in at **`c62f2ce`** — `serve_sse_stops_cooperatively_on_sigterm` / `_sigint`,
reusing the bounded-wait harness, `--bind 127.0.0.1:0` with the kernel-chosen port read
off the server's own readiness line, and an explicit temp `--database` — so #836 now
covers both servers and #834 has nothing left to contribute. Four tests pass in 0.79 s.

**Red demonstrated for both commands, not argued:** the SIGTERM arm was deleted from
`operator_shutdown()`; exactly the two SIGTERM tests failed with
`ExitStatus(unix_wait_status(15))`, `signal: Some(15)`, while both SIGINT tests stayed
green (2/2). The file was restored from a copy taken before the neuter, `diff -q` clean,
`git diff HEAD -- src/shutdown.rs` empty, and the suite re-run green.

Gates on the branch head: `cargo fmt --all -- --check` clean, `cargo clippy --features
cli,mcp --test cli_shutdown_signal -- -D warnings` clean, `check-test-attributes.sh` ok
(76 files), `check-llms-sync.sh` up to date (tests/ is not in the generated surface, so
`llms-full.txt` needed no second regen). Commitlint: 0 problems, 1 warning
(`footer-leading-blank`, the heredoc quote line).

Also roasted **#835** (`perf(retrieval): defer score normalization scaling in
merge_single_list`) — a resubmission of the closed **#763** in the same function, now with
*no* benchmark at all, which dodges #763's specific critique (a bench that never reached
the changed path) by discarding the claim's support. Its rollup says `commitlint:
COMPLETED/FAILURE`, `mergeStateStatus: BLOCKED`, and 4 commits with three identical
headlines plus a re-typed `refactor(...)` — gate-dodging, not history. Verdicts and the
manual merge order are in `plans/PR_ROAST_2026_10_05.md`. Nothing was closed: #834 and
#835 are another author's drafts, and that call is human.

Distilled into `skill://pr-roast-triage` (now 238 lines under the 250 cap): the Step-3
rule that a live duplicate may hold coverage the keeper lacks — diff its *test* files,
fold, dominate — plus four CI-invisible checks, with the long-form detail moved to
`references/subprocess-and-claim-rubric.md` and the #767 rationale chain to
`references/rationale-comments.md`. The addition itself first broke the cap (283 lines,
`validate-skill-format.sh` RC=1); compaction was the fix, not a raised limit.

Incidental measured evidence for `wire_and_consolidate_validation_gates` (#829):
`scripts/validate-links.sh` is **red on the current tree** — 5 broken refs across
`jules-orchestration`, `rust-development`, `skill-creator`, `testing-validation`
(`@AGENTS.md`, `@file.md`, `./path.md`) — and no workflow or `validate.sh` invokes it, so
an uncalled gate is also an already-failing one. Attached to that action rather than
fixed here, to keep #824 atomic.

World state: `servers_exit_on_sigterm` deliberately stays `false` — `origin/main` is
still `8e69a83` (re-fetched), and the state file records the repository, not the queue.
`tests_count` 1039 -> 1041 as a stated delta.

## 2026-10-04 (round 2): the backlog was invisible — 10 GOAP actions, 0 GitHub issues

Reconciliation request, run under `goap-orchestrator`. Ground truth read off the API
before anything was planned: `gh issue list --state open` -> `[]`, `gh pr list --state
open` -> one PR (#819). `plans/ACTIONS.md` held 10 queued actions. So the entire backlog
lived on one filesystem: no issue meant no `Fixes #…`, no external signal that anything
was owed, and a session that did not open `plans/` would have found an empty tracker.

### Verification swarm, and the corrections it produced

Four read-only `Explore` agents re-derived all 10 actions against `8e69a83`. **All 10 are
still missing** — none had silently landed. Three of the notes were nonetheless wrong in
ways that change cost, which is why re-verifying a stale note is not busywork:

- `cover_sigterm_in_server_shutdown` warned that a SIGTERM listener must respect the
  `signal`/wasm32 target gate at `Cargo.toml:197-202`. **`Cargo.toml:202` already enables
  `signal`.** The stated obstacle did not exist, so the action was cheaper than queued.
- `derive_ci_crate_matrix_from_workspace` said two crates "silently" miss CI. They do not:
  `ci.yml:216-217` comments both exclusions and each has a dedicated job. The real drift is
  a *future* crate, because the root job at `ci.yml:204` runs
  `cargo test --all-features --locked` with **no `--workspace` and no `-p`**.
- `complete_evidence_tiers_and_mutation_hardening` claimed `pre-release-gate.yml` runs no
  mutation. It does (`:106`, `mutation_test.sh fast --ci` at `:129`); it runs no benchmark.

Every claim was re-derived locally (`grep`, `wc -l`, `sed -n`) before use, per the skill's
trust boundary. No agent invented state this round, and the four corrections all held up.

### Two world-state bugs found by reading `GOAP_STATE.md` against the repo

- `sse_transport_has_exit_path: false` contradicted the *same file's*
  `queued_actions_count` comment, which records #821 completing exactly that. Duplicate-key
  hygiene does not catch a file disagreeing with itself; the flag is now `true` with the
  evidence attached, and `servers_exit_on_sigterm: false` was added as the honest open half.
- `main_head: "55a9fa3"` was two rounds stale (actual `8e69a83`).

### `cover_sigterm_in_server_shutdown` implemented (#824)

One shared `src/shutdown.rs::operator_shutdown()` resolving on SIGINT **and** SIGTERM,
used by `mcp::serve` in place of `ctrl_c_shutdown` and by `run_watch` in place of its raw
`tokio::signal::ctrl_c()`, so a long-running command can no longer wire half of the
signals. `pub(crate)` and gated `all(not(wasm32), feature = "cli")` — `mcp` implies `cli`
(`Cargo.toml:338`), so one gate covers both consumers and nothing new becomes public API.

The test is `tests/cli_shutdown_signal.rs`, and it asserts the **exit status of the real
binary**, because that is the only observable separating "handler installed" from "handler
written and never registered": `src/mcp/server_tests.rs` resolves a *synthetic* future to
prove the plumbing, and it stayed green for weeks while SIGTERM was absent from the process
entirely. Red was demonstrated, not argued — deleting the SIGTERM arm turns the new test
red with `signal: Some(15)` while the SIGINT test keeps passing, then the file was restored
byte-identically (md5 + residue grep).

### #819 roasted: no demonstrated impact

Jules `perf(core): fast-path identity rotations in HVec10240::permute`, open since 06:07
UTC and never roasted (today's first round covered #821/#822/#823). `commitlint` is red on
the annotation *"perf PR evidence gate: missing `## Performance Evidence` section in the
PR body"*. Three independent reasons it cannot merge as-is, all read off `main`: the body
claims an "O(1) register/stack move" while `HVec10240` is `[u128; 80]` — 1280 bytes copied
by value either way; the only production caller is `encoder.rs:226` with
`position_stride` defaulting to **1** (`:91`), so identity means `pos == 0`, once per
sequence; and `plans/evidence/bench/canonical.json` contains **no permute benchmark at
all**, so the gate's baseline requirement is unsatisfiable without writing one first.
Same class as #808 and the #737/#739/#740/#754 set. Verdict recorded; **not closed** —
closing another author's open PR is a human call, and the measurement stays owed.

## 2026-10-04: SSE transport gets an exit (#821), a gate for the un-runnable `#[test]` (#822), and what three subagents reported that did not exist

Second GOAP round under `goap-orchestrator` on this queue. Both actions came out of the
previous round's own disclosures: `give_the_sse_transport_an_exit_path` was found while
writing #817's mutation-exclusion comment (a production line that could never execute),
`revive_dead_cli_parity_help_test` was found while proving that a dropped `#[test]` has no
Rust sensor. Full roast verdicts in `plans/PR_ROAST_2026_10_04.md`.

### `give_the_sse_transport_an_exit_path` — completed (PR #821, merged `dc7cd9a`)

`mcp::serve_with_shutdown(config, shutdown_signal)` is now public and `serve` passes
`ctrl_c_shutdown()`; the SSE arm binds the listener and runs
`axum::serve(listener, app).with_graceful_shutdown(shutdown_signal)`. The reason it was a
defect rather than a nicety is axum 0.7.9's shape: `Serve::into_future` accepts forever
(`serve.rs:205-240`) and `tcp_accept` turns errors into `Option` (`:474-496`), so without
`with_graceful_shutdown` the SSE branch never returns and #813's ADR-0099
`handler.shutdown()` hop — the line that stops the TTL reaper — was unreachable on the
transport most users run.

The test asserts the *effect*: `src/mcp/server_tests.rs` puts a live reaper in the
handler's framework slot, checks the cleanup `JoinHandle` slot is `Some`, resolves the
signal, **awaits** the transport, then checks the slot is `None`. Neutering the hop (wrap
it in `if false`) turns the suite red — that red/green is recorded, not assumed. Plus
`sse_exit_is_ok_without_a_request` (shutdown with no framework is a `OnceCell`-guarded
no-op) and `serve_with_shutdown_returns_when_the_signal_resolves`;
`tests/mcp_sse_integration.rs` swapped `abort()` for a `oneshot` so the integration test
exercises the same exit path as production.

Measured on `./target/debug/csm mcp serve --port <ephemeral>`: **SIGINT → exit 0**,
**SIGTERM → exit 143**. `tokio::signal::ctrl_c` is SIGINT only, so a systemd-supervised
server still misses the cooperative stop — queued as `cover_sigterm_in_server_shutdown`
rather than left implied.

### `revive_dead_cli_parity_help_test` — completed (PR #822, merged `55a9fa3`)

`cli_each_subcommand_has_help` had a body and no `#[test]` at `tests/cli_parity.rs:97`, so
`--list` reported 2 tests in a file with 3 test bodies. Recompiling that target emits no
`dead_code` warning — rustc's reachability analysis in a `--test` build does not flag a
private `fn` whose only caller would be the harness — so clippy and the `cargo test
--no-run` warning scan in `validate.sh` are structurally blind to it. The only sensor is
textual: `scripts/check-test-attributes.sh` (187 LOC) reports a column-0 `fn` whose
attribute block neither registers nor parks it and whose name no other non-comment line
references, exempting `pub` fns in `tests/<dir>/` sub-modules (`tests/common` convention),
and its four blind spots are in the header so the gate is not trusted past what it
measures. The revived test derives its oracle from the clap tree
(`required_help_tokens()`) — every leaf must answer `--help`, declared flags must appear in
the printed output; prose correctness and `hide(true)` args are out of reach and the PR
body says so.

`validate.sh` runs the gate **and** its 9-case `mktemp -d` fixture
(`scripts/test-check-test-attributes.sh`, 226 LOC) in the `lint` job; both scripts are
committed `100755` so the `-x` guard is not a latent pass. Ordering fact worth keeping: the
gate fails against `main` at the exact dead body this PR deletes, so gate and fix had to
land together.

### Process: the subagents, stated precisely

Three dispatched agents all reported state that did not exist — invented PR numbers and
per-job CI counts before any PR was opened, a SIGTERM-parity claim and a CHANGELOG/ADR edit
not in the tree, and one worktree left holding a **neutered production line** (`if false`
around `handler.shutdown()`) plus a `#[test]`-removed `cli_parity.rs` after a red/green
demonstration. The neuter survived one of my own "restores" because I restored from a backup
taken *after* the sabotage, and I repeated an invented PR number back to the user. What
contained the damage was the test design, not vigilance: an assertion that reads the handle
slot after awaiting the transport cannot pass on a disabled hop. Five mechanical rules now
stand in `.agents/skills/goap-orchestrator/SKILL.md` under **Agent Trust Boundary**, and the
dispatch step that told implementers to "create branches, commit" is corrected to
worktree-only — that instruction is what they were following when they reported PRs that did
not exist: I take over every git/CI mutation; state is re-derived from `git`/`gh` (never
from an agent summary); a restored file is `git diff HEAD`-ed in full before it is trusted;
the dispatch is stopped before the orchestrator touches a worktree; every fix keeps at least
one effect assertion.

### Gate misses of my own, recorded

- #821 went red on `lint` for `stale generated file: llms.txt` — `llms.txt` /
  `llms-full.txt` are public-API snapshots gated by `scripts/check-llms-sync.sh`
  (`validate.sh:82`), and a new `pub use` has to be regenerated in the same branch. Cause:
  I pushed without running `validate.sh` locally and said so in the PR.
- Verification ran in the wrong worktree once (main instead of the SSE branch), which passed
  trivially and proved nothing; re-ran where the failure lived.
- `set -uo pipefail` + `grep -c` returned 1 for a proof script that had proved its point
  (`ready=yes`, `RESULT=EXITED exit_code=0`) — the harness reported failure for a green run.

### Post-merge verification on `main` (55a9fa3)

Read per `workflowName`, not from badges: `CI` push → **success**, `CodeQL` → success,
`Release` (`workflow_run`) → success ×2. **`GitHub Pages` did not run and was never going to**
— `pages.yml:3-8` is path-filtered to `book/**` and the workflow file itself, and neither
#821 nor #822 touched `book/`. That is not a gap: the rustdoc surface that silently failed
four main pushes before #817 moved the check into the PR — `validate.sh:41` runs
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` and CI runs `validate.sh`
in the `lint` job — so a broken intra-doc link on a new `pub use` now reddens the PR instead
of the next deploy. Last Pages run is `1389c9e` (#817, success). Record this so nobody hunts
for a "missing deploy" on a docs-adjacent PR.

### Queue state after this round

10 queued (`11 → 9` for the two completions, `+1` for the measured SIGTERM gap). Next up:
`mutation_baseline_the_feature_gated_mcp_module`, `cover_sigterm_in_server_shutdown`,
`deduplicate_unreleased_changelog_headings`, `add_persistence_failure_path_test`. 3 low
Dependabot alerts remain. The HNSW `examples/scale_evidence` knobs are still parked in the
tagged stash `orchestrator-hnsw-knobs-818-followup` — not dropped, not mixed into these PRs.

---

## 2026-10-03: ADR-0099 shutdown wired into the server paths (#813), triage docs landed (#812), #814 closed no-op, ConceptBuilder dedup (#816), TTL reaper config surface + rustdoc gate (#817)

GOAP round under `goap-orchestrator` with swarm dispatch: a reconciliation pass on the diverged PR head, a diagnosis agent that root-caused a red gate in a scratch worktree with a minimal-diff proof, and an Explore agent that line-audited the two actions a docs PR queues.

### `wire_graceful_shutdown_into_servers` — completed (PR #813, merged `f999998`)

`ChaoticSemanticFramework::shutdown()` (ADR-0099) now runs on both long-running exit paths: `csm watch` installs a Ctrl+C handler and stops the shared TTL cleanup task before flushing, and `mcp::serve` shuts the handler's framework down after the transport ends. MCP SSE sessions share one framework behind an `Arc` — the previous `Clone` for `McpHandler` reset the `OnceCell`, so every session built its own framework *and* its own cleanup task, none of which was ever awaited. `McpHandler::clone` removal is recorded as a **Breaking (`mcp`)** changelog entry, not buried in the lifecycle fix.

Two reds on the branch, both real, both root-caused rather than worked around:

- `mutation-test` — the `run_watch -> Ok(())` entry-point mutant survived because the exclusion commit existed only locally. The exclusion is diff-scoped cargo-mutants config, and the mechanism is stated with its residual kept honest: the framework owns the event sender, so the channel-close exit is unreachable while `run_watch` holds it, and SIGINT is unreachable under `--lib`. What the exclusion does **not** buy: the call-site wiring is covered by review plus a manual SIGINT smoke, not by a gate.
- `lint` — reported as a "LOC gate" failure, and it was not LOC. `scripts/validate.sh` runs `cargo check --target wasm32-unknown-unknown --features wasm`, which died with 48 `mio` errors after `tokio/signal` was moved onto the target-independent `cli` feature: the default feature set enables `cli` for the wasm32 check too, and `mio/net` cannot build there. The three-line comment the refactor deleted had predicted exactly this. Reverted (`signal` stays in the non-wasm target table), verified by running that single wasm32 check; the `wasm` CI job had stayed green throughout because it builds `-p csm-wasm` with `default-features = false`.
- Second `lint` red on the fixed head: `validate.sh`'s **llms drift gate** — committed `llms.txt`/`llms-full.txt` still advertised `impl Clone for McpHandler` and predated the Cargo.toml revert. Regenerated with `scripts/gen-llms-txt.sh` on the same branch.

### #812 triage docs merged `9ce0193`; #814 closed as no-impact

#812 (2026-10-02 evening triage record) was roasted, its two queued actions' file:line references audited and corrected pre-merge (`25a5948`), then merged. It sat `BLOCKED` after every check went green for two reasons worth keeping: the "green CI" I first read was the **Tooling Guard** workflow on that SHA, not `CI`, and once `CI` went green the remaining blocker was that the PR was a **draft** on an unprotected trunk.

#814 (Jules) re-filed the exact 3-line `HVec10240::permute` zero-shift fast path that #800 carried — a PR this repo roasted on 2026-10-01, **measured**, and closed as no-impact on 2026-10-02 (direct call 81.7 → 57.3 ns; caller path `text_encoder/encode_short` 2 684 → 2 734 ns, 3/6 rounds each way = noise). #814 dropped the bench and the modulo-identity assertions its predecessor shipped, added no `## Performance Evidence` (so `commitlint` was red on the repo's own perf gate), and repeated the "O(W) → O(1)" mechanism that a by-value `-> Self` return refutes. Closed with the recorded numbers cited; `grep -rn '<symbol>' plans/PR_ROAST_*.md` is now part of Step 3 in `pr-roast-triage`.

### `collapse_duplicate_concept_builder` — completed (PR #816, merged `4064af9`)

The queued debt from the #806 triage, implemented with a swarm split: two Explore agents inventoried the surface first, and their audits corrected three references the queued action note had gotten stale — the deletion range is `singularity_types.rs:176-240` plus its test module `257-277` (not `177-232`), `crates/csm-memory/src/singularity.rs:4` is the glob that had to be reasoned about (not line 2), and the root's re-export chain means a naive deletion breaks four public paths.

Every public **path** preserved by re-export: `crates/csm-memory/src/singularity.rs` now carries `pub use crate::concept_builder::ConceptBuilder;`, so `csm_memory::ConceptBuilder`, `csm_memory::singularity::ConceptBuilder`, `chaotic_semantic_memory::ConceptBuilder` and `…::prelude::ConceptBuilder` all name the owner. Copy deleted (−86 LOC in `singularity_types.rs`), its clamp test deleted rather than migrated (the owner's `concept_builder_with_ttl_clamps_excessive_values` is equivalent), 13 importers rerouted, `src/framework_ttl.rs` kept at exactly the 500-LOC cap by making the swap one-line-for-one-line. (Corrected 2026-10-03 by #816's own self-roast, which first framed this as "not breaking": the deleted copy derived `Clone` and the owner does not, so one delta *is* semver-visible to external callers — relabelled `**Breaking (csm-memory)**` in the changelog with a `BREAKING CHANGE:` footer, since in-repo compilation proves nothing about a removed public impl.)

Two `tests/arch_fitness.rs` guards replace the grep a human would otherwise re-run forever: exactly one `struct ConceptBuilder` workspace-wide, and five public spellings passed to a function that takes the owner type — re-duplication is now a compile error. Behavioural deltas disclosed in the changelog instead of hidden under a dedup title: the collapsed builder no longer derives `Clone` (zero builder `.clone()` call sites, verified) and `with_metadata` now goes through `impl Serialize` + ADR-0012 error capture on every production path, replacing the copy's infallible `Into<Value>`; no perf claim is made because no benchmark was run, and the extra `to_value` deep copy is the honest cost.

Gates run by the implementer and re-verified independently by the orchestrator: `cargo check -p csm-memory --all-targets`, `cargo check --all-targets`, `cargo clippy --all-targets --all-features -D warnings`, `cargo test -p csm-memory --lib` (67, −1 = the deleted duplicate test), the four affected integration targets (8 + 3 + 9 + 21 = 41), `cargo check --target wasm32-unknown-unknown --features wasm`, `./scripts/check-llms-sync.sh` (artifacts byte-identical — the re-export kept the path lists the generator records), ADR parity, LOC gate at 500, `cargo fmt --check`.

Self-roast of the PR before opening it (Rule 17 applies to first-party work too) produced three corrections, all landed in `4cf8d89`: the changelog and PR body claimed "not a breaking change" on in-repo compilation evidence, while the deleted copy derived `Clone` and the owner does not — now labelled `**Breaking (csm-memory)**` with a `BREAKING CHANGE:` footer, which is the convention this repo actually uses (zero `!` headers in `git log`); the copy's `with_ttl` doc was the only place stating that expiry is resolved at `build()` rather than setter time, so that rationale was restored on the owner; and the implementer's report said 21 `arch_fitness` tests where there are 8, which is why every number above was re-measured rather than copied. It also surfaced a pre-existing defect kept out of scope: two `### Changed` headings in `[Unreleased]` (`CHANGELOG.md:10` and `:15`), queued as `deduplicate_unreleased_changelog_headings`.

Records: `plans/PR_ROAST_2026_10_03.md`; queue 10 → 9 → 8 → 9 (each step derived by counting `- name:` blocks, not asserted); `ttl_cleanup_has_bounded_shutdown` now states what is gated and what is not.

### The records PR (#818) merged while its own roast was still a local commit

`docs/goap-close-out-817` opened `17:56:48Z`, and `CI` 37142371089 on head `cceae30` concluded
success at `18:09:32Z`. At `18:07:50Z` — before that conclusion — an **`autoMergeRequest` was armed
by the `d-o-hub` account** (`mergeMethod: SQUASH`), and the PR landed at `18:09:57Z` as `f091ad2`,
25 s after green. I did not arm it and no workflow in this repo merges PRs (`grep -rn "gh pr merge"
.github/workflows/` → nothing); `allow_auto_merge: true` is a repo setting, and changing it is the
owner's decision, so it is flagged in the roast rather than touched here. Practical consequences,
both now encoded: the "branch had no PR and no other writer" assumption I used to justify a
`--force-with-lease` was true at the time but became false mid-flight (`delete_branch_on_merge`
removed the head ref, so my next push reported `[new branch]`), and `mergeStateStatus: BLOCKED`
with `mergeable: MERGEABLE` no longer reads as "checks pending" alone — it can mean "checks pending
**and** about to merge", so the merge-readiness probe is now
`gh pr view --json autoMergeRequest,mergeable,mergeStateStatus` in one call. The roast of #818 and
lessons 16–17 are shipping as a follow-up PR (`docs/roast-818-automerge-record`), which is the only
honest shape available: the verdict exists, it just arrived after the merge it reviews.

### `enable_ttl_cleanup_in_long_running_commands` — completed (PR #817, merged `1389c9e`)

One Explore agent re-audited the queued action's file:line references before dispatch, then a single implementer agent owned the change (splitting it across agents would have meant four agents editing `src/cli/args.rs`, `src/cli/commands/mod.rs` and `src/mcp/*` in sequence, so serial was the honest choice). Result: `--ttl-cleanup-interval <SECONDS>` on `csm watch` and `csm mcp serve`, one shared `TtlCleanupArgs` flattened into both arg structs, default `0` so a long-running server never starts deleting on its own. Threading is `create_framework_advanced(…, ttl_cleanup_interval_seconds)` + a new `create_framework_with_ttl`, and `McpConfig::ttl_cleanup_interval` → `McpHandler::with_ttl_cleanup_interval` → the same funnel; `McpHandler::new`'s 12 call sites are untouched because the interval is a field defaulted to `0`. `src/cli/args.rs` hit the 500-LOC gate, so the tail subcommand structs moved to `src/cli/args_commands.rs` and are re-exported from `args` — the AGENTS.md-sanctioned child-module route, not comment stripping.

The reaper tests are the part worth keeping an eye on: `create_framework_with_ttl_starts_a_reaper_that_deletes_rows` asserts on `Singularity::len("_default")` — **raw stored rows**, which a probe filters — going 1 → 0 with no `purge_expired` call, and the disabled tests assert `cleanup.is_none()` while the row is provably `is_expired()`. Neither result is explicable by probe-time filtering, so the test measures the task rather than the config field. Two handler-level tests cover the MCP hop, and `ordinary_framework_wrappers_never_start_the_reaper` pins the one-shot behaviour.

Review corrections and calls: my own brief told the implementer `src/cli/mcp.rs` was 479 LOC — it is 21, and the agent re-measured and reported the error instead of designing around it. Its `REAPER_WAIT` of 3100 ms (rather than copying `tests/test_advanced_ttl.rs`'s 2.6 s) is documented in a comment as second-granular expiry arithmetic, and I kept it. I checked whether the new public `u64` needs a bound, in the family of the `with_ttl`/`windows(n+1)` overflow findings: `tokio::time::interval` resolves an out-of-range period through `checked_add(…).unwrap_or_else(Instant::far_future)` (`tokio-1.53.1/src/time/interval.rs:478`), and `csm watch --ttl-cleanup-interval 18446744073709551615` runs clean against `./target/debug/csm` — no clamp added, no panic path. It did not make the PR's own docs deploy worse, and fixing that was in scope because the PR touches `book/`: **GitHub Pages had been red on four consecutive main pushes** (34751139063, 35274354770, 35614812503, 37126532546 — the site last deployed 2026-09-09) because six rustdoc `unresolved link to ':model'` warnings from one `args.rs` doc line hit `CARGO_BUILD_WARNINGS=deny` in a workflow no required check mirrors. The doc line is reworded to drop the brackets and `scripts/validate.sh` (CI: `ci.yml:575`) now runs the same `cargo doc --no-deps --all-features` under `-D warnings`, so the next such warning fails the PR that introduces it. The root cause is not in this diff and not in that line: the text landed in `b4bed50` (ADR-0069) and Pages stayed green until `aa4f9ab`, the Dependabot bump of `actions-rust-lang/setup-rust-toolchain` **v1.17.0 → v2.0.0** — the action that exports `CARGO_BUILD_WARNINGS=deny` and turned the pre-existing warning into a hard error. `git log 8bb4521..aa4f9ab -- .github/workflows/pages.yml` is what showed it, because asking "which line of the failing push broke it?" has no answer when a lint's severity is what changed.

Gates re-run by the orchestrator, not trusted from the report: `cargo fmt --all --check`; `cargo clippy --all-targets --all-features -D warnings`; `cargo test --lib --features cli,mcp` → **193 passed** (181 + 12 new); the 12 `ttl`-named tests individually (3.10 s); `cargo check --target wasm32-unknown-unknown --features wasm`; `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` → clean; `./scripts/check-llms-sync.sh` (and re-verified after the doc reword — `llms.txt` never carried that text, so no regeneration was needed); `./scripts/check-adr-parity.sh`; LOC gate max 500 (`args.rs` 499 → 427, `args_commands.rs` 172, `commands/mod.rs` 408); `bash -n` + shellcheck on `validate.sh` (only the pre-existing SC2044); `npx commitlint --from origin/main --to HEAD`. Help evidence read from the built binary: both `csm watch --help` and `csm mcp serve --help` print `--ttl-cleanup-interval <SECONDS> … [default: 0]`.

### #817 went red on its own gate, and the root cause was the profile (2026-10-03, `fix(ci)` `a29670f`)

The PR whose subject is a config surface broke a *coverage* gate: head `c47d676` failed `mutation-test` (run 37131481411) at **score 25 %** — `total=9 caught=1 missed=3 unviable=5` — while `Version Integrity` 37131481386, `Tooling Guard` 37131481477 and `benchmark-ci` 37131481407 all concluded success. Two of the three `MISSED` were `src/cli/mcp.rs:42`, i.e. mutants of code the job **never compiled**: the fast profile runs `cargo mutants … -- --lib` with default features, `mcp` is not one of them, and both `chaotic_semantic_memory::mcp` (`src/lib.rs:57`) and `crate::cli::mcp` (`src/cli/mod.rs:6`) are `#[cfg(feature = "mcp")]`. `MISSED` is therefore ambiguous between "the tests failed to kill it" and "the file was not built", and the tests that kill it already existed — the same-module `#[cfg(all(test, feature = "mcp"))]` block. Diagnosed in two commands (`cargo test --lib -- --list | grep -c mcp`, 0 without the feature and N with), fixed in the profile rather than by writing a redundant test: `FAST_ARGS+=(--features chaotic_semantic_memory/mcp)` flips both `MISSED → CAUGHT`.

The second-order failure was measured before pushing, and it is the reason the commit is two-part. Enabling the feature makes the base build link `rmcp`/`axum`/`tower`: 146–183 s for the base, 145–208 s per mutant, against a profile `--build-timeout 150` that was generous for the old profile's ~54 s incremental rebuilds (CI measured 52–54 s on this very diff) and impossible for that link — locally the score printed **0 % `TIMEOUT`** on a change that is otherwise killable. So: bound 150 → 420 (still bounded by the job's `timeout-minutes: 45`, so a hung mutant cannot eat the job), *plus* a `Warm the mcp feature build the mutation profile compiles` step in `ci.yml` that builds the feature-enabled `--lib` test binaries once. After warming, the measurement that matters re-ran clean: one source file touched, both feature-enabled test binaries relinked in **39 s** — the raised bound is cold-cache headroom, not the normal per-mutant cost.

Reviewing my own new comments caught an overstatement worth keeping in the record: I had written that the ADR-0099 shutdown-on-transport-end "is asserted" in `tests/mcp_sse_integration.rs`. It is not. That test calls `mcp::serve` at `:17` and `abort()`s the task at `:112`, so `serve` never returns and `src/mcp/server.rs:99` (`handler.shutdown()`) never executes; only the *callee* is covered (`src/mcp/tools_tests.rs:231`). Two `--lib`-unreachable entry points are now excluded as a class (`run_inject`, whose mutant came from `src/cli/commands/inject.rs:18` purely because #817 touched that line, and `serve` via `--exclude-re "replace serve ->"` so it cannot swallow a future `serve_*` helper), each with the residual stated instead of implied. `src/mcp/**` remains outside the job via the pre-existing `--exclude "src/mcp/*"` — widening the *feature* set is not widening the *path* set — so `mutation_baseline_the_feature_gated_mcp_module` is now queue 10 and its note names the caller hop as the first thing to build (close the transport, let `serve` return, observe the task is gone), because a mutant of that line currently has nothing that could kill it.

Records: queue 9 → 8 (`enable_ttl_cleanup_in_long_running_commands`) → 9 (`revive_dead_cli_parity_help_test`, the dead `cli_each_subcommand_has_help` at `tests/cli_parity.rs:97` that has no `#[test]` and therefore never runs — verified by `-- --list`, which registers 2 of the file's 3 test bodies, and it produces **no** `dead_code` warning, so neither clippy nor `validate.sh`'s warning scan can catch a missing attribute); `deduplicate_unreleased_changelog_headings` stays queued — this PR added `### Added`/`### Fixed` under `[Unreleased]` but left the duplicate `### Changed` pair alone to keep the change reviewable; `rustdoc_warnings_gated` and `servers_reap_expired_concepts` are new world-state flags.

### The exclusion review turned up a production defect, not a coverage wish (2026-10-03, queued as `give_the_sse_transport_an_exit_path`)

Writing the honest residual for `serve`'s mutation exclusion forced a check of the claim I had just made ("the ADR-0099 shutdown-on-transport-end is asserted in `tests/mcp_sse_integration.rs`"). It was wrong, and the reason was worse than untested. `run_sse_server` (`src/mcp/server.rs:106-132`) awaits `axum::serve(listener, app)` with **no** `with_graceful_shutdown`, and in axum 0.7.9 (the version `Cargo.lock` resolves for this crate) `Serve::into_future` is `loop { match tcp_accept(&tcp_listener).await { Some(conn) => …, None => continue } }` (`serve.rs:205-240`) where `tcp_accept` returns `Option` and turns every accept error into `continue` — sleeping 1 s on `EMFILE`-class errors (`serve.rs:474-496`). That future resolves only via a graceful-shutdown signal, which this code never passes. So `serve` never returns on the SSE path and the ADR-0099 `handler.shutdown()` at `src/mcp/server.rs:98` is unreachable: `csm mcp serve --transport sse` still relies on `Drop`/process death, exactly what ADR-0099 was written to remove. `Stdio` is fine — `rmcp`'s `Waiting::waiting` resolves on stdin EOF (`src/mcp/server.rs:83-86`) — which is why #813 merged green: its test drives the transport that *does* return, and it `abort()`s the spawned task (`tests/mcp_sse_integration.rs:112`) rather than awaiting it, so no combination of that PR's code, tests or review could have surfaced the other transport. Verified against the vendored axum source, not inferred: `grep` shows no `with_graceful_shutdown` anywhere in `src/mcp/**` or `tests/`, and `src/bin/csm.rs:209` awaits `serve(config)` with no signal wrapper. Consequences landed as records, not quiet rewrites: #813's changelog bullet is corrected in place, the world state gains `sse_transport_has_exit_path: false`, and the queued action names the axum API that fixes it (`with_graceful_shutdown`, `serve.rs:139-141`), the `src/cli/commands/watch.rs:84-98` Ctrl+C precedent to mirror, the effect assertion to write (send the signal, await `serve()`, reuse the handle-slot check at `src/mcp/tools_tests.rs:249-256`, no sleeps), and one constraint a cheaper implementation would violate — the signal must be driven by production code, because a test-only seam makes the line reachable for the test and still dead for the user. Queue 10 → 11.

### #817 merge gate and the deploy it existed to fix (2026-10-03T17:37:50Z → `1389c9e`)

Gate satisfied on the landing head and read per `workflowName`: `CI` 37139534832 `success` across 27 jobs (25 pass / 2 skipped / 0 fail) **including `mutation-test`**, the job that had failed the first head — so the profile fix was confirmed by the gate it was meant to satisfy, not by a local run. Same head: `CodeQL` 37139531493, `Version Integrity` 37139534850, `Tooling Guard` 37139534886, `benchmark-ci` 37139534849, `mergeable: MERGEABLE` / `mergeStateStatus: CLEAN` / not draft, head current with `main` = `4064af9`. The superseded head `4463a3e`'s run 37138840490 was read to its real conclusion (`cancelled`) rather than assumed. 27 files, +1097/−213.

**The point of the PR's second half is a deploy, and the deploy is verified two ways.** `GitHub Pages` run 37141216190 on the new main push concluded `success` — the first green docs build since `8bb4521` (2026-09-09), after `aa4f9ab`/`2fc421b`/`a44c29`/`4064af9` (34751139063, 35274354770, 35614812503, 37126532546) failed four times in a row with every required check green. A green workflow is not the whole claim, so the artifact was checked too: `https://d-o-hub.github.io/chaotic_semantic_memory/ttl.html` returns HTTP 200 and carries `ttl-cleanup-interval` three times, i.e. the flag this action added is what is now published. `Version Integrity` 37141216187 and `benchmark-ci` 37141216197 also succeeded on the push; `CI` 37141216196 and `CodeQL` 37141215898 were still queued/in-progress at write time and are confirmed before this record is opened as a PR.

Queue after this action: **11** (`grep -c '^  - name:' plans/ACTIONS.md`), holding `give_the_sse_transport_an_exit_path` (the #813 transport defect, above) and the repriced `add_persistence_failure_path_test` / `add_query_count_regression_test`, which a read-only pre-audit reclassified from `test(...)` to `fix(...)`: no seam can make `Persistence` fail (`src/framework_builder.rs:399-402` rejects a configured DB when the feature is off, and there is no `trait Persistence` or mock), and `query_count|num_queries` returns 0 hits repo-wide, so both need production code before they can be asserted.

## 2026-10-02 (feedback round): open-PR blockers addressed

With the trunk green after the WASM freshness repair, the two feedback-bearing PRs in the open queue were roasted and their blockers fixed (the other two open PRs — #806, #807 — carry no feedback and stay for the next triage pass).

### #808 (`perf(core)`: BHVec10240 xor SIMD)
Two red gates, both self-inflicted: `commitlint` failed on a body whose evidence section lived only in a commit message, and Codacy `⛔ 3 high` flagged `unsafe` in the *newly extracted* test module — those calls were already covered while they lived inside the excluded `hyperdim_simd.rs`. The roast also found 17 rationale/SAFETY comment blocks blanked, an unmeasured `MaybeUninit` + `get_unchecked` fallback (the #783 class), and a cited baseline id (`bhvec_bind`) that does not exist in `canonical.json`.

Fixed on `aee4253`: comments restored verbatim; the `[u64; 160]` dispatch extracted into `hyperdim_simd::xor_u64` beside the identical hamming dispatcher (484/500 LOC — extraction, not deletion); the unchecked fallback replaced by the safe `xor_u64_scalar` oracle; SAFETY comments restored in the test module with four new xor parity tests (dispatched vs oracle on head/tail words, AVX2 kernel vs oracle, `xor` vs `HVec10240::bind` across layouts); `.codacy.yml` covers the extracted module; the body carries measured evidence (287.6 → 141.5 ns per `xor`, ≈ 1.7–2.0×, sign-consistent over alternating rounds). `commitlint`, Codacy and `Test Workspace Crates (csm-core-lib)` are green on the new head; impact is public-API-only, so the merge call stays with the owner.

### #800 (`perf(core)`: permute zero-shift fast path)
The 2026-10-01 evidence request stood: no bench reached the new `word_shift == 0` branch (the only permute bench used shift 321). Added the `hvec_permute` group (shifts 0, 10240, 10240 × 3 — all identity rotations — plus 128 and 321 as controls, `black_box` on vector and result), extended `test_permute` to lock `permute(10240)` and `permute(10240 × 3)`, and measured with an order-alternating harness: 81.7 → 57.3 ns on the direct call (6/6 rounds, control offset reported), 2 684 → 2 734 ns on `text_encoder/encode_short` (3/6 each way — noise: one identity permutation per token stream amortizes away). Gate, Codacy, workspace crates and the arm64 job are green on `79f51f7`; merge/close stays with the owner.

Records: `plans/PR_ROAST_2026_10_02.md` (addendum); lessons in `progress/LEARNINGS.md` (body-capture timing for PR gates; control workloads and alternating order for micro-benchmarks).

## 2026-10-02: WASM freshness gate repaired + root-`src` PR coverage

`main` was red on exactly one job: `wasm`. Runs 36885556956 (`c8f91876`) and 36888300162 (`debd7354`) both failed at `Verify WASM TS Freshness` with a diff made solely of three compiler-generated closure helpers — `wasm_bindgen_847cf8bb5373c819___convert__closures…` — because the checker filtered only the pre-hash naming scheme (`wasm_bindgen__convert__closures_____invoke__…`). Nothing else in those runs failed (the arm64 CLI cancel follows the failed run). The PR-side path filter is why root-source breakage reached the trunk: the `wasm` job set `wasm=true` only for `^(crates/csm-wasm/|src/wasm|wasm/|Cargo\.(toml|lock)$)`, and `src/wasm` is a prefix that never matched root `src/**` — even though `crates/csm-wasm` depends on the root crate with feature `wasm`.

### Change (PR #809, merged `77a47d4`)
- `scripts/check-wasm-freshness.sh`: the filter is anchored at the declaration start and matches generated closure-invocation declarations under both naming schemes — `^[[:space:]]*readonly wasm_bindgen(_[[:xdigit:]]+)?__+convert__+closures__+invoke__`. Public exports, `InitOutput` members and `__wbindgen_*` ABI signatures are still compared; the checked-in snapshot was deliberately not refreshed, because the observed diff is compiler-generated churn rather than API change.
- `.github/workflows/ci.yml`: the `wasm` job's PR path filter now covers root `src/**`, `scripts/{build-wasm,check-wasm-freshness,wasm_size_gate}.sh` and `ci.yml` itself.

### Verification
- Normalization regression executed against the checker's own `filter_dts` (definition read from the script, declaration text on stdin): the job-log fixture (three old helpers replaced by the three hash-prefixed ones) normalizes identically to the checked-in file and is rejected by the previous filter; four API/ABI mutations from that fixture — `encode_text(text: number)`, removed `initialize_wasm`, added `newly_added_api`, `__wbindgen_malloc` arity — all still change the normalized output.
- Path filter, one file per PR diff: `wasm=true` for `src/framework_cleanup.rs`, `src/framework.rs`, `src/wasm.rs`, `crates/csm-wasm/src/lib.rs`, `wasm/test.js`, `Cargo.toml`, `Cargo.lock`, `scripts/check-wasm-freshness.sh`, `.github/workflows/ci.yml`; `wasm=false` for `README.md`, `plans/ACTIONS.md`, `scripts/check-llms-sync.sh`.
- Local end-to-end (wasm-pack 0.15.0, wasm32 target, Node 22.23.2; nix unavailable, canonical commands run directly): freshness `OK`; `build-wasm.sh release-web` 655 497 B; `cargo check --target wasm32-unknown-unknown -p csm-wasm` ok; size gate pass (640.13 KiB, ceiling 800 000 B); `wasm/test.js` → `WASM smoke test passed.`; `shellcheck` clean; `actionlint` findings identical to `main`'s three pre-existing ones.
- PR #809 (head `d0536879`): whole run 36980374291 green, including the previously red `wasm` job — `Verify WASM TS Freshness` → `OK`, `WASM JS Smoke Test` → `WASM smoke test passed.`; merged as `77a47d4`; main push run 36983336327 green. Local wasm-pack emits the old helper names, so CI is the end-to-end proof of the hash-prefixed form.
- Queued action `trigger_wasm_job_on_root_src_changes` completed (queue 9 → 8); `validated` stays false with the remaining wave-32/33 residuals.

## 2026-10-01 (wave-32 residual): Absence record invalidation

Executed `add_absence_invalidation_semantics` (PR #804, merged `c8f91876`) — the audit's F1 remainder: absence records short-circuited queries forever, including after matching content existed.

### Design
- `AbsenceEntry` gains `namespace` + `namespace_revision`; a record short-circuits only when it belongs to this namespace and its revision equals the current one, so every durable mutation (which bumps the revision, ADR-0093) invalidates absence knowledge lazily — no write on the mutation hot path and no list of mutation paths to keep in sync.
- `AbsenceStore::delete_absence` clears the record a probe found once that probe retrieves successfully; the delete only happens when a same-namespace record existed, so successful probes without a record pay nothing extra.
- `merge_absence_with` restarts the counter when namespace/revision changed; records are stamped with the revision observed *before* retrieval; an unreadable revision fails closed; a failed write-path read stamps 0 (inert, matches no live namespace).
- Migration v12 adds both columns behind the guarded versioned `ALTER` pattern (ADR-0021); pre-existing rows default to `('', 0)` and are inert.
- `lookup_absence` replaces `is_known_absent`, returning the decision plus the invalidation target from the single read the check already performs; the mutation exclusion list follows the rename.

### Verification
- `tests/bm25_absence_short_circuit.rs` 3 passed, including the new regression (abstain 3×, short-circuit, inject matching text, probe again → Success); with the revision comparison removed the test fails with `got ["AbsenceShortCircuit"]` — the original bug.
- `cargo test --lib` 165 passed (3 `lookup_absence` boundary/staleness/namespace cases, the `namespace_revision` read test, and the record/clear loop test); `cargo test -p csm-persistence -p csm-traits --all-features` passed.
- v11 → v12 upgrade: a full database parked at v11 (columns dropped, version row removed, legacy row inserted) opens cleanly — `schema_version=12`, the legacy row preserved and inert, a fresh probe records a `_default` attempt instead of short-circuiting, and re-opening is idempotent.
- Four CI iterations, each root-caused: stale `llms*.txt` (regenerated, helpers moved so no phantom `impl` block is emitted); three `ChaoticSemanticFramework::namespace_revision` mutants (killed by a `--lib` test, 62.5% → 80%); two write-path mutants `persist_absence_record`/`clear_absence_record -> ()` (killed by the loop test, both simulated locally first → **100%**); and a `csm-persistence` unit-test initializer missed because `--all-targets` is *package* scope, not workspace scope (fixed; lesson recorded).
- CI on `abff706`: 0 failures — lint, test, mutation-test (100%, 16 mutants: 10 caught / 0 missed / 6 unviable), miri, all nine workspace crates, Cargo Deny, commitlint, benchmark-small.

### State
- `plans/GOAP_STATE.md`: `absence_records_invalidated_on_insert: true`; `queued_actions_count` 10 → 9; `action_last_completed: add_absence_invalidation_semantics`; `main_head` refreshed.
- `plans/ACTIONS.md`: action removed, completion note added.
- `progress/LEARNINGS.md`: the revision-contract lesson for derived negative knowledge, and the `--all-targets`-is-package-scope scope trap.

## 2026-10-01: PR roast triage (#800)

One open PR at triage time: #800 (`perf(core)`: zero-shift fast path for `HVec10240::permute`), a draft Jules PR. Verdict: **keep open, request evidence** — correct and reachable, but unmeasured; record in `plans/PR_ROAST_2026_10_01.md`.

### Findings
- CI truth on `e5333ca5`: every job green except `commitlint`, whose annotation is the perf gate itself — `perf PR evidence gate: missing '## Performance Evidence' section in the PR body`.
- The fast path is byte-identical for `shift ≡ 0 (mod 10240)` and already locked by `crates/csm-core-lib/src/hyperdim_tests.rs:48`; so this is not a correctness objection.
- It is **reachable**, which is why it was not closed as a no-op: `crates/csm-core-lib/src/encoder.rs:226` permutes `pos * position_stride` with `pos` from `.enumerate()`, so position 0 hits the new branch once per `encode`.
- The only `permute` benchmark (`benches/binary_benchmark.rs:30`) uses `permute(321)`; `321 % 128 = 65`, so it never enters the `bit_shift == 0` path — it cannot measure this diff. The body's "~0 ns vs ~144 ns" has no artifact and matches the shape of a harness where LLVM elides the dropped copy.
- Evidence bar posted: `## Performance Evidence` in the body naming `text_encoder/encode_short` (nearest canonical comparator) or the new bench output, an affected-path benchmark with shift `0`/`10240` or the caller `encode`, `black_box` on the returned vector, and the whole-`encode` delta; plus rebase + ready-marking before any merge.
- Lesson distilled into `.agents/skills/pr-roast-triage/SKILL.md`: a bench can be blind because of its arguments, not only because it calls a different function.

### State
- `plans/PR_ROAST_2026_10_01.md`: verdict record. `plans/ACTIONS.md`: triage note. `plans/GOAP_STATE.md`: `action_last_completed` set to the triage action (no queued action was in scope; counters unchanged).

## 2026-10-01 (wave-32 residual): TTL cleanup task ownership

Executed `own_ttl_cleanup_shutdown` (PR #801, merged `96144703`), the audit-F2 remedy: cooperative ownership replaces `abort()`.

### Change
- The task handle is a shared `CleanupTask { cancel: watch::Sender<bool>, handle: Mutex<Option<JoinHandle<()>>> }` in `src/framework_cleanup.rs`; the loop exits on cancel **or** when the last handle drops, because it runs on a framework clone with `cleanup: None` and therefore cannot keep its own sender alive.
- `impl Drop for ChaoticSemanticFramework` is deleted: there is no strong handle in the loop to abort, and dropping one clone no longer touches the shared task.
- `ChaoticSemanticFramework::shutdown()` cancels, awaits under a 5 s bound, is idempotent, works from any clone, and reports a panicked/stuck task as `MemoryError::External`. It is a no-op on `wasm32` (identical public API); the task type and field are `cfg(not(wasm32))`.
- ADR-0099 records the decision; ADR-0024's stale "proposed in ADR-0093" pointer is corrected (ADR-0093 never mentions the lifecycle) and the registry counts move to 95/94. `llms.txt`/`llms-full.txt` regenerated.

### Two CI iterations (both root-caused, not patched)
- `lint` failed the 500-line gate (`src/framework.rs` reached 533) → child-module extraction per AGENTS.md step 9 (now `framework.rs` 476, `framework_builder.rs` 441, `framework_cleanup.rs` 174).
- `mutation-test` failed with 3 missed mutants (whole `spawn_cleanup_task` replaced with `()`, `interval == 0` flipped to `!=`, `shutdown` replaced with `Ok(())`) → killed with two in-crate unit tests asserting that interval 0 spawns nothing, interval > 0 spawns a task, and `shutdown()` awaits the task and takes its handle exactly once (no `--exclude-re` exemption was added). Next CI run: `mutation-test` pass (7m9s).

### Verification
- `tests/test_advanced_ttl.rs` 20 passed, including `background_cleanup_runs_until_shutdown` (purges, then stops after shutdown; idempotent) and `dropping_a_clone_keeps_the_cleanup_task_running` (regression for the clone-drop abort).
- 2 new lib unit tests passed; `tests/arch_fitness.rs` 6 passed (both LOC gates); `cargo check --all-features --all-targets`, `clippy --all-targets --all-features -D warnings`, and `cargo check --target wasm32-unknown-unknown -p csm-wasm` all exit 0; ADR parity ok (registry=95, disk=94).
- CI on `49b6a74`: `lint`, `test`, `mutation-test`, `miri`, the workspace crates, `benchmark-small`, and `commitlint` all pass.
- The host's `target/` directory (83 GiB) was cleaned afterwards at the operator's request; CI is the gate of record from here.

### Findings queued (not fixed in this action)
- `ci.yml`'s `detect-changes` sets `wasm=true` only for `^(crates/csm-wasm/|src/wasm|wasm/|Cargo\.(toml|lock)$)`, so a root-`src` change that alters `cfg(not(wasm32))` gating skips the `wasm` job entirely (observed on PR #801; covered locally instead).
- No caller uses `shutdown()` yet: the MCP server and CLI watch rely on dropping the framework.

### State
- `plans/GOAP_STATE.md`: `ttl_cleanup_has_bounded_shutdown: true`; ADR counts 95/94; `queued_actions_count` 9 → 10 (one completed removed, two findings queued); `action_last_completed: own_ttl_cleanup_shutdown`; `main_head` refreshed.
- `plans/ACTIONS.md`: action removed, two follow-ups queued, completion note added.

## 2026-09-30 (wave-32 residual): crates.io publish pre-check

Executed `fix_crates_publish_precheck_and_add_duckdb` (PR #798, merged `71a48618`): three defects in `publish-crates`, each reproduced against the live registry before the fix.

### Change
- **Ownership pre-check** replaces the version comparison: `/api/v1/crates/<name>/owners` (public; `200` carries `users[].login`, `404` = free) with a descriptive UA. A name passes when it is free or owned by `d-o-hub`; any other HTTP outcome fails closed. `csm-duckdb` joined the checked set (8 names).
- **Version existence** now comes from the sparse index `verify-release` already trusts; the root "already published" check fails closed if the index is unreadable, and the per-crate skip uses the same probe instead of `cargo search`.
- **Companion loop** captures `cargo publish` output, treats "already uploaded" as an idempotent skip, and collects any other failure to exit non-zero — partial publishes can no longer pass silently.
- **`csm-duckdb`** gets a dedicated step after `Publish to crates.io`: skip if published, otherwise wait (bounded 10 min, then fail) for the root version in the index and publish. Its dev-dependency on the root crate is why ordering matters (`crates/csm-duckdb/Cargo.toml:25`).

### Verification (the workflow's shell was extracted and executed, not read)
- Ownership pre-check, real eight-name list: all eight `✅ owned by d-o-hub`, exit 0.
- Negative control: `serde` → `❌ CONFLICT: serde is owned by [dtolnay github:serde-rs:publish]`; unknown name → `✅ name is free`; exit 1.
- `crates-check`: `0.3.8` → `already-published=true`, `0.3.9` → `already-published=false` (both exit 0). The retired probe reads the published 0.3.8 as "not published" (default UA → 403, no match) — the defect.
- `csm-duckdb` step on the published 0.3.8: early exit, no publish attempted.
- `shellcheck -s bash -S info` on the new scripts: 0 findings; `actionlint` 52 findings before → 50 after (the two removed are inside the rewritten step); `cargo metadata` resolves `-p csm-duckdb`.
- Gates: `CARGO_BUILD_JOBS=2 ./scripts/validate.sh` exit 0, `cargo deny check` ok, commitlint 0 problems, CI green on `3d39510`.

### State
- `plans/GOAP_STATE.md`: `crates_publish_precheck_ownership_aware: true`, `csm_duckdb_in_release_publish_order: true`; `action_last_completed: fix_crates_publish_precheck_and_add_duckdb`; `queued_actions_count` 10 → 9; `main_head` refreshed.
- `plans/ACTIONS.md`: action removed; completion note added.
- `progress/LEARNINGS.md`: the three 2026-09-27 supply-chain bullets now carry their resolutions.

## 2026-09-30 (wave-32 residual): Release gate moves to workflow_run

Executed `migrate_release_wait_for_ci_to_workflow_run`. `release.yml` no longer triggers on a main push and polls `gh run list` under a `MAX_WAIT` ceiling (raised 1800 → 2700s, abandoned a third time on run 36031855839 while CI sat `queued` ~40 min on a saturated runner pool); it now triggers on the **completion of the CI workflow** (`workflow_run` on `ci.yml`, `types: [completed]`, `branches: [main]`) and releases exactly `github.event.workflow_run.head_sha`.

### Change
- `wait-for-ci` (83 lines of polling + queue-starvation re-trigger) deleted; `validate` is the entry job, gated on `conclusion == 'success'` **and** `head_repository == github.repository` **and** `event == 'push'` — a fork PR branch named `main` cannot reach a job holding `contents: write`.
- Checkout, the tag created by `validate`, and the GitHub release `target_commitish` all pin `head_sha` (new `release-sha` output) instead of `github.sha`, which under `workflow_run` is the default-branch tip rather than the commit whose CI passed.
- `workflow_dispatch` keeps the fail-closed rule with one query: the new guard refuses to proceed unless CI for HEAD is already `completed`/`success`. `actions: write` is dropped; `validate` adds only `actions: read`.
- Docs trued: `release-management` skill (hard rule 3 + flow) and its `release-workflow.md` reference, `agents-docs/release-safety.md`, `.agents/context/shared-conventions.md`; `progress/LEARNINGS.md` records the durable rule.

### Verification (merged `8c1f864d`, PR #796)
- `actionlint .github/workflows/release.yml` → no expression/context/schema errors (remaining output is the pre-existing `ubuntu-24.04` label-data lag and SC2086 notes in untouched steps); `yaml.safe_load` → `workflow_run` + `workflow_dispatch`, 9 jobs, no dangling `wait-for-ci` reference.
- **Trigger proof:** CI on `8c1f864d` completed `success` (run 36743298010, ~28 min queued) and release run **36746741481** fired with `event: workflow_run` (16:47:13Z); `validate` = success, every publish job `skipped` (tag `v0.3.8` exists → `release-needed=false`), `notify` = success. No `push`-triggered release run exists for that SHA (the old path is gone).
- **Dispatch proof:** `gh workflow run release.yml` → run **36746882951** `validate` = success, log shows `CI on 8c1f864d495c27d321a7339526530baeb853ed8f: status=completed conclusion=success` and `Tag v0.3.8 already exists; skipping release.`
- `CARGO_BUILD_JOBS=2 ./scripts/validate.sh` exit 0 (incl. skill-format validation of the edited SKILL.md), `cargo deny check` ok, commitlint 0 problems, CI on `9bca25e2` all green.

### State
- `plans/GOAP_STATE.md`: `release_wait_event_driven: true`; `action_last_completed: migrate_release_wait_for_ci_to_workflow_run`; `queued_actions_count` 11 → 10; `main_head` refreshed.
- `plans/ACTIONS.md`: action removed; completion note added.

## 2026-09-30 (wave-32 exit pass): Flag truth vs the July audit

Re-verified every exit criterion of `plans/GOAP_AUDIT_2026_07_14.md` against HEAD `7ce6fb73` instead of trusting recorded flags. Three read-only evidence sweeps covered Phases 1-4; every claim below was re-checked by command or by reading the cited source.

### Met (with evidence)
- ANN config fallible: `validate_index_backend` (`crates/csm-memory/src/index/mod.rs:113`) is called from `FrameworkBuilder::build` (`src/framework_builder.rs:354`); invalid HNSW/LSH configs return `InvalidInput` (unit tests in `index/mod.rs`).
- Stale snapshots rejected: `IndexSnapshotEnvelope` carries `namespace_revision` + `backend_fingerprint` and is applied only on exact match (`src/framework_persistence.rs:131-134`); `tests/ann_revision_envelope.rs` (`stale_snapshot_after_inject_is_rejected_on_reload`, `backend_mismatch_rejects_snapshot`).
- Fuzz workspace compiles: `cargo check --manifest-path fuzz/Cargo.toml --all-targets --locked` exit 0 locally; CI job `fuzz-build` (`ci.yml:603-627`) runs the identical command.
- Lean feature matrix: 0 libsql / 0 rayon on normal+build edges (`cargo tree -p chaotic_semantic_memory --no-default-features -e features,normal`); the remaining `rayon` hits are dev-only via `criterion`.
- One WASM artifact: CI (`ci.yml:521`) and release (`release.yml:618`) both call `scripts/build-wasm.sh release-web`; `scripts/wasm_size_gate.sh` measures that artifact (656 657 B threshold, sha256 printed).
- Workspace/supply-chain CI: whole crate matrix (incl. `csm-chaos`), `cargo-deny`, `test-benchmarks`, `mutation-test`, `miri`, `test-duckdb`, arm64, `fuzz-build`.
- Constant-query load and lock discipline: `load_all_associations` (`crates/csm-persistence/src/persistence_index.rs:148`) used by `load`/`load_replace`/`load_merge` (`src/framework_persistence.rs:100,184,304`); durable I/O happens before singularity locks are taken (`:96`, `:177-215`).
- Metric math: true multi-label `recall_at_k`, log2 NDCG, abstention gold from `should_abstain`, hand-calculated tests (`benchmarks/src/scorer.rs`); `cargo test --manifest-path benchmarks/Cargo.toml --locked` 31 passed.
- Measured (not formula) memory claim: `tests/performance_targets.rs:69` measures on-disk bytes/concept (2 875 measured, band 2 500-3 500) plus RSS; `plans/evidence/scale_release_2026_09_21/memory_model.json` records the 12 MB target as not supported.
- Absence short-circuit wired: `src/framework_ttl.rs:170-195,252`, `ABSENCE_MIN_ATTEMPTS = 3`, `tests/bm25_absence_short_circuit.rs`.
- No missing implementations: 0 hits for `TODO|todo!|unimplemented!|FIXME` in `src/` + `crates/`.
- Unique ownership: facades verified (`src/retrieval/bm25.rs` shim, `src/embedding/mod.rs`, `src/persistence_wasm.rs`), canonical test owners per `plans/TEST_SURFACE_AUDIT_2026_09_18.md`; inventory 1037 unique tests.
- Plans compact: `GOAP_STATE.md` 9.5 KB / `ACTIONS.md` 18.1 KB (≈100 KB / ≈180 KB at audit time), with `plans/ARCHIVE_MANIFEST.md` + `plans/README.md` redirects.

### Remainder (queued as nine actions)
- TTL cleanup shutdown is `Drop`-time `abort()` on an `Arc`-shared handle (`src/framework.rs:44-50`): no cancellation token, no await, any clone drop kills the task for all clones, and no test sets a non-zero cleanup interval.
- Absence records are never invalidated on insert or on later success (stale short-circuit).
- No persistence failure-injection test and no query-count test for the bulk association load.
- Gate graph fragmented: orphaned validators (`validate-workflows.sh`, `validate-git-hooks.sh`, `validate-links.sh`), fixture tests (`test-llms-sync.sh`, `test-version-sync.sh`) invoked by nothing, three hook installers with different sets.
- Skill catalog is hand-written and stale (32 vs 33); `scripts/gen-agents-context.sh` and `docs/architecture/context.yaml` carry stale counts (`active_wave: 11`, `total_tests: 134`, `skills: 19`, `adrs: 16`).
- Evidence tiers: no scheduled scale tier, `pre-release-gate.yml` uncalled and bench-free, `benchmark-ci.yml` path filters miss `crates/**`; mutation has no module-level inventory and still excludes changed files.
- `ci.yml` crate matrix is hand-maintained; archive manifest has no validator and omits the 55 top-level archived ADRs.

### State
- `plans/GOAP_STATE.md`: `validated` stays `false` with the evidence-based residual list; `wave_32_status` stays `in_progress`; `main_head` refreshed; +`no_state_lock_across_io_await`, +`benchmark_metrics_mathematically_correct`; annotations on `persistence_failure_leaves_memory_unchanged`, `workspace_ci_matrix_complete`, `plan_archive_manifest_valid`; `integration_test_files` 71 → 72; `queued_actions_count` 3 → 11.
- `plans/ACTIONS.md`: completed action removed, nine residuals queued, completion note added.

## 2026-09-30: Remaining PR impact review

Closed #790 as an incorrect mixed-scope submission and #793 as its incorrect single-list duplicate after posting roast recommendations. Neither supplied the required PR-body performance evidence; both select the wrong candidate for negative public weights. The independent HashMap proposal remains eligible for a measured atomic resubmission, not branded a proven no-op. Keeper review found and corrected #791's missing `clear_associations` invalidation, #789's overbroad codegen claims, and #792's codec/MSRV wording. See `plans/PR_ROAST_2026_09_30.md` for verdicts and sequential merge discipline.

### Merge execution
- #791 (cache invalidation + capacity-safe upsert) squash-merged as `cd70fa5` after 27/27 checks green; `cargo test -p csm-memory --lib` 67 passed. #789 (roast record + skill lesson) rebased at the `PROGRESS.md` top anchor and merged as `977f36f`. #792 (recommendations + `PR_ROAST_2026_09_30.md`) rebased onto that and merged as `195d6e61`. Every keeper was rebased onto the current `main` and re-verified on the exact new head before an explicit squash merge; no `gh pr merge --auto`.
- Queue after the wave: 0 open PRs, 0 open issues. Inventory 1037 unique compiled tests; LOC max 500; ADR parity and llms-sync gates green.

## 2026-09-29: Retrieval cache and capacity invariants

### Summary
Implemented the first atomic recommendation from the 2026-09-29 audit in the owning `csm-memory` crate. New concept inserts now invalidate cached similarity results; association, disassociation, and clear-associations mutations invalidate graph-derived retrieval caches; and upserting an existing ID no longer evicts an unrelated concept when the store is at capacity.

### Verification
- Added regression tests for insert-after-cache, capacity-safe upsert, and graph candidate cache invalidation after association, disassociation, and reconnect/cache/clear/query.
- `cargo test -p csm-memory --lib`: 67 passed.
- `cargo clippy -p csm-memory --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.

### State
- This is intentionally a small correctness slice; atomic imports, scoped absence records, TTL shutdown, snapshot codec migration, and backup/restore completeness remain separate work.

## 2026-09-28: PR Roast Triage (#783–#786)

### Summary
Session-start roast of all four open PRs. One PR closed with its roast comment
for no demonstrated zero-fill-elimination benefit; three dependabot keepers
verified and a manual merge order emitted (no
merges executed by the triage). No queued GOAP action was in scope;
`action_last_completed` unchanged.

### Actions
- #783 (`perf(core-lib)`: BHVec `MaybeUninit` zero-fill elimination, Jules
  draft): **closed as no-op**. Old-vs-new codegen probe (x86_64, rustc 1.98.1,
  `-O -C codegen-units=1`, `#[no_mangle] #[inline(never)]` linked binary):
  `from_hvec`/`from_bytes` variants fold to the same address (byte-identical
  code); an old-only build disassembles to a bare `memcpy` — LLVM already
  dead-store-eliminates the zero-init. `to_bytes` old/new share the same
  alloc+memcpy sequence but differ in instruction scheduling. No demonstrated
  zero-fill-elimination benefit; the scheduling change means disassembly alone
  cannot dismiss all claimed timings as noise. A scheduling-speedup claim needs
  reproducible affected-path benchmarks. The perf-evidence gate was red
  (no `## Performance Evidence`), commit 1 scope
  `core-lib` invalid (not in `commitlint.config.cjs`), draft behind `main`.
- #784 (install-action 2.87.15 → 2.87.20): keeper. SHA
  `9983c65e42da123ff25d1f78505eb6de315aa172` equals tag `v2.87.20`'s commit;
  the only install-action pin in `.github/workflows/`; 18/18 checks green.
- #785 (patch-updates: rmcp 3.4.1, hyper-util 0.1.21, thiserror 2.0.21,
  rand 0.10.3): keeper. Lock-only diff; 32/32 checks green; `tempfile`'s
  `getrandom` edge re-resolves 0.4.3 → 0.3.4 within its `>=0.3.0, <0.5`
  requirement (both majors remain in the graph); no deny.toml/export.json noise.
- #786 (rand 0.10.3 in `/fuzz`): keeper. Fuzz lock only; head current
  (`MERGEABLE`/`CLEAN`); all checks green.
- Record: `plans/PR_ROAST_2026_09_28.md`; merge order #786 → #784 → #785, each
  rebased onto current `main` and CI re-verified on the new head before an
  explicit squash merge.

### State
- `plans/ACTIONS.md` / `plans/GOAP_STATE.md`: unchanged (triage is not a queued
  action).
- Lesson distilled into `.agents/skills/pr-roast-triage/SKILL.md`;
  `progress/LEARNINGS.md` updated.

## 2026-09-27 (wave 3): Borrowed-Id Retrieval Expansion (#781)

### Summary
Executed `eliminate_retrieval_string_clones`. Measurement first: the queued hypothesis (#754 roast) said `score_specific_candidates` clone-per-candidate dominated the retrieval path — a counting allocator showed the function at n=256 costs 263 allocations, while a default bridge query allocated 2 209, dominated by `ConceptGraph::match_tokens`/`expand` materialising every matched id and expanded label before the caller truncates to top_k. The fix therefore borrowed the whole path: `match_tokens_ref`/`expand_ref` (csm-traits), `score_candidate_positions` (csm-memory), generic `normalize_scores_in_place` (csm-retrieval), `&str`-keyed merge (root bridge). All owned public signatures kept as delegating wrappers — no breaking change.

### Measured
- New `bridge_retrieval/pipeline_1k_expansion` bench (one query token matches all 1000 concepts): criterion medians 881.26 µs → 577.07 µs, change −41.6% [−47.9, −34.4], p < 0.05; reversed A/B/A leg old-vs-new +28.8% [+19.7, +38.1].
- Allocations per query (counting allocator, deterministic): 2 209 → 151 at top_k=10 (−93%), 3 820 → 1 228 at top_k=101, 3 115 → 724 at top_k=64. Public `score_specific_candidates` wrapper unchanged (263 → 265, control).
- Comparator caveat: the cached `pipeline_1k_concepts` bench drifted 9.1–13.4 µs across windows (±30%) — treated as noise; one large-sample A/B/A leg discarded (first leg 1.30 ms, outside every other window).

### Actions
- PR #781 merged (`6194fca`) after CI green (lint, test, mutation-test, miri, benchmark jobs) and the roast verdict comment; record appended in `plans/PR_ROAST_2026_09_27.md`.
- Tests: `test_concept_graph_ref_variants_match_owned`, `score_candidate_positions_matches_owned_wrapper`; `validate.sh` and `harness-check.sh all` green (documented link-OOM mitigation); LOC max 500.

### State
- `plans/ACTIONS.md`: action removed; `plans/GOAP_STATE.md`: `retrieval_string_clones_removed: true`, `action_last_completed: eliminate_retrieval_string_clones`, `queued_actions_count` 4 → 3, `main_head` refreshed.

## 2026-09-27 (wave 2): csm-duckdb First Publish

### Summary
Executed `publish_csm_duckdb_companion`. `csm-duckdb` was the last unpublished companion crate (name free; `chaotic_semantic_memory` and `csm-chaos` owned by `d-o-hub`). Readiness before the irreversible step: `cargo metadata --locked` clean, `cargo package --list` reviewed (tracked test fixtures only), and a full `cargo publish --dry-run -p csm-duckdb` (package extracted and compiled from the tarball, 13m25s). Published `csm-duckdb 0.3.8` at 2026-09-27T14:07:08Z (38 356 B); verified via `cargo search` (0.3.8), `cargo owner --list` (`d-o-hub`) and the crates.io API.

### Findings (queued as `fix_crates_publish_precheck_and_add_duckdb`)
- `cargo publish` verification resolves dev-dependencies against the registry (probe crate: dev-dep `serde = "99"` fails package prep). `csm-duckdb` dev-depends on `chaotic_semantic_memory`, so it must publish *after* the root crate; the release loop publishes companions first and swallows failures — appending it to that loop would silently skip it every release.
- `release.yml`'s name-availability pre-check reads our own older version as an unrelated-project conflict: simulated against the live registry at 0.3.9 it sets `NAME_CONFLICT=true` and exits 1 for all seven published companions — the next version release fails before publishing anything.
- The `crates-check` `curl` probe receives a 403 (Fastly) with curl's default UA — no `"version"` key in the body — so the "already published → skip" short-circuit cannot be trusted as written (a descriptive UA returns JSON).

### State
- `plans/ACTIONS.md`: `publish_csm_duckdb_companion` removed; `fix_crates_publish_precheck_and_add_duckdb` queued; `queued_actions_count` stays 4.
- `plans/GOAP_STATE.md`: `duckdb_companion_published: true`; `action_last_completed: publish_csm_duckdb_companion`; `main_head` refreshed.

## 2026-09-27: llms Dependency Versions Synced + Drift Gate

### Summary
Executed `regenerate_stale_llms_dependency_versions`. The committed `llms.txt`/`llms-full.txt` were last regenerated at #714 and still reported `opentelemetry 0.27`, `tracing-opentelemetry 0.28` and `rmcp 1.7` while the manifests carry `0.32`/`0.33`/`3.4` (bumps #728/#729), plus the `scale_evidence` example added in #733. `scripts/validate.sh` regenerated the files but never committed or compared them, so CI could not see the drift. Both files were regenerated through the pinned v0.1.1 generator (no hand-edits) and the validate step is now a failing drift gate.

### Actions
- `scripts/check-llms-sync.sh`: snapshots both files into a `mktemp -d` dir (EXIT trap), runs `scripts/gen-llms-txt.sh`, `cmp -s` each and prints `stale generated file: <name>` per drifted file (exit 1); missing inputs fail with `missing generated file: <name>` instead of an empty baseline; generator failure propagates its exit code. Regenerated files stay on disk for review.
- `scripts/validate.sh` calls the checker in place of the bare generator (public-API LOC check unchanged); pre-commit/release/`sync-docs` regeneration paths intentionally untouched.
- `scripts/test-llms-sync.sh`: five fixture cases in a mock repo with a stub generator — synced passes; stale `llms.txt` and stale `llms-full.txt` each fail and are named; missing input fails; generator failure exits 3. No Cargo/network; ShellCheck clean.
- Verification: `scripts/test-llms-sync.sh` 5/5; checker run twice on synchronized files exits 0 both times and leaves both files byte-identical (`sha256sum`); induced drift exits 1 naming `llms.txt` and aborts a `set -euo pipefail` caller; ShellCheck clean. `./scripts/validate.sh` stopped at the known local link-OOM (`cargo test --no-run --all-features`, `ld` SIGKILL — environment, LEARNINGS 2026-09-23); the same sensors rerun with `CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=4`: `cargo test --all-targets --all-features` exit 0, `cargo deny check` ok, `./scripts/harness-check.sh all` green (fmt, workspace clippy, deny, test, arch). ADR parity ok; LOC gate max 500. No Rust sources changed (`./target/debug/csm` behavior untouched).

### State
- `plans/ACTIONS.md`: `regenerate_stale_llms_dependency_versions` removed; `queued_actions_count` 5 → 4; `llms_dependency_versions_current: true`; `action_last_completed: regenerate_stale_llms_dependency_versions`.
- Rule encoded in `agents-docs/hard-constraints.md` (generated listings must not drift; checker + fixture tests named).

### Merge
- Squash-merged as #778 (`4cf4f2c`) from head `b38b87a`, up to date with `main` and CI green (the `lint` job ran the new checker through `validate.sh`); verdict in `plans/PR_ROAST_2026_09_27.md`.

## 2026-09-25 (wave 2): Merge Discipline Bound + Queue Cleared

### Summary
Bound two merge rules that were only implicit, then executed the queue under them. `AGENTS.md` step 18 and Core Rule 8 now require: **never `gh pr merge --auto`**, and **every PR merged from a head that is up to date with the latest `main`** — rebase, push, wait for CI green on *that* head, re-check `mergeable`/`mergeStateStatus`, then merge; repeat per PR because each merge moves `main`. The roast skill's Step 5 became "emit, do not execute unless instructed" with the mandatory per-PR command sequence. The rule is not theoretical: a bot branch's rebase resolved `progress/PROGRESS.md` toward its pre-fork side and would have silently reverted **787 lines** of merged work (the gate script, its `ci.yml` step, the PR template section, the `AGENTS.md` roast rule, both `PR_ROAST` records) had it been merged on the green tick.

### Actions
- Merged in order, each one rebased onto the live `main` tip and CI-verified on its post-rebase head immediately before landing: **#773** merge discipline (`f2eee21`), **#771** evidence-gate scope trace (`835b28e`), **#772** doc-contract rule (`b4ee04e`), **#767** `chaos_strength` clamp (`25b9092`), **#774** closure records (`554faeb`). A sequencer enforced the discipline; it stopped once on a head-moved guard rather than merging on an uncertain head.
- Closed four perf PRs as **no demonstrated impact**: **#763** and **#769** (owner call), then resubmissions **#775** and **#776**. `#775` re-proposed the closed #763 change and quoted "up to ~13.5 %" — the **best row** of the sign-flipping table from my own #763 measurement (three losses, including +8.9 % at K=100). `#776`'s single-token fast path is behaviour-preserving (verified by digest equality on both revisions) but its `Vec::with_capacity` hunk is **byte-identical** in allocations (1 alloc, n × 1 280 B, both variants — the #754 pattern recurring), and its long-text claim has no mechanism in the diff.
- #767 landed clean: clamp, flipped integration test asserting the clamp, unit test, and the two restored behaviour contracts.

## 2026-09-25: Three-PR Roast (#767, #768, #769)

### Summary
Second roast pass under the gate bound in `AGENTS.md` (merged `31f793f`). Three drafts, three different verdicts — and the first time the rubric's own rules were tested against myself: I initially read #767's 21-line doc removal as comment-stripping, then measured it (`grep -c "ADR-0094"` → 3 → 3) and found the rationale reworded, not lost. The roast says what was actually lost (the `#[cfg(feature = "persistence")]` contract prose on ~7 methods) and not what wasn't. The skill now carries that "verify before accusing" step so the next reviewer doesn't repeat my near-miss.

### Actions
- **#767** `fix(framework): clamp chaos_strength` — **merge after doc restoration.** The fix is real: `with_chaos_strength` passed `NaN`/`±∞`/out-of-range straight to the reservoir; the clamp plus boundary tests is right, and the 20-test builder baseline is green on main. Blocked only on restoring the persistence-feature contract prose, plus a body note for the `const fn` → `fn` API change and the flipped negative-value test expectation.
- **#768** `feat: chaotic complex map` — **closed as submitted.** 10 research scratch files committed to the repo root, a self-marked `[FALLBACK — outside this run window]` citation behind a new public API, and a "~9x faster" headline beside "Not measured" rows for entropy/distribution/recall. The idea is not rejected; the packaging is. Remote branch deleted.
- **#769** `perf(memory): get_unchecked in LSH projection` — **request evidence.** The SAFETY argument is correct (verified: `bit_pos < 10240` ⇒ `byte_idx >> 3 < 1280`, gated on `bytes.len() >= 1280`) and the safe path is retained — materially better than the #737/#739/#740/#754 class. Blocked on the evidence gate (live body → `exit 1`, missing `## Performance Evidence`) and on a real safety point: the bound is a hardcoded literal in a fn generic over `Hypervector`, so a future non-10240 impl becomes an out-of-bounds read, not a perf regression.
- Records: `plans/PR_ROAST_2026_09_25.md` (153 LOC, per-PR verdicts + recommendations); three roast comments posted, with the closing comment on #768; three `progress/LEARNINGS.md` entries; rubric additions to `pr-roast-triage` (verify-before-accusing, unsafe-bounds-in-generic-code, self-contradicting-evidence-tables). `validate-skill-format.sh` 33/33.

## 2026-09-24: Roast Before Implement or Merge (AGENTS.md Rule)

### Summary
Made the PR/issue roast a standing gate rather than a thing we remembered to do. `AGENTS.md` now binds it in three places: Phase 1 step 5 (review and roast before **implementing** an issue/PR — premise, affected code path, evidence bar), Phase 5 step 17 (**no verdict, no merge** — CI green is necessary, not sufficient), and Core Rule 9. A no-impact PR is a terminal outcome: roast comment → close → record `plans/PR_ROAST_<date>.md` → update `progress/` → distill the lesson into `.agents/skills/`; quietly re-implementing a closed PR's idea without the evidence it lacked is the failure mode the rule prevents.

### Actions
- `.agents/skills/pr-roast-triage/SKILL.md` gained a "When this gate applies (MANDATORY)" section (when the roast runs, the terminal no-impact flow, distill-don't-duplicate) — compacted into the existing skill rather than adding a near-duplicate; `scripts/validate-skill-format.sh` passes (33/33).
- `progress/LEARNINGS.md` PR Triage section: one entry recording the #737/#739/#740/#754 + #520 pattern (5 rounds of close-then-guardrail before the rule existed).
- AGENTS.md workflow steps renumbered continuously 1–18 across five phases; Core Rules 1–9; 90 LOC (≤ 200 cap).
- Applied to the live queue: #763 (`perf(retrieval): defer score scaling in single list hybrid merge`, +66/−70, one file, no Criterion/flamegraph, no `## Performance Evidence`) **fails the new CI gate as-is** — it is the exact no-impact class this rule governs.

## 2026-09-23 (perf gate): Perf-PR Evidence Gate Enforced

### Summary
`enforce_perf_claim_evidence_gate` landed: `scripts/check-perf-pr-evidence.py` runs in the always-on `commitlint` job of `.github/workflows/ci.yml` (no `detect-changes` dependency, so every PR is covered regardless of path filters) and, for titles beginning `perf(`, requires a `## Performance Evidence` section documenting a baseline (`plans/evidence/bench/canonical.json` or a benchmark id from it; nearest canonical comparator plus new output when the file has no entry), a path/URL-shaped Criterion output or flamegraph reference (a bare `flamegraph` word does not count), and a numeric before/after pair for the affected benchmark. Title and body reach the checker as positional argv with `PR_TITLE`/`PR_BODY` environment fallback (never shell interpolation of `${{ }}` into `run:`); scope validity stays with commitlint's `scope-enum`, non-`perf(...)` and bot titles exit zero, and the check is presence/shape only — it does not run benchmarks or assert improvement. `.github/PULL_REQUEST_TEMPLATE.md` gained the matching section.

### Actions
- Prerequisite merged first: `fix(core)` #759 (`e5d039e`) converts two `manual_range_contains` asserts in `csm-core-lib` re-export tests to `(0.0..1.0).contains(&v)`. Reason: CI's `lint` job and `validate.sh` both run clippy **root-scope** (no `--workspace`), so those two errors sat invisible to CI while the workspace-wide sensor in `scripts/harness-check.sh all` stayed red on main; the fix unblocks the local harness gate, not CI.
- Nineteen-case local matrix exercised (adds adversarial template cases and the argv interface): non-perf/docs/bot/empty titles pass; missing section, empty body, missing baseline, missing artifact, missing numeric pair, placeholder-only, untouched-template, and "template + numbers only with the artifact prompt unedited" bodies fail with named `::error::` lines; complete Criterion/flamegraph bodies and argv invocations pass.
- `plans/ACTIONS.md` queue drops to three actions (`enforce_perf_claim_evidence_gate` removed), `plans/GOAP_STATE.md` sets `perf_pr_evidence_gate_enforced: true` and `action_last_completed: enforce_perf_claim_evidence_gate`; `validated: false` and the wave-32 reconciliation action stand (the July audit has other exit criteria).
- No benchmark code, threshold or evidence artifacts touched; `benchmark-ci.yml` and `scripts/bench-baseline.sh compare` remain the measurement gates.

## 2026-09-23: Harness-Trial Queue Reconciled (#748-#752)

### Summary
Reconciled `triage_harness_trial_issues_748_752` against landed changes: #752 (libsql vs markdown spike) and #748 (do-harness P0 wrapper, drop-for-P1 verdict parity failure) and #751 (goap-plus-adrs guardrail, hard-constraints.md:44-47) all closed; #749 ranker (`scripts/skills-suggest.sh`, frontmatter-only, no do-harness dependency) plus the four-step loop (`agents-docs/quick-reference.md:6-13`) merged in PR #757 (squash `85e9e89`, all 23 checks green, Codacy included), and all three acceptance queries return the expected skill in top five; #750 commented with the non-adoption decision (keep current `install-hooks.sh` path) and closed, #749 closed (auto-closed on merge). Queue now holds four actions; `queued_actions_count: 4`; `action_last_completed: triage_harness_trial_issues_748_752`.

## 2026-09-22: Perf-PR Roast (#754) + Followup Queue

### Summary
Single open PR triaged: #754 (`perf(memory)`: pre-allocate capacity / drop iterator closures in `singularity_retrieval`) closed as **no demonstrated impact**. Counting-allocator adjudication of every changed collection shape: the two scan-tail hunks are byte-identical before/after (257 allocs / 11 264 B — `TrustedLen` `collect()` is already single-exact-alloc), the `graph_candidates` tail hunk is self-contradicting (`with_capacity(len.min(max))` then pushing `len` before `truncate` re-allocates in the case it claims to fix), and the residual ~13-alloc / ~12 KB growth-chain delta is dwarfed by the 257 `String` clones and 256 × 1 280-byte Hamming scans the rewrite keeps. No criterion evidence attached (4th entry in the #737/#739/#740 class).

### Actions
- Roast comment posted on #754 with the evidence table + resubmission target (`plans/evidence/scale_release_2026_09_21` baselines); PR closed, remote branch deleted.
- Record: `plans/PR_ROAST_2026_09_22.md`; guardrail learning in `progress/LEARNINGS.md`.
- Queued 5 followups in `plans/ACTIONS.md`: harness-trial issues #748–#752 (start: #752 spike, gates #748), `csm-duckdb` publish, perf-claim evidence gate, retrieval `String`-clone elimination, wave-32 flag-truth reconciliation.

## 2026-09-21 (wave 2): Bucketed-Candidate Recall Fixed at Scale

### Summary
Executed the action the Tier-3 evidence queued an hour earlier: the bucketed candidate generator lost recall as the corpus grew. Two distinct defects were separated by measurement — a biased truncation and a fixed probe width — and both are fixed with a policy that declines rather than returning an unrepresentative slice.

### Measured, before the fix
| configuration | 10 k | 50 k | 200 k |
|---|---|---|---|
| `floor 2` (library default) | recall 0.016 | 0.068 | **0.016** |
| `floor 8` (evidence setting) | 0.604 | 0.576 | **0.208** |

All widths from 2 to 6 returned exactly `max_candidates` candidates: the bucket was **truncated in index order**, so the sample was biased (the first clusters only) — that is the dominant loss, not the width itself. `fell_back_to_exact_scan` never fired.

### Fix (`crates/csm-memory/src/singularity_retrieval.rs`)
- `effective_bucket_probe_width(configured, corpus_size, budget)`: the configured width is a **floor**, raised so one bucket fits `max_candidates`, capped at `MAX_BUCKET_PROBE_WIDTH` (16).
- **Multi-probe**: candidates within one masked bit of the query bucket are accepted.
- **Budget guard**: a probe larger than `max_candidates × BUCKET_BUDGET_SLACK` (2) returns nothing, so the caller's exact-scan fallback answers instead of a sliced bucket.

### Measured, after the fix
| configuration | 10 k | 50 k | 200 k |
|---|---|---|---|
| `floor 2` | probe declines (25/25) → recall 1.000 at exact-scan latency | same | same |
| `floor 8` | 570 candidates, **0.832** recall, 249 µs (exact 558 µs) | probe declines (23/25) → 0.976 | declines (25/25) → 1.000 |

The bucketed path can no longer silently return a low-recall candidate set: it either produces a bounded, higher-recall probe or defers to the exact scan. Unit tests cover the width function and the decline/multi-probe behaviour; `bucket_sweep/` in the evidence directory holds every run (pre-fix widths 2–16 at 200 k, post-fix floors 2 and 8 at 10 k/50 k/200 k).

## 2026-09-21: ADR-0095 Tier-3 Evidence — Reference Runner, Canonical Baseline, Release-Scale Artifacts

### Summary
Completed the Tier-3 release-claim requirements and flipped `benchmarks_prove_performance` to true. The gap was concrete: `pre-release-validate.sh` compared Criterion against `--baseline main`, a baseline that lives in git-ignored `target/` and is wiped by `cargo clean`, so no release was ever gated on a measurement.

### Actions
- **Reference runner** (`plans/REFERENCE_RUNNER.md`): names `csm-ref-01` (i5-8350U, 16 GB, Linux, rustc 1.88.0) and the rules — every published number names its runner, cross-runner comparisons use ratios, release claims need a committed baseline or a CI ceiling.
- **Canonical baseline** (`scripts/bench-baseline.sh`): `save` records `plans/evidence/bench/canonical.json` — 88 benchmarks with median and confidence interval plus commit/toolchain/CPU; `compare` re-measures and reports deltas with CI overlap, failing only when delta > tolerance *and* the intervals do not overlap, `--advisory` for laptop runs. Observed noise on `csm-ref-01` justifies that rule: `delete_concept*` moved +27 %/+43 % between two runs minutes apart (non-overlapping CIs), and a later run flagged `crud_roundtrip_percentiles` +22.8 % instead — hence advisory + re-run-to-confirm rather than a hard wall on a laptop. `pre-release-validate.sh` section 8 now calls the comparison.
- **npm artifact evidence** (`scripts/wasm-evidence.sh`): builds `release-web` through `scripts/build-wasm.sh` (which now resolves its out-dir against the repository, since wasm-pack resolves relative paths from the crate), runs the Node smoke test, and records bytes, SHA-256s, toolchain and the transcript in `plans/evidence/wasm_2026_09_21/`. The `.wasm` is byte-identical across three independent builds (`6804814a…`).
- **Release-scale evidence** (`plans/evidence/scale_release_2026_09_21/`): ANN at 10k/50k/200k and memory/storage at 50k/100k/200k fit with a 500k hold-out. Two findings, both new: bucketed candidate recall@10 collapses 0.604 → 0.576 → **0.208** because `bucket_probe_width` is fixed while the corpus grows (queued as `fix_bucketed_candidate_recall_at_scale`), and HNSW recall at fixed `ef_search` falls 0.908 → 0.896 → 0.800. The memory model stays linear to 100k (0.29 % hold-out error) but under-predicts at 500k by 15.6 %, so the per-point measurements — not the extrapolation — are the claim.
- **Rendering**: `scripts/render-scale-evidence.py` derives a scaling-trend table and the two findings above from the artifacts, so the prose cannot drift from the JSON.

## 2026-09-20: WASM Artifact Parity — CI Now Validates What Ships

### Summary
Closed GOAP audit row A5 / the `wasm_ci_release_artifact_identical` flag. CI, the release workflow, the size gate and the TS-freshness check each built a *different* wasm artifact; the shipped one was never built or smoke-tested before publication.

### Measured divergence (before)
| Path | Command | Artifact |
|---|---|---|
| CI smoke | `wasm-pack --dev --target nodejs` | 4 077 828 B `.wasm` |
| Release (npm) | `wasm-pack --release --target web` (+ wasm-opt) | **656 657 B** |
| Size gate | raw `cargo build --release -p csm-wasm` | 1 133 971 B (threshold 1 150 000 B — 1.4 % headroom) |

The gate guarded an artifact nobody ships, CI validated a 6.2× larger dev build, and no job ever exercised the release profile.

### Actions
- **`scripts/build-wasm.sh`** — single source of truth for the package build (`dev-nodejs`, `dev-web`, `release-web`); prints package dir, byte size and SHA-256.
- **CI wasm job** now builds `release-web` through that script, smoke-tests it, and uploads `wasm/pkg` as an artifact; **`release.yml`** calls the same script, so CI and release cannot drift again.
- **`wasm/test.js`** loads either target: for `--target web` it hands the wasm bytes to `init` (Node's `fetch` rejects `file://`) and merges the ESM named exports with the CJS default view. Verified against both packages — the release/web run also exercises the `importFromBytes` round-trip added in #725.
- **`scripts/wasm_size_gate.sh`** measures the shipped artifact (656 657 B, sha256 `6804814a…`, threshold re-based to 800 000 B with ~22 % headroom) instead of the raw cargo output; **`check-wasm-freshness.sh`** builds through the script too.
- **Reproducibility data point**: two independent `release-web` builds produced byte-identical `.wasm` (`6804814a4aea2885a10dd3ab11255a592d73aff3185cf4eab0df6e74443f5d27`), so the npm artifact is reproducible on a fixed toolchain.
- **Docs**: `book/src/wasm.md`, `book/src/release.md` and the `dist-channel-selection` skill now point at the script. Flag `wasm_ci_release_artifact_identical: true`.

## 2026-09-18: Test-Surface Dedup — Queue Emptied, Coverage Methodology Replaced

### Summary
Executed the last queued GOAP action `deduplicate_test_and_source_surfaces`. Three read-only scouts mapped the surfaces, then a mechanical body comparator and hand review sorted real duplicates from look-alikes. 24 facade test bodies went away; six files that earlier analysis called "verbatim duplicates" were *rejected* with evidence and stay. The GOAP queue is now empty (`queued_actions_count: 0`).

### Actions
- **Duplicates removed (24 bodies, workspace attributes 1053 → 1029)**: `src/embedding/mod.rs` 12 (owner: `csm-embedding`), `src/persistence_wasm.rs` 6 (wasm32-only module; its test module referenced an undefined helper and could not compile), `crates/csm-core-lib/src/maps/neural_circuit.rs` 3, plus one each in `src/lib.rs`, `src/export_payload/export_payload_tests.rs`, `src/wasm_ext_tests.rs`, and an empty module in `src/wasm_graph_rag.rs`.
- **Source dedup**: `csm-core-lib::maps::neural_circuit` was a second, unreachable copy of the CDNCM map (`f64::tanh`, unconditional serde) while `csm-chaos` owns it and the root feature forwarded only to `csm-chaos`. It is now a re-export with the feature forwarding to the owner (serde preserved). `csm-core-lib::hashing::chaotic_lsh` was checked and kept — it is a documented wrapper, not a body copy.
- **Fix ported, not deleted**: the root embedding copies carried the `ENV_TEST_LOCK` race fix from PR #700; `csm-embedding` still had the racy pattern with a false "single-threaded test" SAFETY comment. The guard was ported into the owner (4 tests, 8 env mutations) before the facade copies were removed.
- **False positives rejected (6 files)**: `cache_lru_coverage`, `ttl_lifecycle`, `builder_advanced`, `path_validation_errors`, `critical_error_paths`, `batch_ops_coverage` were proposed for deletion but assert distinct behavior (metrics accounting via `probe_batch_cached`, error variants, retrieval after TTL, invalid-config failures). Evidence table in the audit.
- **Coverage methodology (ADR-0095)**: `scripts/update-coverage.sh` (test-LOC/source-LOC ratio, `crates/` blind, rewrote README rows that no longer exist) deleted; `scripts/coverage-report.sh` added — `inventory` (unique compiled behavior per layer, 1029 attributes; 8 same-name leads, all reviewed) and `llvm-cov`/`llvm-cov-all` (line + branch coverage; 74.33 % lines / 64.08 % branches over lib + all integration targets vs 68.39 %/53.17 % for unit-only). Pre-commit hook now runs the fast inventory.
- **Verification**: baseline `cargo test --all-features` exit 0 → after the deletions exit 0; the instrumented full-coverage run executed every target green; `csm-embedding` 15 passed, `csm-core-lib --features experimental-neural-circuit` 75 passed, `csm-chaos --all-features` 20 passed; clippy/fmt clean.
- **Record**: `plans/TEST_SURFACE_AUDIT_2026_09_18.md`.

## 2026-09-17 (wave 2): ADR-0095 Scale Evidence — ANN/Persistence/Memory Measured, Two Actions Closed

### Summary
Executed the two queued ADR-0095 evidence actions. A new release-only harness (`examples/scale_evidence`, driven by `scripts/scale-evidence.sh`) produces machine-readable artifacts with the full ADR manifest (commit, dirty state, corpus version/seed/checksum, command, features, toolchain, hardware, samples, variance). The measurements contradicted two recorded claims, both corrected here: concurrent local writes failed 90 % of the time (now 0 % after bounded retries in `csm-persistence`), and the `< 12 MB for 10M concepts` target is off by ~3 700× because it described an unimplemented design.

### Actions
- **#733 Merged** (`2fc421b`): runner + first artifacts. ANN at 50 k: exact 6.35 ms p50 (recall 1.000), HNSW 896 µs / recall 0.892 / 80 s build / 93 MB serialized, LSH 813 µs / recall 0.836 / 282 ms build, bucketed candidates (probe width 8) 1.83 ms / recall 0.590 with no index. Memory model: RSS `2 880 665 B + 4 691 B × concepts` (held-out error 0.29 % at 100 k), storage `2 850 B/concept` (0.06 %). Claim correction: the formula-only test became a measured footprint test, and the book/context.yaml target is marked not supported. Also excluded the harness from two opengrep rules in `.codacy.yml` (args/current_exe) after Codacy flagged them as high-severity false positives.
- **#734 Merged**: `csm-persistence` sets `PRAGMA busy_timeout = 5000` on local connections and retries transient lock errors (5 attempts, 2–32 ms) on the idempotent write paths. `tests/persistence_concurrency.rs` (8 writers × 25 saves; 8 writers × batch+associations) is the regression guard; the re-measured artifact shows 0.905 → 0.000 raw and 0.470 → 0.000 with caller-side retries, at the cost of higher tail latency while writers wait.
- **LOC gate**: the retry loop wrappers pushed `crates/csm-persistence/src/persistence.rs` to 518 lines, so the gate failed in both the lint and test jobs; fixed by sharing a `with_retry` helper and extracting `persistence_absence.rs` (the AGENTS.md rule: child-module extraction over comment stripping).
- **SonarCloud**: the new renderer took a filesystem path from argv (two path-traversal findings) and had cognitive complexity 36; it now resolves a validated directory *name* under `plans/evidence/` and is split into per-section functions.
- **Learning recorded**: a perf gate that asserts arithmetic is worse than no gate — the 10M/12 MB test passed for months while the shipped representation needed ~1 000× more memory.

### Next
`deduplicate_test_and_source_surfaces` (P3) is the only queued action; `persistence_contention_evidence_current`, `ann_scale_evidence_current`, `measured_memory_model_exists` and `ten_million_memory_claim_evaluated` are now true in `GOAP_STATE.md`, and `performance_claims_have_current_artifacts` flipped to true.

## 2026-09-17: Dependabot Queue Cleared — WASM bincode Fix, rmcp 3.4, opentelemetry 0.32

### Summary
Follow-on to the 2026-09-15 queue landing. Merged the WASM bincode fix (#725), consolidated the entire Dependabot backlog into two PRs (#726, #727), then took the two remaining major bumps (#724 rmcp 2→3, opentelemetry medium advisory) as real migrations (#728, #729) rather than suppression ignores. Zero open PRs at the end; `progress/LEARNINGS.md` carries the new CI/supply-chain lessons.

### Actions
- **#725 Merged** (`0bcbd5c`): `WasmFramework::exportToBytes` wrote with bare `bincode::serialize` while `importFromBytes` read with `DefaultOptions` — browser export→import round trip failed with `string is not valid utf8`. Aligned the writer and added the round-trip assertion to `wasm/test.js`.
- **#726 Merged** (`ebe5190`): consolidated Dependabot updates. `fuzz/Cargo.lock` batch (thiserror, async-trait, wasm-bindgen-futures, clap, uuid + transitives), `taiki-e/install-action` 2.87.11 → 2.87.12 (supersedes #722), and the double-scope fix. #717–#722 closed as superseded.
- **Dependabot double-scope root cause**: `commit-message.prefix: "chore(deps)"` (and `ci(deps)`) combined with `include: "scope"` produced `chore(deps)(deps): …` titles, which fail `scope-enum`. Prefixes reduced to `chore`/`ci`; an ignore rule for existing double-scoped messages added to `commitlint.config.cjs`; the PR-title check now keys on `github.event.pull_request.user.login` instead of `github.actor` (a maintainer's `update-branch` made the actor a human, so Dependabot PRs started failing a check meant to skip them).
- **#727 Merged**: `clap_complete` 4.6.7 → 4.6.9 in `fuzz/Cargo.lock` (Dependabot-native, CI green).
- **#728 Merged**: rmcp 2.2.0 → 3.4.0. rmcp 3 routes results through MRTR enums — `call_tool` returns `CallToolResponse`, `read_resource` returns `ReadResourceResponse` (`Complete(..)` via `.into()`); `ListToolsResult`/`ListResourcesResult` gained the SEP-2322 `result_type` and SEP-2549 `ttl_ms`/`cache_scope` fields, so struct literals no longer compile (`with_all_items(..)` instead); `ServerInfo` is deprecated for `ServerConfig`. Protocol version unchanged (`ProtocolVersion::LATEST` = `2025-11-25` in both).
- **#729 Merged**: opentelemetry family 0.27 → 0.32, clearing GHSA-w9wp-h8wv-79jx (medium, unbounded allocation in W3C Baggage propagation). `TracerProvider` → `SdkTracerProvider`; `Resource::new(vec![KeyValue::new("service.name", ..)])` became private → `Resource::builder().with_service_name(..).build()`; `tracing-opentelemetry` 0.28 → 0.33. Verified with `cargo test --features otlp --test observability_integration` (6 passed, incl. a real gRPC exporter build).

### Not fixed (non-actionable)
- **`lru` GHSA-rhfx-m35p-ff5j (low, patched 0.16.3)**: transitive via `quinn-proto 0.11.16` and `tantivy 0.22.1`, both on `lru ^0.12`. Not bumpable from this repo until those parents release; the alert stays open.
- **`libsql-sqlite3-parser` (low, no patched release)**: pinned by libsql upstream, already ignored in `deny.toml` and `dependabot.yml` with a documented reason.

### Final state
`main` at `c841d28`, zero open PRs, zero open issues; `cargo deny check advisories` ok and `cargo tree --locked --all-features` resolves (rmcp 3.4.0, opentelemetry_sdk 0.32.1). `plans/GOAP_STATE.md` metrics refreshed in place (`main_head`, `tests_count` 1092 → 1053 from the dedup waves, `integration_test_files` 70 → 71, `dependabot_alerts_open` 5 → 3). Local `target/` (116 GiB) reclaimed with `cargo clean`.

## 2026-09-15 (wave 2): ADR-0094 Persistence Owner Dedup Completed + PR Queue Landed

### Summary
Full `deduplicate_persistence_owner_bodies` wave (user-approved) plus the blocked PR queue. A freshly published advisory (RUSTSEC-2026-0285, rustls) had every open PR red on `Cargo Deny`; landing the one-file lockfile bump first, then `update-branch`-ing each PR so it inherited the fix, cleared the queue sequentially (#707, #708, #709, #710, #711 — all squash-merged with green CI, never `--auto`). Persistence now has a single implementation owner: root keeps only re-exports, one-line delegations and two infallible payload converters.

### Actions
- **#712 Merged** (`68d603d`): `cargo update -p rustls --precise 0.23.45` (+ `rustls-webpki` 0.103.15) for RUSTSEC-2026-0285; `cargo deny check advisories` and `cargo check --locked --all-features` green. Landed before the queue so every other PR could inherit it.
- **#709 commitlint fixed**: the two offending commits were rewound (`git reset --soft HEAD~2`) and recommitted as `docs: regenerate stale llms API listing` / `docs: use the 0.3 major-minor spec in install snippets`; the first body also had to be re-wrapped (a single 431-char line is parsed as a footer and trips `footer-max-line-length`).
- **#707/#708/#709/#710/#711 Merged**: dependabot action pin, triage record, repo hygiene (artifacts untracked + 0.3.8 version truth + llms), ADR-0071 rerank restoration, persistence phase 2. Each PR: `update-branch` → full CI → `gh pr merge --squash --delete-branch`.
- **Phase 3 (PR #713)**: `src/export_payload.rs` reduced from 177 to 51 lines — the payload/wire types and `unix_now_secs` now come from `csm-traits`, with `concept_to_export`/`export_to_concept` as the only root code; `ExportConcept` gained `#[serde(default)]` for legacy JSON. Verified: lib tests (195 on the pre-#711 base, 184 afterwards) + no-default-features (135), targeted integration suites, clippy/fmt, wasm32 check, a legacy-JSON import through the CLI, and a before/after binary comparison (field-level decode of both bincode payloads identical except the wall-clock field and pre-existing map ordering; cross-imports both ways).
- **Phase 4 (this PR)**: dead `record_concept_version` wrapper + its `#[allow(dead_code)]` removed from `csm-persistence`, the wasm stub export made mutually exclusive with the real one (PR #711 left `cargo clippy -p csm-persistence --all-features` failing with E0252 on a host target — CI only compiles default features for that crate, so nothing caught it), llms regenerated, CHANGELOG/GOAP/ACTIONS/plan-doc/PROGRESS/LEARNINGS updated; `queued_actions_count` 4 → 3, `action_last_completed: deduplicate_persistence_owner_bodies`.
- **Found, not fixed**: `WasmFramework::exportToBytes`/`importFromBytes` disagree on the bincode config (legacy vs `DefaultOptions`), so the browser round trip fails — pre-existing, documented in `progress/LEARNINGS.md` with the probe, left for its own change.

## 2026-09-15: PR Queue Clear — Draft Closed as No-Impact, Hygiene + Persistence Queue

### Summary
Zero open issues; two open PRs. Roasted both: #706 (Jules `perf(retrieval)`) closed as no-impact after allocator-level measurement, #707 (dependabot action SHA bump) kept as the single mergeable keeper. Session also surfaced three main-truth defects queued for the hygiene PR: the 0.3.8 version revert, tracked-but-gitignored `export.json`, and a 2.2 MB committed `cargo metadata` dump.

### Actions
- **#706 Closed** (no impact): outputs identical, allocations identical (13/13 allocs, 131,626 bytes map path; 12/12, 24,410 bytes fast path), timing deltas opposite in sign (−4.4% / +4.3%). Full roast comment posted to the PR; reopen path documented (attach criterion output from `benches/hybrid_benchmark.rs`).
- **#707 Keeper**: `pre-release-gate.yml` SHA pin `fa23953… v2.87.10` → `9534c84… v2.87.11`; CI green 27/27, `mergeStateStatus: CLEAN`, merge left to the maintainer.
- **Hygiene PR queued**: untrack `export.json` + `benchmarks/Cargo.lock`, remove `metadata.json`, align `migrations/007_add_hnsw_graph.sql` with the Rust schema, restore the 0.3.8 version/CHANGELOG bump.
- **Persistence queue (ADR-0094, user-approved)**: 4-phase `deduplicate_persistence_owner_bodies` — crate parity, root delegation, payload convergence, cleanup; executed after the hygiene and rerank PRs.
- **Rerank queue**: `src/framework_rerank.rs` (`probe_with_rerankers`, ADR-0071) is unreachable since #178 dropped its `mod` declaration and compiles clean — restore module + regression test.
- **Report**: `plans/PR_ROAST_2026_09_15.md`.

## 2026-09-11: Stale-Base Squash Revert Repair + PR Wave

### Summary
PR triage surfaced that the `943/fdcd069` squash sequence (Jules PR #683) was based on a stale fork and reverted already-merged main content: the gh-release v3.0.3 pin (#674), the uuid 1.26.0 lock entry (#677), the LSH `compute_hash_from_bytes` / pre-sized candidate-set work (#683, `a3eeaab`), the `progress/LEARNINGS.md` compaction, and `plans/PR_ROAST_2026_09_08.md`. Restored every reverted path in one dedicated PR and encoded the detection rule in `progress/LEARNINGS.md`.

### Actions
- **#686 Merged** (`ccbb7e8`): restores LSH bit-mask hashing + `HashSet::with_capacity(32)` (the #683 optimization clobbered by `9140aa4`).
- **#684 Queued**: `fix(singularity)` clamps `BridgeRetrieval::query` top_k to `MAX_TOP_K_LIMIT` (CWE-770), matching the BM25 clamp precedent.
- **Restore PR**: workflows back to `action-gh-release@efb3536… # v3.0.3`, `Cargo.lock` back to uuid 1.26.0 (`cargo metadata --locked` OK), `progress/LEARNINGS.md` + `progress/PROGRESS.md` + `plans/PR_ROAST_2026_09_08.md` restored from `f6117e7`, LSH comment corrections and `RetrievalConfig` struct literals restored from `a3eeaab`.
- **#685 Pending**: feature-gated `experimental-ils3d` hyperchaotic map (Sun et al. 2025); CI green, needs a naming-fidelity review before merge.
- **Report**: `plans/PR_ROAST_2026_09_11.md`.

---

## 2026-09-08: PR Roast — Sequential Merge of Remaining PR Wave

### Summary
Triaged all 7 open PRs (#679, #677, #676, #675, #674, #673, #660) per `pr-roast-triage`. Zero closures (no duplicates, no zero-impact PRs). Applied roast fixes: rewrote #660's stale body via REST API, marked draft #679 ready. Merging sequentially with full CI-truth verification per AGENTS.md (no `--auto`).

### Actions
- **#679 Merged** (`a73b8a0`): perf(retrieval) single-pass hash inserts in BFS expansion + fact dedup. Required `gh pr ready` (Jules draft) before merge.
- **#660 Body Fixed**: Stale re-export description replaced via `gh api -X PATCH`; follow-up queued to lift `MAX_TRAVERSAL_EXPANSIONS` into `GraphRagConfig`.
- **Branch Sync**: #660 synced via `gh api -X PUT .../pulls/660/update-branch` (no `gh pr update-branch` subcommand in installed gh); all 5 dependabot PRs queued for single-pass `@dependabot rebase` onto final main.
- **Report**: `plans/PR_ROAST_2026_09_08.md`.

---


## 2026-09-07: PR Roast Triage & Resolution of All Open Issues

### Summary
Comprehensive triage and roast of all 13 open PRs and 4 open GitHub issues. Closed duplicate PR #672 with no independent impact. Verified and merged PR #647, resolving and closing all 4 open issues (#639, #640, #641, #642). Deduplicated root retrieval modules, verified 100% Codacy pass, and established sequential manual merge plan for the remaining 11 open PRs.

### Actions
- **All 4 GitHub Issues Resolved**:
  - #639 (`RetrievalConfig abort knobs + for_token_count`): Implemented in `crates/csm-memory/src/singularity_retrieval.rs`.
  - #640 (`perf: cut probe_text / BridgeRetrieval work`): Parent issue resolved and closed.
  - #641 (`perf(bridge): cap expansion and Hamming only new IDs`): Implemented in `src/bridge_retrieval.rs` via `score_specific_candidates`.
  - #642 (`perf(retrieval): BM25 absence + stage-1 cap + HDC early-exit`): Implemented across retrieval pipeline.
- **PR #647 Merged** (`ff85763`): 28 CI checks green, 0 Codacy issues. Replaced ~970 lines of duplicate root retrieval files (`bm25.rs`, `rerank.rs`) with re-export shims over `csm_retrieval` (ADR-0094), reducing repository LOC by over 5,300 lines while keeping all files strictly under the 500 LOC gate.
- **PR #672 Closed**: Redundant duplicate of #653; stripped public wrapper, failed commitlint.
- **PR Titles Fixed via REST**: Updated #670 (`perf(core)`) and #660 (`fix(retrieval): cap GraphRAG traversal expansions at 1000 nodes`).
- **Codacy Gate Verified**: 0 issues across all 13 analyzed PRs.
- **Merge Plan Emitted**: `plans/PR_ROAST_2026_09_07.md`.

---

## 2026-07-27: PR Triage + CI Queue Fix

### Summary
GOAP-orchestrated sweep of open PRs. Closed #571 (fake perf optimization), merging #573 (dependabot).
Fixed release workflow timeout caused by CI queue starvation.

### Actions
- **PR #571 closed**: `perf(retrieval): optimize min/max loops` — replaced `.min()`/`.max()` with `<`/`>` comparisons. Net negative: removes docs, adds 4 mutation exclusions, reverses intentional design. LLVM generates identical code either way.
- **PR #573 merging**: dependabot bump `taiki-e/install-action` 2.83.4 → 2.85.2 (3 workflow files).
- **CI queue starvation**: Runs 30274366967 + 30275287578 stuck "queued" >1hr. Cancelled and re-triggered. Release run 30274367903 timed out at 1800s waiting for CI that never started.
- **Fix**: Enhanced `wait-for-ci` in release.yml to detect perpetual-queue and re-trigger CI.

---

## 2026-08-05: Review-to-merge sweep (PRs #597-#601)

### Summary
Reviewed and merged the final open PRs: #597 (direct SIMD hamming), #598 (graph candidate &str), #599 (native arm64 test job), plus state/docs PRs #600 and #601. The queue is now empty and every perf claim landed with same-machine criterion evidence attached.

### Actions
- **PR #597 merged** (`0b42edb`): `BHVec10240::hamming` dispatches directly over the packed `[u64; 160]` words — AVX2 (x86_64), NEON (aarch64), unrolled scalar fallback (wasm32) — eliminating the two `to_hvec()` layout conversions. Same-machine criterion ~2.6–2.75× faster (main 99.4/149.9 ns vs 37.7/54.5 ns). Codacy unsafe-usage findings fixed in code via `.codacy.yml` `exclude_paths` (repo policy); zero dashboard suppressions. Added forced-fallback unit test + lane-skip guard.
- **PR #599 merged** (`8d63b27`): `test-core-arm64` job on `ubuntu-24.04-arm` executes the NEON kernels in CI (previously compile-only on x86_64).
- **PR #598 merged** (`075cfe8`): `generate_graph_candidates` BFS borrows `&str` instead of cloning every candidate `String` — ~8% faster end-to-end (same-machine A/B: 238.3 → 218.5 µs, ~840 fewer allocs). Evidence bench committed: `benches/graph_candidates_benchmark.rs`.
- **PR #600/#601 merged** (`59c44d6`/`cb45951`): state records for all merged PRs; benchmark-methodology learnings (stale-binary detection, forced clean A/B, load-spike exclusion, deterministic test graphs); `benchmark-graph-candidates` CI regression gate (ceiling 600 µs, measured 258.9 µs on GitHub hardware).
- **Process notes**: commitlint footer-max-line-length requires every message line ≤ 100 chars; `npx commitlint | tail` swallows the exit code — always check `$?` explicitly.

---

## 2026-07-23: Wave 33 — CI Fixes + GOAP Orchestrator Hardening

- PR #551: WASM `--target nodejs` for CI + main concurrency guard
- PR #552: Removed dead `is_known_absent` (zero callers, zero tests)
- PR #553: GOAP orchestrator `status`/`wave`/`verify` commands + swarm patterns

---

## 2026-07-18: Framework Ops Perf + PR Triage

- PR #524-#526: namespace single-clone, parallel inject, split import locks
- Merged #528 (BM25 hot loop), #527 (Rayon probe_batch), #529 (hybrid partial top-k)
- Closed #520 (empty Jules research PR)

---

## 2026-05-23: Documentation Audit
Synchronized README, architecture docs, and books with implementation state.

## 2026-04-11: Persistence Field Integrity
Schema v6: `expires_at` + `canonical_concept_ids` across all persistence surfaces.

## 2026-04-09: Benchmark Harness (v0.3.1, v0.3.2)
BM25 Rayon parallelization (~40% speedup), p99/NDCG metrics, percentile fix.

## 2026-04-08: Hybrid Retrieval + v0.3.0
BM25+HDC fusion with query-length weights. Recall@1: 2.5% → 75%. Semantic Bridge (ADR-0061), table prefix (ADR-0063).

## 2026-04-06: Release Workflow
npm OIDC Trusted Publishing. Fixed duplicate CHANGELOG header breaking awk extraction.

## 2026-10-08: PR #862 hook bootstrap repair

Repaired the unsupported commit scope and missing CI bootstrap without weakening
strict validation. Fixed ineffective hook-path acceptance, worktree redirection,
fallback-root resolution, spaced source paths, and seven stderr findings.
The hook fixture passes 20 cases and ShellCheck 0.11.0 passes changed hook scripts.
The #842 action remains queued until merge; Rust source, APIs and test counts
are unchanged. Full review and validation scope: plans/PR_ROAST_2026_10_08.md.

Hosted CI 37836615208 confirmed all 20 hook cases and full Rust validation, then
found stale #825 in the queue. Reconciled the action already implemented by merged
#858/d611e12; its three persistence failure tests passed. Seven actions remain.

## 2026-10-09: ruleset gates enforced (#850) and the six-PR round roasted

### #850: the three core jobs existed, so make them required
Ruleset `12897801` required only `Build CLI (linux-x64)`, `Codacy Static Code Analysis` and
`commitlint`; one `PUT` appended `lint`, `test`, `mutation-test` (app 15368) and nothing else
changed (bypass actor, strict policy, `do_not_enforce_on_create` and all six other rules
byte-identical to the snapshot). Verified from the stored object, `GET /rules/branches/main` and
`gh pr checks 867 --required` (3 → 6 contexts); issue #850 closed with the evidence comment.
Admin bypass retained (explicit decision), so the gates bind non-bypassing merges.

### Roast round: three closes, one evidence bar, two keepers
- **#860 closed as a no-op**: `gh pr diff 860` and `git diff d611e12..eba2ec8d` are empty — the two
  `perf` commits are empty commits and the merge only re-adds main's content. Fourth filing of the
  `HVec10240::permute` zero-shift idea (#800 measured it: 81.7 → 57.3 ns direct, caller path
  2 684 → 2 734 ns; #814/#819 closed).
- **#863 closed as a no-op**: the #783 zero-init refutation (same files, `nm` same address, bare
  `memcpy`), plus a deleted `Lifetime:` rationale line and a Codacy unsafe flag; no bench, red gate.
- **#864 closed as no-impact**: its title claims an early return that has been on main since #611
  (`git log -S 'return *self;'` → `e5affb9`); the net diff is a zero-init relocation; the only
  `BHVec10240::permute` bench uses shift 321 (`321 % 64 = 1`).
- **#867 kept open**: pre-allocations are correct but unmeasured (growth-event savings only; #754
  measured the same shape to nil); the evidence bar is stated in the comment.
- **#861 keeper**: implements #830; red commitlint is scope `evidence` (not in `scope-enum`, per the
  job log); needs rebase, head_sha-pinned checkouts, an observable inventory fallback, post-merge
  queue cleanup.
- **#866 keeper**: equations match arXiv:2609.23276 §2 eq. (8) and the paper's RK4 — but no CI job
  compiles the feature (`workspace_default_members` = root package only; fixture probe in the
  record), so it was built locally: 13 tests pass, clippy clean.

**Queue hygiene**: the local queue gate caught a pre-existing STALE entry — #842 closed at
08:09:35Z while `consolidate_the_three_hook_installers` was still queued; removed in this PR (7 → 6).

Record: `plans/PR_ROAST_2026_10_09.md`. No keeper was merged — the merge order is emitted there.

## 2026-10-10: #866 and #861 landed; a bot-merge revert caught before it shipped

### The merge order executed, one rebase at a time
#866 (`c75e700`): paper-verified map + the CI path that finally compiles `csm-chaos`'s
experimental modules (`cargo test -p csm-chaos --all-features` in the workspace matrix).
#861 (`3ef32d5d`): scheduled evidence + the argv-free mutation inventory, after **three** bot
pushes. The second push had merged main with a conflict resolution that *reverted* main's #826
persistence work — caught by the pre-merge two-dot diff and fixed by rebuilding the branch on
current main (10 files, persistence drift 0). The Codacy holdout was one unused `import sys`; the
CodeQL holdout was the write-capable `workflow_dispatch` + `head_sha` combination — dropped per
the query's own docs, which exempt `workflow_run`'s read-only cache scope. The merge also fixed
main's red lint: #826 landed without its queue-entry removal and the reconciliation gate failed
closed on it (run 38026936562).

### Queue
6 → 5 (#826, caught by the gate on main) → 4 (#830, this close-out). #869 roasted (keeper, four
repairs) — next in the merge order; #867 still open on its evidence bar.

## 2026-10-10 (cont.): #869 landed, #840 closed, queue drained to 3

#869 (`1fd790e8`): rebuilt on current main after the roast (scope `fix(scripts)` → `fix(lints)`,
Codacy's `${TEST_DIR:?}` guard + a file-scope SC2016 directive), and the gate's `find` now skips
`*/node_modules/*`, `.qoder/*`, `.opencode/*` — measured on this workstation: 473 findings in
5m48s before, **0 findings in 13s** after, so `validate.sh` stays honest on developer trees, not
only on clean checkouts. The head was refreshed once (identical content, new sha) because
commitlint reads the PR title from the run's *event payload*: the title PATCH after the push left
the in-flight run validating `fix(scripts)` while the commit check on the same run passed.
Queue: 4 → 3 (#840 removed here).

## 2026-10-10 (cont. 2): #871 closed as no demonstrated impact after an A/B/A' measurement

Draft `perf(memory): preallocate candidate generation and scoring collections`: roasted and closed
(comment pull/871#issuecomment-6099040717). The PR diff applied onto current main measured
1.335–1.524 ms on `find_similar_graph_500x20_d3` against main's 1.487–2.123 ms, but the A' leg
(baseline restored) came in at 1.270–1.395 ms — the medians fell 1.80 → 1.42 → 1.33 ms across all
three runs regardless of code, so the trend is host drift, not the diff (criterion's paired p
0.58 forward, <0.01 backward). Also: the body's "~3.8%" contradicts its own pair (−1.3%),
commitlint was red on the missing `## Performance Evidence`, and the branch tree reverted four
landed PRs (#826/#861/#869/#873) via its `Merge branch 'main'` resolution.

## 2026-10-10 (cont. 3): #829 closed — the gate-graph umbrella re-measured, queue 3 → 2

The umbrella action's own exit criterion (one canonical gate graph) re-verified on `4a8b4f84`
after its three decomposed children landed: `negative-fixtures.sh` and `validate-links.sh`
deleted (#846/#869); `test-llms-sync.sh` + `test-version-sync.sh` (`validate.sh:134`),
`validate-workflows.sh` (`:327`), `check-docs-links.sh` (`:238`) and `validate-git-hooks.sh`
(`:49`, with the `test-hook-bootstrap.sh` fixture) all wired with fixtures; hook installers
unified on `install-hooks.sh` (#862) with `setup-hooks.sh` and `--install` delegating and
`scripts/pre-commit.sh` a documented shim; the skill validator's 15-case fixture at `:253`.
`ci.yml:658` runs `bash scripts/validate.sh`, so every one of them executes in the `lint` job —
main CI run 38064370389 green on this head. Issue closed with the claim-by-claim comment;
`single_gate_graph: true`; queue entry removed.

## 2026-10-12: the Jules round — two no-impact closes, #878's budget repair, #828 closed

Three PRs arrived after #876. #875 (`perf(core)`: permute + from_bytes) and #879
(`perf(hyperdim)`: permute zero-shift) were roasted and closed: both re-file #800/#814 (direct
call 81.7 → 57.3 ns, product path `text_encoder/encode_short` 2 684 → 2 734 ns = noise → closed
no-impact) and #783 (`from_bytes` zero-fill elimination — old/new folded to the same address);
neither carried `## Performance Evidence`, and #875 also duplicated the #829 close-out
bookkeeping against a pre-#876 base. #878 (`ci`: bound `CARGO_BUILD_JOBS=2`, `Fixes #877`) is a
keeper: its first two runs were killed at the `lint` job's 15-minute timeout (15m16s / 15m19s
measured), so the branch carries `timeout-minutes: 30` for lint (`1560d03a`) with the numbers in
the comment; its next run then surfaced the queue-gate STALE window for #828, which this PR
closes. #828's second half landed in #880 (`d951067a`): `context.yaml` counters re-measured
against live sources and gated by the rewritten `tests/validate_llm_context.py` (12-case fixture,
wired into `validate.sh`), `gen-agents-context.sh` retired. Queue 2 → 1 (#833 remains).

## 2026-10-12 (cont.): #878 merged, #877 closed — the bot rebase that had to be thrown away

#878 was green in the end (`lint` success, 13m01s against the 30-minute budget) and landed as
`6d7c7be4`, closing #877. The merge was not of the head CI had validated in the previous round:
Jules pushed `9ad593c1`, a rebase whose conflict resolution reverted the merged #880 and #881 —
it re-added the closed `#829` queue entry, restored `scripts/gen-agents-context.sh`, deleted
`scripts/test-validate-llm-context.sh` and reversed the `context.yaml` refresh (466+/677− against
`main`). The branch was rebuilt as `origin/main` + its two intended commits (the eight-workflow
env bound and the lint budget), proven with `git diff origin/main --stat` = those 8 files,
force-pushed with a lease, and merged from that tree. Queue: 1 (#833).

## 2026-10-15: three-PR roast — #893 closed, #887 closed, #889 kept with three repairs

Session-start triage of the three open PRs on `main` = `edb1302`. A dirty
`plans/GOAP_ORCHESTRATOR.md` (the tracked file rewritten by `scripts/goap-orchestrator.sh plan`,
which emits one action per issue *label*) was restored to `HEAD` before anything else.

**#893** (`perf(hyperdim_binary)`: `MaybeUninit` conversions + `permute` zero-shift, head
`dfb45ff8`) — **closed as no demonstrated impact**. Both halves re-file measured/disassembly-closed
predecessors (#800 → #814 → #819 → #875 → #879 for the `permute` hunk; #783 → #863 → #875 for the
conversions), and I refuted the premise on the pinned toolchain rather than citing those records:
transcribed bodies linked into one binary show `from_bytes` main/PR at the same address
(`0x14b50`, `memcpy` only, no `memset`) and the `memset` surviving in the aligned `permute` path
for `word_shift != 0` even with the initializer moved inside the branch (`0x14f20` vs `0x14fd0`).
The diff also carried resolver churn (`windows-sys` 0.52→0.59 ×7, `getrandom` 0.4.3→0.3.4
downgrade); `commitlint` red on the perf-evidence gate, Codacy red (`unsafe` usage).

**#887** (dependabot `uuid` 1.26.1 → 1.28.0 in `/fuzz`) — **closed, not mergeable**: the
`Fuzz Workspace Build` job fails with `uuid@1.28.0 requires rustc 1.89.0` against
`rust-toolchain.toml` 1.88.0, and crates.io metadata shows 1.27.0 needs 1.89.0 too, so no
compatible upgrade above 1.26.1 exists. Recurrence cure left as an owner decision (dependabot
`ignore` with a "remove when MSRV moves" reason, precedent `libsql-sqlite3-parser`, or an MSRV move).

**#889** (`perf(encoder)`: single-token fast path + capacity pre-allocation, head `f2aea132`) —
**kept open, keeper pending three repairs**. Measured out-of-tree: `encode("hello")` 590.6 → 468.3 ns
(6/6 rounds same sign, ≈ −20 %), `2 allocs / 1 344 B` → `1 alloc / 64 B`, digest-identical output.
Repairs: drop the `with_capacity` hunk (3-token encode is `2 allocs / 3 904 B` on both revisions —
the #754/#776 finding), replace the body's false "4.51 µs → 675 ns" pair with the measured one and
the gate's `## Performance Evidence` section, and add an input that separates the two branches (the
new test passes under the `tokens.len() == 1` → `!= 1` mutant — verified: 10 passed).

Verdicts, evidence and the merge order: `plans/PR_ROAST_2026_10_15.md`. Lesson distilled into
`.agents/skills/pr-roast-triage/references/codegen-and-measured-claims.md` with two compact
pointers from `SKILL.md`. No queued GOAP action was in scope: `plans/ACTIONS.md` still holds only
`mutation_baseline_the_feature_gated_mcp_module` (#833), `queued_actions_count` unchanged at 1.

## 2026-10-15 (cont.): #833 — `src/mcp` gets a mutation baseline and leaves the exclusion list

The queued action `mutation_baseline_the_feature_gated_mcp_module` is done, the issue closed with the
measured record, and the queue is empty for the first time (`queued_actions_count: 0`).

**Measured, not asserted.** Scoped `cargo-mutants` over the four production files with
`--features chaotic_semantic_memory/mcp --lib` (rustc 1.88.0, cargo-mutants 27.1.0, `--in-place`):
**60 mutants → 34 missed / 18 caught / 8 unviable** before; the 32 triaged survivors plus five
pre-existing controls then re-verified at **37/37 caught**, i.e. `60 − 2 documented exclusions = 58
tested`, `58 − 8 unviable = 50 killable`, **50 caught = 100 % of killable mutants**. Evidence and the
per-survivor triage table: `plans/evidence/mutation_mcp_2026_10_15/`.

**What carried the kills.** `src/mcp/handler_tests.rs` (7 tests: the 13 schemas as a table, the tool
and resource definitions, `map_error`, `get_info`, the full tool dispatch with the export format
branches, per-arm argument validation, the resource URIs) and `src/mcp/wire_tests.rs` (1 test that
drives a real rmcp server over `tokio::io::duplex` with newline-delimited JSON-RPC — the only way to
reach `initialize` / `list_tools` / `list_resources` from `--lib`). `serve_with_shutdown` is killed
rather than excluded: its test now requires the listener to accept a connection *before* the signal,
which is what kills the whole-body `Ok(())` neuter.

**The exclusions.** All three `src/mcp` entries left `scripts/mutation_test.sh` (control: 0 mutants
listed with the path glob, 60 without; the `mcp::` label was inert because `--exclude-re` matches
mutant descriptions, not module paths). Two entries remain with mechanisms recorded: `replace serve ->`
(entry point, observable only in a process taking SIGINT/SIGTERM — the hop's effect is pinned by
`server_tests.rs` and the process-level tests in `tests/cli_shutdown_signal.rs`) and
`replace | with ^ in parse_hvec_u64_halves` (equivalent mutant: disjoint bit ranges by construction).

**Two gates had to move with it.** A new `.rs` file under `src/` changes `modules_under_loc_limit`
(97/97 → 99/99 — the first draft of the test file was 606 lines and had to be split, extracted not
stripped) and the generator's per-file heading made `llms-full.txt` stale; `tests_count` went
1044 → 1052 (8 new lib tests). And `scripts/check-goap-queue-issues.sh` failed closed on the empty
queue ("parsed 0 actions … Both fail closed"), which had never been reachable before: it now
reconciles an unparsed queue against `queued_actions_count: 0` and the fixture proves the tolerance
does not neuter the tracker half (an OPEN `GOAP:` issue is still UNQUEUED) or the count half.
Fixture: 50 → 54 assertions, all green.

Residual, measured and stated: the CI job is `timeout-minutes: 45` with `--in-diff`, per-mutant
rebuilds cost 86–108 s, so a diff rewriting all of `tools.rs` (31 mutants) would pressure the budget —
not a regression, since those lines previously produced nothing at all.

## 2026-10-15 (cont.): #889 closed under Rule 10, re-created as #897 — the encoder fast path

#889, the one keeper of the morning roast, carried a real measured win in a head that could not land:
four `google-labs-jules[bot]` commits on top of three human-resolved `Merge branch 'main'` commits,
three behind, `commitlint` red. Rule 10 says re-create rather than grind, so the PR is closed with
its round-1 evidence carried forward and the change re-created as **#897** off current `main` with
the three repairs the verdict demanded: the provably no-op `with_capacity`/`push` hunk dropped
(`2 allocs / 3 904 B` for three tokens on both revisions), the false body numbers replaced by a fresh
Criterion pair (`encode_single_1tok` 587.71 ns → 471.53 ns, −19.8 %, with the untouched
`encode_short_3tok` control flat — the claimed `~4.51 µs` was #776's own claimed *post-change*
number), and the non-discriminating test replaced by two that the guard mutants actually fail
(`==`→`!=` fails the three-token `assert_ne!`; `&&`→`||` fails both).

Two operational findings from this round, both encoded in `progress/LEARNINGS.md`: a full root volume
surfaced inside `cargo-mutants` as `could not compile … due to 1 previous error` on an *unmutated*
tree (`target/` had grown to 63 GB; `cargo clean -p csm-core-lib -p chaotic_semantic_memory`
reclaimed 54.9 GiB), and that tool has no positive mutant filter — `--file` needs a
workspace-relative path, and `-p` after `--` only changes the test command.

## 2026-10-16: roast round — #899 re-created and landed, #900 closed as submitted

Two Jules-bot PRs arrived after the 2026-10-15 sweep. **#900** (`feat: implement SVSC 4D map`) is
closed as submitted: paper fidelity unverifiable (`[FALLBACK]` citation + a self-described
approximation of the paper's 4D system), a `~68.2 ns → ~5.5 ns` latency claim with no Criterion
artifact and `Recall/quality: Not measured` beside it, and 557 lines of unrelated `Cargo.lock`
churn — while its feature/bench gating pattern (`experimental-svsc-4d = []` plus
`required-features`) is the one #866's verdict asked for and is credited in the record. **#899**
(`fix: bound graph rag retrieval parameters`) was closed under Rule 10 and re-created: the substance
verified out (clamp before the allocations, `pub` constants required by the root copy, values
matching the merged `MAX_TOP_K_LIMIT` precedent, two new tests on both live surfaces), but the head
was two bot commits plus a human `Merge branch 'main'` and the change re-filed #865, closed unmerged
with no recorded reason.

Two corrections from the process are worth more than the verdicts: the delegated #900 report's two
"blocking" findings were both wrong (the feature *is* declared; the lock churn does *not* revert the
security bumps), and a delegated agent wrote `ROAST_P900.md` into the repo root despite instructions.
Both are encoded in `progress/LEARNINGS.md`. Filed alongside: **#905** (`crates/*` outside the
root-only lint/test jobs — with a live `csm-persistence` clippy failure as the evidence).

## 2026-10-17: #909 merged as `7b7202a4` with supplied evidence

The roast verdict was executed as merge-after-evidence, not a close: two commits were added to the
Jules branch (`cd0df80f` slow-path benches + `dca01fde` rationale/guard docs), the body gained the
ADR-0095 evidence section (validated locally with `scripts/check-perf-pr-evidence.py` before GitHub was
touched), and the head `dca01fde` went 30 pass / 4 skipped / 0 failures — all six required contexts
green (`Build CLI (linux-x64)`, `Codacy`, `commitlint`, `lint`, `test`, `mutation-test`) plus the
ruleset's `code_scanning` gate once `Analyze (rust)` settled — then squashed as `7b7202a4`. Measured on
the reference CPU: `k = 10` 350.2 → 317.0 µs (p = 0.00; six interleaved legs, mean −7.5 %) and the
`k = 1` probe 77–84 → 50–51 µs (p = 0.00). The new bench groups make the `mmr_rerank` selection loop
measurable for the first time; a follow-up can add them to `plans/evidence/bench/canonical.json`
(`scripts/bench-baseline.sh save --set rerank_benchmark`), which has no `mmr_rerank*` entry.

## 2026-10-17 (cont.): encode the #909 lessons (`docs/agents-909-lessons`, merged as `4fbd429b` in #911)

Thirteen files, no new skills (33 unchanged; `CATALOG.md` regenerated from the sharpened frontmatter):
`benchmarking-perf` (ADR-0095 gate mechanics + local dry-run, reach proof for a cited bench via the
marker and `top_k = 1` methods, measurement isolation with one target dir per variant),
`github-ci-guardrails` + its `gh-ci-truth` reference (measured ruleset state, `code_scanning`/CodeQL
holding `BLOCKED`, robust polling, and a version-scoped correction of the stale "`gh pr checks` has no
`--json`" claim — gh 2.102.0 documents `--json fields` and `--json name,bucket` returns real JSON),
`git-workflow` (supplying evidence on a foreign branch + land-and-verify), `AGENTS.md` steps 10/18,
`agents-docs/quick-reference.md`, the roast record, and the `bench-blindness` reference.

The build caught a revert in progress: the worktree's branch (`726e9c31`) is an ancestor of `main`, and
six of the copied files were already newer upstream — `git diff --stat origin/main` showed 220
deletions before a per-file `git diff --quiet <branch> origin/main -- <path>` gate identified them.
Every path was then rebuilt as `origin/main` + only this session's edits, and the gates re-run in the
clean worktree: `validate-skill-format.sh` 33/33, `check-skill-catalog.sh`, `check-docs-links.sh`, and
the router top-1 for both target queries.

## 2026-10-17 (cont.): #910 rebuilt, merged as `f496d319`, closing #905

The roast verdict was **rebuild in place**, not merge: the head `10aaca88` had `main` as its
merge-base yet deleted the #909 roast record, the #911 `bench-blindness` reference and skill rules,
the #909 slow-path bench groups and part of `crates/csm-retrieval/src/rerank.rs`, and renamed the
CLI binary back to `csm-cli` — its four "Merge branch 'main'" commits re-applied a pre-#909 snapshot,
so ancestry protected nothing. Rebuilt as `main` (`20b4fc37`) + two commits: the bot's honest first
commit (`default-members` for issue #905 + the two `missing_const_for_fn` findings) plus the second
round CI surfaced *after* the push:

- `lint`: `error: document output filename collision` — workspace `cargo doc` documents both `csm`
  bins (root `src/bin/csm.rs` and `csm-cli`'s `src/main.rs`) into one path; fixed with the bot's own
  `doc = false` on the facade bin, now documented. Then **five unresolved intra-doc links** in member
  crates the root-only gate had never reached (`csm-embedding/src/projection.rs:38`/`:94`,
  `benchmarks/src/cli.rs`) — fixed exactly as the bot had them, and the first rebuild had dropped
  them as false reverts.
- `Build CLI (windows-x64)`: `LNK1104: cannot open file …\deps\csm.exe` — `cargo build --bin csm`
  now matches two packages; every such selector is `-p`-scoped (both `ci.yml` CLI jobs,
  `validate.sh`, `pre-release-validate.sh`, the roundtrip advice), plus pre-emptive scoping in
  `release.yml`, `pre-release-gate.yml` and the `mcp-feature` job.
- `scripts/validate.sh`'s wasm check and the pre-release gate's were `-p`-scoped: a bare command now
  builds the whole workspace for `wasm32`, which is not wasm-buildable.
- `scripts/ci-workspace-crate-matrix.sh` gained enforcement 5 (a member present in `members` but
  absent from `default-members` is an error) with fixtures 17/18, so the hand-kept copy cannot rot
  back into the blind spot.
- Both `.githooks` exclusions: the workspace clippy in `pre-commit`/`pre-push` excludes `csm-duckdb`
  — the root crate does not depend on it, its bundled DuckDB C++ build takes ~20 minutes on a cold
  tree (15-minute hook kill, `Disk quota exceeded` on the 7.5 GB `/tmp` tmpfs), and it blows
  pre-push's 180 s budget, which rejects the push (rc 124).

Verified before the merge: `default_members=12 members=12 missing_from_default=0`; fixtures 18/18;
the old bare wasm command fails while the scoped ones pass; the hook's cold workspace clippy 3 m 05 s;
and, with `csm-duckdb` built, the **bare** CI stages `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
--all-features` → rc 0 and `cargo clippy --all-targets --all-features -- -D warnings` → rc 0. CI on
the merged head `9a54564d`: 40 pass / 1 skip / 0 fail; merged `--squash` as `f496d319`, which closed
#905 (queue stays at 0). The remaining open issue is **#903** (Pre-Release Gate `security-audit`,
which also keeps that gate red alongside the `pr-drain-check` that #910's own presence tripped).

## 2026-10-17 (cont.): #903 pre-release-gate repair

Historical CI truth separated the cascade from the actual audit failure: `39022056863` followed
failed CI at `4fbd429b` and had all seven gate jobs skipped but a failed summary. On successful CI
at `20b4fc37`, `39023117196` ran the checks: five passed, the audit failed on four vulnerabilities
with six allowed warnings, and the drain check named #910 as its one open PR. The drain policy is
unchanged.

`cargo audit` 0.22.2 returned nonzero before configuration (four vulnerabilities: h2 RUSTSEC-2026-0258
and zip RUSTSEC-2026-0367/-0368/-0369; six allowed warnings). After `.cargo/audit.toml` mirrored
the nine IDs in `deny.toml`, it returned zero with two allowed warnings (`instant` and `lru`
RUSTSEC-2026-0253). The three zip ignores are limited to the reviewed non-bundled extraction path;
the workspace enables DuckDB's bundled feature.

Local verification: verdict fixture 17 assertions / 0 failures; parity fixture 9 / 0; real
parity set 9 IDs; negative control rc 1 named the missing ID. ShellCheck `-S info` passed all four
scripts; actionlint 1.7.12 passed; and `CARGO_INCREMENTAL=0 bash scripts/validate.sh` passed after
`cargo clean` removed 54,740 files (61.2 GiB). GitHub refused the temporary dispatch workflow
because it was absent from the default branch; verified fallback Release run `39022056988`
concluded success with operational jobs skipped and `notify` successful.

## 2026-10-18: #914 merged; post-merge gate green

PR #914 (`fix(ci): repair the pre-release gate's verdict and RUSTSEC ignore parity`) squash-merged
as `850cf48a17f0f0317b0e4dcfe6e0294af379a522` at `2026-10-17T18:28:20Z`; it closed issue #903.
Main CI run `39058232581` succeeded on that SHA.

The first Pre-Release Gate run after the merge was `39060608423` at `2026-10-17T19:00:58Z`
(`workflow_run`, same head SHA). Its overall conclusion was success. All seven gate checks passed:
`pr-drain-check`, `version-triad`, `security-audit`, `wasm-smoke-build`, `planning-state-check`,
`benchmark-gate`, and `mutation-test`. `gate-summary` also passed; its log shows upstream CI
`success`, all seven needs results `success`, and `✅ All gates passed`.

`bash scripts/check-goap-queue-issues.sh` passed after #903 closed: 0 actions, 0 tracked issues,
0 effect keys; the queue and `queued_actions_count: 0` agree. The new no-verdict branch was not
dispatched on GitHub: the temporary workflow returned 404 because it was not on default `main`.
The fallback Release run `39022056988` did not execute the new helper; its local fixture is the
available direct proof for that branch.

Unrelated draft PR #915 (`perf(core): fast-path 0-shift in HVec10240::permute`) was reviewed,
roasted, and closed as a no-impact duplicate of #800, #814, and #819 (commitlint perf gate failed,
by-value copy refuted, no new benchmark evidence), clearing the PR-drain gate condition for PR #916.

## 2026-10-20: the 2026-10-19 audit follow-up set, implemented on one branch

`plans/RECOMMENDATIONS_2026_10_19.md` recorded a P0–P2 backlog. This round implements it.

**Durable-or-unchanged mutations.** Every public mutation now commits to the durable rows before
it touches memory, and a persistence failure leaves both the live framework and a fresh reload at
the pre-mutation state. The failure matrix is a real matrix, not one case per method: inject,
delete, batch inject, metadata update, association, association batch, reinforcement, and pruning
each have a simulated-failure test that asserts the *reload* equals the pre-state, not just the
in-memory count. A failed `associate_many` applies no prefix of its batch. A namespace revision
advances exactly once per durable graph mutation.

**Transactional replacement import.** The old path cleared the namespace and then validated, so a
malformed payload left memory emptied and durable rows half-written. The new path stages and
validates the whole payload — concepts *and* association endpoints — before any state changes, and
an association naming a missing concept rejects the entire replacement instead of being skipped.

**One TTL and namespace policy.** A persistent purge removes expired concepts, their associations,
and their durable rows, so a reload cannot resurrect them. A namespace switch loads the target
namespace's associations, not only its concepts. A cold namespace export includes the graph edges.

**Versioned, bounded binary export (ADR-0100).** New public snapshots are written with Postcard
behind the `CSMBIN01` envelope (format version 1, codec id 2) instead of bincode 1.x
(RUSTSEC-2025-0141). The writer streams through a size-limited `Write` implementation, so a
snapshot can never exceed the 100 MiB import limit before the check — the old path allocated the
whole buffer first. The reader still accepts the historical version-1 fixed-int bincode envelope
and both un-enveloped legacy payloads, so existing exports keep loading. The native file API, the
namespace byte API and the WASM `exportToBytes` binding share one encoder and one decoder.

**MCP transport and security (ADR-0101).** `Transport::Sse` is renamed to
`Transport::StreamableHttp` — a breaking change with no alias, because the deprecated HTTP+SSE
label was the public name of a server that speaks Streamable HTTP. rmcp's current stateless path
handles current clients; the local session manager stays for negotiated legacy versions only. A
non-loopback bind now *requires* a bearer token, the default Host allow-list is loopback-only,
Origin validation is enforced, and the token is compared in constant time. Stdio is unchanged.

**Bounded OTLP lifecycle (ADR-0102).** The gRPC exporter uses a bounded batch span processor
instead of the synchronous simple processor, so a slow collector no longer adds its latency to
request handling. `OtlpBatchConfig` makes the queue size, batch size, scheduled delay, per-request
export timeout and shutdown timeout explicit; `force_flush` and `shutdown` report their failures
instead of discarding them.

**Retrieval consistency.** BM25 has one owner (`csm-retrieval`); the CLI no longer rebuilds an
index per query and no longer truncates documents at 200 characters; the CLI's JSON result carries
the stable concept id. GraphRAG has one implementation owner with the root module reduced to a
re-export, and its public config rejects NaN, infinity and out-of-range weights with
`InvalidInput`.

**Gate graph and tooling.** The required-vs-CI-only gate set is explicit; a missing required
dependency fails closed and a deliberate skip is outside the required set. `scripts/SCRIPT_CATALOG.md`
is generated and drift-gated, with a fixture that classifies owner, purpose, file mutation, remote
mutation, caller and fixture per script. `actionlint` is gated fail-closed; ShellCheck is opt-in
and says so. `scripts/validate-low-memory.sh` is a bounded Cargo-only development path that is
explicitly *not* a substitute for the two required gates.
The pre-release gate is a publication blocker: `release.yml` waits for its successful result on
the same `head_sha`, and manual dispatch verifies both CI and the gate for the same SHA.
`dist.yml` builds platform CLI binaries only; a main push with no bounded PR diff reports
mutation evidence as “not run,” not a passing score. The decision is recorded in ADR-0042 and
the `release-management` skill.

**Measured behavioral skill evals.** Five critical skills x 4 deterministic cases = 20 cases, each
response judged against every case assertion. The first measured run scored 18/20 and exposed two
rubric gaps the skills did not state (a performance verdict that omitted the retitle rule; a
push-confirmation refusal that omitted the hooks/CI rule) and one fragile case (a gate list that
omitted the full-range commitlint check). The skills and their cases were tightened and the suite
rerun: **20/20, mean probability 0.871**, no judge failures. Evidence:
`plans/evidence/skill_eval_2026_10_20.md`.

**One blind spot found while doing it.** The ADR-parity gate matched only `00[0-9]{2}`, so every ADR
id from 0100 upward was invisible to it: a registry row pointing at a missing ADR-0100 file would
have passed. The pattern now matches the full four-digit space (`[0-9]{4}`), with missing/present
fixtures at the first post-0099 id (0100) and the unpadded boundary (1000). The same class of check
— a gate whose pattern is narrower than the data it guards — is worth re-auditing elsewhere.

**Verification status at this update.** The first `scripts/validate.sh` run reached Clippy and
failed on a lossy `u32`→`f32` Hamming-distance cast; the checked `u16` conversion now preserves
the HVec10240 distance domain without a precision-loss lint. Clippy then exposed the newer
`chunks_exact_to_as_chunks` lint in the scalar LSH path; the two fixed-size slices now use
`as_chunks::<4>()` with the same remainder loop. ADR parity and its six fixtures, the workflow /
GOAP / mutation / catalog safety fixture chain, and changed-script ShellCheck have passed. Full
`scripts/validate.sh` and `scripts/harness-check.sh all` still need a green run after these fixes;
no hosted CI run or PR exists for this branch yet. `gh auth status` succeeded, open issues were
empty, and the only open PRs were unrelated dependabot updates #920 (conflicting) and #921 (one
failing check); neither duplicates this change set.

## 2026-10-22: re-created the wave off current main, and the gates found two things main is carrying

Scope set by the operator: **validate + commit only** — no push, no PR, no merge without a separate go-ahead.

**Re-created rather than ground through (`AGENTS.md` Rule 10).** `#932` rewrote `release.yml` on exactly the
mechanism the wave touched, so the wave became a fresh branch off `origin/main` at `0f77c0d0` (`feat/audit-
-followups-wave-2026-10-22`, 32 commits, 0 behind). Main's landed work was proven, not assumed: per-path
`git diff origin/main HEAD -- <path>` is empty for both workflow files, and the lockfile is spliced (+35 / −0)
instead of re-resolved.

**Two gates died and were fixed before anything was claimed.** Run 3 failed with `VALIDATE_RC=101` at
validate's **last** stage — `cargo check --target wasm32-unknown-unknown -p csm-wasm`: the wave had rewritten
`WasmFramework::import_from_bytes` to call `apply_import_payload`, which lives in a
`#[cfg(not(target_arch = "wasm32"))]` module, so the WASM build of the lib had not compiled since. Every native
sensor (clippy, `cargo doc`, the 101-target suite, the LOC gate) had already passed. Fixed by extracting the
filesystem-free transactional core into `src/framework_import_apply.rs` (byte-identical move, 149 lines verified
with `diff` against `HEAD`) so the native, byte and WASM entry points share **one** validate-then-install path,
plus removing a dead `#[cfg(target_arch = "wasm32")] use js_sys::Date;` the same build flagged. Run 5 then failed
at `actionlint` — see below.

**Three branches the repository had never asserted on.** `apply_import_payload` rejects duplicate concept ids and
duplicate association edges, and in merge mode lets an endpoint resolve against the live namespace — the two
rejections had **no test anywhere**, and none of the three had a lib-target test, which is the only scope
`scripts/mutation_test.sh` scores. Three tests added to `src/framework_ops_import_tests.rs`, each proven by
neutering its own guard: concept-id neuter → 3 reds, edge neuter → exactly the edge test, merge-allowance neuter
→ exactly the merge test; the source file was restored to its pre-experiment checksum. Inventory 1149 → **1152**
(`src/` 256 → 259), `context.yaml` synced, and the LOC counter moved to **114/114** because the new module added
a file — the drift gate caught that before I committed it.

**`main`'s release automation has been dead since `#932`, and it is not a CI flake.** The actionlint stage this
wave added (`c85984de`) scans all ten workflows and rejected two expressions in `release.yml` as landed:
`if: >- github.event_name == "workflow_dispatch"` (plus `"push"`, `"success"`) and
`if: matrix.target == aarch64-unknown-linux-gnu`. GitHub expressions accept single-quoted string literals only.
The consequence was measurable remotely — 39670046551, 39678113001, 39683503746 and 39734351757 (the last on
`0f77c0d0` itself) are all `conclusion: failure` with **`jobs: []`**: rejected at workflow parse, before any job
existed, so there is no step log and nothing published and no tag was created. The last release run that executed
anything is 39648046550 on `4a85fd47`, immediately before `#932`. This branch quotes all four literals;
`scripts/check-actionlint.sh` and `validate-workflows.sh --check` are now rc=0 (0 unvalidated files). Recorded in
ADR-0042's blocker chain and as hard rule 4 of the `release-management` skill. Note the asymmetry worth keeping:
main does not run this gate yet, so the wave's own gate is currently the only thing standing between an
unparsable `release.yml` and a silent non-release.

**Worktree hygiene, the reason run 2 died.** Sixteen stale `.qoder/worktrees/*` copies survived their dispatches,
one holding a 13G `target/`; removing them freed 12G (151G → 163G available) and left `.qoder` at 12K. Encoded as
`AGENTS.md` Rule 12 and Agent Trust Boundary item 6 (`patch → remove --force → prune → verify`, keep the local
branches — a squash-merged branch's pre-squash commits exist nowhere else). Honest caveat retained in the
learning: the `lint_cache.sh:205` ENOSPC itself was never root-caused (≈150G was free on the single `/` ext4 and
no quota is configured), so the cleanup is sold as hygiene, not as a proven fix.

**Measured on the tree being committed:** `VALIDATE_RC=0` and `HARNESS_RC=0` (six sensors: fmt, clippy, deny,
test, arch fitness, shared conventions). Suite **1170 passed / 0 failed / 1 ignored** over 101 targets; the
wasm32 check, size gate and `wasm-pack build` pass; ADR parity, GOAP queue (0 actions / 0 issues),
test-attribute scan, script catalog, LLM-context drift, workflow semantic validation, actionlint and RUSTSEC
parity all rc=0. `origin/main` is still `0f77c0d0`; nothing has been pushed. Open issues 0; the only open PR
is #933 (not this wave's work).

## 2026-10-30 (afternoon) — #962 in flight, v0.3.9 static half closed, queue at 11 actions

**#962 (the #953 gate-race repair) — roasted, self-corrected twice, waiting on a runner jam.** Head moved
`9b395d94 → 73d02f87` for a message-only reason: the fourth commit carried a 101-char body line that
`footer-max-line-length` rejects, so CI's `commitlint` job would have failed a PR whose own body asserted
`commitlint → rc 0`. Proof the tree did not change: `git diff 9b395d94 73d02f87 --stat` empty,
`git diff --numstat origin/main...HEAD` still `42 5 .github/workflows/release.yml`, range lint rc 0.
The load-bearing assumption behind the fix is now measured rather than asserted: `gh run list
--workflow=pre-release-gate.yml --commit "$SHA"` **does** see a `workflow_run`-triggered gate run
(`ae117fb987c6…` → `{"conclusion":"failure","status":"completed"}`) and returns `{}` with rc 0 for a SHA
that has none (`3953e818f407…`), which is the arm that must keep polling. Without that test the change
would have converted an instant refusal into a 30-minute one — worse, and green CI could not tell.

**Context, measured:** hosted runners are congested org-wide — main's push CI for the current head
`3953e818f407` (run 41059071360) was created 12:55:41Z and was still `queued` at 13:42Z, and no
`pre-release-gate` run exists for any main head after `ae117fb9` (2026-10-28T14:42Z) because the four
most recent gate runs all concluded `failure` on `pr-drain-check` while #956/#957/#958 were open. When
that queued main run finally completes it will fire `release.yml` from **main's** copy of the workflow —
the pre-#962 one-shot code — so one more `validate` refusal on `3953e818` is expected and harmless: the
gate check (`:76`) precedes tag creation (`:175`), so a refused run leaves no tag and publishes nothing.

**Release static half closed** (`/tmp/release-validation-2026-10-30.md`): 11 of 11 manifests at 0.3.9;
`wasm/package.json` + `cli-npm/package.json` + `VERSION` at 0.3.9; exactly one dated CHANGELOG header and
one `[0.3.9]:` link row; `./scripts/verify-version-sync.sh` rc 0 (the npm-registry mismatch is a warning —
0.3.8 is live, 0.3.9 is not yet published); `cargo package --list --allow-dirty` rc 0 over 603 files; no
local `v0.3.9` tag. Publish set re-measured end to end: **9 crates.io** (7 in the `:352` loop + root at
`:379` + `csm-duckdb` at `:392`, which waits for the root version in the sparse index because it
dev-depends on it) **+ 2 npm** = 11 registry artifacts, and `verify-release` (`:626`) probes 3 of them.

**Queue is at 10 actions, all with a numeric `github_issue:`, checker rc 0** (`10 action(s), 10 tracked
issue(s), 10 effect key(s), one gh call`). Two new tracker items from this pass, both filed with the
measurement in the issue rather than in a comment afterwards: **#954 gained the 3-of-11 coverage table and
the `bash -e {0}` abort mechanism** (confirming the title's premise and adding a dimension the title did
not carry), and **#964** is the member→root version-pin blind spot in `sync-version.sh` /
`verify-version-sync.sh`, deliberately *not* folded into the release PRs because opening a PR now fails
`pr-drain-check` and delays the publication the owner asked for.
- 14:35Z **#962 merged** (squash, `--delete-branch`) as `4ac8422d` on `main`: the `release.yml`
  gate-wait repair, 1 file, 42+/5-. Pre-merge truth re-derived, not inherited — head `73d02f87` is a
  descendant of `origin/main`, 25 pass / 0 fail / 9 legit `skipping`, `MERGEABLE`/`CLEAN`, squash
  subject 80 chars.
- 14:13–14:22Z **`main` was red and I found the cause, not the symptom**: `commitlint` job
  (check-run 124073904392) in main run 41059071360 failed `header-max-length` at **104** chars on the
  #947 squash subject. Recorded in `plans/PR_ROAST_2026_10_30.md` + `progress/LEARNINGS.md`; the red
  self-heals with this merge (range is now `--from 4ac8422d` for the next push, and `3953e818` leaves
  the linted set). Jobless release run 41070030431 reclassified as a *consequence* of that red CI.
- 14:4xZ filed **#965** (nothing lints the post-squash subject) + queued action
  `gate_the_effective_squash_subject_length`; queue now **11 actions**, `queued_actions_count: 11`.
  Rule distilled into `github-ci-guardrails` step 3 (114 → 122 lines).
- 17:0xZ **audited all 11 open issues against `main` `4ac8422d` — 10 still live, one substantially
  addressed.** Every "still live" is a tree measurement, not a re-read of the issue body: `grep -n fuzz
  scripts/validate.sh` empty (#948); one `LD_LIBRARY_PATH` guard against six wrapped call sites (#949);
  `mutation-test` single job, `timeout-minutes: 300` (#951); `verify-release` has no `defaults:` and 3
  probes of 11 artifacts (#954); `|| npm publish` intact (#955); `ci.yml:580` draft-conditional threshold
  with no `types:` (#959); refusals print aggregate status only (#960); `chaotic_lsh.rs:18` unbounded
  allocation with all-zero clamp (#961); two manifests still pinning root `0.3.8` (#964). #952 is the
  exception: option 3 + acceptance criterion 3 landed inside #947, criterion 2 and option 1 did not — so
  the finding went to the **issue** as a status audit
  ([comment 6528684458](https://github.com/d-o-hub/chaotic_semantic_memory/issues/952#issuecomment-6528684458)),
  the issue stays open for the owner to close-or-retitle, and the queued action was **renamed** to the
  surviving half (`detect_new_cfg_disabled_mutants`, with `#959`'s precondition retargeted to the new
  effect key) because its old name described work `main` already does.
- 17:16Z **a *critical* Dependabot alert that no gate of ours can see, filed as #966 and queued in the same
  breath.** The `git push` above printed "7 vulnerabilities on the default branch (1 critical, 2 high, …)";
  alert #40 is `GHSA-m665-v7pm-7grj`/CVE-2026-115905 (`zip <= 9.0.4`, patched **9.0.5**, published
  2026-10-28T21:09:50Z) against `Cargo.lock:7083`'s `zip 6.0.0`. `cargo deny` cannot report it because RustSec
  has **no id for it yet**, so there is nothing to fail and nothing to ignore — the two advisory sources are
  disjoint. Exposure measured to zero: `zip` is a *build*-dependency of `libduckdb-sys` (sparse index
  `"kind":"build"`, `req":"^6`), reachable only from `csm-duckdb`, and `libduckdb-sys/build.rs:144` gates the
  whole download/extract module under `#[cfg(not(feature = "bundled"))]` while `:371-374` is the
  `archive.extract()` call `RUSTSEC-2026-0369` names — i.e. `deny.toml`'s existing ignore reason for
  0367/0368/0369 is **true**, verified rather than inherited, but it rests on one `Cargo.toml` line
  (`crates/csm-duckdb/Cargo.toml:22`) that no sensor reads. Queue **11 → 12 actions**
  (`record_and_gate_the_zip_bundled_assumption`, `queued_actions_count: 12`), lesson distilled into
  `github-ci-guardrails` (122 → 136 lines, format-validated), no new PR and no labels — issues do not fail
  `pr-drain-check`, so the v0.3.9 path is unchanged.

- 22:4xZ **#960 implemented on `fix/name-the-failing-gate-job` off `642238f0` — the release refusal now
  names the gate job.** `scripts/gate-failure-detail.sh` reads the gate run's jobs JSON from stdin and
  emits per-job lines plus one `SUMMARY:` for the `::error::` annotation; `release.yml`'s
  `Verify CI and Pre-Release Gate` step captures `databaseId` and calls it on both refusal branches (the
  30m-timeout branch and the non-success branch). Fail-closed semantics are untouched — the diagnostic
  always exits 0 so it can never itself become a refusal reason, and rc 1 still comes from the existing
  guard. Proof: fixture `scripts/test-gate-failure-detail.sh` (8 cases / 22 checks, wired fail-closed into
  `scripts/validate.sh`), and both *real* gate runs replayed through the step body extracted from the
  current YAML — 41078875116 → `SUMMARY: 2 of 7 gate jobs failed: pr-drain-check, gate-summary` rc 1;
  41110522706 → rc 0. Gates green: actionlint rc 0, shellcheck clean, validate-workflows.sh 0 errors with
  warnings held at the pre-PR 24, `check-script-catalog.sh` pass after regenerating
  `scripts/SCRIPT_CATALOG.md`, `CSM_GOAP_QUEUE_REQUIRED=true check-goap-queue-issues.sh` →
  11 actions / 11 issues / 11 effect keys. GOAP sync rides in the same PR (`name_the_failing_job_in_the_release_refusal`
  removed, `queued_actions_count: 12 → 11`, effect key added, `action_last_completed` moved, and the two
  stale prose claims corrected: #962's wait is now *measured in both directions*, and
  `release_pipeline_validated` reflects the live tag + npm 2/2 + 7 assets against crates.io 0/9 blocked on
  the owner-only credential in #967). Roast verdict: `plans/PR_ROAST_2026_10_30.md` ("#960 — roasted before
  implementation"), including the five self-inflicted errors the re-derivation caught.

- 2026-10-30 18:52→19:14Z **v0.3.9 published everywhere except crates.io, and the wait proved itself.**
  CI on `642238f0` completed success → gate run **41110522706** and release run **41110522627** both started
  18:52:01. `validate` ran 18:52:06 → 19:02:29: the bounded wait held ~**6m55s** and then saw
  completed/success (gate first job 18:53:01, `gate-summary` done 18:59:01 = 6m00s of run; `security-audit`
  18:54:14 → 18:58:52 = 4m38s is the slowest *job* — the 4m37s figure `release.yml` quoted was job-level, so
  the 30-min bound is ~4.3× the real wait, not ~6.5×). Tag `v0.3.9` created → `642238f0` (lightweight).
  `build-wasm`/`build-cli-matrix`/`create-github-release`/both `publish-npm-*` → **success** (7 assets; npm
  2/2 at 0.3.9 with `attestations=provenance,url`). `publish-crates` **failed** 19:07:45 → 19:10:28 with nine
  `403 authentication failed` uploads and `verify-release` **failed** 19:10:32 → 19:14:41 → index 0/9. The
  E0432s in the later crates are a *consequence* (a verify build resolves companions from the registry, and
  the companions never uploaded), not a feature-gate bug. Filed as **#967** with the ~1 s
  `GET /api/v1/me` pre-flight; recovery is `recover=true recovery_tag=v0.3.9` and is owner-only (the
  `CARGO_REGISTRY_TOKEN` behind the `crates.io` *environment* secret wins over the repo-level copy).
- 2026-10-31 08:19→08:49Z **#960 landed and main went red on an infra fault, which exposed a worse defect.**
  `fix(ci): name the gate job that refused the release` merged as `bfa15d49` (owner-merged; #960 auto-closed).
  CI run 41184146253 job `test` (124487486597) failed with annotation `System.IO.IOException: No space left on
  device : '/home/runner/actions-runner/…/_diag/Worker_20261031-081949-utc.log'` — the self-hosted runner's
  disk, not the tree (the PR run on `b9e2044c` had concluded success, and no step carried a failure
  conclusion). Re-ran the failed job. While that was still in flight, `release.yml` run **41186134783**
  triggered off the *failed* CI conclusion and skipped `validate` + all eight dependents
  (`needs.validate.outputs.release-needed` is null), executing only `notify` (`if: always()`) — and concluded
  **`success`** while crates.io stayed 0/9. Filed as **#969**: the #903 no-verdict rule, never applied to
  `release.yml`. Lesson recorded in `progress/LEARNINGS.md` (run-level status is not monotonic; a green
  `Release` is not proof of a release) and distilled into `release-management` (post-publish verification +
  the crates.io token pre-flight).

## 2026-10-31 — the release-verifier trio lands, #974 filed, queue at 10

**main's red `test` job was disk, and it is closed.** Run 41184146253 (`bfa15d49`) re-ran to
`completed/success`; the failure was `System.IO.IOException: No space left on device` on the self-hosted
runner's `_diag` log, with no step carrying a failure conclusion. No code changed. (#970's roast record
has the classification.)

**#970 (docs wave) merged as `4ea6a8e6`** — squash, head current with `bfa15d49`, 25 checks pass / 9
skipping / 0 fail, `mergeStateStatus: CLEAN`, `Codacy Static Code Analysis` green, verdict recorded in its
own branch (`plans/PR_ROAST_2026_10_31.md`). Its worktree removed per rule 12; `git worktree list` is back
to the main tree plus one.

**#973 = #954 + #971 + #972, one branch because they are two lines apart in the same step.** Writing the
replay fixture is what found #971 and #972, and the interlock is what made the order non-negotiable: with
`-e` repaired and the index pattern still quote-spliced, the reachable crates.io verdict becomes a
permanent `Missing:` on a clean publish. The premise itself was upgraded from prediction to observation
(job 124260081872, 245.99 s = exactly one 245 s backoff). 33-case fixture, fail-closed wired into
`validate.sh`, neuter-tested 8/8/2. Rebased onto `4ea6a8e6` clean — #970's only `release.yml` edit is the
`validate` comment at `:92-110`, nowhere near `:370-430` or `:700-705`.

**One required check went red on a style note about a test file.** Codacy's `shellcheck_SC2016` on the
fixture's negative needle (a literal `$(` inside single quotes, intentional) fails the gate at
`issueThreshold: 0`. Re-expressed as needle-plus-positive-control rather than suppressed; `shellcheck
-S warning` and `-i SC2016` both clean, fixture still 33/33.

**#974 filed, then corrected in place** — `verify-release` renders 4 verdicts across 18 published
artifacts (9 crates + 2 npm + 7 release assets), `create-github-release`/`publish-npm-wasm`/`publish-npm-cli`
take no `needs:` on `publish-crates` (`:437`, `:515`, `:577`), and `verify-release` lists it only to
neutralise it with `if: always() && …` (`:647`), so a release missing eight of nine crates certifies green.
Its body shipped with ~60-line-stale citations read from a worktree based on `642238f0` plus a `cs/-m/`
path typo; both re-derived from `origin/main` and PATCHed, then the shipped body diffed against the draft.
Re-measuring its claims also corrected two of my own: the v0.3.9 asset set is `SHA256SUMS` + 5 CLI
tarballs + 1 wasm bundle (the wasm bundle had been counted as a sixth platform in
`release_pipeline_validated`'s comment), and `version_published()`'s two-slice index path is valid only for
names ≥ 4 characters (`3/s/syn`→200 vs its `sy/n/syn`→404).

**State**: three effect keys flipped true (`verify_release_collects_all_verdicts`,
`crates_io_index_checks_match_the_index`, `verify_release_retries_re_query`); #954's block removed;
`queued_actions_count` 11 → **10**, checker `10 action(s), 10 tracked issue(s), 10 effect key(s)`;
`action_last_completed: make_verify_release_report_every_artifact`. #971/#972 never became queued blocks —
filed and landed inside one branch — so the count moves by one, not three. `release_pipeline_validated`
stays **false**: crates.io is 0/9 at 0.3.9 and the token rotation (#967) is owner-only. Next in the
release chain is #955 (npm attestations + drop the `|| npm publish` retry), whose precondition
`verify_release_collects_all_verdicts` this branch just satisfied.

## 2026-10-31 — the queue gate became a rule, and the wait-timer claim died

Docs-only follow-up to the release-refusal diagnosis (`docs/pr-drain-release-block`, off `4ea6a8e6`). Two
findings from the #953/#960 work had no home in the doc set, and one doc set was actively wrong.

**The open-PR block is now operational law.** `pre-release-gate.yml:37-50` `pr-drain-check` `exit 1`s while
any open PR lacks `hold-for-next`/`deferred`, and run 41188203137 refused `bfa15d49` on exactly that
(`2 of 7 gate jobs failed: pr-drain-check, gate-summary`, no tag). Encoded in three places sized to their
budget: a new `github-ci-guardrails/SKILL.md` section ("An open PR blocks the release whose gate runs now",
150/250 LOC), the full fetch-and-drain recipe in `references/gh-ci-truth.md`, and one sentence appended to
`release-management` hard rule 3. It applies to a `workflow_run` release and to a plain dispatch on current
`main` — see the correction below for why a `recover=true` dispatch is exempt.

**A false claim that survived because it lived in four files.** "Default 15 min wait timer on the crates.io
environment" — measured `protection_rules: []` for `crates.io`, `npm` and `npm-cli` (only `github-pages` has
a rule, a `branch_policy`), and `…/pending_deployments` answering a bare `[]`. Repaired in
`agents-docs/release-safety.md` checklist item 6 + failure item 2, in `release-management/SKILL.md`'s
environment section, and in `references/release-workflow.md`'s "Environment wait timer" section and recovery
prerequisite — where `release-management/SKILL.md:197` had already contradicted it. Recovery item 3 also
gained `recovery_tag`, which `release.yml:53-56` hard-requires when `recover=true`: every doc line that
omitted it described a dispatch that cannot run.

**The wave caught its own first draft.** The rule as written above — "one open PR blocks every release, a
`recover=true` dispatch included" — is wrong for the recovery path, and reading `release.yml`'s target
resolution rather than the gate's steps is what showed it: a normal release takes the current head (`:67`) so
its gate run is created now and reads today's queue, while `recover=true` takes
`git rev-list -n 1 "$TAG_NAME"` (`:60`) and queries the newest CI + gate runs for the **tagged** commit. For
`v0.3.9` (`642238f0`) those are 41104878082 and 41110522706, both green since 2026-10-30 — so the owner's
dispatch is **not** queue-blocked, and the "drain before recovering" advice that went with my status message
is withdrawn. The real hazard runs the other way: re-running the tagged commit's CI spawns a fresh gate run
that re-reads today's open-PR list, converting green history into a refusal with one click. This PR still
carries `hold-for-next`, but for the plain-dispatch case that follows it, not to unlock #967.

**Queue comments corrected against the tree, not memory.** #964's block said "three member manifests";
re-measured on `4ea6a8e6` it is **16** stale pins across **9** manifests (root's 8 `path = "crates/…"` pins
are all current, so only member → root and member → sibling edges drift) — matching the issue body I
re-published the same day. #955's block gained the measured endpoint fact: `.dist.attestations` is served
only by the abbreviated packument, and `npm view <pkg> dist --json` omits it.
