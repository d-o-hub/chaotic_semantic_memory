# Local Gates

Run before pushing:

```bash
cargo check
cargo test --all-features
cargo fmt --check
cargo clippy -- -D warnings
cargo deny check                    # Supply chain audit (advisories, bans, licenses)
```

## LOC Gate (workspace-wide)

```bash
find src crates -name '*.rs' -not -path '*/target/*' -exec wc -l {} + | sort -rn | head -20
# Every file must be ≤ 500 LOC — applies to BOTH src/ and crates/
```

## Commitlint

When adding new workspace crates or package scopes, update `commitlint.config.cjs`:
- Add the scope name to `scope-enum` array
- Valid scopes: singularity, reservoir, framework, persistence, cli, cli-npm, wasm,
  retrieval, embedding, mcp, observability, bridge, duckdb, chaos, memory, core,
  traits, deps, ci, codacy, docs, release, clippy, lints, build, loc-gate, workspace

**Always validate the full PR range** (CI does this; last-commit-only is insufficient):

```bash
npx commitlint --from origin/main --to HEAD --verbose
```

Do not invent scopes (`ops`, `plans`, `goap` unless added to the enum first).

## Hook bootstrap

On a fresh checkout, run `scripts/install-hooks.sh` before strict validation.
Keep `scripts/validate.sh` read-only for hook configuration. Run
`scripts/test-hook-bootstrap.sh` to prove real Git invocation in a clone,
parent and linked worktree; checking a nonempty `core.hooksPath` is insufficient.
Use relative repo-local `.githooks`, and check both local and effective paths.
Pre-push uses GNU `timeout`/`gtimeout` with a configurable 180-second budget.

Correct PR title/body before pushing a repaired commit. Actions snapshots PR
metadata in the event; rerunning an old event can retain the invalid old title.

## `Cargo Deny` can go red repo-wide with no diff involved (2026-10-21, #928)

`main` was green at 18:05Z; at 06:12Z `Cargo Deny` was red on **every** open PR (#926, #927). The
advisory DB had published RUSTSEC-2026-0381/0382 overnight. Diagnose before touching any PR:

```bash
gh run list --workflow=ci.yml --branch main --limit 4 --json headSha,conclusion,updatedAt
gh pr checks <n> | grep -i deny          # red on all PRs + green on main ⇒ DB event, not a diff
```

**Read the advisory's `[versions]` block, including `unaffected`.** `webpki-roots 0.26.11` looked
unfixable — it is the last release of that line, pinned by `ureq 2.12.1` via `hf-hub`/`ort-sys`
(both through `fastembed`), so lifting it means a major `hf-hub` jump plus a deeper `-rc` pin. The
advisory answered for free:

```toml
patched = [">= 1.0.10"]
unaffected = ["= 0.26.11"]     # that release never bundled the emSign root
```

That line landed at 06:20:38Z — **eight minutes after** the CI run that flagged it. Compare the local
DB timestamp to the run's before concluding a finding is real:
`git -C ~/.cargo/advisory-db log -1 --format='%h %ci %s'`. Findings expire upstream.

**Prove the fix is attributable, both directions.** A local green can mean "my DB is stale", and a
local green can mean "my graph is narrower than CI's" (this run emitted
`warning[advisory-not-detected]` for three pre-existing ignores whose crates CI does see). Run the
check against the *pre*-change lock and the *post*-change lock:

```bash
git show <base>:Cargo.lock > /tmp/pre.lock   # or a worktree at the base SHA
cargo deny check advisories                  # must reproduce the failures, exit 1
cargo update -p <crate> --precise <patched>
cargo deny check                             # advisories, bans, licenses, sources ok, exit 0
```

**Do not reach for `deny.toml` when a patch bump exists.** Ignores are keyed by advisory **id**, not
crate@version, so ignoring a partially-fixable advisory also silences the *fixable* copy for the whole
tree, permanently. Additive-only ignores stay reserved for a genuinely pinned line with no patched
release — the `h2 0.3.27` / `time 0.3.45` / `lru 0.12.5` pattern already in that file.

## A green `cargo deny` is not proof the lock diff is minimal (#928, 2026-10-21)

`cargo update -p rustls-webpki --precise 0.103.16 -p webpki-roots --precise 1.0.10` re-solved more
than it was asked: the commit touched `Cargo.lock` in 17 hunks, 9 of them `windows-sys 0.52.0 →
0.59.0` edges on packages neither bumped crate constrains. Same class as the resolver churn #889 was
roasted for, and it passed every gate.

Measure the diff before you publish it — the intended change has a countable shape (per bumped
package: one `version` line, one `checksum`, plus its incoming dependency edges):

```bash
git diff --stat HEAD~1 -- Cargo.lock      # 17/17 here, 8/8 once pruned
git diff HEAD~1 -- Cargo.lock | grep -c '^\+.*windows-sys'
```

Churn vs forced is checkable, not a judgement call: hand-prune the unrelated hunks back to their
previously locked versions and re-validate. If the gates accept the smaller lock, the resolver was
not forced and the extra edges are noise:

```bash
cargo metadata --locked --format-version 1 >/dev/null   # lock <-> manifests still consistent
cargo deny check                                       # all four checks ok
grep -c '"windows-sys 0.59.0"' Cargo.lock              # both versions still referenced => keep both
```
