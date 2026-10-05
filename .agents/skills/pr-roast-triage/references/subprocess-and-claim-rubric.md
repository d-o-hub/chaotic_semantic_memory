# Rubric detail: subprocess tests, async tests, mutation exclusions, API widening

Extracted from `SKILL.md` (250-LOC skill cap) on 2026-10-05, from the #834/#836
SIGTERM duplicate. The first four are readable off a diff without running anything;
the fifth was only reachable by letting CI fail on my own PR.

## A subprocess test must be bounded and hermetic (#834)

Three checks a green CI cannot make for you:

1. **Every wait on a spawned child has a deadline.** `child.wait()` after
   `kill -TERM` converts a functional regression into a job-length hang, which reads
   as infrastructure rather than as a failed assertion. Correct shape: poll
   `try_wait()`, fail at a deadline comfortably above the production bound
   (`EXIT_DEADLINE` 15 s vs the 5 s internal `shutdown()`), then kill-and-report.
2. **Readiness is synchronised on output, not on a sleep.** `sleep(200ms)` with the
   comment "give it a moment to initialize signal handlers" has two silent failure
   modes: too short and the signal lands before the handler installs (exit 143, red
   for the wrong reason); too long and a loaded runner is flaky. The child prints a
   line at exactly the moment in question — `Press Ctrl+C to stop.`, `MCP SSE server
   listening on http://…` — so wait for that line, then settle.
3. **A `csm` spawn needs an explicit `--database`.** With none,
   `src/cli/args.rs:20` falls back to git-local storage, so `csm watch` /
   `csm mcp serve` create and write `.git/memory-index/csm.db` inside the checkout
   the test suite is running in. Non-hermetic, and CI never tells you. **Reproduced** in a
   throwaway `git init` repo: nothing exists under `.git/memory-index` before the run, and
   `.git/memory-index/csm.db` is present after `csm watch` + SIGTERM.

Related: dropping the read end of a child's stderr makes the child panic with exit
101 on its next `eprintln!` (EPIPE), which the harness then blames on the production
handler. Drain stderr on a dedicated thread for the child's whole lifetime; that
also removes the full-pipe deadlock that reads as a hang. See
`progress/LEARNINGS.md` 2026-10-04.

## An `async` test that never awaits is a compile check wearing a `#[test]`

