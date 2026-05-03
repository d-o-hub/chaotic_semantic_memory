use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

const BINARY: &str = "csm";

fn csm(db: &PathBuf) -> Command {
    let mut cmd = Command::new(BINARY);
    cmd.arg("--database").arg(db);
    cmd
}

fn run(cmd: &mut Command) -> Output {
    cmd.output().expect("failed to execute csm binary")
}

fn assert_success(output: &Output, context: &str) {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("{context} failed: {stderr}");
    }
}

fn assert_failure(output: &Output, context: &str) {
    if output.status.success() {
        panic!("{context} should have failed but succeeded");
    }
}

fn temp_db() -> PathBuf {
    tempfile::NamedTempFile::new()
        .expect("temp file")
        .into_temp_path()
        .to_path_buf()
}

fn temp_file() -> PathBuf {
    tempfile::NamedTempFile::new()
        .expect("temp file")
        .into_temp_path()
        .to_path_buf()
}

fn inject_with_random_vector(db: &PathBuf, concept_id: &str) {
    let output = run(csm(db).arg("inject").arg(concept_id));
    assert_success(&output, &format!("inject {concept_id}"));
    println!("[OK] Injected concept '{concept_id}' with random vector");
}

fn inject_from_file(db: &PathBuf, concept_id: &str, vector_file: &PathBuf) {
    let output = run(csm(db)
        .arg("inject")
        .arg(concept_id)
        .arg("--from-file")
        .arg(vector_file));
    assert_success(&output, &format!("inject {concept_id} from file"));
    println!("[OK] Injected concept '{concept_id}' from file");
}

fn probe(db: &PathBuf, concept_id: &str, top_k: usize, threshold: Option<f64>) -> Output {
    let mut cmd = csm(db);
    cmd.arg("probe")
        .arg(concept_id)
        .arg("-k")
        .arg(top_k.to_string());
    if let Some(t) = threshold {
        cmd.arg("--threshold").arg(t.to_string());
    }
    let output = run(&mut cmd);
    assert_success(&output, &format!("probe {concept_id}"));
    println!(
        "[OK] Probed for '{concept_id}' (top_k={top_k}, threshold={threshold:?})"
    );
    output
}

fn associate(db: &PathBuf, source: &str, target: &str, strength: f64) {
    let output = run(csm(db)
        .arg("associate")
        .arg(source)
        .arg(target)
        .arg("-s")
        .arg(strength.to_string()));
    assert_success(&output, &format!("associate {source} -> {target}"));
    println!(
        "[OK] Associated '{source}' -> '{target}' (strength={strength})"
    );
}

fn export_to_json(db: &PathBuf, output_path: &PathBuf) {
    let output = run(csm(db).arg("export").arg("-o").arg(output_path));
    assert_success(&output, "export");
    println!("[OK] Exported to '{}'", output_path.display());
}

fn import_from_json(db: &PathBuf, input_path: &PathBuf, merge: bool) {
    let mut cmd = csm(db);
    cmd.arg("import").arg(input_path);
    if merge {
        cmd.arg("--merge");
    }
    let output = run(&mut cmd);
    assert_success(&output, "import");
    println!("[OK] Imported from '{}'", input_path.display());
}

fn main() {
    let db = temp_db();
    let export_file = temp_file();
    let vector_file = temp_file();

    fs::write(&vector_file, "0.1 0.2 0.3 0.4 0.5").expect("write vector file");

    println!("=== 1. Injecting concepts with vectors ===\n");
    inject_with_random_vector(&db, "cat");
    inject_with_random_vector(&db, "dog");
    inject_with_random_vector(&db, "mammal");
    inject_from_file(&db, "custom_vector", &vector_file);

    println!("\n=== 2. Probing for similar concepts ===\n");
    let probe_output = probe(&db, "cat", 10, None);
    println!(
        "Probe result:\n{}",
        String::from_utf8_lossy(&probe_output.stdout)
    );

    let probe_filtered = probe(&db, "cat", 5, Some(0.5));
    println!(
        "Filtered probe:\n{}",
        String::from_utf8_lossy(&probe_filtered.stdout)
    );

    println!("\n=== 3. Creating associations ===\n");
    associate(&db, "cat", "mammal", 0.9);
    associate(&db, "dog", "mammal", 0.85);
    associate(&db, "cat", "dog", 0.3);

    println!("\n=== 4. Exporting data ===\n");
    export_to_json(&db, &export_file);

    let export_content = fs::read_to_string(&export_file).expect("read export");
    println!("Export preview: {} bytes", export_content.len());

    println!("\n=== 5. Importing data (new DB) ===\n");
    let new_db = temp_db();
    import_from_json(&new_db, &export_file, false);

    let probe_after_import = probe(&new_db, "cat", 10, None);
    println!(
        "Post-import probe:\n{}",
        String::from_utf8_lossy(&probe_after_import.stdout)
    );

    println!("\n=== 6. Edge cases ===\n");

    let duplicate = run(csm(&db).arg("inject").arg("cat"));
    assert_success(&duplicate, "inject duplicate (should succeed/overwrite)");
    println!("[OK] Duplicate injection handled");

    let missing_probe = run(csm(&new_db).arg("probe").arg("nonexistent"));
    assert_failure(&missing_probe, "probe nonexistent");
    println!("[OK] Missing concept probe fails as expected");

    let missing_assoc = run(csm(&new_db).arg("associate").arg("x").arg("y"));
    assert_failure(&missing_assoc, "associate missing concepts");
    println!("[OK] Association with missing concepts fails as expected");

    let neg_strength = run(csm(&db)
        .arg("associate")
        .arg("cat")
        .arg("dog")
        .arg("-s")
        .arg("-1.0"));
    assert_failure(&neg_strength, "negative strength");
    println!("[OK] Negative strength rejected as expected");

    let zero_k = run(csm(&db).arg("probe").arg("cat").arg("-k").arg("0"));
    assert_failure(&zero_k, "zero top_k");
    println!("[OK] Zero top_k rejected as expected");

    let version = run(Command::new(BINARY).arg("version"));
    assert_success(&version, "version");
    println!(
        "[OK] Version: {}",
        String::from_utf8_lossy(&version.stdout).trim()
    );

    println!("\n=== All CLI operations completed successfully! ===");
}
