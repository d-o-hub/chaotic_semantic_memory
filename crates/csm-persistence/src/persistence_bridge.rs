//! Canonical concept graph persistence (ADR-0061, ADR-0094).
//!
//! Durable CRUD for `csm_traits::{CanonicalConcept, ConceptGraph}` over the
//! `csm_canonical` table. Migrated here from the root facade so the schema has
//! exactly one owner (ADR-0094).

#![cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]

use crate::persistence::Persistence;
use csm_core_lib::error::{MemoryError, Result};
use csm_traits::{CanonicalConcept, ConceptGraph};
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use tempfile::NamedTempFile;

    async fn persistence() -> (NamedTempFile, Persistence) {
        let temp = NamedTempFile::new().expect("temp file");
        let path = temp.path().to_str().expect("path");
        let p = Persistence::new_local(path).await.expect("persistence");
        (temp, p)
    }

    #[tokio::test]
    async fn canonical_concept_round_trip_and_delete() {
        let (_temp, p) = persistence().await;
        let concept = CanonicalConcept::new("concept.agent_memory")
            .with_label("agent memory")
            .with_related("concept.context");

        p.save_canonical_concept("ns", &concept).await.unwrap();
        let loaded = p
            .load_canonical_concept("ns", "concept.agent_memory")
            .await
            .unwrap()
            .expect("concept should exist");
        assert_eq!(loaded.labels, vec!["agent memory".to_string()]);
        assert_eq!(loaded.related, vec!["concept.context".to_string()]);
        assert_eq!(loaded.version, 1);

        // Upsert keeps a single row and updates fields.
        let updated = CanonicalConcept {
            version: 2,
            labels: vec!["renamed".to_string()],
            ..concept.clone()
        };
        p.save_canonical_concept("ns", &updated).await.unwrap();
        let all = p.load_all_canonical_concepts("ns").await.unwrap();
        assert_eq!(all.len(), 1, "upsert must not duplicate the row");
        assert_eq!(all[0].version, 2);

        p.delete_canonical_concept("ns", "concept.agent_memory")
            .await
            .unwrap();
        assert!(
            p.load_canonical_concept("ns", "concept.agent_memory")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn canonical_concepts_are_namespace_scoped() {
        let (_temp, p) = persistence().await;
        p.save_canonical_concept("a", &CanonicalConcept::new("c1").with_label("one"))
            .await
            .unwrap();
        p.save_canonical_concept("b", &CanonicalConcept::new("c2").with_label("two"))
            .await
            .unwrap();

        let a = p.load_all_canonical_concepts("a").await.unwrap();
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].id, "c1");
        assert!(
            p.load_canonical_concept("a", "c2").await.unwrap().is_none(),
            "namespace a must not see namespace b rows"
        );
    }

    #[tokio::test]
    async fn save_concept_graph_replaces_and_reloads_whole_namespace() {
        let (_temp, p) = persistence().await;

        let mut graph = ConceptGraph::new();
        graph.add_concept(CanonicalConcept::new("c1").with_label("alpha"));
        graph.add_concept(CanonicalConcept::new("c2").with_related("c1"));
        p.save_concept_graph("ns", &graph).await.unwrap();

        let loaded = p.load_concept_graph("ns").await.unwrap();
        assert_eq!(loaded.concept_count(), 2);
        assert_eq!(loaded.label_count(), 1);
        assert!(loaded.get_concept("c2").is_some());

        // Saving a smaller graph replaces the namespace contents instead of appending.
        let mut smaller = ConceptGraph::new();
        smaller.add_concept(CanonicalConcept::new("c9"));
        p.save_concept_graph("ns", &smaller).await.unwrap();

        let replaced = p.load_concept_graph("ns").await.unwrap();
        assert_eq!(replaced.concept_count(), 1);
        assert!(replaced.get_concept("c9").is_some());
        assert!(replaced.get_concept("c1").is_none());
    }
}
