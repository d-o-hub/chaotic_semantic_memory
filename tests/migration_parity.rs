//! Structural parity checks for migration files and ADR references.
//!
//! 1. Migration header parity: each `migrations/NNN_*.sql` file must have
//!    `-- Migration NNN:` as its first line, matching the filename prefix.
//! 2. ADR migration reference parity: every `` `NNN_name.sql` `` reference
//!    in `plans/adr/*.md` must correspond to an existing file in
//!    `migrations/`. Proposed-status ADRs are allowed to reference future
//!    migrations that do not yet exist on disk.

use std::path::{Path, PathBuf};

fn crate_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn migrations_dir() -> PathBuf {
    crate_root().join("migrations")
}

fn adr_dir() -> PathBuf {
    crate_root().join("plans").join("adr")
}

fn is_proposed_adr(path: &Path) -> bool {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    content.lines().any(|l| l.trim().starts_with("Proposed"))
}

#[test]
fn migration_file_headers_match_filenames() {
    let dir = migrations_dir();
    let mut failures = Vec::new();

    for entry in std::fs::read_dir(&dir).expect("migrations/ directory not found") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("sql") {
            continue;
        }

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("non-UTF8 migration filename");

        let expected_prefix: &str = &stem[..3];
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

        let first_line = content.lines().next().unwrap_or("");
        let expected_header = format!("-- Migration {expected_prefix}:");
        if !first_line.starts_with(&expected_header) {
            let filename = path.file_name().unwrap().to_str().unwrap();
            failures.push(format!(
                "{filename}: expected header starting with \"{expected_header}\", got \"{first_line}\"",
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "Migration header/filename mismatches:\n  {}",
        failures.join("\n  "),
    );
}

#[test]
fn adr_migration_references_exist_on_disk() {
    let adr = adr_dir();
    let mig = migrations_dir();

    if !adr.exists() {
        eprintln!("plans/adr/ not found — skipping ADR migration reference check");
        return;
    }

    let mut failures = Vec::new();
    let mut proposed_skipped = Vec::new();

    for entry in std::fs::read_dir(&adr).expect("plans/adr/ directory not found") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let is_proposed = is_proposed_adr(&path);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

        let adr_filename = path.file_name().unwrap().to_str().unwrap().to_string();
        for (lineno, line) in content.lines().enumerate() {
            let lineno = lineno + 1;
            for captured in line
                .split('`')
                .enumerate()
                .filter(|(i, _)| i % 2 == 1)
                .map(|(_, s)| s)
            {
                let trimmed = captured.trim();
                if !trimmed.ends_with(".sql") {
                    continue;
                }
                if !trimmed.chars().take(3).all(|c| c.is_ascii_digit()) {
                    continue;
                }
                let mig_path = mig.join(trimmed);
                if !mig_path.exists() {
                    if is_proposed {
                        proposed_skipped.push(format!(
                            "{adr_filename}:{lineno} (Proposed, skipped): references \"{trimmed}\" (not yet on disk)",
                        ));
                    } else {
                        failures.push(format!(
                            "{adr_filename}:{lineno}: references \"{trimmed}\" but no such file in migrations/",
                        ));
                    }
                }
            }
        }
    }

    if !proposed_skipped.is_empty() {
        eprintln!(
            "info: Proposed ADRs reference future migrations (not checked):\n  {}",
            proposed_skipped.join("\n  "),
        );
    }

    assert!(
        failures.is_empty(),
        "ADR migration reference mismatches:\n  {}",
        failures.join("\n  "),
    );
}
