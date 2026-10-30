//! Index JSONL file content into memory.
//!
//! Streams JSONL line-by-line, extracts text field, preserves metadata.

use crate::cli::args::{IndexJsonlArgs, OutputFormat};
use crate::cli::commands::{
    create_framework_with_namespace, print_success, print_warning, truncate_preview,
};
use crate::cli::error::{CliError, Result};
use csm_core_lib::encoder::TextEncoder;

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

    let framework: crate::framework::ChaoticSemanticFramework =
        create_framework_with_namespace(db_path, &args.namespace).await?;

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
            .map_or_else(|| format!("jsonl:{}", line_num + 1), |s| s.to_string());

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
        let preview = truncate_preview(&text, 200);
        metadata.insert(
            "searchable_text".to_string(),
            serde_json::Value::String(text),
        );
        metadata.insert(
            "text_preview".to_string(),
            serde_json::Value::String(preview),
        );
        metadata.insert(
            "tags".to_string(),
            serde_json::Value::Array(tags.into_iter().map(serde_json::Value::String).collect()),
        );

        framework
            .inject_concept_with_metadata(&id, hv, metadata)
            .await
            .map_err(|e| CliError::Persistence(format!("Failed to store concept: {e}")))?;

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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::run_index_jsonl;
    use crate::cli::args::{IndexJsonlArgs, OutputFormat};
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn args_for(file: PathBuf) -> IndexJsonlArgs {
        IndexJsonlArgs {
            namespace: "_default".to_string(),
            file,
            field: "text".to_string(),
            id_field: None,
            tag_field: None,
            code_aware: false,
        }
    }

    // The whole `--lib` mutation surface for this entry point is the missing-file
    // guard: with `persistence` off the success path would create a database in the
    // checkout, so this asserts the error the `-> Ok(())` mutant cannot produce.
    #[tokio::test]
    async fn index_jsonl_rejects_a_file_that_does_not_exist() {
        let dir = tempdir().expect("tempdir");
        let missing = dir.path().join("absent.jsonl");
        let result = run_index_jsonl(args_for(missing.clone()), None, OutputFormat::Quiet).await;
        assert!(
            result.is_err(),
            "indexing a nonexistent file must report an error, got Ok"
        );
        assert!(
            !missing.exists(),
            "the guard must not create the file it refused to read"
        );
    }
}
