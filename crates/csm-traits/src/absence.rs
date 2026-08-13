//! Absence records: persisted retrieval events that found no matching concepts.
//!
//! Used to prevent re-querying known-absent concepts and to surface memory
//! gaps to operators. Owner-neutral persistence contract (ADR-0094): durable
//! CRUD over these types belongs to `csm-persistence`; root adapters build
//! entries from framework-level abstention events.

use chrono::{DateTime, Utc};
use csm_core_lib::error::Result;

/// A persisted record of a retrieval event that found no matching concepts.
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
    pub fn fnv1a_hash(bytes: &[u8]) -> u64 {
        const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01b3;
        let mut hash = OFFSET_BASIS;
        for &byte in bytes {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(PRIME);
        }
        hash
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
