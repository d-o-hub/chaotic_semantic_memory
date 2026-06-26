use chaotic_semantic_memory::persistence::Persistence;

#[tokio::main]
async fn main() {
    let db_path = "test_migration.db";
    if std::path::Path::new(db_path).exists() {
        std::fs::remove_file(db_path).unwrap();
    }

    // Creating persistence with current code should initialize it to latest version (8)
    let p = Persistence::new_local(db_path)
        .await
        .expect("Failed to create persistence");

    let version = p
        .schema_version()
        .await
        .expect("Failed to get schema version");
    println!("Schema version: {version}");

    // We can also try to downgrade and upgrade if we had the old code,
    // but here we just want to verify the schema of the newly created DB.
}
