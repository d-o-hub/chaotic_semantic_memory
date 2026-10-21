#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::cli::args::{IndexDirArgs, OutputFormat};
use crate::cli::commands::create_framework_with_namespace;
use std::collections::HashMap;
use std::fs;

fn sample_summary() -> IndexDirSummary {
    IndexDirSummary {
        files_processed: 4,
        chunks_indexed: 7,
        chunks_replaced: 2,
        chunks_deleted: 1,
        skipped: 3,
        namespace: "ns".to_string(),
        scope: "scope-x".to_string(),
        sync_mode: true,
    }
}

#[test]
fn test_index_dir_report_json_format() {
    let IndexDirReport::Json(text) = index_dir_report(&sample_summary(), OutputFormat::Json) else {
        panic!("OutputFormat::Json must render the JSON report");
    };
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["status"], "success");
    assert_eq!(value["files_processed"], 4);
    assert_eq!(value["chunks_indexed"], 7);
    assert_eq!(value["chunks_replaced"], 2);
    assert_eq!(value["chunks_deleted"], 1);
    assert_eq!(value["skipped"], 3);
    assert_eq!(value["namespace"], "ns");
    assert_eq!(value["scope"], "scope-x");
    assert_eq!(value["sync_mode"], true);
}

#[test]
fn test_index_dir_report_message_for_non_json_formats() {
    for format in [OutputFormat::Table, OutputFormat::Quiet] {
        let IndexDirReport::Message(message) = index_dir_report(&sample_summary(), format) else {
            panic!("non-Json formats must render the success message");
        };
        assert_eq!(
            message,
            "Indexed 7 new chunks (2 replaced, 1 deleted) from 4 files (3 skipped) [scope: scope-x]"
        );
    }
}

#[tokio::test]
async fn test_run_index_dir_returns_summary_counts() {
    let temp_dir = tempfile::tempdir().unwrap();
    let doc = temp_dir.path().join("doc.md");
    fs::write(&doc, "# Title\n\n## One\nFirst\n\n## Two\nSecond\n").unwrap();
    let db_path = temp_dir.path().join("summary.db");
    let glob_pat = format!("{}/doc.md", temp_dir.path().display());

    let args = IndexDirArgs {
        namespace: "summary_ns".to_string(),
        glob: vec![glob_pat],
        heading_level: 2,
        code_aware: false,
        sync: true,
        scope: Some("summary_scope".to_string()),
    };

    let first = run_index_dir(args.clone(), Some(&db_path), OutputFormat::Json)
        .await
        .unwrap();
    assert_eq!(first.files_processed, 1);
    assert_eq!(first.chunks_indexed, 2);
    assert_eq!(first.chunks_replaced, 0);
    assert_eq!(first.chunks_deleted, 0);
    assert_eq!(first.scope, "summary_scope");
    assert!(first.sync_mode);

    fs::write(&doc, "# Title\n\n## One\nUpdated\n").unwrap();
    let second = run_index_dir(args, Some(&db_path), OutputFormat::Quiet)
        .await
        .unwrap();
    assert_eq!(second.files_processed, 1);
    assert_eq!(second.chunks_indexed, 0);
    assert_eq!(second.chunks_replaced, 1);
    assert_eq!(second.chunks_deleted, 1);
    assert_eq!(second.namespace, "summary_ns");
}

#[tokio::test]
async fn test_run_index_dir_deletes_scope_labeled_and_glob_matching_concepts() {
    let temp_dir = tempfile::tempdir().unwrap();
    let doc = temp_dir.path().join("live.md");
    fs::write(&doc, "# Title\n\n## Live\nCurrent\n").unwrap();
    let db_path = temp_dir.path().join("disj.db");

    let framework = create_framework_with_namespace(Some(&db_path), "disj_ns")
        .await
        .unwrap();

    let mut scope_only = HashMap::new();
    scope_only.insert(
        "source".to_string(),
        serde_json::Value::String("legacy-notes.txt".to_string()),
    );
    scope_only.insert(
        "scope".to_string(),
        serde_json::Value::String("managed".to_string()),
    );
    framework
        .inject_concept_with_metadata(
            "chunk:scope-only",
            csm_core_lib::hyperdim::HVec10240::random(),
            scope_only,
        )
        .await
        .unwrap();

    let mut glob_only = HashMap::new();
    glob_only.insert(
        "source".to_string(),
        serde_json::Value::String(normalize_source_path(&temp_dir.path().join("ghost.md"))),
    );
    glob_only.insert(
        "scope".to_string(),
        serde_json::Value::String("other".to_string()),
    );
    framework
        .inject_concept_with_metadata(
            "chunk:glob-only",
            csm_core_lib::hyperdim::HVec10240::random(),
            glob_only,
        )
        .await
        .unwrap();

    let mut foreign = HashMap::new();
    foreign.insert(
        "source".to_string(),
        serde_json::Value::String("elsewhere.md".to_string()),
    );
    foreign.insert(
        "scope".to_string(),
        serde_json::Value::String("elsewhere".to_string()),
    );
    framework
        .inject_concept_with_metadata(
            "chunk:foreign",
            csm_core_lib::hyperdim::HVec10240::random(),
            foreign,
        )
        .await
        .unwrap();
    drop(framework);

    let args = IndexDirArgs {
        namespace: "disj_ns".to_string(),
        glob: vec![format!("{}/*.md", temp_dir.path().display())],
        heading_level: 2,
        code_aware: false,
        sync: true,
        scope: Some("managed".to_string()),
    };
    let summary = run_index_dir(args, Some(&db_path), OutputFormat::Quiet)
        .await
        .unwrap();

    assert_eq!(summary.files_processed, 1);
    assert_eq!(summary.chunks_indexed, 1);
    assert_eq!(summary.chunks_replaced, 0);
    assert_eq!(summary.chunks_deleted, 2);

    let reloaded = create_framework_with_namespace(Some(&db_path), "disj_ns")
        .await
        .unwrap();
    let ids: Vec<String> = reloaded
        .singularity()
        .read()
        .await
        .all_concepts("disj_ns")
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert!(
        ids.contains(&"chunk:foreign".to_string()),
        "concept outside both scope and glob membership must survive"
    );
    assert!(
        !ids.contains(&"chunk:scope-only".to_string()),
        "scope-labeled concept must be reconciled away by scope membership"
    );
    assert!(
        !ids.contains(&"chunk:glob-only".to_string()),
        "glob-matching concept must be reconciled away even with a foreign scope label"
    );
}
