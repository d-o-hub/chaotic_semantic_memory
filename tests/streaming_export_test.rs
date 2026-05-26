use chaotic_semantic_memory::prelude::*;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_streaming_export_json_roundtrip() -> Result<()> {
    let framework = FrameworkBuilder::new()
        .without_persistence()
        .build()
        .await?;

    // Inject some concepts and associations
    for i in 0..100 {
        framework
            .inject_concept(format!("c{}", i), HVec10240::random())
            .await?;
    }
    for i in 0..99 {
        framework
            .associate(&format!("c{}", i), &format!("c{}", i + 1), 0.5)
            .await?;
    }

    let temp = NamedTempFile::new().map_err(MemoryError::Io)?;
    let path = temp.path().to_str().unwrap();

    // Export using streaming
    framework.export_json(path).await?;

    // Import and verify
    let new_framework = FrameworkBuilder::new()
        .without_persistence()
        .build()
        .await?;

    let count = new_framework.import_json(path, false).await?;
    assert_eq!(count, 100);

    let stats = new_framework.stats().await?;
    assert_eq!(stats.concept_count, 100);

    Ok(())
}

#[tokio::test]
async fn test_import_exceeds_max_size() -> Result<()> {
    let framework = FrameworkBuilder::new()
        .without_persistence()
        .build()
        .await?;

    // Create a dummy file that exceeds 100MB (but our new limit is 512MB)
    // Actually, let's just test the error path by providing a large dummy vec.
    #[allow(clippy::cast_possible_truncation)]
    let large_data = vec![0u8; (MAX_IMPORT_SIZE + 1) as usize];
    let temp = NamedTempFile::new().map_err(MemoryError::Io)?;
    std::fs::write(temp.path(), large_data).map_err(MemoryError::Io)?;

    let result = framework
        .import_json(temp.path().to_str().unwrap(), false)
        .await;
    assert!(result.is_err());
    match result.unwrap_err() {
        MemoryError::InvalidInput { field, .. } => assert_eq!(field, "import_data"),
        e => panic!("Expected InvalidInput error, got {:?}", e),
    }

    Ok(())
}

#[tokio::test]
async fn test_streaming_export_binary_roundtrip() -> Result<()> {
    let framework = FrameworkBuilder::new()
        .without_persistence()
        .build()
        .await?;

    // Inject some concepts and associations
    for i in 0..100 {
        framework
            .inject_concept(format!("c{}", i), HVec10240::random())
            .await?;
    }
    for i in 0..99 {
        framework
            .associate(&format!("c{}", i), &format!("c{}", i + 1), 0.5)
            .await?;
    }

    let temp = NamedTempFile::new().map_err(MemoryError::Io)?;
    let path = temp.path().to_str().unwrap();

    // Export using streaming
    framework.export_binary(path).await?;

    // Import and verify
    let new_framework = FrameworkBuilder::new()
        .without_persistence()
        .build()
        .await?;

    let count = new_framework.import_binary(path, false).await?;
    assert_eq!(count, 100);

    let stats = new_framework.stats().await?;
    assert_eq!(stats.concept_count, 100);

    Ok(())
}

#[tokio::test]
#[cfg(feature = "persistence")]
async fn test_streaming_export_persistence_roundtrip() -> Result<()> {
    let db_temp = NamedTempFile::new().map_err(MemoryError::Io)?;
    let db_path = db_temp.path().to_str().unwrap();

    let framework = FrameworkBuilder::new()
        .with_local_db(db_path)
        .build()
        .await?;

    // Inject some concepts and associations
    for i in 0..50 {
        framework
            .inject_concept(format!("p{}", i), HVec10240::random())
            .await?;
    }
    for i in 0..49 {
        framework
            .associate(&format!("p{}", i), &format!("p{}", i + 1), 0.7)
            .await?;
    }

    let export_temp = NamedTempFile::new().map_err(MemoryError::Io)?;
    let export_path = export_temp.path().to_str().unwrap();

    // Export using streaming from persistence
    framework.export_json(export_path).await?;

    // Import into a new framework
    let db_temp2 = NamedTempFile::new().map_err(MemoryError::Io)?;
    let db_path2 = db_temp2.path().to_str().unwrap();
    let new_framework = FrameworkBuilder::new()
        .with_local_db(db_path2)
        .build()
        .await?;

    let count = new_framework.import_json(export_path, false).await?;
    assert_eq!(count, 50);

    let stats = new_framework.stats().await?;
    assert_eq!(stats.concept_count, 50);

    Ok(())
}
