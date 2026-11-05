# GitHub Checks (Source of Truth)

Use `gh` CLI:

```bash
gh pr status
gh pr checks --watch
gh run list --branch <branch> --limit 5
gh run view <run-id> --log-failed
```

Rules:
- Do not mark complete if required checks are pending/failing.
- Prefer PR check status over local inference.

## Reading a blocked merge (measured 2026-10-16)

`gh pr checks`'s **default tab-separated rows** carry check names containing spaces
(`Build CLI (linux-x64)`, `Codacy Static Code Analysis`). Parsing that output by whitespace
(`awk '{print $1 $2}'`) silently reads those rows as nameless with no status — which produced a
wrong diagnosis (a superseded run blamed for the block) and a *rerun that itself* put the required
contexts back to `queued`, holding the merge until it finished.

Use these instead:

```bash
# authoritative per-context state for a head (JSON, no parsing hazards)
gh api repos/<owner>/<repo>/commits/<sha>/check-runs \
  --jq '.check_runs[] | "\(.status)\t\(.conclusion)\t\(.name)"'
# the rules that actually gate the merge (branch protection 404 => repository rulesets)
gh api repos/<owner>/<repo>/rulesets --jq '.[] | "\(.id) \(.name) \(.enforcement)"'
gh api repos/<owner>/<repo>/rulesets/<id> --jq '.rules[] | select(.type=="required_status_checks")'
# unresolved review threads, when the ruleset sets required_review_thread_resolution
gh api graphql -f query='{repository(owner:"<o>",name:"<r>"){pullRequest(number:<n>){reviewThreads(first:20){nodes{isResolved path}}}}}'
```

`MERGEABLE/BLOCKED` never says *why*: enumerate the ruleset's requirements instead — every required
context must be `success` (a queued rerun of one is not), the head must be up to date when
`strict_required_status_checks_policy` is on, review threads must be resolved when required, and the
approving-review count must be met. **Re-running a workflow to "clear" a block re-arms the gate**: it
is the right move only when a required context is genuinely absent or stale, never when the reason is
still unknown.

## A `BLOCKED` merge with every required context green (measured 2026-10-17, #909)

The ruleset's non-status rules gate too: `code_scanning` + `code_quality` mean the merge waits for the
CodeQL analyses, so a *pending* `Analyze (rust)` holds the PR at `BLOCKED` after all six required
contexts have gone `success` (it settled ~18 min in). A queued CodeQL job is not a red verdict — check
it before hunting for a code problem.

`gh pr checks --watch` is not a reliable long poll — it exited 1 after ~17 min on
`Post https://api.github.com/graphql: net/http: TLS handshake timeout` while CI itself was fine. Poll
the head's check-runs in a bounded loop (`sleep 60`); a failed call is a retry, never a state.

Version note on the "no `--json`" claim above: gh **2.102.0** does ship `gh pr checks --json fields`
(`bucket`, `state`, `name`, …), and `bucket` categorises `state` into
`pass`/`fail`/`pending`/`skipping`/`cancel` — so `--json name,bucket` is a safe snapshot. The hazard was
always the *default* tab-separated output, not a missing flag; confirm with `gh pr checks --help`.

## Flake, or verdict? Reproducibility decides, and the control may already exist (measured 2026-10-28, #947)

A `lint` failure text of `git-remote-https: … version 'GLIBC_ABI_DT_X86_64_PLT' not found`
inside `nix develop` read as an infra flake (late `exit 1`, network-shaped words) and then as
an `ubuntu-latest` image-migration breakage. Both were wrong; the second was falsified by the
fix's own run — the pinned `ubuntu-24.04` job failed the byte-identical line. Three rules:

1. **A re-run that reproduces the same stage is a verdict, not a flake.** `exit 101` is a
   cargo/clippy verdict; `exit 1` late in a job that compiled cleanly is *environment-shaped*,
   which is a hypothesis, not a classification.
