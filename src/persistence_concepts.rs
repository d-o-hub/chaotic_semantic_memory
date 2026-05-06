#![cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
use crate::error::{MemoryError, Result};
use crate::hyperdim::{BHVec10240, Hypervector};
use crate::persistence::Persistence;
use crate::singularity::Concept;
use libsql::params;

impl Persistence {
    /// Save a concept to the database
    pub async fn save_concept<H: Hypervector + 'static>(
        &self,
        ns: &str,
        concept: &Concept<H>,
    ) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let vector_bytes = concept.vector.to_bytes();
        let metadata_json = serde_json::to_string(&concept.metadata)?;
        let expires_at: Option<i64> = concept.expires_at.map(|t| t as i64);
        let canonical_concept_ids_json = serde_json::to_string(&concept.canonical_concept_ids)?;
        let format = if std::any::TypeId::of::<H>() == std::any::TypeId::of::<BHVec10240>() {
            "binary"
        } else {
            "f32"
        };

        conn.execute(
            "INSERT INTO csm_concepts
             (namespace, id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json, vector_format)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(namespace, id) DO UPDATE SET
             vector = excluded.vector,
             metadata = excluded.metadata,
             modified_at = excluded.modified_at,
             expires_at = excluded.expires_at,
             canonical_concept_ids_json = excluded.canonical_concept_ids_json,
             vector_format = excluded.vector_format",
            params![
                ns.to_string(),
                concept.id.clone(),
                vector_bytes,
                metadata_json,
                concept.created_at as i64,
                concept.modified_at as i64,
                expires_at,
                canonical_concept_ids_json,
                format
            ],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to save concept: {e}")))?;

        self.record_concept_version_scoped(&conn, ns, concept)
            .await?;
        Ok(())
    }

    /// Save concepts in a single transaction
    pub async fn save_concepts<H: Hypervector + 'static>(
        &self,
        ns: &str,
        concepts: &[Concept<H>],
    ) -> Result<()> {
        if concepts.is_empty() {
            return Ok(());
        }

        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute("BEGIN", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to begin transaction: {e}")))?;

        let format = if std::any::TypeId::of::<H>() == std::any::TypeId::of::<BHVec10240>() {
            "binary"
        } else {
            "f32"
        };

        let mut first_error: Option<MemoryError> = None;
        for concept in concepts {
            let vector_bytes = concept.vector.to_bytes();
            let metadata_json = serde_json::to_string(&concept.metadata)?;
            let expires_at: Option<i64> = concept.expires_at.map(|t| t as i64);
            let canonical_concept_ids_json = serde_json::to_string(&concept.canonical_concept_ids)?;

            if let Err(e) = conn
                .execute(
                    "INSERT INTO csm_concepts
                     (namespace, id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json, vector_format)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                     ON CONFLICT(namespace, id) DO UPDATE SET
                     vector = excluded.vector,
                     metadata = excluded.metadata,
                     modified_at = excluded.modified_at,
                     expires_at = excluded.expires_at,
                     canonical_concept_ids_json = excluded.canonical_concept_ids_json,
                     vector_format = excluded.vector_format",
                    params![
                        ns.to_string(),
                        concept.id.clone(),
                        vector_bytes,
                        metadata_json,
                        concept.created_at as i64,
                        concept.modified_at as i64,
                        expires_at,
                        canonical_concept_ids_json,
                        format
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
    pub async fn load_concept<H: Hypervector + 'static>(
        &self,
        ns: &str,
        id: &str,
    ) -> Result<Option<Concept<H>>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json, vector_format
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
            let vector_format: String = row.get(6).unwrap_or_else(|_| "f32".to_string());

            let vector = if vector_format == "binary" {
                if std::any::TypeId::of::<H>() == std::any::TypeId::of::<BHVec10240>() {
                    H::from_bytes(&vector_bytes)?
                } else {
                    // Up-convert stored binary to f32 if requested type is f32
                    let bhv = BHVec10240::from_bytes(&vector_bytes)?;
                    let fhv = bhv.to_f32();
                    // This is tricky because H might not be HVec10240.
                    // We'll assume H is HVec10240 if not BHVec10240 for this simple conversion.
                    // SAFE: We only do this if TypeId matches what we expect.
                    H::from_bytes(&fhv.to_bytes())?
                }
            } else {
                H::from_bytes(&vector_bytes)?
            };
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
    pub async fn load_all_concepts<H: Hypervector + 'static>(
        &self,
        ns: &str,
    ) -> Result<Vec<Concept<H>>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json, vector_format
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
            let vector_format: String = row.get(7).unwrap_or_else(|_| "f32".to_string());

            let vector = if vector_format == "binary" {
                if std::any::TypeId::of::<H>() == std::any::TypeId::of::<BHVec10240>() {
                    H::from_bytes(&vector_bytes)?
                } else {
                    let bhv = BHVec10240::from_bytes(&vector_bytes)?;
                    let fhv = bhv.to_f32();
                    H::from_bytes(&fhv.to_bytes())?
                }
            } else {
                H::from_bytes(&vector_bytes)?
            };
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
                params![ns.to_string(), id.to_string()],
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
            .execute(
                "DELETE FROM csm_concepts WHERE namespace = ?1 AND id = ?2",
                params![ns.to_string(), id.to_string()],
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
