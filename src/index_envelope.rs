//! Revisioned ANN index snapshot envelope (ADR-0093).
//!
//! Canonical implementation moved to `csm-memory` (owner of ANN index state,
//! ADR-0094). This module preserves the root public path.
//! Re-export only — no second implementation.

pub use csm_memory::index_envelope::{
    INDEX_ENVELOPE_MAGIC, INDEX_ENVELOPE_VERSION, IndexSnapshotEnvelope, VECTOR_FORMAT_HVEC10240,
    backend_fingerprint, fnv1a64,
};
