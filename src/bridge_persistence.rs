//! Persistence for canonical concept graph.
//!
//! Feature-gated persistence layer for storing the symbolic semantic graph
//! used by the bridge retrieval pipeline.

// Casts are intentional for version serialization

use crate::persistence::Persistence;
use crate::retrieval::hybrid::RetrievalAbstention;
use crate::semantic_bridge::{CanonicalConcept, ConceptGraph};
use chrono::{DateTime, Utc};
use csm_core::error::{MemoryError, Result};
use libsql::params;

impl Persistence {
    /// Save a canonical concept to the database.
    pub async fn save_canonical_concept(&self, ns: &str, concept: &CanonicalConcept) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let labels_json = serde_json::to_string(&concept.labels)?;
        let related_json = serde_json::to_string(&concept.related)?;

        conn.execute(
            "INSERT INTO csm_canonical (namespace, id, version, labels_json, related_json)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(namespace, id) DO UPDATE SET
             version = excluded.version,
             labels_json = excluded.labels_json,
             related_json = excluded.related_json",
            params![
                ns.to_string(),
                concept.id.clone(),
                concept.version as i64,
                labels_json,
                related_json
            ],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to save canonical concept: {e}")))?;

        Ok(())
    }

    /// Delete a canonical concept from the database.
    pub async fn delete_canonical_concept(&self, ns: &str, id: &str) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        conn.execute(
            "DELETE FROM csm_canonical WHERE namespace = ?1 AND id = ?2",
            params![ns.to_string(), id],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to delete canonical concept: {e}")))?;

        Ok(())
    }

    /// Load a canonical concept by namespace and ID.
    pub async fn load_canonical_concept(
        &self,
        ns: &str,
        id: &str,
    ) -> Result<Option<CanonicalConcept>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT id, version, labels_json, related_json FROM csm_canonical WHERE namespace = ?1 AND id = ?2",
                params![ns.to_string(), id],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to load canonical concept: {e}")))?;

        if let Some(row) = rows.next().await.map_err(|e| {
            MemoryError::database(format!("Failed to read canonical concept row: {e}"))
        })? {
            let id: String = row.get(0).map_err(|e| {
                MemoryError::database(format!("Failed to read canonical concept id: {e}"))
            })?;
            let version: i64 = row.get(1).map_err(|e| {
                MemoryError::database(format!("Failed to read canonical concept version: {e}"))
            })?;
            let labels_json: String = row.get(2).map_err(|e| {
                MemoryError::database(format!("Failed to read canonical concept labels: {e}"))
            })?;
            let related_json: String = row.get(3).map_err(|e| {
                MemoryError::database(format!("Failed to read canonical concept related: {e}"))
            })?;

            let labels: Vec<String> = serde_json::from_str(&labels_json)?;
            let related: Vec<String> = serde_json::from_str(&related_json)?;

            Ok(Some(CanonicalConcept {
                id,
                version: u32::try_from(version).unwrap_or(0),
                labels,
                related,
            }))
        } else {
            Ok(None)
        }
    }

    /// Load all canonical concepts for a namespace from the database.
    pub async fn load_all_canonical_concepts(&self, ns: &str) -> Result<Vec<CanonicalConcept>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT id, version, labels_json, related_json FROM csm_canonical WHERE namespace = ?1",
                params![ns.to_string()],
            )
            .await
            .map_err(|e| {
                MemoryError::database(format!("Failed to load canonical concepts: {e}"))
            })?;

        let mut concepts = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| {
            MemoryError::database(format!("Failed to read canonical concept row: {e}"))
        })? {
            let id: String = row.get(0).map_err(|e| {
                MemoryError::database(format!("Failed to read canonical concept id: {e}"))
            })?;
            let version: i64 = row.get(1).map_err(|e| {
                MemoryError::database(format!("Failed to read canonical concept version: {e}"))
            })?;
            let labels_json: String = row.get(2).map_err(|e| {
                MemoryError::database(format!("Failed to read canonical concept labels: {e}"))
            })?;
            let related_json: String = row.get(3).map_err(|e| {
                MemoryError::database(format!("Failed to read canonical concept related: {e}"))
            })?;

            let labels: Vec<String> = serde_json::from_str(&labels_json)?;
            let related: Vec<String> = serde_json::from_str(&related_json)?;

            concepts.push(CanonicalConcept {
                id,
                version: u32::try_from(version).unwrap_or(0),
                labels,
                related,
            });
        }

        Ok(concepts)
    }

    /// Save an entire concept graph to the database for a namespace.
    pub async fn save_concept_graph(&self, ns: &str, graph: &ConceptGraph) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        conn.execute("BEGIN", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to begin transaction: {e}")))?;

        // Clear existing concepts for this namespace
        if let Err(e) = conn
            .execute(
                "DELETE FROM csm_canonical WHERE namespace = ?1",
                params![ns.to_string()],
            )
            .await
        {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(MemoryError::database(format!(
                "Failed to clear canonical concepts: {e}"
            )));
        }

        // Insert all concepts
        let mut first_error: Option<MemoryError> = None;
        for concept in graph.all_concepts() {
            let labels_json = match serde_json::to_string(&concept.labels) {
                Ok(j) => j,
                Err(e) => {
                    first_error = Some(MemoryError::Serialization(e));
                    break;
                }
            };
            let related_json = match serde_json::to_string(&concept.related) {
                Ok(j) => j,
                Err(e) => {
                    first_error = Some(MemoryError::Serialization(e));
                    break;
                }
            };

            if let Err(e) = conn
                .execute(
                    "INSERT INTO csm_canonical (namespace, id, version, labels_json, related_json)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        ns.to_string(),
                        concept.id.clone(),
                        concept.version as i64,
                        labels_json,
                        related_json
                    ],
                )
                .await
            {
                first_error = Some(MemoryError::database(format!(
                    "Failed to save canonical concept: {e}"
                )));
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

    /// Load an entire concept graph from the database for a namespace.
    pub async fn load_concept_graph(&self, ns: &str) -> Result<ConceptGraph> {
        let concepts = self.load_all_canonical_concepts(ns).await?;
        let mut graph = ConceptGraph::new();
        for concept in concepts {
            graph.add_concept(concept);
        }
        Ok(graph)
    }
}

/// A persisted record of a retrieval event that found no matching concepts.
/// Used to prevent re-querying known-absent concepts and to surface
/// memory gaps to operators.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AbsenceEntry {
    /// Stable ID derived from a hash of the normalized query string
    pub id: String,
    /// The original query that produced no results
    pub query: String,
    /// Normalized form of the query (lowercased, trimmed)
    pub normalized_query: String,
    /// How many times this query has been attempted with no result
    pub attempt_count: u32,
    /// Threshold that was not met on the last attempt
    pub last_threshold: f32,
    /// Best score seen across all attempts
    pub best_score_ever: Option<f32>,
    /// Timestamp of first absence event for this query
    pub first_seen: DateTime<Utc>,
    /// Timestamp of most recent absence event
    pub last_seen: DateTime<Utc>,
}