2. **Look for the control before pinning infrastructure.** The same workflow already ran the
   identical devShell and identical `cargo deny --locked check` in another job on the identical
   label — and passed every attempt. The single remaining difference was the wrapper
   (`scripts/run-with-libs.sh` exports `LD_LIBRARY_PATH=<nix-store libs>`), which the host `git`
   CLI cargo-deny spawns inherits. An A/B the repo is already paying for beats a plausible story.
3. **Reject knobs you cannot connect to the evidence.** `deny.toml` `git-via-https = true` was
   applied, then dropped: the logged clone URL was already HTTPS, so an ssh→https switch cannot
   affect it. Read the command line the failure prints, not the docs of the tool you trust.
4. **A job shape that has never completed has no budget evidence.** Clearing the deny failure
   revealed the next one: the same `lint` job was *cancelled at its 30-minute timeout*, because
   the devShell toolchain cannot reuse the host toolchain's `Cache Rust dependencies` entry and
   clippy alone took 13m57s. `green on main` proves nothing about a command main never runs —
   main's `validate.sh` had no `cargo deny` stage at all until this branch unified the gate. When
   a PR moves a gate into a different toolchain, price the timeout from a run of *that* shape.

### Runner-exhaustion signatures, and why `skipped` hides them (measured 2026-11-05, runs 41869624859 + 41847436489)

Two consecutive `main` heads failed on the *same* job name with different environment causes, and the job
list made both look like partial runs — `mutation-shard`/`mutation-test` **skipped**, macOS builds
**cancelled**, because `lint` is their dependency. The causes were visible only in `gh run view --log-failed`:

- `error: failed to build archive at target/debug/deps/libimage-*.rlib: No space left on device (os error 28)` —
  the runner filled its own disk mid-compile. Nothing in the diff can cause this.
- `HTTP error 418` / `GitHub Actions Cache throttled Magic Nix Cache. Not trying to use it again on this run`
  followed by `error: substituter 'http://127.0.0.1:37515' is disabled` — the magic-nix-cache proxy got
  throttled, so the job lost its substitute and rebuilt from source, which is also how it ends up disk-full.

Both were attempt 1 and had never been re-run. Re-running the failed jobs is the diagnostic that *classifies*
the failure, and per rule 1 above, a second identical stage failure is a verdict: a repeated `No space left`
on a push that only touches docs means the workflow needs cleanup/`--no-fallback` cache posture, not that the
code is wrong. Do not open a code fix for a job that never reached the code's assertions.

### A green run on an older head is not a passed check (2026-11-05)

State files that record `ci_all_checks_passed: true` from `gh run list --limit N` will cite whichever recent
run succeeded, not the current tip. The measured gap: the flag cited run 41833935753 (`c9b69ca5`) while the
two newer `main` pushes above it were both red — three heads of drift, and `lint` is a required ruleset
context, so the flag was simultaneously stale *and* implied merges that were unavailable. Always pair the
run id with the SHA and re-derive:

```bash
TIP=$(git rev-parse --short=8 origin/main)
gh run list --workflow=ci.yml --branch main --limit 1 \
  --json headSha,conclusion,databaseId \
  --jq '.[0] | {head:(.headSha[0:8]), conclusion, run:.databaseId}'
# Compare .head to $TIP. Truth only when they match — `--jq` takes a bare filter, so
# `--jq --arg tip "$TIP"` is parsed as a subcommand and errors ("unknown command \"tip\"").
```

## The queue gate: read the refusal, then drain (measured 2026-10-31, runs 41188203137 + 41192441437)

`release.yml` can refuse a perfectly green `main` because `pre-release-gate.yml`'s `pr-drain-check` job
(`:25-50`, step `:37-50`) fails on **any** open PR lacking `hold-for-next`/`deferred`. The release log already
names the job — read it before looking for a quality regression:

