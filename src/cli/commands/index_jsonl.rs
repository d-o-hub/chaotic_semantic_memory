//! Index JSONL file content into memory.
//!
//! Streams JSONL line-by-line, extracts text field, preserves metadata.

use crate::cli::args::{IndexJsonlArgs, OutputFormat};
use crate::cli::commands::{create_framework, print_success, print_warning, truncate_preview};
use crate::cli::error::{CliError, Result};
use crate::encoder::TextEncoder;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub async fn run_index_jsonl(
    args: IndexJsonlArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    // Validate file exists
    if !args.file.exists() {
        return Err(CliError::Validation(format!(
            "File not found: {}",
            args.file.display()
        )));
    }

    let framework = create_framework(db_path).await?;

    // Create encoder based on code_aware flag
    let encoder = if args.code_aware {
        TextEncoder::new_code_aware()
    } else {
        TextEncoder::new()
    };

    let file = File::open(&args.file).map_err(|e| {
        CliError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to open file: {}", args.file.display()),
        ))
    })?;
    let reader = BufReader::new(file);

    let mut indexed_count = 0;
    let mut skipped_count = 0;

    for (line_num, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| {
            CliError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read line {}", line_num + 1),
            ))
        })?;

        if line.trim().is_empty() {
            skipped_count += 1;
            continue;
        }

        // Parse JSON
        let json: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                print_warning(
                    &format!("Line {}: Invalid JSON, skipping: {}", line_num + 1, e),
                    format,
                );
                skipped_count += 1;
                continue;
            }
        };

        // Extract text field
        let text = match json.get(&args.field) {
            Some(v) => v.as_str().unwrap_or("").to_string(),
            None => {
                print_warning(
                    &format!(
                        "Line {}: Missing field '{}', skipping",
                        line_num + 1,
                        args.field
                    ),
                    format,
                );
                skipped_count += 1;
                continue;
            }
        };

        if text.trim().is_empty() {
            skipped_count += 1;
            continue;
        }

        // Extract metadata
        let id = args
            .id_field
            .as_ref()
            .and_then(|f| json.get(f))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("jsonl:{}", line_num + 1));

        let tags: Vec<String> = args
            .tag_field
            .as_ref()
            .and_then(|f| json.get(f))
            .and_then(|v| v.as_str())
            .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
            .unwrap_or_default();

        // Encode and store
        let hv = encoder.encode(&text);

        // Create metadata map
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "source".to_string(),
            serde_json::Value::String(args.file.display().to_string()),
        );
        metadata.insert(
            "line".to_string(),
            serde_json::Value::Number((line_num + 1).into()),
        );
        metadata.insert(
            "text_preview".to_string(),
            serde_json::Value::String(truncate_preview(&text, 200)),
        );
        metadata.insert(
            "tags".to_string(),
            serde_json::Value::Array(tags.into_iter().map(serde_json::Value::String).collect()),
        );

        framework
            .inject_concept_with_metadata(&id, hv, metadata)
            .await
            .map_err(|e| CliError::Persistence(format!("Failed to store concept: {}", e)))?;

        indexed_count += 1;
    }

    print_success(
        &format!(
            "Indexed {} entries from {} ({} skipped)",
            indexed_count,
            args.file.display(),
            skipped_count
        ),
        format,
    );

    Ok(())
}
