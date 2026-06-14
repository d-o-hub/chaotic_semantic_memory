use crate::persistence::Persistence;
use csm_core::error::{MemoryError, Result};
use csm_memory::{Concept, ConceptVersion};
use libsql::{Connection, params};

#[allow(dead_code)]
impl Persistence {
    pub(crate) async fn record_concept_version<H: csm_core::hyperdim::Hypervector>(
        &self,
        conn: &Connection,
        concept: &Concept<H>,
    ) -> Result<()> {
        self.record_concept_version_scoped(conn, "_default", concept, None, None)
            .await
    }

    /// Records a new concept version, optionally using pre-computed vector bytes and metadata JSON.
    /// Performance Optimization: Accepting pre-computed values avoids redundant serialization and
    /// allocations when this is called from batch operations or normal save paths.
    pub(crate) async fn record_concept_version_scoped<H: csm_core::hyperdim::Hypervector>(
        &self,
        conn: &Connection,
        ns: &str,
        concept: &Concept<H>,
        vector_bytes: Option<&[u8]>,
        metadata_json: Option<&str>,
    ) -> Result<()> {
        let mut rows = conn
            .query(
                "SELECT COALESCE(MAX(version), 0) FROM csm_versions WHERE namespace = ?1 AND concept_id = ?2",
                params![ns, concept.id.as_str()],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to query concept version: {e}")))?;

        let current = if let Some(row) = rows.next().await.map_err(|e| {
            MemoryError::database(format!("Failed to fetch concept version row: {e}"))
        })? {
            row.get::<i64>(0).map_err(|e| {
                MemoryError::database(format!("Failed to read version from row: {e}"))
            })?
        } else {
            0
        };
        let next_version = current + 1;
        let vector_bytes_owned: Vec<u8>;
        let vector_ref = if let Some(v) = vector_bytes {
            v
        } else {
            vector_bytes_owned = concept.vector.to_bytes();
            &vector_bytes_owned
        };

        let metadata_owned: String;
        let metadata_ref = if let Some(m) = metadata_json {
            m
        } else {
            metadata_owned = serde_json::to_string(&concept.metadata)?;
            &metadata_owned
        };

        conn.execute(
            "INSERT INTO csm_versions (namespace, concept_id, version, vector, metadata, modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                ns,
                concept.id.as_str(),
                next_version,
                vector_ref,
                metadata_ref,
                concept.modified_at as i64
            ],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to save concept version: {e}")))?;

        conn.execute(
            "DELETE FROM csm_versions
             WHERE namespace = ?1 AND concept_id = ?2
             AND version <= (
                SELECT MAX(version) - ?3 FROM csm_versions WHERE namespace = ?1 AND concept_id = ?2
             )",
            params![ns, concept.id.as_str(), self.version_retention as i64],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to prune concept versions: {e}")))?;

        Ok(())
    }

    /// Load a specific concept version from the database.
    pub async fn get_version_scoped<H: csm_core::hyperdim::Hypervector>(
        &self,
        ns: &str,
        id: &str,
        version: u64,
    ) -> Result<Option<Concept<H>>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT v.vector, v.metadata, v.modified_at, c.created_at, c.expires_at, c.canonical_concept_ids_json
                 FROM csm_versions v
                 LEFT JOIN csm_concepts c ON v.namespace = c.namespace AND v.concept_id = c.id
                 WHERE v.namespace = ?1 AND v.concept_id = ?2 AND v.version = ?3",
                params![ns, id, version as i64],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to load concept version: {e}")))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch row: {e}")))?
        {
            let vector_bytes: Vec<u8> = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to get vector bytes: {e}")))?;
            let metadata_json: String = row
                .get(1)
                .map_err(|e| MemoryError::database(format!("Failed to get metadata JSON: {e}")))?;
            let modified_at: i64 = row
                .get(2)
                .map_err(|e| MemoryError::database(format!("Failed to get modified_at: {e}")))?;

            let created_at: i64 = row.get(3).unwrap_or(modified_at);
            let expires_at: Option<i64> = row.get::<Option<i64>>(4).ok().flatten();
            let canonical_concept_ids_json: Option<String> =
                row.get::<Option<String>>(5).ok().flatten();

            let vector = H::from_bytes(&vector_bytes)?;
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

    /// List all versions of a concept in a namespace.
    pub async fn list_versions_scoped<H: csm_core::hyperdim::Hypervector>(
        &self,
        ns: &str,
        id: &str,
    ) -> Result<Vec<ConceptVersion<H>>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT version, vector, metadata, modified_at
                 FROM csm_versions
                 WHERE namespace = ?1 AND concept_id = ?2
                 ORDER BY version ASC",
                params![ns, id],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to list concept versions: {e}")))?;

        let mut list = Vec::new();
        let mut prev_vector: Option<Vec<u8>> = None;
        let mut prev_metadata: Option<String> = None;

        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch row: {e}")))?
        {
            let version: i64 = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to get version: {e}")))?;
            let vector_bytes: Vec<u8> = row
                .get(1)
                .map_err(|e| MemoryError::database(format!("Failed to get vector bytes: {e}")))?;
            let metadata_json: String = row
                .get(2)
                .map_err(|e| MemoryError::database(format!("Failed to get metadata JSON: {e}")))?;
            let modified_at: i64 = row
                .get(3)
                .map_err(|e| MemoryError::database(format!("Failed to get modified_at: {e}")))?;

            let vector_changed = if let Some(ref prev_v) = prev_vector {
                *prev_v != vector_bytes
            } else {
                true
            };

            let metadata_changed = if let Some(ref prev_m) = prev_metadata {
                *prev_m != metadata_json
            } else {
                true
            };

            prev_vector = Some(vector_bytes);
            prev_metadata = Some(metadata_json);

            list.push(ConceptVersion {
                concept_id: id.to_string(),
                version: version as u64,
                timestamp_unix: modified_at as u64,
                vector: None,
                metadata: None,
                vector_changed: Some(vector_changed),
                metadata_changed: Some(metadata_changed),
            });
        }

        Ok(list)
    }
}
