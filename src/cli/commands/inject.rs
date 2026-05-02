use std::collections::HashMap;
use std::path::Path;

use tracing::instrument;

use crate::cli::args::{InjectArgs, OutputFormat, VectorSource};
use crate::cli::error::{CliError, Result};
use crate::hyperdim::HVec10240;

use super::{create_framework, print_success, print_warning, validate_concept_id};

#[instrument(name = "cli_inject")]
pub async fn run_inject(
    args: InjectArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.concept_id)?;

    let framework = create_framework(db_path).await?;

    let vector = match args.vector_source {
        VectorSource::Random => HVec10240::random(),
        VectorSource::File | VectorSource::Stdin => {
            if let Some(ref file_path) = args.from_file {
                let content = std::fs::read_to_string(file_path).map_err(|e| {
                    CliError::Io(std::io::Error::new(
                        e.kind(),
                        format!("failed to read vector file: {}", file_path.display()),
                    ))
                })?;
                parse_vector(&content)?
            } else {
                let mut input = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).map_err(|e| {
                    CliError::Io(std::io::Error::new(
                        e.kind(),
                        "failed to read vector from stdin",
                    ))
                })?;
                parse_vector(&input)?
            }
        }
    };

    let existing = framework
        .get_concept(&args.concept_id)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to check concept: {e}")))?;

    // Handle metadata if provided
    if let Some(metadata_json) = args.metadata {
        let metadata: HashMap<String, serde_json::Value> = serde_json::from_str(&metadata_json)
            .map_err(|e| CliError::Validation(format!("invalid metadata JSON: {e}")))?;

        framework
            .inject_concept_with_metadata(&args.concept_id, vector, metadata)
            .await
            .map_err(|e| {
                CliError::Persistence(format!(
                    "failed to inject concept with metadata '{}': {e}",
                    args.concept_id
                ))
            })?;
    } else {
        framework
            .inject_concept(&args.concept_id, vector)
            .await
            .map_err(|e| {
                CliError::Persistence(format!(
                    "failed to inject concept '{}': {e}",
                    args.concept_id
                ))
            })?;
    }

    match format {
        OutputFormat::Json => {
            let status = if existing.is_some() {
                "updated"
            } else {
                "created"
            };
            println!(
                "{}",
                serde_json::json!({"status": status, "concept_id": args.concept_id})
            );
        }
        OutputFormat::Table => {
            if existing.is_some() {
                print_warning(
                    &format!("concept '{}' updated (existed)", args.concept_id),
                    format,
                );
            } else {
                print_success(&format!("concept '{}' injected", args.concept_id), format);
            }
        }
        OutputFormat::Quiet => {}
    }

    Ok(())
}

fn parse_vector(input: &str) -> Result<HVec10240> {
    let trimmed = input.trim();

    if trimmed.starts_with('[') {
        let values: Vec<f32> = serde_json::from_str(trimmed)
            .map_err(|e| CliError::Input(format!("invalid JSON array for vector: {e}")))?;
        return bytes_to_hvec(&values);
    }

    if trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return hex_to_hvec(trimmed);
    }

    let values: Vec<f32> = trimmed
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<f32>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| CliError::Input(format!("invalid numeric values in vector: {e}")))?;

    bytes_to_hvec(&values)
}

fn bytes_to_hseq(values: &[f32]) -> Result<[u8; 1280]> {
    if values.len() != 320 {
        return Err(CliError::Validation(format!(
            "vector dimension mismatch (expected 320 floats, got {})",
            values.len()
        )));
    }
    let mut bytes = [0u8; 1280];
    for (i, chunk) in values.chunks(4).enumerate() {
        if chunk.len() != 4 {
            return Err(CliError::Validation(format!(
                "incomplete float at position {i}"
            )));
        }
        let f0 = chunk[0].to_le_bytes();
        let f1 = chunk[1].to_le_bytes();
        let f2 = chunk[2].to_le_bytes();
        let f3 = chunk[3].to_le_bytes();
        let start = i * 16;
        bytes[start..start + 4].copy_from_slice(&f0);
        bytes[start + 4..start + 8].copy_from_slice(&f1);
        bytes[start + 8..start + 12].copy_from_slice(&f2);
        bytes[start + 12..start + 16].copy_from_slice(&f3);
    }
    Ok(bytes)
}

fn bytes_to_hvec(values: &[f32]) -> Result<HVec10240> {
    let bytes = bytes_to_hseq(values)?;
    HVec10240::from_bytes(&bytes)
        .map_err(|e| CliError::Validation(format!("failed to create hypervector from bytes: {e}")))
}

fn hex_to_hvec(hex: &str) -> Result<HVec10240> {
    let hex = hex.trim_start_matches("0x").trim_start_matches("0X");
    if hex.len() != 2560 {
        return Err(CliError::Validation(format!(
            "hex vector length mismatch (expected 2560 chars for 1280 bytes, got {})",
            hex.len()
        )));
    }
    let mut bytes = [0u8; 1280];
    for i in 0..1280 {
        bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|_| CliError::Validation(format!("invalid hex at position {}", i * 2)))?;
    }
    HVec10240::from_bytes(&bytes)
        .map_err(|e| CliError::Validation(format!("failed to create hypervector from hex: {e}")))
}
