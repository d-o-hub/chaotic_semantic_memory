//! Atomic namespace-level mutations used by the framework's durable-first paths.

use crate::persistence::Persistence;
use csm_core_lib::error::{MemoryError, Result};
use csm_core_lib::hyperdim::Hypervector;
use csm_memory::Concept;
use libsql::params;

impl Persistence {
    /// Replace all durable namespace state in one transaction.
    pub async fn replace_namespace_transactional<H: Hypervector>(
        &self,
        ns: &str,
        concepts: &[Concept<H>],
        associations: &[(String, String, f32)],
    ) -> Result<u64> {
        self.import_namespace_transactional(ns, concepts, associations, true)
            .await
    }

    /// Merge concepts and associations in one durable transaction.
    pub async fn merge_namespace_transactional<H: Hypervector>(
        &self,
        ns: &str,
        concepts: &[Concept<H>],
        associations: &[(String, String, f32)],
    ) -> Result<u64> {
        self.import_namespace_transactional(ns, concepts, associations, false)
            .await
    }

    async fn import_namespace_transactional<H: Hypervector>(
        &self,
        ns: &str,
        concepts: &[Concept<H>],
        associations: &[(String, String, f32)],
        replace: bool,
    ) -> Result<u64> {
        crate::persistence_retry::with_retry(|| {
            self.import_namespace_transactional_once(ns, concepts, associations, replace)
        })
        .await
    }

    async fn import_namespace_transactional_once<H: Hypervector>(
        &self,
        ns: &str,
        concepts: &[Concept<H>],
        associations: &[(String, String, f32)],
        replace: bool,
    ) -> Result<u64> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute("BEGIN IMMEDIATE", ()).await.map_err(|e| {
            MemoryError::database(format!("Failed to begin import transaction: {e}"))
        })?;

        let result = async {
            if replace {
                for table in [
                    "csm_associations",
                    "csm_versions",
                    "csm_concepts",
                    "csm_hnsw_graph",
                    "csm_canonical",
                    "csm_absences",
                ] {
                    conn.execute(
                        &format!("DELETE FROM {table} WHERE namespace = ?1"),
                        params![ns],
                    )
                    .await
                    .map_err(|e| {
                        MemoryError::database(format!(
                            "Failed to clear {table} during replace: {e}"
                        ))
                    })?;
                }
            }

            let concept_sql = if replace {
                "INSERT INTO csm_concepts
                 (namespace, id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
            } else {
                "INSERT INTO csm_concepts
                 (namespace, id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(namespace, id) DO UPDATE SET
                    vector = excluded.vector,
                    metadata = excluded.metadata,
                    modified_at = excluded.modified_at,
                    expires_at = excluded.expires_at,
                    canonical_concept_ids_json = excluded.canonical_concept_ids_json"
            };
            for concept in concepts {
                let vector_bytes = concept.vector.to_bytes();
                let metadata_json = serde_json::to_string(&concept.metadata)?;
                let canonical_ids = serde_json::to_string(&concept.canonical_concept_ids)?;
                conn.execute(
                    concept_sql,
                    params![
                        ns,
                        concept.id.as_str(),
                        vector_bytes.as_slice(),
                        metadata_json.as_str(),
                        concept.created_at as i64,
                        concept.modified_at as i64,
                        concept.expires_at.map(|value| value as i64),
                        canonical_ids.as_str()
                    ],
                )
                .await
                .map_err(|e| MemoryError::database(format!("Failed to import concept: {e}")))?;
                self.record_concept_version_scoped(
                    &conn,
                    ns,
                    concept,
                    Some(&vector_bytes),
                    Some(&metadata_json),
                )
                .await?;
            }

            let association_sql = if replace {
                "INSERT INTO csm_associations (namespace, from_id, to_id, strength, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)"
            } else {
                "INSERT INTO csm_associations (namespace, from_id, to_id, strength, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(namespace, from_id, to_id) DO UPDATE SET
                    strength = excluded.strength, created_at = excluded.created_at"
            };
            let now = csm_memory::unix_now_secs() as i64;
            for (from, to, strength) in associations {
                conn.execute(
                    association_sql,
                    params![ns, from.as_str(), to.as_str(), *strength, now],
                )
                .await
                .map_err(|e| MemoryError::database(format!("Failed to import association: {e}")))?;
            }

            if replace || !concepts.is_empty() || !associations.is_empty() {
                self.bump_namespace_revision_with_conn(&conn, ns).await
            } else {
                Ok(0)
            }
        }
        .await;

        let revision = match result {
            Ok(revision) => revision,
            Err(error) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                return Err(error);
            }
        };
        conn.execute("COMMIT", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to commit import: {e}")))?;
        Ok(revision)
    }

    /// Atomically delete a set of concepts and every association/version they own.
    pub async fn delete_concepts(&self, ns: &str, ids: &[String]) -> Result<usize> {
        if ids.is_empty() {
            return Ok(0);
        }
        crate::persistence_retry::with_retry(|| self.delete_concepts_once(ns, ids)).await
    }

    async fn delete_concepts_once(&self, ns: &str, ids: &[String]) -> Result<usize> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute("BEGIN IMMEDIATE", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to begin concept deletion: {e}")))?;

        let result = async {
            let mut changed = false;
            let mut deleted_concepts = 0usize;
            for id in ids {
                for sql in [
                    "DELETE FROM csm_versions WHERE namespace = ?1 AND concept_id = ?2",
                    "DELETE FROM csm_associations WHERE namespace = ?1 AND (from_id = ?2 OR to_id = ?2)",
                    "DELETE FROM csm_canonical WHERE namespace = ?1 AND id = ?2",
                ] {
                    changed |= conn
                        .execute(sql, params![ns, id.as_str()])
                        .await
                        .map_err(|e| MemoryError::database(format!("Failed to delete concept state: {e}")))?
                        > 0;
                }
                let removed = conn
                    .execute(
                        "DELETE FROM csm_concepts WHERE namespace = ?1 AND id = ?2",
                        params![ns, id.as_str()],
                    )
                    .await
                    .map_err(|e| MemoryError::database(format!("Failed to delete concept: {e}")))?;
                changed |= removed > 0;
                deleted_concepts += usize::try_from(removed).unwrap_or(usize::MAX);
            }
            if changed {
                self.bump_namespace_revision_with_conn(&conn, ns).await?;
            }
            Ok::<usize, MemoryError>(deleted_concepts)
        }
        .await;

        let deleted = match result {
            Ok(deleted) => deleted,
            Err(error) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                return Err(error);
            }
        };
        conn.execute("COMMIT", ()).await.map_err(|e| {
            MemoryError::database(format!("Failed to commit concept deletion: {e}"))
        })?;
        Ok(deleted)
    }

