#![cfg(all(feature = "cli", unix))]
//! Long-running commands must stop cooperatively on both operator signals (ADR-0099).
//!
//! Asserted against the real binary because the observable is the exit status: a
//! cooperative stop returns from the command's own run loop (`run_watch`, or
//! `mcp::serve` after its transport resolves), awaits the bounded `shutdown()`, and
//! exits `0`. An unhandled SIGTERM kills the process by default disposition instead,
//! which surfaces as `signal: 15` (shell exit `143`).
//!
//! This is why the assertion is not a unit test with an injected future:
//! `src/mcp/server_tests.rs` resolves a synthetic signal to prove the plumbing, and
//! that test stayed green while SIGTERM was missing from the process entirely. Only
//! the exit code distinguishes "the handler was installed" from "the handler exists
//! and was never registered".
//!
//! Run with: `cargo test --test cli_shutdown_signal --features cli,mcp`

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::BufRead;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

/// How long to wait for the child to announce itself at all.
const READY_DEADLINE: Duration = Duration::from_secs(10);

/// How long to let the child settle after it announces itself, so the signal is
/// delivered after the handler is registered. The child installs the handler within
/// microseconds of printing this line, so this bound is not what makes the test slow.
const READY_SETTLE: Duration = Duration::from_millis(500);

/// Upper bound on a cooperative stop. `shutdown()` is itself bounded to 5s, so 15s
/// cannot fail for a legitimate reason.
const EXIT_DEADLINE: Duration = Duration::from_secs(15);

/// What each long-running command prints once it is serving and about to park on
/// its signal handlers.
const WATCH_READY: &str = "Press Ctrl+C to stop.";
#[cfg(feature = "mcp")]
const SERVE_READY: &str = "MCP SSE server listening on";

/// `--database` is mandatory for every command here: with none given, `csm` falls
/// back to git-local storage and a test would write into the real repository.
fn csm(db: &Path, args: &[&str]) -> Child {
    Command::new(env!("CARGO_BIN_EXE_csm"))
        .arg("--database")
        .arg(db)
        .args(args)
        // A live reaper makes the cooperative stop meaningful: the shutdown hop has
        // a real background task to stop rather than a `None` handle to skip.
        .args(["--ttl-cleanup-interval", "1"])
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn csm")
}

fn spawn_watch(db: &Path) -> Child {
    csm(db, &["watch"])
}

/// `csm mcp serve --transport sse` — the ADR-0099 hop this issue was filed about.
#[cfg(feature = "mcp")]
fn spawn_serve(db: &Path) -> Child {
    Command::new(env!("CARGO_BIN_EXE_csm"))
        // tracing sits at ERROR by default; two `-v` raise it to INFO so the
        // readiness line below is emitted at all.
        .args(["-vv", "mcp", "serve", "--transport", "sse"])
        // Port 0 lets the kernel choose, and the server logs the address it really
        // bound, so there is no port to reserve and no reservation to collide with.
        .args(["--bind", "127.0.0.1:0"])
        .arg("--database")
        .arg(db)
        .args(["--ttl-cleanup-interval", "1"])
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn csm mcp serve")
}

/// Drain the child's stderr on a dedicated thread.
///
/// Two reasons: leaving the pipe unread would let a chatty child block on a full
/// buffer (which reads as a hang, not a failure), and stopping at the ready line
/// would discard the panic message if the child dies before it gets there — the
/// single most useful thing to print when this test goes red.
fn drain_stderr(child: &mut Child) -> std::sync::Arc<std::sync::Mutex<Vec<String>>> {
    use std::sync::{Arc, Mutex};
    let lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&lines);
    let stderr = child
        .stderr
        .take()
        .expect("csm was spawned with a piped stderr");
    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(line) => sink.lock().expect("stderr sink poisoned").push(line),
                Err(_) => break,
            }
        }
    });
    lines
}

fn logged(lines: &std::sync::Arc<std::sync::Mutex<Vec<String>>>, needle: &str) -> bool {
    lines
        .lock()
        .expect("stderr sink poisoned")
        .iter()
        .any(|line| line.contains(needle))
}

/// Wait until the child announces that it is up — i.e. until the point just before
/// it registers its signal handlers — then let it settle.
fn wait_until_logged(
    child: &mut Child,
    needle: &str,
) -> std::sync::Arc<std::sync::Mutex<Vec<String>>> {
    let lines = drain_stderr(child);
    let deadline = Instant::now() + READY_DEADLINE;
    while !logged(&lines, needle) {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("csm never logged {needle:?}; stderr was:\n{}", dump(&lines));
        }
        sleep(Duration::from_millis(20));
    }
    sleep(READY_SETTLE);
    lines
}