```bash
gh run view <release-run> --json jobs -q '.jobs[] | select(.name=="validate") | .databaseId'   # job id
gh api repos/<o>/<r>/check-runs/<job_id>/annotations --jq '.[] | "\(.level) \(.message[0:400])"'
```

Fetch-shape traps measured on that run: `actions/jobs/<id>/annotations` is **404** (the route is
`check-runs/<job_id>/annotations`), and `level` came back `null` while `message` held *both* the
`::error::` text and GitHub's generic "Process completed with exit code 1." — so filter on substring, not
on `level == "error"`.

Two production refusals, one signature. `bfa15d49` (release 41188203137) and `4ea6a8e6` (release
41192441437) each ended `failure` with **no tag created**, and in both the paired gate run (41188203123,
41192441477) reported the same 2-of-7: `pr-drain-check` + `gate-summary` red, the other five green. That
shape *is* the queue. A real gate failure names one check, so read the job list before opening a hunt.

**The verdict belongs to the newest run, not to the commit.** `release.yml` asks
`gh run list --workflow=pre-release-gate.yml --commit "$SHA" --limit 1` — one run, newest first — and reads
only its `status`/`conclusion`. `bfa15d49` was gate-green at 08:49Z (41186134728) and gate-red at 09:19Z
(41188203123) with no code change in between; the earlier green never ran the drain check at all, because the
upstream CI attempt behind it had concluded failure and the `if:` skipped every real job. So "this
commit already passed" is not evidence a *new* dispatch will pass, and a refusal that lands minutes after a
merge is not a regression.

**A green run can have verified nothing — in both workflows.** `pre-release-gate.yml:27-30` and
`release.yml:34-38` carry the same two-clause condition: `workflow_run.event == 'push'` **and**
`workflow_run.conclusion == 'success'`. It is the conclusion clause that bites. CI run 41184146253 on
`bfa15d49` was a `push` whose **attempt 1 concluded failure** (self-hosted runner: `No space left on device`
writing `_diag/Worker_20261031-081949-utc.log`), and that 08:49:08Z completion lit *two* green runs at once:
gate 41186134728 (six of seven jobs `skipped`, only `gate-summary` ran) and release 41186134783 (`validate`
skipped, so all seven substantive jobs skipped and only `notify`, an `always()`, ran). The re-run — attempt 2,
`completed/success` at 09:19:08Z — then produced the real gate run 41188203123, which executed the drain check
and refused. `release.yml`'s gate read consumes run-level `status`/`conclusion` alone, so it accepts the first
kind. Read the per-job shape before trusting either verdict:

```bash
gh api "repos/<o>/<r>/actions/runs/<gate_run>/jobs?per_page=100" \
  --jq '"jobs=\(.jobs|length): " + ([.jobs[] | .name + "=" + (.conclusion // "-")] | join(", "))'
```

The recovery target passes that test — `41110522706` for `642238f0` is seven-for-seven `success` including
`pr-drain-check` — so the exemption below rests on a real green, not a vacuous one.

A third quiet green belongs here because it is the one an operator meets first: when `v{version}` already
exists, `validate` sets `release-needed=false` and every downstream job skips while the run still concludes
`success` — measured on release 41195178865 (`14687660`), green and publishing nothing. A green *release* run
is therefore not evidence a registry has the artifact; only `verify-release`'s rows, or the registries
themselves, answer that. The release-side defect (a run that skips everything and still reports success when
the upstream CI did not conclude `push`/`success`) is filed as **d-o-hub#969** and is *not* fixed by anything
in this doc set — read that issue before concluding the workflow is healthy.

Then, and only as the release's own step (never silently, never as a way to keep a red PR open):

```bash
gh api "repos/<o>/<r>/labels?per_page=100" --jq '.[].name' | grep -E 'hold-for-next|deferred'  # default page = 30 of 53
gh pr list --state open --json number,title,labels \
  --jq '.[] | select([.labels[].name] | any(. == "hold-for-next" or . == "deferred") | not) | "\(.number) \(.title)"'
gh api -X POST "repos/<o>/<r>/issues/<n>/labels" -f labels[]=hold-for-next   # additive; PUT replaces the set
```

