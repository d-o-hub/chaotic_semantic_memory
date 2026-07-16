//! ANN index persistence and namespace revision (ADR-0093).
//!
//! Extracted from persistence.rs to satisfy the 500 LOC gate.

use crate::index_envelope::IndexSnapshotEnvelope;
use crate::persistence::Persistence;
use csm_core::error::{MemoryError, Result};
use libsql::params;

impl Persistence {
    /// Save the serialized index state to the database (raw or pre-wrapped bytes).
    pub async fn save_index(&self, ns: &str, id: &str, data: &[u8]) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute(
            "INSERT OR REPLACE INTO csm_hnsw_graph (namespace, id, data, modified_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                ns.to_string(),
                id,
                data,
                crate::singularity::unix_now_secs() as i64
            ],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to save index: {e}")))?;
        Ok(())
    }

    /// Save a revisioned index envelope.
    pub async fn save_index_envelope(
        &self,
        ns: &str,
        id: &str,
        envelope: &IndexSnapshotEnvelope,
    ) -> Result<()> {
        envelope.validate_integrity()?;
        let encoded = envelope.encode()?;
        self.save_index(ns, id, &encoded).await
    }

    /// Load the serialized index state from the database.
    pub async fn load_index(&self, ns: &str, id: &str) -> Result<Option<Vec<u8>>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let mut rows = conn
            .query(
                "SELECT data FROM csm_hnsw_graph WHERE namespace = ?1 AND id = ?2",
                params![ns.to_string(), id],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to load index: {e}")))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch index row: {e}")))?
        {
            let data: Vec<u8> = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to get index data: {e}")))?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    /// Load and parse a revisioned index envelope. Legacy raw blobs yield `Ok(None)`.
    pub async fn load_index_envelope(
        &self,
        ns: &str,
        id: &str,
    ) -> Result<Option<IndexSnapshotEnvelope>> {
        match self.load_index(ns, id).await? {
            Some(bytes) => IndexSnapshotEnvelope::try_decode(&bytes),
            None => Ok(None),
        }
    }

    /// Current namespace revision (0 if no row).
    pub async fn get_namespace_revision(&self, ns: &str) -> Result<u64> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let mut rows = conn
            .query(
                "SELECT revision FROM csm_namespace_meta WHERE namespace = ?1",
                params![ns],
            )
            .await
            .map_err(|e| {
                MemoryError::database(format!("Failed to load namespace revision: {e}"))
            })?;

        if let Some(row) = rows.next().await.map_err(|e| {
            MemoryError::database(format!("Failed to fetch namespace revision row: {e}"))
        })? {
            let revision: i64 = row.get(0).map_err(|e| {
                MemoryError::database(format!("Failed to parse namespace revision: {e}"))
            })?;
            Ok(revision as u64)
        } else {
            Ok(0)
        }
    }

    /// Atomically increment namespace revision; returns the new value.
    pub async fn bump_namespace_revision(&self, ns: &str) -> Result<u64> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        self.bump_namespace_revision_with_conn(&conn, ns).await
    }

    /// Increment revision using an existing connection (for transactional callers).
    pub(crate) async fn bump_namespace_revision_with_conn(
        &self,
        conn: &libsql::Connection,
        ns: &str,
    ) -> Result<u64> {
        conn.execute(
            "INSERT INTO csm_namespace_meta (namespace, revision)
             VALUES (?1, 1)
             ON CONFLICT(namespace) DO UPDATE SET revision = revision + 1",
            params![ns],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to bump namespace revision: {e}")))?;

        let mut rows = conn
            .query(
                "SELECT revision FROM csm_namespace_meta WHERE namespace = ?1",
                params![ns],
            )
            .await
            .map_err(|e| {
                MemoryError::database(format!("Failed to read namespace revision: {e}"))
            })?;

        let row = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch revision row: {e}")))?
            .ok_or_else(|| MemoryError::database("namespace revision missing after bump"))?;

        let revision: i64 = row
            .get(0)
            .map_err(|e| MemoryError::database(format!("Failed to parse revision: {e}")))?;
        Ok(revision as u64)
    }

    /// Load all associations for a namespace in one query (ADR-0093 P1).
    pub async fn load_all_associations(&self, ns: &str) -> Result<Vec<(String, String, f32, u64)>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT from_id, to_id, strength, created_at
                 FROM csm_associations WHERE namespace = ?1",
                params![ns],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to bulk-load associations: {e}")))?;

        let mut associations = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch association row: {e}")))?
        {
            let from_id: String = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to get from_id: {e}")))?;
            let to_id: String = row
                .get(1)
                .map_err(|e| MemoryError::database(format!("Failed to get to_id: {e}")))?;
            let strength: f64 = row
                .get(2)
                .map_err(|e| MemoryError::database(format!("Failed to get strength: {e}")))?;
            let created_at: i64 = row
                .get(3)
                .map_err(|e| MemoryError::database(format!("Failed to get created_at: {e}")))?;
            associations.push((from_id, to_id, strength as f32, created_at as u64));
        }

        Ok(associations)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::index_envelope::IndexSnapshotEnvelope;

    #[tokio::test]
    async fn test_save_and_load_index_roundtrip() {
        let path = "/tmp/test_index_persist_rt.db";
        let _ = std::fs::remove_file(path);
        let persistence = Persistence::new_local(path).await.unwrap();

        let test_data = vec![1u8, 2, 3, 4, 5, 42, 99];
        persistence
            .save_index("default", "test-idx", &test_data)
            .await
            .unwrap();

        let loaded = persistence.load_index("default", "test-idx").await.unwrap();
        assert_eq!(loaded, Some(test_data));

        let missing = persistence
            .load_index("default", "no-such-idx")
            .await
            .unwrap();
        assert_eq!(missing, None);

        std::fs::remove_file(path).ok();
    }

    #[tokio::test]
    async fn namespace_revision_bumps() {
        let path = "/tmp/test_ns_revision.db";
        let _ = std::fs::remove_file(path);
        let persistence = Persistence::new_local(path).await.unwrap();
        assert_eq!(persistence.get_namespace_revision("ns").await.unwrap(), 0);
        assert_eq!(persistence.bump_namespace_revision("ns").await.unwrap(), 1);
        assert_eq!(persistence.bump_namespace_revision("ns").await.unwrap(), 2);
        assert_eq!(persistence.get_namespace_revision("ns").await.unwrap(), 2);
        std::fs::remove_file(path).ok();
    }

    #[tokio::test]
    async fn envelope_save_load() {
        let path = "/tmp/test_env_index.db";
        let _ = std::fs::remove_file(path);
        let persistence = Persistence::new_local(path).await.unwrap();
        let env = IndexSnapshotEnvelope::new(3, "bruteforce", vec![7, 8, 9]);
        persistence
            .save_index_envelope("ns", "main", &env)
            .await
            .unwrap();
        let loaded = persistence
            .load_index_envelope("ns", "main")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.namespace_revision, 3);
        assert_eq!(loaded.index_data, vec![7, 8, 9]);
        std::fs::remove_file(path).ok();
    }

    #[tokio::test]
    async fn bulk_associations_load() {
        let path = "/tmp/test_bulk_assoc.db";
        let _ = std::fs::remove_file(path);
        let persistence = Persistence::new_local(path).await.unwrap();
        let c1 = crate::singularity::ConceptBuilder::new("a")
            .with_vector(csm_core::hyperdim::HVec10240::zero())
            .build()
            .unwrap();
        let c2 = crate::singularity::ConceptBuilder::new("b")
            .with_vector(csm_core::hyperdim::HVec10240::zero())
            .build()
            .unwrap();
        persistence.save_concept("ns", &c1).await.unwrap();
        persistence.save_concept("ns", &c2).await.unwrap();
        persistence
            .save_association("ns", "a", "b", 0.9)
            .await
            .unwrap();
        let all = persistence.load_all_associations("ns").await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, "a");
        assert_eq!(all[0].1, "b");
        std::fs::remove_file(path).ok();
    }
}
