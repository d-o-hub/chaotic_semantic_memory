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

## The queue gate: read the refusal, then drain (measured 2026-10-31, run 41188203137)

`release.yml` can refuse a perfectly green `main` because `pre-release-gate.yml`'s `pr-drain-check`
(`:37-50`) fails on **any** open PR lacking `hold-for-next`/`deferred`. The release log already names the
job — read it before looking for a quality regression:

```bash
gh run view <release-run> --json jobs -q '.jobs[] | select(.name=="validate") | .databaseId'   # job id
gh api repos/<o>/<r>/check-runs/<job_id>/annotations --jq '.[] | "\(.level) \(.message[0:400])"'
```

Fetch-shape traps measured on that run: `actions/jobs/<id>/annotations` is **404** (the route is
`check-runs/<job_id>/annotations`), and `level` came back `null` while `message` held *both* the
`::error::` text and GitHub's generic "Process completed with exit code 1." — so filter on substring, not
on `level == "error"`.

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

## After green: land and verify

1. `git fetch origin main` — the branch base must equal current `main`.
2. `mergeStateStatus: CLEAN` — not just `MERGEABLE`.
3. `git diff --stat origin/main...<branch>` — intended files only, no surprise reverts.
4. `gh pr merge <n> --squash --delete-branch` (never `--auto`).
5. Verify the landing: `git ls-remote origin 'refs/heads/<branch>'` → 0 refs;
   `git log --oneline -1 origin/main` → the squash subject.