    /// Delete specified graph edges atomically and advance the namespace revision once.
    pub async fn delete_associations(&self, ns: &str, edges: &[(String, String)]) -> Result<usize> {
        if edges.is_empty() {
            return Ok(0);
        }
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute("BEGIN IMMEDIATE", ()).await.map_err(|e| {
            MemoryError::database(format!("Failed to begin association deletion: {e}"))
        })?;

        let result = async {
            let mut deleted = 0usize;
            for (from, to) in edges {
                let changed = conn
                    .execute(
                        "DELETE FROM csm_associations WHERE namespace = ?1 AND from_id = ?2 AND to_id = ?3",
                        params![ns, from.as_str(), to.as_str()],
                    )
                    .await
                    .map_err(|e| MemoryError::database(format!("Failed to delete association: {e}")))?;
                deleted += usize::try_from(changed).unwrap_or(usize::MAX);
            }
            if deleted > 0 {
                self.bump_namespace_revision_with_conn(&conn, ns).await?;
            }
            Ok::<usize, MemoryError>(deleted)
        }
        .await;

        let deleted = match result {
            Ok(deleted) => deleted,
            Err(error) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                return Err(error);
            }
        };
        conn.execute("COMMIT", ()).await.map_err(|e| {
            MemoryError::database(format!("Failed to commit association deletion: {e}"))
        })?;
        Ok(deleted)
    }

    /// Reset an edge's decay clock and advance the namespace revision atomically.
    pub async fn reinforce_association(&self, ns: &str, from: &str, to: &str) -> Result<()> {
        let now = csm_memory::unix_now_secs() as i64;
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute("BEGIN IMMEDIATE", ()).await.map_err(|e| {
            MemoryError::database(format!("Failed to begin association reinforcement: {e}"))
        })?;

        let updated = match conn
            .execute(
                "UPDATE csm_associations SET created_at = ?4
                 WHERE namespace = ?1 AND from_id = ?2 AND to_id = ?3",
                params![ns, from, to, now],
            )
            .await
        {
            Ok(updated) => updated,
            Err(error) => {
                let _ = conn.execute("ROLLBACK", ()).await;
                return Err(MemoryError::database(format!(
                    "Failed to reinforce association: {error}"
                )));
            }
        };
        if updated == 0 {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(MemoryError::NotFound {
                entity: "Association".to_string(),
                id: format!("{from} -> {to}"),
            });
        }
        if let Err(error) = self.bump_namespace_revision_with_conn(&conn, ns).await {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(error);
        }
        conn.execute("COMMIT", ()).await.map_err(|e| {
            MemoryError::database(format!("Failed to commit association reinforcement: {e}"))
        })?;
        Ok(())
    }
}
