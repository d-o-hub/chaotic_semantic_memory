use std::path::Path;

use anyhow::{Context, Result};

use crate::cli::args::{ImportFormat, ImportArgs, OutputFormat};

use super::{create_framework, print_error, print_success, print_warning};

pub async fn run_import(args: ImportArgs, db_path: Option<&Path>, format: OutputFormat) -> Result<()> {
    if !args.input.exists() {
        print_error(&format!("file not found: {}", args.input.display()));
        anyhow::bail!("import file does not exist: {}", args.input.display());
    }

    let framework = create_framework(db_path)
        .await
        .context("failed to initialize framework")?;

    let path_str = args.input.to_string_lossy();
    let detected_format = detect_format(&args);

    if matches!(format, OutputFormat::Table) {
        let mode = if args.merge { "merge" } else { "replace" };
        eprintln!(
            "Importing from {} (format: {:?}, mode: {})...",
            args.input.display(),
            detected_format,
            mode
        );
    }

    let result = match detected_format {
        ImportFormat::Json => framework.import_json(&path_str, args.merge).await,
        ImportFormat::Binary => framework.import_binary(&path_str, args.merge).await,
        ImportFormat::Auto => {
            if is_binary_file(&args.input)? {
                framework.import_binary(&path_str, args.merge).await
            } else {
                framework.import_json(&path_str, args.merge).await
            }
        }
    };

    match result {
        Ok(count) => {
            if !args.merge {
                print_warning("existing state cleared before import", format);
            }
            print_success(
                &format!("imported {} concepts from {}", count, args.input.display()),
                format,
            );
            if matches!(format, OutputFormat::Json) {
                println!(
                    r#"{{"imported":{},"path":"{}","merge":{}}}"#,
                    count,
                    args.input.display(),
                    args.merge
                );
            }
        }
        Err(e) => {
            let err_str = e.to_string();
            let msg = if err_str.contains("version") || err_str.contains("deserialize") {
                format!("import failed: incompatible or corrupted file - {}", err_str)
            } else if err_str.contains("permission") || err_str.contains("denied") {
                format!("permission denied: {}", args.input.display())
            } else {
                format!("import failed: {}", err_str)
            };
            print_error(&msg);
            return Err(anyhow::anyhow!(msg));
        }
    }

    Ok(())
}

fn detect_format(args: &ImportArgs) -> ImportFormat {
    match args.format {
        ImportFormat::Auto => {
            let ext = args.input.extension().and_then(|s| s.to_str()).unwrap_or("");
            match ext.to_lowercase().as_str() {
                "bin" | "binary" | "dat" => ImportFormat::Binary,
                _ => ImportFormat::Json,
            }
        }
        other => other,
    }
}

fn is_binary_file(path: &Path) -> Result<bool> {
    let metadata = std::fs::metadata(path).context("failed to read file metadata")?;
    if metadata.len() < 4 {
        return Ok(false);
    }
    let mut file = std::fs::File::open(path).context("failed to open file")?;
    let mut header = [0u8; 4];
    use std::io::Read;
    file.read_exact(&mut header).context("failed to read file header")?;
    let is_text = header.iter().all(|&b| b.is_ascii_graphic() || b.is_ascii_whitespace() || b == b'{');
    Ok(!is_text)
}
