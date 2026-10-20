#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::cli::args::{IndexDirArgs, OutputFormat};
use crate::cli::commands::create_framework_with_namespace;
use std::fs;
use std::path::Path;

#[test]
fn test_normalize_source_path() {
    assert_eq!(
        normalize_source_path(Path::new("./docs/readme.md")),
        "docs/readme.md"
    );
    assert_eq!(
        normalize_source_path(Path::new("docs/readme.md")),
        "docs/readme.md"
    );
    assert_eq!(
        normalize_source_path(Path::new("docs\\readme.md")),
        "docs/readme.md"
    );
}

#[test]
fn test_compute_fingerprint() {
    let f1 = compute_fingerprint("hello world");
    let f2 = compute_fingerprint("hello world");
    let f3 = compute_fingerprint("different content");
    assert_eq!(f1, f2);
    assert_ne!(f1, f3);
    assert_eq!(f1.len(), 64);
}

#[test]
fn test_parse_heading_valid() {
    assert_eq!(parse_heading("# Title"), Some((1, "Title".to_string())));
    assert_eq!(
        parse_heading("## Section"),
        Some((2, "Section".to_string()))
    );
    assert_eq!(
        parse_heading("### Subsection"),
        Some((3, "Subsection".to_string()))
    );
    assert_eq!(parse_heading("#### Deep"), Some((4, "Deep".to_string())));
}

#[test]
fn test_parse_heading_invalid() {
    assert_eq!(parse_heading("Not a heading"), None);
    assert_eq!(parse_heading("####### Too many"), None);
    assert_eq!(parse_heading("# "), None);
}

#[test]
fn test_chunk_by_heading_basic() {
    let content = "\
# Document Title

## Introduction

This is the introduction content.

## Methods

This is the methods content.

### Details

Detailed methods here.

";

    let chunks = chunk_by_heading(content, 2);

    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].heading, "Introduction");
    assert_eq!(chunks[0].level, 2);
    assert_eq!(chunks[1].heading, "Methods");
    assert_eq!(chunks[1].level, 2);
    assert_eq!(chunks[2].heading, "Details");
    assert_eq!(chunks[2].level, 3);
}

#[test]
fn test_chunk_by_heading_min_level() {
    let content = "\
# Title

## Section 1
Content 1

### Subsection
Content 2

";

    let chunks = chunk_by_heading(content, 3);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].heading, "Subsection");
}

#[test]
fn test_chunk_by_heading_empty() {
    let content = "";
    let chunks = chunk_by_heading(content, 2);
    assert!(chunks.is_empty());
}

#[tokio::test]
async fn test_run_index_dir_sync_reconciles_removed_section_and_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file1 = temp_dir.path().join("doc1.md");
    let file2 = temp_dir.path().join("doc2.md");

    fs::write(
        &file1,
        "# Title\n\n## Section A\nContent A\n\n## Section B\nContent B\n",
    )
    .unwrap();
    fs::write(&file2, "# Title 2\n\n## Section C\nContent C\n").unwrap();

    let db_path = temp_dir.path().join("test.db");
    let glob_pattern = format!("{}/doc*.md", temp_dir.path().display());

    let args = IndexDirArgs {
        namespace: "test_ns".to_string(),
        glob: vec![glob_pattern.clone()],
        heading_level: 2,
        code_aware: false,
        sync: false,
        scope: None,
    };

    // Initial additive index
    run_index_dir(args.clone(), Some(&db_path), OutputFormat::Quiet)
        .await
        .unwrap();

    let framework = create_framework_with_namespace(Some(&db_path), "test_ns")
        .await
        .unwrap();
    let sing_lock = framework.singularity();
    let sing = sing_lock.read().await;
    let initial_concepts = sing.all_concepts("test_ns");
    assert_eq!(initial_concepts.len(), 3);
    drop(sing);

    // Modify file1 (remove Section B) and delete file2
    fs::write(&file1, "# Title\n\n## Section A\nUpdated Content A\n").unwrap();
    fs::remove_file(&file2).unwrap();

    // Reindex with sync = true
    let mut sync_args = args;
    sync_args.sync = true;
    run_index_dir(sync_args, Some(&db_path), OutputFormat::Quiet)
        .await
        .unwrap();

    // Reload framework from persistence to verify durable state
    let reloaded_fw = create_framework_with_namespace(Some(&db_path), "test_ns")
        .await
        .unwrap();
    let sing_lock2 = reloaded_fw.singularity();
    let sing2 = sing_lock2.read().await;
    let synced_concepts = sing2.all_concepts("test_ns");
    assert_eq!(synced_concepts.len(), 1);
    assert!(
        synced_concepts[0]
            .metadata
            .get("content_preview")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("Updated Content A")
    );
}

#[tokio::test]
async fn test_run_index_dir_json_output() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file1 = temp_dir.path().join("doc.md");
    fs::write(&file1, "# Title\n\n## Section 1\nHello\n").unwrap();

    let db_path = temp_dir.path().join("test.db");
    let glob_pattern = format!("{}/doc.md", temp_dir.path().display());

    let args = IndexDirArgs {
        namespace: "json_ns".to_string(),
        glob: vec![glob_pattern],
        heading_level: 2,
        code_aware: true,
        sync: true,
        scope: Some("custom-scope".to_string()),
    };

    run_index_dir(args, Some(&db_path), OutputFormat::Json)
        .await
        .unwrap();
}
