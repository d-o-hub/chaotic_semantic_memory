//! Revisioned ANN index snapshot envelope (ADR-0093).
//!
//! Pure encoding helpers with no persistence backend dependency so WASM and
//! no-persistence builds can still reference the types.

use csm_core_lib::error::{MemoryError, Result};
use serde::{Deserialize, Serialize};

/// Magic prefix for envelope-wrapped index blobs (`CSMIDX01`).
pub const INDEX_ENVELOPE_MAGIC: &[u8; 8] = b"CSMIDX01";

/// Envelope schema version (not DB schema version).
pub const INDEX_ENVELOPE_VERSION: u32 = 1;

/// Canonical vector format identifier for 10240-bit HDC vectors.
pub const VECTOR_FORMAT_HVEC10240: &str = "HVec10240";

/// Revisioned ANN snapshot envelope (ADR-0093).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexSnapshotEnvelope {
    /// Envelope format version.
    pub envelope_version: u32,
    /// Authoritative namespace revision at snapshot time.
    pub namespace_revision: u64,
    /// Backend + config fingerprint (must match live config to apply).
    pub backend_fingerprint: String,
    /// Vector layout identifier.
    pub vector_format: String,
    /// Raw backend-serialized index bytes.
    pub index_data: Vec<u8>,
    /// FNV-1a checksum of `index_data` (bit-rot / accidental corruption only;
    /// not a cryptographic integrity or authenticity boundary).
    pub checksum: u64,
}

impl IndexSnapshotEnvelope {
    /// Build an envelope around raw index bytes.
    #[must_use]
    pub fn new(
        namespace_revision: u64,
        backend_fingerprint: impl Into<String>,
        index_data: Vec<u8>,
    ) -> Self {
        let checksum = fnv1a64(&index_data);
        Self {
            envelope_version: INDEX_ENVELOPE_VERSION,
            namespace_revision,
            backend_fingerprint: backend_fingerprint.into(),
            vector_format: VECTOR_FORMAT_HVEC10240.to_string(),
            index_data,
            checksum,
        }
    }

    /// Validate internal integrity (checksum + envelope version + format).
    pub fn validate_integrity(&self) -> Result<()> {
        if self.envelope_version != INDEX_ENVELOPE_VERSION {
            return Err(MemoryError::InvalidInput {
                field: "envelope_version".to_string(),
                reason: format!(
                    "unsupported index envelope version {} (expected {INDEX_ENVELOPE_VERSION})",
                    self.envelope_version
                ),
            });
        }
        if self.vector_format != VECTOR_FORMAT_HVEC10240 {
            return Err(MemoryError::InvalidInput {
                field: "vector_format".to_string(),
                reason: format!(
                    "unsupported vector format {} (expected {VECTOR_FORMAT_HVEC10240})",
                    self.vector_format
                ),
            });
        }
        let expected = fnv1a64(&self.index_data);
        if self.checksum != expected {
            return Err(MemoryError::InvalidInput {
                field: "checksum".to_string(),
                reason: "index envelope checksum mismatch (corrupt snapshot)".to_string(),
            });
        }
        Ok(())
    }

    /// Encode envelope as magic + bincode payload.
    pub fn encode(&self) -> Result<Vec<u8>> {
        let payload = bincode::serialize(self).map_err(|e| {
            MemoryError::database(format!("Failed to serialize index envelope: {e}"))
        })?;
        let mut out = Vec::with_capacity(INDEX_ENVELOPE_MAGIC.len() + payload.len());
        out.extend_from_slice(INDEX_ENVELOPE_MAGIC);
        out.extend_from_slice(&payload);
        Ok(out)
    }

    /// Decode envelope; returns `None` for legacy raw blobs (treat as stale).
    pub fn try_decode(bytes: &[u8]) -> Result<Option<Self>> {
        if bytes.len() < INDEX_ENVELOPE_MAGIC.len()
            || &bytes[..INDEX_ENVELOPE_MAGIC.len()] != INDEX_ENVELOPE_MAGIC.as_slice()
        {
            return Ok(None);
        }
        let env: Self =
            bincode::deserialize(&bytes[INDEX_ENVELOPE_MAGIC.len()..]).map_err(|e| {
                MemoryError::database(format!("Failed to deserialize index envelope: {e}"))
            })?;
        env.validate_integrity()?;
        Ok(Some(env))
    }
}

/// FNV-1a 64-bit hash for envelope checksums.
#[must_use]
pub fn fnv1a64(data: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// Fingerprint an [`crate::index::IndexBackend`] for envelope matching.
#[must_use]
pub fn backend_fingerprint(backend: &crate::index::IndexBackend) -> String {
    use crate::index::IndexBackend;
    match backend {
        IndexBackend::BruteForce => "bruteforce".to_string(),
        #[cfg(feature = "ann-hnsw")]
        IndexBackend::Hnsw {
            m,
            ef_construction,
            ef_search,
        } => format!("hnsw:m={m}:efc={ef_construction}:efs={ef_search}"),
        #[cfg(feature = "ann-lsh")]
        IndexBackend::Lsh {
            num_tables,
            hash_bits,
        } => format!("lsh:tables={num_tables}:bits={hash_bits}"),
        #[allow(unreachable_patterns)]
        _ => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn envelope_roundtrip_and_checksum() {
        let env = IndexSnapshotEnvelope::new(7, "bruteforce", vec![1, 2, 3, 4, 5]);
        let bytes = env.encode().unwrap();
        let decoded = IndexSnapshotEnvelope::try_decode(&bytes).unwrap().unwrap();
        assert_eq!(decoded, env);
    }

    #[test]
    fn legacy_raw_blob_is_not_envelope() {
        let raw = vec![0u8, 1, 2, 3];
        assert!(IndexSnapshotEnvelope::try_decode(&raw).unwrap().is_none());
    }

    #[test]
    fn corrupt_checksum_rejected() {
        let mut env = IndexSnapshotEnvelope::new(1, "bruteforce", vec![9, 9, 9]);
        env.checksum ^= 0xff;
        let payload = bincode::serialize(&env).unwrap();
        let mut bytes = INDEX_ENVELOPE_MAGIC.to_vec();
        bytes.extend(payload);
        let err = IndexSnapshotEnvelope::try_decode(&bytes).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("checksum") || msg.contains("Invalid") || msg.contains("mismatch"));
    }
}
