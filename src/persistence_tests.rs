#[cfg(test)]
#[cfg(feature = "persistence")]
mod tests {
    use crate::hyperdim::HVec10240;
    use crate::persistence::Persistence;
    use crate::singularity::Concept;
    use std::collections::HashMap;
    use tempfile::NamedTempFile;

    fn make_concept(id: &str) -> Concept {
        Concept {
            id: id.to_string(),
            vector: HVec10240::random(),
            metadata: HashMap::new(),
            created_at: 0,
            modified_at: 0,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        }
    }

    #[tokio::test]
    async fn save_and_load_concept_roundtrip() {
        let temp = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp.path().to_str().expect("Invalid path");
        let persistence = Persistence::new_local(path)
            .await
            .expect("Failed to create persistence");

        let concept = make_concept("test-concept");

        persistence
            .save_concept(&concept)
            .await
            .expect("Failed to save");
        let loaded = persistence
            .load_concept("test-concept")
            .await
            .expect("Failed to load")
            .expect("Concept not found");
        assert_eq!(loaded.id, concept.id);
    }

    #[tokio::test]
    async fn delete_concept_removes_from_db() {
        let temp = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp.path().to_str().expect("Invalid path");
        let persistence = Persistence::new_local(path)
            .await
            .expect("Failed to create persistence");

        let concept = make_concept("delete-test");

        persistence
            .save_concept(&concept)
            .await
            .expect("Failed to save");
        persistence
            .delete_concept("delete-test")
            .await
            .expect("Failed to delete");
        let result = persistence.load_concept("delete-test").await;
        assert!(result.expect("Query failed").is_none());
    }

    #[tokio::test]
    async fn schema_version_initialized() {
        let temp = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp.path().to_str().expect("Invalid path");
        let persistence = Persistence::new_local(path)
            .await
            .expect("Failed to create persistence");

        let version = persistence
            .schema_version()
            .await
            .expect("Failed to get version");
        assert!(version > 0);
    }
}