fn dump(lines: &std::sync::Arc<std::sync::Mutex<Vec<String>>>) -> String {
    lines.lock().expect("stderr sink poisoned").join("\n")
}

fn send_signal(child: &Child, signal: &str) {
    // `kill` rather than a libc binding: the point is the real OS disposition, and
    // no signal dependency is otherwise in the tree.
    let status = Command::new("kill")
        .arg(format!("-{signal}"))
        .arg(child.id().to_string())
        .status()
        .expect("failed to run kill; is procps installed?");
    assert!(status.success(), "`kill -{signal}` failed");
}

fn wait_for_exit(child: &mut Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + EXIT_DEADLINE;
    loop {
        match child.try_wait().expect("failed to wait on csm") {
            Some(status) => return status,
            None if Instant::now() >= deadline => {
                child.kill().expect("failed to kill a hung csm process");
                let _ = child.wait();
                panic!(
                    "csm did not exit within {EXIT_DEADLINE:?} of the signal; \
                     the shutdown is not cooperative"
                );
            }
            None => sleep(Duration::from_millis(50)),
        }
    }
}

/// Drive one signal end to end and return the exit status with the child's stderr.
fn stop_with(
    spawn: fn(&Path) -> Child,
    ready: &str,
    signal: &str,
) -> (std::process::ExitStatus, String) {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let db = dir.path().join("shutdown-signal.db");
    let mut child = spawn(&db);
    let lines = wait_until_logged(&mut child, ready);
    send_signal(&child, signal);
    let status = wait_for_exit(&mut child);
    let stderr = dump(&lines);
    // Keep `dir` alive until the child is reaped, then drop it.
    drop(dir);
    (status, stderr)
}

/// The assertion every command/signal pair shares: exit 0 is only reachable if the
/// process noticed the signal and returned from its own run loop, so a missing
/// handler cannot pass by accident of the default disposition.
fn assert_cooperative(label: &str, signal: &str, status: std::process::ExitStatus, stderr: &str) {
    assert!(
        status.success(),
        "SIG{signal} must stop `{label}` cooperatively (exit 0), but it exited as \
         {status:?} — signal {:?} means no SIG{signal} handler was installed, so ADR-0099's \
         bounded TTL-cleanup stop never ran and a service-managed server is killed \
         rather than stopped.\n--- {label} stderr ---\n{stderr}",
        status.signal(),
    );
    assert_eq!(
        status.code(),
        Some(0),
        "SIG{signal} must exit `{label}` with code 0, got {status:?}.\nstderr:\n{stderr}"
    );
}

#[test]
fn watch_stops_cooperatively_on_sigterm() {
    let (status, stderr) = stop_with(spawn_watch, WATCH_READY, "TERM");
    assert_cooperative("csm watch", "TERM", status, &stderr);
    assert!(
        stderr.contains("Interrupted."),
        "the signal arm must actually break the watch loop; stderr was:\n{stderr}"
    );
}

#[test]
fn watch_stops_cooperatively_on_sigint() {
    let (status, stderr) = stop_with(spawn_watch, WATCH_READY, "INT");
    assert_cooperative("csm watch", "INT", status, &stderr);
}

/// The transport `#821` was filed about: without a shutdown signal the axum accept
/// loop never yields, so before that change nothing downstream of
/// `with_graceful_shutdown` ran on either signal.
///
/// Scope note, stated rather than glossed: no MCP request is issued here, so the
/// handler's framework is never initialized and `handler.shutdown()` has no reaper
/// to stop. This test pins the *process exit path*; `src/mcp/server_tests.rs` pins
/// the shutdown hop itself.
#[cfg(feature = "mcp")]
#[test]
fn serve_sse_stops_cooperatively_on_sigterm() {
    let (status, stderr) = stop_with(spawn_serve, SERVE_READY, "TERM");
    assert_cooperative("csm mcp serve --transport sse", "TERM", status, &stderr);
}

#[cfg(feature = "mcp")]
#[test]
fn serve_sse_stops_cooperatively_on_sigint() {
    let (status, stderr) = stop_with(spawn_serve, SERVE_READY, "INT");
    assert_cooperative("csm mcp serve --transport sse", "INT", status, &stderr);
}
