# ADR-0099: TTL Cleanup Task Lifecycle and Bounded Shutdown

## Status

Accepted (2026-10-01)

## Context

`ChaoticSemanticFramework` can spawn an opt-in background task that purges
expired concepts when `ttl_config.cleanup_interval_seconds > 0`
(`src/framework_builder.rs`). The task is a process-level side effect over
shared state, and every clone of a framework shares it. The pre-2026-10
implementation stored `Option<Arc<tokio::task::JoinHandle<()>>>` and stopped
the task from `Drop` with `handle.abort()`, which produced two defects
(re-verified 2026-09-30 against the 2026-07-14 audit finding F2):

- **F1 — dropping any clone killed cleanup for all of them.** The handle was
  shared by `Clone::clone` and `clone_with_namespace`, so the first dropped
  clone aborted the single task while other clones still expected it to run.
  The `Arc<JoinHandle>` also made "how many clones exist" silently double as
  task ownership.
- **F2 — no cancellation path and no bounded stop.** There was no
  cancellation token, no await path, and the task held a strong framework
  clone, so nothing but `abort()` could ever end it. `Drop` cannot await, so a
  graceful stop was impossible by construction, and no test exercised
  termination — every existing test configured `cleanup_interval_seconds: 0`.

ADR-0024 recorded that "cancellation/JoinHandle ownership and bounded shutdown
are proposed in ADR-0093"; ADR-0093 contains no such prescription, so the
lifecycle was in fact unspecified. This ADR closes that gap.

## Decision

1. One task per framework, owned by a shared
   `CleanupTask { cancel: watch::Sender<bool>, handle: Mutex<Option<JoinHandle<()>>> }`
   stored as `Option<Arc<CleanupTask>>` on native targets. `wasm32` has no
   background task and carries no handle.
2. **Cancellation is cooperative** (`tokio::sync::watch`), never `abort()`:
   clones share one task, so no single `Drop` may terminate it for the others.
   `tokio-util`'s `CancellationToken` was rejected to avoid a new dependency for
   one boolean.
3. **The loop stops by itself when the last handle is dropped.** The task works
   on a framework clone whose `cleanup` field is `None`, so it never keeps its
   own stop handle (or the watch sender) alive; the sender's disappearance ends
   the loop. This replaces `abort()` as the leak guard.
4. **`Drop` requests a cooperative stop only from the last live handle**
   (`Arc::strong_count == 1`). It cannot await, so it does not; the task exits at
   its next cancellation check.
5. **`ChaoticSemanticFramework::shutdown()`** cancels and awaits the task within
   a five-second bound. It is idempotent, callable on any clone (stopping the
   shared task for all of them), and reports a panicked or stuck task as
   `MemoryError::External` instead of hiding it. On `wasm32` it is a no-op so the
   public API stays identical across targets.
6. Existing semantics are unchanged: interval `0` means no task, the first tick
   is immediate, and a failed purge is logged (`error!`) and retried on the next
   tick.
7. **Every server transport must reach `shutdown()`, which means every transport
   needs an exit.** A transport that never resolves makes the hop dead code on
   that path, so the lifecycle guarantee is only as good as the transport's exit.
   `rmcp`'s stdio transport ends on stdin EOF (`Waiting::waiting`), so
   `Transport::Stdio` needed nothing. axum 0.7.9's SSE transport does:
   `Serve::into_future` is an unbounded accept loop and `tcp_accept` reports
   failures as `Option` — connection errors skipped, anything else logged and
   retried after a sleep — so it never yields an `Err` the caller can observe and
   no request or client disconnect can end it. The decision is therefore
   `Serve::with_graceful_shutdown(signal)` with `signal: Future<Output = ()>`:
   `mcp::serve` supplies the operator's Ctrl+C and is exactly
   `mcp::serve_with_shutdown(config, ctrl_c_shutdown())`, so the embedder-facing
   entry and the CLI share one mechanism rather than the test owning a seam.
   Two consequences of that shape are deliberate: if the SIGINT handler cannot be
   installed the signal `pending()`s instead of resolving, because a failed
   handler install must not read as "shutdown requested" and tear down a server
   that just started; and the signal drives only the SSE arm, so stdio keeps the
   exit path it already had rather than racing two.

## Consequences

- Dropping a clone no longer stops cleanup; dropping the last handle stops it
  cooperatively, and an explicit `shutdown()` gives a bounded, awaitable stop.
- A stuck or panicked cleanup task is visible to the caller rather than silent.
- Per-clone tasks were rejected: N clones would mean N tasks purging the same
  store, with duplicated error logs and wasted wakeups.
- Requiring callers to `shutdown()` before `Drop` was rejected: a forgotten call
  would leak a running task (the pre-2026-10 failure mode the abort was hiding).

## Verification

Tests in `tests/test_advanced_ttl.rs`:

- `background_cleanup_runs_until_shutdown` — with a 1s interval the task purges
  an expired concept without any explicit purge call, `shutdown()` stops it
  (idempotently), and a concept expiring afterwards is no longer purged.
- `dropping_a_clone_keeps_the_cleanup_task_running` — regression for F1: it
  fails on the `Arc<JoinHandle>` + `abort()` design and passes on this one.

Server exit paths (decision 7) — `src/mcp/server_tests.rs`, plus
`tests/mcp_sse_integration.rs`:

- `sse_exit_stops_the_ttl_cleanup_task` — binds the SSE transport on an
  ephemeral port, confirms readiness by a successful connect (no sleep), sends
  the shutdown signal, **awaits** `serve_with_handler`, and then asserts the
  cleanup task's handle slot is empty. Before decision 7 the awaited transport
  never resolved, so this test could not exist.
- `sse_exit_is_ok_without_a_request` — an SSE server that never served a request
  has no framework, so the exit path must return `Ok(())` instead of building one
  just to stop it. Covers the caller;
  `test_handler_shutdown_is_a_noop_without_a_framework` in
  `src/mcp/tools_tests.rs` covers the callee.
- `test_sse_transport_lifecycle` (`tests/mcp_sse_integration.rs`) drives the real
  HTTP handshake and now exits by signal + await. Its previous shape —
  `tokio::spawn(serve(..))` then `handle.abort()` — is the reason decision 7 was
  found a month late: an aborted task observes nothing after the awaited
  transport, so it cannot see that the transport never returns.

## References

- ADR-0024 (concept expiration / TTL), ADR-0026 (namespace isolation)
- `src/framework.rs` (`CleanupTask`, `shutdown`, `Drop`),
  `src/framework_builder.rs` (spawn), `src/framework_namespaces.rs` (clone)
