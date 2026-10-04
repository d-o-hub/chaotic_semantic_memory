#![cfg(all(feature = "cli", unix))]
//! Long-running commands must stop cooperatively on both operator signals (ADR-0099).
//!
//! Asserted against the real binary because the observable is the exit status: a
//! cooperative stop returns from `run_watch`, awaits the bounded
//! `framework.shutdown()`, and exits `0`. An unhandled SIGTERM kills the process by
//! default disposition instead, which surfaces as `signal: 15` (shell exit `143`).
//!
//! This is why the assertion is not a unit test with an injected future:
//! `src/mcp/server_tests.rs` resolves a synthetic signal to prove the plumbing, and
//! that test stayed green while SIGTERM was missing from the process entirely. Only
//! the exit code distinguishes "the handler was installed" from "the handler exists
//! and was never registered".
//!
//! Run with: `cargo test --test cli_shutdown_signal --features cli`

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

fn spawn_watch(db: &Path) -> Child {
    // An explicit `--database` is mandatory: with none given, `csm` falls back to
    // git-local storage and a test would write into the real repository.
    Command::new(env!("CARGO_BIN_EXE_csm"))
        .arg("watch")
        .arg("--database")
        .arg(db)
        // A live reaper makes the cooperative stop meaningful: `shutdown()` has a
        // real background task to stop rather than a `None` handle to skip.
        .args(["--ttl-cleanup-interval", "1"])
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn csm watch")
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
        .expect("csm watch was spawned with a piped stderr");
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

/// Wait until the child reports it is watching, i.e. until the point just before it
/// registers its signal handlers, then let it settle.
fn wait_until_watching(child: &mut Child) -> std::sync::Arc<std::sync::Mutex<Vec<String>>> {
    let lines = drain_stderr(child);
    let deadline = Instant::now() + READY_DEADLINE;
    while !logged(&lines, "Press Ctrl+C to stop.") {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "csm watch never announced that it was watching; stderr was:\n{}",
                dump(&lines)
            );
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
        match child.try_wait().expect("failed to wait on csm watch") {
            Some(status) => return status,
            None if Instant::now() >= deadline => {
                child.kill().expect("failed to kill a hung csm watch");
                let _ = child.wait();
                panic!(
                    "csm watch did not exit within {EXIT_DEADLINE:?} of the signal; \
                     the shutdown is not cooperative"
                );
            }
            None => sleep(Duration::from_millis(50)),
        }
    }
}

/// Drive one signal end to end and return the exit status with the child's stderr.
fn stop_with(signal: &str) -> (std::process::ExitStatus, String) {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let db = dir.path().join("shutdown-signal.db");
    let mut child = spawn_watch(&db);
    let lines = wait_until_watching(&mut child);
    send_signal(&child, signal);
    let status = wait_for_exit(&mut child);
    let stderr = dump(&lines);
    // Keep `dir` alive until the child is reaped, then drop it.
    drop(dir);
    (status, stderr)
}

#[test]
fn watch_stops_cooperatively_on_sigterm() {
    let (status, stderr) = stop_with("TERM");
    assert!(
        status.success(),
        "SIGTERM must stop `csm watch` cooperatively (exit 0), but it exited as \
         {status:?} — signal {:?} means no SIGTERM handler was installed, so ADR-0099's \
         bounded TTL-cleanup stop never ran and a service-managed server is killed \
         rather than stopped.\n--- csm watch stderr ---\n{stderr}",
        status.signal(),
    );
    assert_eq!(
        status.code(),
        Some(0),
        "SIGTERM must exit `csm watch` with code 0, got {status:?}.\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Interrupted."),
        "the signal arm must actually break the watch loop; stderr was:\n{stderr}"
    );
}

#[test]
fn watch_stops_cooperatively_on_sigint() {
    let (status, stderr) = stop_with("INT");
    assert!(
        status.success(),
        "SIGINT must stop `csm watch` cooperatively (exit 0), but it exited as \
         {status:?}.\nstderr:\n{stderr}"
    );
    assert_eq!(
        status.code(),
        Some(0),
        "SIGINT must exit `csm watch` with code 0, got {status:?}.\nstderr:\n{stderr}"
    );
}
