#![cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;
use crate::persistence::Persistence;
use crate::singularity::Concept;
use libsql::params;

impl Persistence {
    /// Save a concept to the database
    pub async fn save_concept(&self, ns: &str, concept: &Concept) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let vector_bytes = concept.vector.to_bytes();
        let metadata_json = serde_json::to_string(&concept.metadata)?;
        let expires_at: Option<i64> = concept.expires_at.map(|t| t as i64);
        let canonical_concept_ids_json = serde_json::to_string(&concept.canonical_concept_ids)?;

        conn.execute(
            "INSERT INTO csm_concepts
             (namespace, id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(namespace, id) DO UPDATE SET
             vector = excluded.vector,
             metadata = excluded.metadata,
             modified_at = excluded.modified_at,
             expires_at = excluded.expires_at,
             canonical_concept_ids_json = excluded.canonical_concept_ids_json",
            params![
                ns,
                concept.id.as_str(),
                vector_bytes.as_slice(),
                metadata_json.as_str(),
                concept.created_at as i64,
                concept.modified_at as i64,
                expires_at,
                canonical_concept_ids_json.as_str()
            ],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to save concept: {e}")))?;

        self.record_concept_version_scoped(
            &conn,
            ns,
            concept,
            Some(&vector_bytes),
            Some(&metadata_json),
        )
        .await?;
        Ok(())
    }

    /// Save concepts in a single transaction
    pub async fn save_concepts(&self, ns: &str, concepts: &[Concept]) -> Result<()> {
        if concepts.is_empty() {
            return Ok(());
        }

        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute("BEGIN", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to begin transaction: {e}")))?;

        let mut first_error: Option<MemoryError> = None;
        for concept in concepts {
            let vector_bytes = concept.vector.to_bytes();
            let metadata_json = serde_json::to_string(&concept.metadata)?;
            let expires_at: Option<i64> = concept.expires_at.map(|t| t as i64);
            let canonical_concept_ids_json = serde_json::to_string(&concept.canonical_concept_ids)?;

            if let Err(e) = conn
                .execute(
                    "INSERT INTO csm_concepts
                     (namespace, id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                     ON CONFLICT(namespace, id) DO UPDATE SET
                     vector = excluded.vector,
                     metadata = excluded.metadata,
                     modified_at = excluded.modified_at,
                     expires_at = excluded.expires_at,
                     canonical_concept_ids_json = excluded.canonical_concept_ids_json",
                    params![
                        ns,
                        concept.id.as_str(),
                        vector_bytes.as_slice(),
                        metadata_json.as_str(),
                        concept.created_at as i64,
                        concept.modified_at as i64,
                        expires_at,
                        canonical_concept_ids_json.as_str()
                    ],
                )
                .await
            {
                first_error = Some(MemoryError::database(format!(
                    "Failed to batch save concept: {e}"
                )));
                break;
            }

            if let Err(e) = self
                .record_concept_version_scoped(
                    &conn,
                    ns,
                    concept,
                    Some(&vector_bytes),
                    Some(&metadata_json),
                )
                .await
            {
                first_error = Some(e);
                break;
            }
        }

        if let Some(error) = first_error {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(error);
        }

        conn.execute("COMMIT", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to commit transaction: {e}")))?;

        Ok(())
    }

    /// Load a concept from the database
    pub async fn load_concept(&self, ns: &str, id: &str) -> Result<Option<Concept>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json
                 FROM csm_concepts WHERE namespace = ?1 AND id = ?2",
                params![ns, id],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to load concept: {e}")))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch row: {e}")))?
        {
            let vector_bytes: Vec<u8> = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to get vector: {e}")))?;
            let metadata_json: String = row
                .get(1)
                .map_err(|e| MemoryError::database(format!("Failed to get metadata: {e}")))?;
            let created_at: i64 = row
                .get(2)
                .map_err(|e| MemoryError::database(format!("Failed to get created_at: {e}")))?;
            let modified_at: i64 = row
                .get(3)
                .map_err(|e| MemoryError::database(format!("Failed to get modified_at: {e}")))?;
            let expires_at: Option<i64> = row.get(4).ok();
            let canonical_concept_ids_json: Option<String> = row.get(5).ok();

            let vector = HVec10240::from_bytes(&vector_bytes)?;
            let metadata = serde_json::from_str(&metadata_json)?;
            let canonical_concept_ids = canonical_concept_ids_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?
                .unwrap_or_default();

            Ok(Some(Concept {
                id: id.to_string(),
                vector,
                metadata,
                created_at: created_at as u64,
                modified_at: modified_at as u64,
                expires_at: expires_at.map(|t| t as u64),
                canonical_concept_ids,
            }))
        } else {
            Ok(None)
        }
    }

    /// Load all concepts from the database for a specific namespace
    pub async fn load_all_concepts(&self, ns: &str) -> Result<Vec<Concept>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json
                 FROM csm_concepts WHERE namespace = ?1",
                params![ns],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to load concepts: {e}")))?;

