//! Index Markdown files from directory into memory.
//!
//! Supports glob patterns, heading-based chunking, code-aware encoding,
//! and source synchronization mode for reconciling obsolete chunks and deleted files.

use crate::cli::args::{IndexDirArgs, OutputFormat};
use crate::cli::commands::{create_framework_with_namespace, print_success, truncate_preview};
use crate::cli::error::{CliError, Result};
use csm_core_lib::encoder::TextEncoder;
use sha2::{Digest, Sha256};

use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Normalize source file path to ensure stable identity across invocation directories and relative spellings.
pub fn normalize_source_path(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    let trimmed = path_str.strip_prefix("./").unwrap_or(&path_str);
    trimmed.replace('\\', "/")
}

/// Compute SHA-256 fingerprint hex digest for chunk content.
pub fn compute_fingerprint(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub async fn run_index_dir(
    args: IndexDirArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    let framework: crate::framework::ChaoticSemanticFramework =
        create_framework_with_namespace(db_path, &args.namespace).await?;

    // Create encoder based on code_aware flag
    let encoder = if args.code_aware {
        TextEncoder::new_code_aware()
    } else {
        TextEncoder::new()
    };

    let declared_scope = args.scope.clone().unwrap_or_else(|| {
        args.glob
            .iter()
            .map(|g| normalize_source_path(Path::new(g)))
            .collect::<Vec<_>>()
            .join(",")
    });

    struct StagedChunk {
        id: String,
        heading: String,
        level: usize,
        content: String,
        fingerprint: String,
        source_path: String,
        modified_at: Option<u64>,
    }

    struct StagedFile {
        normalized_path: String,
        chunks: Vec<StagedChunk>,
    }

    let mut staged_files: Vec<StagedFile> = Vec::new();
    let mut skipped_count = 0;

    // Stage 1: Read and validate all matched files before modifying index (Requirement 4)
    for pattern in &args.glob {
        let glob_paths = glob::glob(pattern)
            .map_err(|e| CliError::Validation(format!("Invalid glob pattern '{pattern}': {e}")))?;

        for path_result in glob_paths {
            let path = path_result
                .map_err(|e| CliError::Io(std::io::Error::other(format!("Glob error: {e}"))))?;

            if !path.exists() || !path.is_file() {
                continue;
            }

            let content = fs::read_to_string(&path).map_err(|e| {
                CliError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to read {}", path.display()),
                ))
            })?;

            let normalized_path = normalize_source_path(&path);
            let chunks_raw = chunk_by_heading(&content, args.heading_level);

            let modified_at = fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs());

            let mut staged_chunks = Vec::new();
            for (chunk_idx, chunk) in chunks_raw.into_iter().enumerate() {
                if chunk.content.trim().is_empty() {
                    skipped_count += 1;
                    continue;
                }

                let id = format!("md:{}:{}:{}", normalized_path, chunk.level, chunk_idx);
                let fingerprint = compute_fingerprint(&chunk.content);

                staged_chunks.push(StagedChunk {
                    id,
                    heading: chunk.heading,
                    level: chunk.level,
                    content: chunk.content,
                    fingerprint,
                    source_path: normalized_path.clone(),
                    modified_at,
                });
            }

            staged_files.push(StagedFile {
                normalized_path,
                chunks: staged_chunks,
            });
        }
    }

    let active_source_paths: HashSet<String> = staged_files
        .iter()
        .map(|f| f.normalized_path.clone())
        .collect();

    let new_chunk_ids: HashSet<String> = staged_files
        .iter()
        .flat_map(|f| f.chunks.iter().map(|c| c.id.clone()))
        .collect();

    // Stage 2: Retrieve existing concepts to determine replacements and obsolete chunks
    let existing_concepts = {
        let sing_lock = framework.singularity();
        let sing = sing_lock.read().await;
        sing.all_concepts(&args.namespace)
    };
    let existing_ids: HashSet<String> = existing_concepts.iter().map(|c| c.id.clone()).collect();

    let mut obsolete_ids: Vec<String> = Vec::new();

    if args.sync {
        let glob_matchers: Vec<glob::Pattern> = args
            .glob
            .iter()
            .filter_map(|g| glob::Pattern::new(g).ok())
            .collect();

        for concept in &existing_concepts {
            if new_chunk_ids.contains(&concept.id) {
                continue;
            }

            let concept_source = concept
                .metadata
                .get("source")
                .and_then(|v| v.as_str())
                .or_else(|| concept.metadata.get("path").and_then(|v| v.as_str()));

            let concept_scope = concept.metadata.get("scope").and_then(|v| v.as_str());

            let is_in_active_source =
                concept_source.is_some_and(|src| active_source_paths.contains(src));

            let is_in_scope = concept_scope == Some(&declared_scope)
                || concept_source.is_some_and(|src| glob_matchers.iter().any(|m| m.matches(src)));

            if ((is_in_active_source && !new_chunk_ids.contains(&concept.id))
                || (is_in_scope
                    && concept_source.is_none_or(|src| !active_source_paths.contains(src))))
                && (concept.id.starts_with("md:") || concept_source.is_some())
            {
                obsolete_ids.push(concept.id.clone());
            }
        }
    }

    let mut indexed_count = 0;
    let mut replaced_count = 0;
    let mut deleted_count = 0;

    // Stage 3: Inject / update concepts
    for staged in &staged_files {
        for chunk in &staged.chunks {
            let is_replacement = existing_ids.contains(&chunk.id);

            let hv = encoder.encode(&chunk.content);

            let mut metadata = std::collections::HashMap::new();
            metadata.insert(
                "source".to_string(),
                serde_json::Value::String(chunk.source_path.clone()),
            );
            metadata.insert(
                "path".to_string(),
                serde_json::Value::String(chunk.source_path.clone()),
            );
            metadata.insert(
                "heading".to_string(),
                serde_json::Value::String(chunk.heading.clone()),
            );
            metadata.insert(
                "level".to_string(),
                serde_json::Value::Number(chunk.level.into()),
            );
            metadata.insert(
                "content_preview".to_string(),
                serde_json::Value::String(truncate_preview(&chunk.content, 200)),
            );
            metadata.insert(
                "fingerprint".to_string(),
                serde_json::Value::String(chunk.fingerprint.clone()),
            );
            metadata.insert(
                "encoding_mode".to_string(),
                serde_json::Value::String(
                    if args.code_aware {
                        "code_aware"
                    } else {
                        "standard"
                    }
                    .to_string(),
                ),
            );
            metadata.insert(
                "scope".to_string(),
                serde_json::Value::String(declared_scope.clone()),
            );

            if let Some(ts) = chunk.modified_at {
                metadata.insert(
                    "modified_at".to_string(),
                    serde_json::Value::Number(ts.into()),
                );
            }

            framework
                .inject_concept_with_metadata(&chunk.id, hv, metadata)
                .await
                .map_err(|e| CliError::Persistence(format!("Failed to store concept: {e}")))?;

            if is_replacement {
                replaced_count += 1;
            } else {
                indexed_count += 1;
            }
        }
    }

    // Stage 4: Delete obsolete chunks in sync mode
    for obsolete_id in obsolete_ids {
        framework.delete_concept(&obsolete_id).await.map_err(|e| {
            CliError::Persistence(format!("Failed to delete concept {obsolete_id}: {e}"))
        })?;
        deleted_count += 1;
    }

    let file_count = staged_files.len();

    if format == OutputFormat::Json {
        let output = serde_json::json!({
            "status": "success",
            "files_processed": file_count,
            "chunks_indexed": indexed_count,
            "chunks_replaced": replaced_count,
            "chunks_deleted": deleted_count,
            "skipped": skipped_count,
            "namespace": args.namespace,
            "scope": declared_scope,
            "sync_mode": args.sync,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    } else {
        print_success(
            &format!(
                "Indexed {indexed_count} new chunks ({replaced_count} replaced, {deleted_count} deleted) from {file_count} files ({skipped_count} skipped) [scope: {declared_scope}]"
            ),
            format,
        );
    }

    Ok(())
}

/// A chunk of markdown content.
#[derive(Debug, Clone)]
pub struct MarkdownChunk {
    /// The heading text (e.g., "Introduction").
    pub heading: String,
    /// The heading level (1-6).
    pub level: usize,
    /// The content under this heading.
    pub content: String,
}

/// Chunk markdown content by heading boundaries.
///
/// Only chunks at headings >= min_level (default 2 for ##).
/// Heading level 1 (#) is typically the document title and skipped.
pub fn chunk_by_heading(content: &str, min_level: usize) -> Vec<MarkdownChunk> {
    let mut chunks: Vec<MarkdownChunk> = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_level: Option<usize> = None;
    let mut current_content: String = String::new();

    for line in content.lines() {
        let heading_match = parse_heading(line);

        if let Some((level, heading_text)) = heading_match {
            if let Some(level) = current_level {
                if level >= min_level && !current_content.trim().is_empty() {
                    chunks.push(MarkdownChunk {
                        heading: current_heading.clone().unwrap_or_default(),
                        level,
                        content: current_content.trim().to_string(),
                    });
                }
            }

            current_heading = Some(heading_text);
            current_level = Some(level);
            current_content = String::new();
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    if let Some(level) = current_level {
        if level >= min_level && !current_content.trim().is_empty() {
            chunks.push(MarkdownChunk {
                heading: current_heading.unwrap_or_default(),
                level,
                content: current_content.trim().to_string(),
            });
        }
    }

    chunks
}

/// Parse a markdown heading line.
///
/// Returns (level, heading_text) if the line is a heading.
/// Level is 1-6 based on number of # symbols.
fn parse_heading(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim();

    if !trimmed.starts_with('#') {
        return None;
    }

    let mut level = 0;
    for c in trimmed.chars() {
        if c == '#' {
            level += 1;
        } else {
            break;
        }
    }

    if level == 0 || level > 6 {
        return None;
    }

    let heading_text = trimmed[level..].trim().to_string();

    if heading_text.is_empty() {
        return None;
    }

    Some((level, heading_text))
}

#[cfg(test)]
#[path = "index_dir_tests.rs"]
mod index_dir_tests;