`shutdown_signal_compiles` (#834) had one body line:

```rust
let _fut = shutdown_signal();
```

Rust `async fn` calls are lazy — constructing the future polls nothing, installs no
handler, asserts only that the crate compiles, which the build already asserts for
free. The registered test count still goes up by one, so the ratio gate is satisfied
by a statement with no behaviour in it. `scripts/check-test-attributes.sh` verifies
that test-shaped functions are *registered*, not that they *do* anything — it cannot
catch this. Ask of every async test: is the future awaited, or merely named?

Worse in combination: the same PR excluded that very file from mutation testing
(`--exclude "src/cli/shutdown.rs"`). A vacuous test plus an exclusion is the opposite
of evidence, even though each looks defensible alone.

## Measure a mutation exclusion; do not read it

**This section first shipped a wrong claim, and the correction is the lesson.** The
original text asserted that `--exclude-re "replace shutdown_signal"` (added by #834)
"matched nothing, because `--exclude-re` takes item names and no Rust path contains the
word `replace`." It does match. The belief came from reading a `tail`-truncated
`cargo mutants --list` output as an experiment: the truncated view looked like "nothing
was excluded," when in fact one mutant had been removed from a list I could not see the
whole of.

Measured with counts on `cargo-mutants 27.1.0`, against this repo's `src/shutdown.rs`
(baseline `--list` = 3 mutants: `operator_shutdown`, `sigint`, `sigterm`):

| `--exclude-re` pattern | mutants listed | conclusion |
|---|---|---|
| `"replace operator_shutdown"` | 2 | the pattern set is the mutant **description**, so `replace <fn>` is a valid form |
| `"operator_shutdown"`, `"sigterm"`, `"replace sigint"` | 2 | ditto |
| `"shutdown::"` | 3 | **module paths do not match** — the inverse of the original claim |
| `"zzz_no_match"` | 3 | required control: a non-matching pattern must change nothing |

Cross-check the pre-existing entries rather than assuming they are alive:
`src/mcp/handler.rs` baselines at 12 and `--exclude-re "McpHandler::"` leaves 8, so that
entry does real work (descriptions carry impl-qualified names such as
`McpHandler::read_resource`); a bare `"McpHandler"` takes it to 2.

The method, not the numbers, is the rule: **run `--list` with and without the pattern,
count lines, and include a control pattern that must not match.** Four commands, no
compilation, and they settle a claim that reading the flag's help text got backwards.

What #834 actually did with those two lines is worse than a dead flag, and was only
visible after the correction: `--exclude-re "replace shutdown_signal"` is redundant with
the `--exclude "src/cli/shutdown.rs"` directly under it, and the module needs excluding
only because its behavioural tests live in `tests/cli_integration.rs` — which the fast
profile never runs (`scripts/mutation_test.sh:142` passes `--lib -p csm-retrieval -p
chaotic_semantic_memory`). `replace shutdown_signal with ()` is therefore genuinely
unkillable in the profile that gates: the in-profile unit test is
`let _fut = shutdown_signal();`, which passes unchanged under the mutant because
constructing a future polls nothing. Excluding the file turns a placement mistake into
policy. Ask instead: put the kill-capable test where the gate can see it.

## Widening public API for an internal need is a cost, counted in `llms.txt`

A `pub mod shutdown` + `pub use shutdown::shutdown_signal` for a process-signal park
is unreachable by library users and shows up as `+6` generated lines in `llms.txt`
plus `+29` in `llms-full.txt`. `pub(crate)` behind the existing target gate shows up
as nothing. Diff `llms.txt` on any PR that adds a module: it is the public-API
tripwire the crate already maintains for you.

The gating detail that belongs with it: `signal` is a *target-gated* tokio feature
(`Cargo.toml:197-202`, non-wasm32 only, because it pulls `mio`/`net`), so a new
signal listener must sit inside the same non-wasm target table. Nothing in CI checks
the root crate for `wasm32` with `cli` enabled — `ci.yml:539` only checks `-p
csm-wasm`, and `pre-release-gate.yml` (which does check `--features wasm`) has no
caller — so an ungated module there is an unverified risk, not a proven break.

## A subprocess test cannot feed the `--lib` mutation gate (#836, against myself)

The rubric above was applied to #834 while the same defect sat unexamined in #836. CI run
37276860430 (head `0107c9c`) scored `0.0000%` with all three in-diff mutants of
`src/shutdown.rs` surviving — `replace operator_shutdown / sigint / sigterm with ()` —
even though `tests/cli_shutdown_signal.rs` asserted the exact regression with a
neuter-proven red. Reason, from `scripts/mutation_test.sh:142`: the fast profile runs
`cargo mutants --lib -p csm-retrieval -p chaotic_semantic_memory`, so `tests/**` is not
built and cannot kill anything.

Two rules follow:

1. **Read which targets the profile executes before claiming a change is covered.**
   Assertion sharpness and harness reachability are independent properties; a perfect test
   in an unseen target scores zero. Ask "can a mutant of the changed line be killed by a
   test the *gate* runs", not "does my test catch it".
2. **Put one assertion per mutant in the mutated target.** `src/shutdown_tests.rs` checks
   each arm stays pending with no signal, at a timeout the correct code can never exceed.
   Verify per-mutant, not per-suite: delete one function body, run, confirm which tests
   fail, restore, `cmp` clean. Reproduced this way the failing sets are diagnostic —
   `operator_shutdown` kills 1 test, `sigint` and `sigterm` each kill 2 (their arm plus
   the composite). A single broad assertion would have passed the gate while telling a
   future reader nothing about which arm broke.

Not covered on purpose: proving "a real SIGTERM resolves it" inside `cargo test --lib`
means `raise()`-ing a process-wide signal into the shared libtest binary. On a mutant that
skips handler installation the entire test process dies by default disposition instead of
one assertion failing. The exit-status claim stays process-level, one child per signal,
bounded by a deadline.
