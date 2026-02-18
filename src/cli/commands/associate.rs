use std::path::Path;

use crate::cli::args::{AssociateArgs, OutputFormat};
use anyhow::{Context, Result};

use super::{
    create_framework, print_error, print_success, print_warning, validate_concept_id,
    validate_strength,
};

pub async fn run_associate(
    args: AssociateArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.source_id)?;
    validate_concept_id(&args.target_id)?;
    validate_strength(args.strength)?;

    let framework = create_framework(db_path)
        .await
        .context("failed to initialize framework")?;

    let source_exists = framework.get_concept(&args.source_id).await?.is_some();
    if !source_exists {
        print_error(&format!("concept '{}' not found", args.source_id));
        anyhow::bail!("concept '{}' not found", args.source_id);
    }

    let target_exists = framework.get_concept(&args.target_id).await?.is_some();
    if !target_exists {
        print_error(&format!("concept '{}' not found", args.target_id));
        anyhow::bail!("concept '{}' not found", args.target_id);
    }

    let is_self_assoc = args.source_id == args.target_id;

    framework
        .associate(&args.source_id, &args.target_id, args.strength as f32)
        .await
        .with_context(|| {
            format!(
                "failed to associate '{}' -> '{}'",
                args.source_id, args.target_id
            )
        })?;

    match format {
        OutputFormat::Json => {
            println!(
                r#"{{"status":"created","source":"{}","target":"{}","strength":{}}}"#,
                args.source_id, args.target_id, args.strength
            );
        }
        OutputFormat::Table => {
            if is_self_assoc {
                print_warning(
                    &format!(
                        "self-association created: {} -> {} (strength: {:.2})",
                        args.source_id, args.target_id, args.strength
                    ),
                    format,
                );
            } else {
                print_success(
                    &format!(
                        "association created: {} -> {} (strength: {:.2})",
                        args.source_id, args.target_id, args.strength
                    ),
                    format,
                );
            }
        }
        OutputFormat::Quiet => {}
    }

    Ok(())
}

pub async fn run_associate_batch(
    associations: Vec<(String, String, f64)>,
    db_path: Option<&Path>,
    format: OutputFormat,
    continue_on_error: bool,
) -> Result<()> {
    let framework = create_framework(db_path)
        .await
        .context("failed to initialize framework")?;

    let mut created = 0usize;
    let mut failed = 0usize;

    for (source_id, target_id, strength) in associations {
        if let Err(e) = async {
            validate_concept_id(&source_id)?;
            validate_concept_id(&target_id)?;
            validate_strength(strength)?;

            framework
                .associate(&source_id, &target_id, strength as f32)
                .await
                .context("association failed")?;
            Ok::<_, anyhow::Error>(())
        }
        .await
        {
            failed += 1;
            if !continue_on_error {
                anyhow::bail!("batch failed at {} -> {}: {}", source_id, target_id, e);
            }
            if matches!(format, OutputFormat::Table) {
                print_warning(
                    &format!("skipped {} -> {}: {}", source_id, target_id, e),
                    format,
                );
            }
        } else {
            created += 1;
        }
    }

    match format {
        OutputFormat::Json => {
            println!(r#"{{"created":{},"failed":{}}}"#, created, failed);
        }
        OutputFormat::Table => {
            print_success(
                &format!("batch complete: {} created, {} failed", created, failed),
                format,
            );
        }
        OutputFormat::Quiet => {}
    }

    Ok(())
}
