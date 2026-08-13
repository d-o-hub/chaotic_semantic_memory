//! Persistence for canonical concept graph.
//!
//! Feature-gated persistence layer for storing the symbolic semantic graph
//! used by the bridge retrieval pipeline.

// Casts are intentional for version serialization

use crate::persistence::Persistence;
use crate::retrieval::hybrid::RetrievalAbstention;
use crate::semantic_bridge::{CanonicalConcept, ConceptGraph};
use csm_core_lib::error::{MemoryError, Result};
use csm_traits::{AbsenceEntry, AbsenceStore};
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

/// Build an `AbsenceEntry` from a `RetrievalAbstention` event.
///
/// Root adapter: `AbsenceEntry`/`AbsenceStore` live in `csm-traits` (ADR-0094);
/// this conversion bridges the framework-level abstention event into the
/// owner-neutral persistence contract.
pub fn absence_from_abstention(abstention: &RetrievalAbstention) -> AbsenceEntry {
    let normalized = AbsenceEntry::normalize(&abstention.query);
    AbsenceEntry {
        id: AbsenceEntry::id_for(&abstention.query),
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
pub fn merge_absence_with(entry: &mut AbsenceEntry, abstention: &RetrievalAbstention) {
    entry.attempt_count += 1;
    entry.last_seen = abstention.timestamp;
    entry.last_threshold = abstention.min_score_threshold;

    match (abstention.best_score_seen, entry.best_score_ever) {
        (Some(new), Some(existing)) => {
            if new > existing {
                entry.best_score_ever = Some(new);
            }
        }
        (Some(new), None) => {
            entry.best_score_ever = Some(new);
        }
        _ => {}
    }
}

/// Persist a RetrievalAbstention event as an AbsenceEntry.
pub async fn persist_absence(
    abstention: &RetrievalAbstention,
    store: &dyn AbsenceStore,
) -> Result<AbsenceEntry> {
    let id = AbsenceEntry::id_for(&abstention.query);
    match store.get_absence(&id).await? {
        Some(mut existing) => {
            merge_absence_with(&mut existing, abstention);
            store.upsert_absence(&existing).await?;
            Ok(existing)
        }
        None => {
            let entry = absence_from_abstention(abstention);
            store.upsert_absence(&entry).await?;
            Ok(entry)
        }
    }
}
