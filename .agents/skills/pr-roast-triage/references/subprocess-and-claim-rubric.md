# Rubric detail: subprocess tests, async tests, mutation exclusions, API widening

Extracted from `SKILL.md` (250-LOC skill cap) on 2026-10-05, from the #834/#836
SIGTERM duplicate. All four are readable off a diff without running anything.

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
   the test suite is running in. Non-hermetic, and CI never tells you.

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

## A mutation exclusion must be provable, or it is a fake decision

`--exclude` / `--exclude-re` in `scripts/mutation_test.sh` match **item paths or
names** — the file's own entries are `"run_watch"`, `"McpHandler::"`,
`"src/mcp/*"`. `--exclude-re "replace shutdown_signal"` therefore matches nothing:
no Rust item path contains the word "replace", and the string is plainly copied out
of a mutant *description* (`replace shutdown_signal -> ...`). A dead exclusion is
worse than none, because the next reader believes the module is deliberately waived.

Prove an exclusion by generating the mutant it claims to skip and confirming it is
absent from the report. cargo-mutants is not installed in every dev workspace, so a
reviewer may have to read `--help` output in CI rather than locally — say which you
did, and do not sentence a line you could not measure.

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
