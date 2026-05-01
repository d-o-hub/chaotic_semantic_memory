use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_cli_parity_lifecycle() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let db_str = db_path.to_str().unwrap();

    // 1. Inject
    let mut cmd = Command::cargo_bin("csm").unwrap();
    cmd.args(&[
        "--database",
        db_str,
        "inject",
        "c1",
        "--metadata",
        "{\"tags\":[\"test\"]}",
    ])
    .assert()
    .success();

    // 2. Get
    let mut cmd = Command::cargo_bin("csm").unwrap();
    cmd.args(&["--database", db_str, "get", "c1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("c1"))
        .stdout(predicate::str::contains("test"));

    // 3. Update
    let mut cmd = Command::cargo_bin("csm").unwrap();
    cmd.args(&[
        "--database",
        db_str,
        "update",
        "c1",
        "--metadata",
        "{\"tags\":[\"updated\"]}",
    ])
    .assert()
    .success();

    let mut cmd = Command::cargo_bin("csm").unwrap();
    cmd.args(&["--database", db_str, "get", "c1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("updated"));

    // 4. Stats
    let mut cmd = Command::cargo_bin("csm").unwrap();
    cmd.args(&["--database", db_str, "stats"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Concepts: 1"));

    // 5. Delete
    let mut cmd = Command::cargo_bin("csm").unwrap();
    cmd.args(&["--database", db_str, "delete", "c1"])
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("csm").unwrap();
    cmd.args(&["--database", db_str, "get", "c1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_cli_parity_graph() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_graph.db");
    let db_str = db_path.to_str().unwrap();

    // Inject nodes
    for id in &["a", "b", "c"] {
        Command::cargo_bin("csm")
            .unwrap()
            .args(&["--database", db_str, "inject", id])
            .assert()
            .success();
    }

    // Associate a -> b -> c
    Command::cargo_bin("csm")
        .unwrap()
        .args(&[
            "--database",
            db_str,
            "associate",
            "a",
            "b",
            "--strength",
            "0.8",
        ])
        .assert()
        .success();
    Command::cargo_bin("csm")
        .unwrap()
        .args(&[
            "--database",
            db_str,
            "associate",
            "b",
            "c",
            "--strength",
            "0.5",
        ])
        .assert()
        .success();

    // Associations
    Command::cargo_bin("csm")
        .unwrap()
        .args(&["--database", db_str, "associations", "a"])
        .assert()
        .success()
        .stdout(predicate::str::contains("b"))
        .stdout(predicate::str::contains("0.8000"));

    // Associations reverse
    Command::cargo_bin("csm")
        .unwrap()
        .args(&["--database", db_str, "associations", "b", "--reverse"])
        .assert()
        .success()
        .stdout(predicate::str::contains("a"));

    // Traverse
    Command::cargo_bin("csm")
        .unwrap()
        .args(&["--database", db_str, "traverse", "a", "--depth", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("a"))
        .stdout(predicate::str::contains("b"))
        .stdout(predicate::str::contains("c"));

    // Path
    Command::cargo_bin("csm")
        .unwrap()
        .args(&["--database", db_str, "path", "a", "c"])
        .assert()
        .success()
        .stdout(predicate::str::contains("a -> b -> c"));

    // Disassociate
    Command::cargo_bin("csm")
        .unwrap()
        .args(&["--database", db_str, "disassociate", "a", "b"])
        .assert()
        .success();

    Command::cargo_bin("csm")
        .unwrap()
        .args(&["--database", db_str, "path", "a", "c"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No path found"));
}

#[test]
fn test_cli_parity_metrics() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_metrics.db");
    let db_str = db_path.to_str().unwrap();

    Command::cargo_bin("csm")
        .unwrap()
        .args(&["--database", db_str, "inject", "m1"])
        .assert()
        .success();

    // In single-shot CLI, metrics are zeroed every run.
    // We just verify the command succeeds and outputs the table.
    Command::cargo_bin("csm")
        .unwrap()
        .args(&["--database", db_str, "metrics"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Total Probes:"));

    Command::cargo_bin("csm")
        .unwrap()
        .args(&["--database", db_str, "metrics", "--reset"])
        .assert()
        .success();
}

#[test]
fn test_cli_parity_probe_filtered() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_filtered.db");
    let db_str = db_path.to_str().unwrap();

    Command::cargo_bin("csm")
        .unwrap()
        .args(&[
            "--database",
            db_str,
            "inject",
            "p1",
            "--metadata",
            "{\"cat\":\"A\"}",
        ])
        .assert()
        .success();
    Command::cargo_bin("csm")
        .unwrap()
        .args(&[
            "--database",
            db_str,
            "inject",
            "p2",
            "--metadata",
            "{\"cat\":\"B\"}",
        ])
        .assert()
        .success();

    Command::cargo_bin("csm")
        .unwrap()
        .args(&[
            "--database",
            db_str,
            "probe-filtered",
            "p1",
            "--filter",
            "{\"Eq\":[\"cat\",\"A\"]}",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("p1"))
        .stdout(predicate::str::contains("p2").not());
}
