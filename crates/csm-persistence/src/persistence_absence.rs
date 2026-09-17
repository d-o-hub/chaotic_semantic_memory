//! Absence tracking (failed retrieval attempts) for the local store.
//!
//! Split out of `persistence.rs` at the 500-LOC gate (AGENTS.md: child module
//! extraction over comment stripping).
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use crate::persistence::Persistence;
use csm_core_lib::error::{MemoryError, Result};
use csm_traits::{AbsenceEntry, AbsenceStore};
use libsql::params;

#[async_trait::async_trait]
impl AbsenceStore for Persistence {
    async fn get_absence(&self, id: &str) -> Result<Option<AbsenceEntry>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT id, query, normalized_query, attempt_count, last_threshold, best_score_ever, first_seen, last_seen FROM csm_absences WHERE id = ?1",
                params![id],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to load absence: {e}")))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch absence row: {e}")))?
        {
            Ok(Some(Self::row_to_absence_entry(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn upsert_absence(&self, entry: &AbsenceEntry) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        conn.execute(
            "INSERT INTO csm_absences (id, query, normalized_query, attempt_count, last_threshold, best_score_ever, first_seen, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
             attempt_count = excluded.attempt_count,
             last_threshold = excluded.last_threshold,
             best_score_ever = excluded.best_score_ever,
             last_seen = excluded.last_seen",
            params![
                entry.id.clone(),
                entry.query.clone(),
                entry.normalized_query.clone(),
                entry.attempt_count as i64,
                entry.last_threshold as f64,
                entry.best_score_ever.map(|s| s as f64),
                entry.first_seen.to_rfc3339(),
                entry.last_seen.to_rfc3339()
            ],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to upsert absence: {e}")))?;

        Ok(())
    }

    async fn list_absences(&self, min_attempts: u32) -> Result<Vec<AbsenceEntry>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT id, query, normalized_query, attempt_count, last_threshold, best_score_ever, first_seen, last_seen FROM csm_absences WHERE attempt_count >= ?1 ORDER BY attempt_count DESC",
                params![min_attempts as i64],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to list absences: {e}")))?;

        let mut entries = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch absence row: {e}")))?
        {
            entries.push(Self::row_to_absence_entry(&row)?);
        }

        Ok(entries)
    }
}

impl Persistence {
    fn row_to_absence_entry(row: &libsql::Row) -> Result<AbsenceEntry> {
        let id: String = row
            .get(0)
            .map_err(|e| MemoryError::database(format!("id: {e}")))?;
        let query: String = row
            .get(1)
            .map_err(|e| MemoryError::database(format!("query: {e}")))?;
        let normalized_query: String = row
            .get(2)
            .map_err(|e| MemoryError::database(format!("normalized_query: {e}")))?;
        let attempt_count: i64 = row
            .get(3)
            .map_err(|e| MemoryError::database(format!("attempt_count: {e}")))?;
        let last_threshold: f64 = row
            .get(4)
            .map_err(|e| MemoryError::database(format!("last_threshold: {e}")))?;
        let best_score_ever: Option<f64> = row
            .get(5)
            .map_err(|e| MemoryError::database(format!("best_score_ever: {e}")))?;
        let first_seen: String = row
            .get(6)
            .map_err(|e| MemoryError::database(format!("first_seen: {e}")))?;
        let last_seen: String = row
            .get(7)
            .map_err(|e| MemoryError::database(format!("last_seen: {e}")))?;

        Ok(AbsenceEntry {
            id,
            query,
            normalized_query,
            attempt_count: attempt_count as u32,
            last_threshold: last_threshold as f32,
            best_score_ever: best_score_ever.map(|s| s as f32),
            first_seen: first_seen
                .parse()
                .map_err(|e| MemoryError::database(format!("parse first_seen: {e}")))?,
            last_seen: last_seen
                .parse()
                .map_err(|e| MemoryError::database(format!("parse last_seen: {e}")))?,
        })
    }
}