`hold-for-next` = "Defer to next release", `deferred` = "Deferred indefinitely" — both satisfy the gate;
the difference is release-note intent, not mechanics.

Scope the drain to the trigger that needs it. `release.yml` resolves the target SHA two ways: a
`workflow_run` release or a plain dispatch takes
`github.event.workflow_run.head_sha || github.sha` (the current head, `:67`), while `recover=true` takes
`git rev-list -n 1 "$TAG_NAME"` (the **tagged** commit, `:60`). The gate check then asks for the newest
`CI` run and the newest `pre-release-gate` run *for that SHA* to be `completed/success`. Measured on the live
recovery target: `v0.3.9` → `642238f0a2678e55c4931500402c671fa2aa02c5`, CI `41104878082` and gate
`41110522706`, both green since 2026-10-30 — so `recover=true recovery_tag=v0.3.9` needs **no** drain, and
telling an operator to empty the queue first is wrong. Verify the historical pair before promising either:

```bash
git fetch --tags --force
SHA=$(git rev-list -n 1 v0.3.9)   # same resolution release.yml uses; ls-remote would give the tag object
gh run list --workflow=ci.yml --commit "$SHA" --limit 1 --json conclusion,headSha
gh run list --workflow=pre-release-gate.yml --commit "$SHA" --limit 1 --json conclusion,headSha
```

The one way a recovery *can* be blocked by the queue is self-inflicted: re-running the tagged commit's CI
fires `workflow_run` again, which creates a new gate run for that SHA and evaluates today's open-PR list.
Green history is a single re-run away from a refusal — do not re-run CI for a tag you are about to recover.

## A Dependabot red: an inherited failure and a fatal one in the same PR (measured 2026-11-03, #988)

Two failing checks on a lockfile-only PR are usually two different causes, and only one is the bump.
#988 (fuzz `uuid` 1.26.1 → 1.29.0) reported `lint` red on the stale #965 queue block — **inherited**,
already fixed on `main` by #990, so the rebase clears it — and `Fuzz Workspace Build` red on
`error: rustc 1.88.0 is not supported by the following package: uuid@1.29.0 requires rustc 1.89.0`
(job 125320036721). Check the repo's pinned `rust-toolchain.toml` before calling a dependency PR flaky:
that pin turns every upstream `rust_version` into a hard gate the branch cannot fix.

Measure the ceiling from the registry index, not from a release note:

```bash
curl -s https://index.crates.io/uu/id/uuid | tail -5
# 1.27.0 / 1.28.0 / 1.29.0 declare rust_version 1.89.0; 1.26.1 declares 1.85.0
```

Then pick one: bump the MSRV in its own PR, or (the cheaper default) add a commented `ignore` for that
dependency's **minor** updates to `.github/dependabot.yml` — `update-types: ["version-update:semver-minor"]`
keeps patch releases flowing — and close the PR with the job id in the comment. Record the ceiling and the
removal condition in the config itself, since the ignore will outlive this conversation.

A local probe does **not** clear this class: `scripts/check-fuzz-lock.sh` runs `cargo metadata --locked`,
which reads an MSRV-incompatible lock without complaint (measured rc=0 on #988's tree). The gate is the
compiling job, `cargo check --manifest-path fuzz/Cargo.toml --all-targets --locked`.

## After green: land and verify

1. `git fetch origin main` — the branch base must equal current `main`.
2. `mergeStateStatus: CLEAN` — not just `MERGEABLE`.
3. `git diff --stat origin/main...<branch>` — intended files only, no surprise reverts.
4. `gh pr merge <n> --squash --delete-branch` (never `--auto`).
5. Verify the landing: `git ls-remote origin 'refs/heads/<branch>'` → 0 refs;
   `git log --oneline -1 origin/main` → the squash subject.
