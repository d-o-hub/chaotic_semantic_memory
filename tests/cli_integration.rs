use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::NamedTempFile;

fn csm() -> Command {
    let mut cmd = cargo_bin_cmd!("csm");
    cmd.current_dir(std::env::current_dir().expect("should have current dir"));
    cmd
}
fn db() -> NamedTempFile {
    NamedTempFile::new().unwrap()
}

// Version command
#[test]
fn version() {
    csm()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("csm"));
}

// Inject command
#[test]
fn inject_requires_id() {
    csm().arg("inject").assert().failure();
}
#[test]
fn inject_ok() {
    csm()
        .arg("--database")
        .arg(db().path())
        .arg("inject")
        .arg("t")
        .assert()
        .success();
}

// Maintenance commands
#[test]
fn prune_json() {
    csm()
        .arg("--output-format")
        .arg("json")
        .arg("prune")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\":\"success\""))
        .stdout(predicate::str::contains("\"pruned_count\":"));
}

#[test]
fn compact_ok() {
    csm()
        .arg("--database")
        .arg(db().path())
        .arg("compact")
        .assert()
        .success();
}
#[test]
fn inject_dup() {
    let d = db();
    let p = d.path();
    csm()
        .arg("--database")
        .arg(p)
        .arg("inject")
        .arg("d")
        .assert()
        .success();
    csm()
        .arg("--database")
        .arg(p)
        .arg("inject")
        .arg("d")
        .assert()
        .success();
}

// Probe command
#[test]
fn probe_requires_id() {
    csm().arg("probe").assert().failure();
}
#[test]
fn probe_missing() {
    csm()
        .arg("--database")
        .arg(db().path())
        .arg("probe")
        .arg("x")
        .assert()
        .failure();
}
#[test]
fn probe_topk_zero() {
    csm()
        .arg("probe")
        .arg("t")
        .arg("--top-k")
        .arg("0")
        .assert()
        .failure();
}

// Associate command
#[test]
fn assoc_requires_ids() {
    csm().arg("associate").assert().failure();
}
#[test]
fn assoc_missing() {
    csm()
        .arg("--database")
        .arg(db().path())
        .arg("associate")
        .arg("a")
        .arg("b")
        .assert()
        .failure();
}
#[test]
fn assoc_neg_strength() {
    csm()
        .arg("associate")
        .arg("a")
        .arg("b")
        .arg("-s")
        .arg("-1")
        .assert()
        .failure();
}

// Export command
#[test]
fn export_empty() {
    csm()
        .arg("--database")
        .arg(db().path())
        .arg("export")
        .assert()
        .success();
}

// Import command
#[test]
fn import_requires_file() {
    csm().arg("import").assert().failure();
}
#[test]
fn import_missing() {
    csm()
        .arg("--database")
        .arg(db().path())
        .arg("import")
        .arg("/x")
        .assert()
        .failure();
}

// Output formats
#[test]
fn fmt_json() {
    csm()
        .arg("--output-format")
        .arg("json")
        .arg("version")
        .assert()
        .success();
}
#[test]
fn fmt_quiet() {
    csm()
        .arg("--output-format")
        .arg("quiet")
        .arg("--database")
        .arg(db().path())
        .arg("inject")
        .arg("t")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

// Global options
#[test]
fn opt_verbose() {
    csm().arg("-v").arg("version").assert().success();
}
#[test]
fn opt_db() {
    csm()
        .arg("--database")
        .arg(db().path())
        .arg("version")
        .assert()
        .success();
}

// Completions
#[test]
fn comp_bash() {
    csm()
        .arg("completions")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_csm"));
}

// Edge cases
#[test]
fn edge_no_cmd() {
    csm().assert().failure();
}
#[test]
fn edge_bad_cmd() {
    csm().arg("bad").assert().failure();
}
#[test]
fn edge_help() {
    csm()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Chaotic Semantic Memory"));
}
#[test]
fn edge_version_flag() {
    csm().arg("--version").assert().success();
}

// Workflow integration tests
#[test]
fn workflow_inject_probe() {
    let d = db();
    let p = d.path();
    csm()
        .arg("--database")
        .arg(p)
        .arg("inject")
        .arg("a")
        .assert()
        .success();
    csm()
        .arg("--database")
        .arg(p)
        .arg("probe")
        .arg("a")
        .assert()
        .success();
}

#[test]
fn workflow_inject_assoc() {
    let d = db();
    let p = d.path();
    csm()
        .arg("--database")
        .arg(p)
        .arg("inject")
        .arg("x")
        .assert()
        .success();
    csm()
        .arg("--database")
        .arg(p)
        .arg("inject")
        .arg("y")
        .assert()
        .success();
    csm()
        .arg("--database")
        .arg(p)
        .arg("associate")
        .arg("x")
        .arg("y")
        .assert()
        .success();
}

#[test]
fn workflow_inject_export() {
    let d = db();
    let o = NamedTempFile::new().unwrap();
    csm()
        .arg("--database")
        .arg(d.path())
        .arg("inject")
        .arg("e")
        .assert()
        .success();
    csm()
        .arg("--database")
        .arg(d.path())
        .arg("export")
        .arg("-o")
        .arg(o.path())
        .assert()
        .success();
}