impl AbsenceEntry {
    /// Normalize a query string for stable ID derivation.
    pub fn normalize(query: &str) -> String {
        query.trim().to_lowercase()
    }

    /// Derive a stable string ID from the normalized query.
    ///
    /// Uses FNV-1a (64-bit) for deterministic hashing across Rust versions
    /// and platforms, matching the crate's text encoding pipeline.
    pub fn id_for(query: &str) -> String {
        let normalized = Self::normalize(query);
        let hash = Self::fnv1a_hash(normalized.as_bytes());
        format!("absence:{hash:016x}")
    }

    /// FNV-1a 64-bit hash (stable across Rust versions/platforms).
    fn fnv1a_hash(bytes: &[u8]) -> u64 {
        const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01b3;
        let mut hash = OFFSET_BASIS;
        for &byte in bytes {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(PRIME);
        }
        hash
    }

    /// Create a new entry from a RetrievalAbstention event.
    pub fn from_abstention(abstention: &RetrievalAbstention) -> Self {
        let normalized = Self::normalize(&abstention.query);
        AbsenceEntry {
            id: Self::id_for(&abstention.query),
            query: abstention.query.clone(),
            normalized_query: normalized,
            attempt_count: 1,
            last_threshold: abstention.min_score_threshold,
            best_score_ever: abstention.best_score_seen,
            first_seen: abstention.timestamp,
            last_seen: abstention.timestamp,
        }
    }

    /// Merge a new abstention event into an existing entry (upsert logic).
    pub fn merge_with(&mut self, abstention: &RetrievalAbstention) {
        self.attempt_count += 1;
        self.last_seen = abstention.timestamp;
        self.last_threshold = abstention.min_score_threshold;

        match (abstention.best_score_seen, self.best_score_ever) {
            (Some(new), Some(existing)) => {
                if new > existing {
                    self.best_score_ever = Some(new);
                }
            }
            (Some(new), None) => {
                self.best_score_ever = Some(new);
            }
            _ => {}
        }
    }
}

/// Persistence backend for absence entries.
#[async_trait::async_trait]
pub trait AbsenceStore: Send + Sync {
    /// Load an absence entry by ID.
    async fn get_absence(&self, id: &str) -> Result<Option<AbsenceEntry>>;
    /// Save or update an absence entry.
    async fn upsert_absence(&self, entry: &AbsenceEntry) -> Result<()>;
    /// Return all absence entries with attempt_count >= min_attempts.
    async fn list_absences(&self, min_attempts: u32) -> Result<Vec<AbsenceEntry>>;
}