        let mut concepts = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch row: {e}")))?
        {
            let id: String = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to get id: {e}")))?;
            let vector_bytes: Vec<u8> = row
                .get(1)
                .map_err(|e| MemoryError::database(format!("Failed to get vector: {e}")))?;
            let metadata_json: String = row
                .get(2)
                .map_err(|e| MemoryError::database(format!("Failed to get metadata: {e}")))?;
            let created_at: i64 = row
                .get(3)
                .map_err(|e| MemoryError::database(format!("Failed to get created_at: {e}")))?;
            let modified_at: i64 = row
                .get(4)
                .map_err(|e| MemoryError::database(format!("Failed to get modified_at: {e}")))?;
            let expires_at: Option<i64> = row.get(5).ok();
            let canonical_concept_ids_json: Option<String> = row.get(6).ok();

            let vector = HVec10240::from_bytes(&vector_bytes)?;
            let metadata = serde_json::from_str(&metadata_json)?;
            let canonical_concept_ids = canonical_concept_ids_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?
                .unwrap_or_default();

            concepts.push(Concept {
                id,
                vector,
                metadata,
                created_at: created_at as u64,
                modified_at: modified_at as u64,
                expires_at: expires_at.map(|t| t as u64),
                canonical_concept_ids,
            });
        }

        Ok(concepts)
    }

    /// Delete a concept from the database
    pub async fn delete_concept(&self, ns: &str, id: &str) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        conn.execute("BEGIN", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to begin transaction: {e}")))?;

        // Delete in FK order: versions first (FK → concepts), then associations (FK → concepts),
        // then the concept itself
        if let Err(e) = conn
            .execute(
                "DELETE FROM csm_versions WHERE namespace = ?1 AND concept_id = ?2",
                params![ns, id],
            )
            .await
        {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(MemoryError::database(format!(
                "Failed to delete concept versions: {e}"
            )));
        }

        if let Err(e) = conn
            .execute(
                "DELETE FROM csm_associations WHERE namespace = ?1 AND (from_id = ?2 OR to_id = ?2)",
                params![ns, id],
            )
            .await
        {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(MemoryError::database(format!(
                "Failed to delete associations: {e}"
            )));
        }

        if let Err(e) = conn
            .execute(
                "DELETE FROM csm_concepts WHERE namespace = ?1 AND id = ?2",
                params![ns, id],
            )
            .await
        {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(MemoryError::database(format!(
                "Failed to delete concept: {e}"
            )));
        }

        conn.execute("COMMIT", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to commit transaction: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
#[cfg(feature = "persistence")]
mod tests {
    use super::*;
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
    async fn test_save_concepts_empty() {
        let temp = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp.path().to_str().expect("Invalid path");
        let persistence = Persistence::new_local(path)
            .await
            .expect("Failed to create persistence");

        let result = persistence.save_concepts("test-ns", &[]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_save_concepts_batch() {
        let temp = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp.path().to_str().expect("Invalid path");
        let persistence = Persistence::new_local(path)
            .await
            .expect("Failed to create persistence");

        let ns = "test-ns";
        let concepts = vec![make_concept("c1"), make_concept("c2"), make_concept("c3")];

        persistence
            .save_concepts(ns, &concepts)
            .await
            .expect("Failed to batch save");

        let loaded = persistence
            .load_all_concepts(ns)
            .await
            .expect("Failed to load");
        assert_eq!(loaded.len(), 3);

        let ids: Vec<String> = loaded.into_iter().map(|c| c.id).collect();
        assert!(ids.contains(&"c1".to_string()));
        assert!(ids.contains(&"c2".to_string()));
        assert!(ids.contains(&"c3".to_string()));
    }

    #[tokio::test]
    async fn test_save_concepts_conflict_update() {
        let temp = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp.path().to_str().expect("Invalid path");
        let persistence = Persistence::new_local(path)
            .await
            .expect("Failed to create persistence");

        let ns = "test-ns";
        let c1 = make_concept("c1");
        persistence
            .save_concept(ns, &c1)
            .await
            .expect("Failed to save");

        let mut c1_updated = c1.clone();
        c1_updated.modified_at = 12345;

        let batch = vec![c1_updated, make_concept("c2")];
        persistence
            .save_concepts(ns, &batch)
            .await
            .expect("Failed to batch save with conflict");

        let loaded_c1 = persistence
            .load_concept(ns, "c1")
            .await
            .expect("Failed to load")
            .unwrap();
        assert_eq!(loaded_c1.modified_at, 12345);

        let all = persistence
            .load_all_concepts(ns)
            .await
            .expect("Failed to load all");
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_load_all_concepts_empty_ns() {
        let temp = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp.path().to_str().expect("Invalid path");
        let persistence = Persistence::new_local(path)
            .await
            .expect("Failed to create persistence");

        let loaded = persistence
            .load_all_concepts("non-existent")
            .await
            .expect("Failed to load");
        assert!(loaded.is_empty());
    }
}
