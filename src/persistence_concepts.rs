use libsql::params;
use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;
use crate::singularity::Concept;
use crate::persistence::Persistence;

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
            "INSERT OR REPLACE INTO csm_concepts
             (namespace, id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                ns.to_string(),
                concept.id.clone(),
                vector_bytes,
                metadata_json,
                concept.created_at as i64,
                concept.modified_at as i64,
                expires_at,
                canonical_concept_ids_json
            ],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to save concept: {e}")))?;

        self.record_concept_version_scoped(&conn, ns, concept).await?;
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
                    "INSERT OR REPLACE INTO csm_concepts
                     (namespace, id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        ns.to_string(),
                        concept.id.clone(),
                        vector_bytes,
                        metadata_json,
                        concept.created_at as i64,
                        concept.modified_at as i64,
                        expires_at,
                        canonical_concept_ids_json
                    ],
                )
                .await
            {
                first_error = Some(MemoryError::database(format!(
                    "Failed to batch save concept: {e}"
                )));
                break;
            }

            if let Err(e) = self.record_concept_version_scoped(&conn, ns, concept).await {
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
                params![ns.to_string(), id.to_string()],
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
                params![ns.to_string()],
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

        if let Err(e) = conn
            .execute(
                "DELETE FROM csm_associations WHERE namespace = ?1 AND (from_id = ?2 OR to_id = ?2)",
                params![ns.to_string(), id.to_string()],
            )
            .await
        {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(MemoryError::database(format!(
                "Failed to delete associations: {e}"
            )));
        }

        if let Err(e) = conn
            .execute("DELETE FROM csm_concepts WHERE namespace = ?1 AND id = ?2", params![ns.to_string(), id.to_string()])
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