/// Persist a RetrievalAbstention event as an AbsenceEntry.
pub async fn persist_absence(
    abstention: &RetrievalAbstention,
    store: &dyn AbsenceStore,
) -> Result<AbsenceEntry> {
    let id = AbsenceEntry::id_for(&abstention.query);
    match store.get_absence(&id).await? {
        Some(mut existing) => {
            existing.merge_with(abstention);
            store.upsert_absence(&existing).await?;
            Ok(existing)
        }
        None => {
            let entry = AbsenceEntry::from_abstention(abstention);
            store.upsert_absence(&entry).await?;
            Ok(entry)
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::semantic_bridge::CanonicalConcept;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_save_and_load_canonical_concept() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_str().unwrap();
        let persistence = Persistence::new_local(path).await.unwrap();

        let concept = CanonicalConcept::new("test-concept")
            .with_label("label1")
            .with_label("label2")
            .with_related("related-concept");

        persistence
            .save_canonical_concept("_default", &concept)
            .await
            .unwrap();

        let loaded = persistence
            .load_canonical_concept("_default", "test-concept")
            .await
            .unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.id, "test-concept");
        assert_eq!(loaded.labels, vec!["label1", "label2"]);
        assert_eq!(loaded.related, vec!["related-concept"]);
    }

    #[tokio::test]
    async fn test_delete_canonical_concept() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_str().unwrap();
        let persistence = Persistence::new_local(path).await.unwrap();

        let concept = CanonicalConcept::new("to-delete");
        persistence
            .save_canonical_concept("_default", &concept)
            .await
            .unwrap();

        persistence
            .delete_canonical_concept("_default", "to-delete")
            .await
            .unwrap();

        let loaded = persistence
            .load_canonical_concept("_default", "to-delete")
            .await
            .unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_save_and_load_concept_graph() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_str().unwrap();
        let persistence = Persistence::new_local(path).await.unwrap();

        let mut graph = ConceptGraph::new();
        graph.add_concept(
            CanonicalConcept::new("c1")
                .with_label("label1")
                .with_related("c2"),
        );
        graph.add_concept(CanonicalConcept::new("c2").with_label("label2"));

        persistence
            .save_concept_graph("_default", &graph)
            .await
            .unwrap();

        let loaded = persistence.load_concept_graph("_default").await.unwrap();
        assert_eq!(loaded.concept_count(), 2);
        assert_eq!(loaded.label_count(), 2);
    }

    #[tokio::test]
    async fn test_absence_id_generation() {
        let q1 = "  Test Query  ";
        let q2 = "test query";
        let id1 = AbsenceEntry::id_for(q1);
        let id2 = AbsenceEntry::id_for(q2);
        assert_eq!(id1, id2);
        assert!(id1.starts_with("absence:"));
    }

    #[tokio::test]
    async fn test_persist_absence_lifecycle() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_str().unwrap();
        let persistence = Persistence::new_local(path).await.unwrap();

        let abstention = RetrievalAbstention {
            query: "unknown".to_string(),
            min_score_threshold: 0.5,
            best_score_seen: Some(0.1),
            attempted_modes: vec!["Auto".to_string()],
            timestamp: Utc::now(),
        };

        // Create
        let mut abstention = abstention;
        abstention.best_score_seen = Some(0.1);
        let entry = persist_absence(&abstention, &persistence).await.unwrap();
        assert_eq!(entry.attempt_count, 1);
        assert!((entry.best_score_ever.unwrap() - 0.1).abs() < f32::EPSILON);

        // Update with higher score
        let mut abstention2 = abstention.clone();
        abstention2.best_score_seen = Some(0.4);
        let entry2 = persist_absence(&abstention2, &persistence).await.unwrap();
        assert_eq!(entry2.attempt_count, 2);
        assert!((entry2.best_score_ever.unwrap() - 0.4).abs() < f32::EPSILON);

        // Update with lower score
        let mut abstention3 = abstention.clone();
        abstention3.best_score_seen = Some(0.2);
        let entry3 = persist_absence(&abstention3, &persistence).await.unwrap();
        assert_eq!(entry3.attempt_count, 3);
        assert!((entry2.best_score_ever.unwrap() - 0.4).abs() < f32::EPSILON); // Stays at 0.4

        // Wave 32: BM25 short-circuit threshold uses DEFAULT_ABSENCE_MIN_ATTEMPTS.
        use crate::retrieval::bm25::{DEFAULT_ABSENCE_MIN_ATTEMPTS, is_known_absent};
        assert!(is_known_absent("unknown", &persistence, DEFAULT_ABSENCE_MIN_ATTEMPTS).await);
        assert!(!is_known_absent("unknown", &persistence, 4).await);
    }
}
