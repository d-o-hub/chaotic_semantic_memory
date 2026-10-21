#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::io::Write;
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

#[test]
fn index_dir_sync_lifecycle() {
    use std::fs;

    let temp_dir = tempfile::tempdir().unwrap();
    let db_file = temp_dir.path().join("csm.db");
    let docs_dir = temp_dir.path().join("docs");
    fs::create_dir_all(&docs_dir).unwrap();

    let doc1 = docs_dir.join("guide.md");
    let doc2 = docs_dir.join("faq.md");

    fs::write(
        &doc1,
        "# Guide\n\n## Section One\nUnique text for section one\n\n## Section Two\nUnique text for section two\n",
    )
    .unwrap();
    fs::write(
        &doc2,
        "# FAQ\n\n## FAQ Section\nUnique text for FAQ section\n",
    )
    .unwrap();

    let glob_pat = format!("{}/docs/*.md", temp_dir.path().display());

    // 1. Initial index and query both sections
    csm()
        .arg("--database")
        .arg(&db_file)
        .arg("index-dir")
        .arg("-g")
        .arg(&glob_pat)
        .arg("--output-format")
        .arg("json")
        .assert()
        .success();

    csm()
        .arg("--database")
        .arg(&db_file)
        .arg("query")
        .arg("section one")
        .assert()
        .success()
        .stdout(predicate::str::contains("guide.md:2:0"));

    csm()
        .arg("--database")
        .arg(&db_file)
        .arg("query")
        .arg("section two")
        .assert()
        .success()
        .stdout(predicate::str::contains("guide.md:2:1"));

    // 2. Remove Section Two from guide.md and reindex in sync mode
    fs::write(
        &doc1,
        "# Guide\n\n## Section One\nUnique text for section one updated\n",
    )
    .unwrap();

    csm()
        .arg("--database")
        .arg(&db_file)
        .arg("index-dir")
        .arg("-g")
        .arg(&glob_pat)
        .arg("--sync")
        .arg("--output-format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"chunks_deleted\": 1"));

    // 3. Delete faq.md and synchronize scope; faq chunk disappears
    fs::remove_file(&doc2).unwrap();

    csm()
        .arg("--database")
        .arg(&db_file)
        .arg("index-dir")
        .arg("-g")
        .arg(&glob_pat)
        .arg("--sync")
        .arg("--output-format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"chunks_deleted\": 1"));

    // 4. Reindex unchanged content; no duplicate accumulation
    csm()
        .arg("--database")
        .arg(&db_file)
        .arg("index-dir")
        .arg("-g")
        .arg(&glob_pat)
        .arg("--sync")
        .arg("--output-format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"chunks_replaced\": 1"))
        .stdout(predicate::str::contains("\"chunks_indexed\": 0"));

    // 5. Malformed/invalid glob syntax fails non-zero
    csm()
        .arg("--database")
        .arg(&db_file)
        .arg("index-dir")
        .arg("-g")
        .arg("[invalid_glob")
        .arg("--sync")
        .assert()
        .failure();
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

#[test]
fn workflow_index_long_jsonl_and_query_returns_stable_json_identifier() {
    let d = db();
    let mut source = NamedTempFile::new().unwrap();
    writeln!(
        source,
        r#"{{"id":"long-document","text":"{}needlequasar"}}"#,
        "padding ".repeat(35)
    )
    .unwrap();

    csm()
        .arg("--database")
        .arg(d.path())
        .arg("index-jsonl")
        .arg("--file")
        .arg(source.path())
        .arg("--id-field")
        .arg("id")
        .assert()
        .success();

    let output = csm()
        .arg("--database")
        .arg(d.path())
        .arg("query")
        .arg("needlequasar")
        .arg("--keyword-only")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let results: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(results[0]["concept_id"], "long-document");
    assert!(
        results[0]["metadata"].get("searchable_text").is_none(),
        "private full-text index content must not leak into CLI JSON"
    );
}
