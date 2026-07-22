//! Index Markdown files from directory into memory.
//!
//! Supports glob patterns, heading-based chunking, and code-aware encoding.

use crate::cli::args::{IndexDirArgs, OutputFormat};
use crate::cli::commands::{create_framework_with_namespace, print_success, truncate_preview};
use crate::cli::error::{CliError, Result};
use csm_core_lib::encoder::TextEncoder;

use std::fs;
use std::path::Path;

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

    let mut indexed_count = 0;
    let mut skipped_count = 0;
    let mut file_count = 0;

    for pattern in &args.glob {
        let paths = glob::glob(pattern)
            .map_err(|e| CliError::Validation(format!("Invalid glob pattern '{pattern}': {e}")))?;

        for path_result in paths {
            let path = path_result
                .map_err(|e| CliError::Io(std::io::Error::other(format!("Glob error: {e}"))))?;

            if !path.exists() || !path.is_file() {
                continue;
            }

            file_count += 1;

            // Read file content
            let content = fs::read_to_string(&path).map_err(|e| {
                CliError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to read {}", path.display()),
                ))
            })?;

            // Chunk by headings
            let chunks = chunk_by_heading(&content, args.heading_level);

            for (chunk_idx, chunk) in chunks.iter().enumerate() {
                if chunk.content.trim().is_empty() {
                    skipped_count += 1;
                    continue;
                }

                // Create unique ID
                let id = format!("md:{}:{}:{}", path.display(), chunk.level, chunk_idx);

                // Encode content
                let hv = encoder.encode(&chunk.content);

                // Create metadata map
                let path_str = path.display().to_string();
                let mut metadata = std::collections::HashMap::new();
                metadata.insert(
                    "source".to_string(),
                    serde_json::Value::String(path_str.clone()),
                );
                metadata.insert("path".to_string(), serde_json::Value::String(path_str));
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

                // Store file modification time as Unix timestamp
                if let Ok(file_meta) = fs::metadata(&path) {
                    if let Ok(modified) = file_meta.modified() {
                        if let Ok(ts) = modified.duration_since(std::time::UNIX_EPOCH) {
                            metadata.insert(
                                "modified_at".to_string(),
                                serde_json::Value::Number(ts.as_secs().into()),
                            );
                        }
                    }
                }

                framework
                    .inject_concept_with_metadata(&id, hv, metadata)
                    .await
                    .map_err(|e| CliError::Persistence(format!("Failed to store concept: {e}")))?;

                indexed_count += 1;
            }
        }
    }

    print_success(
        &format!(
            "Indexed {indexed_count} chunks from {file_count} files ({skipped_count} skipped)"
        ),
        format,
    );

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
        // Check if this is a heading line
        let heading_match = parse_heading(line);

        if let Some((level, heading_text)) = heading_match {
            // Save previous chunk if it meets min_level
            if let Some(level) = current_level {
                if level >= min_level && !current_content.trim().is_empty() {
                    chunks.push(MarkdownChunk {
                        heading: current_heading.clone().unwrap_or_default(),
                        level,
                        content: current_content.trim().to_string(),
                    });
                }
            }

            // Start new chunk
            current_heading = Some(heading_text);
            current_level = Some(level);
            current_content = String::new();
        } else {
            // Add to current content
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Save final chunk
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

    // Check for # heading markers
    if !trimmed.starts_with('#') {
        return None;
    }

    // Count # symbols
    let mut level = 0;
    for c in trimmed.chars() {
        if c == '#' {
            level += 1;
        } else {
            break;
        }
    }

    // Valid heading levels are 1-6
    if level == 0 || level > 6 {
        return None;
    }

    // Extract heading text (after # symbols)
    let heading_text = trimmed[level..].trim().to_string();

    if heading_text.is_empty() {
        return None;
    }

    Some((level, heading_text))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

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
        assert_eq!(parse_heading("# "), None); // Empty heading
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

        // Only level >= 3
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
}
