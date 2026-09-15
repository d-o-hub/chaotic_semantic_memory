//! Export/import payload adapter over the owner types in `csm-traits` (ADR-0094).
//!
//! The payload and wire types live in `csm-traits`; this module only re-exports
//! them for the root facade and converts between the stored `Concept` and its
//! wire form. Both conversions are infallible: `ExportConcept` carries the
//! vector directly, and the binary path decodes bytes inside
//! `csm_traits::BinaryConcept::to_export_concept()`.

#[cfg(test)]
mod export_payload_tests;

pub(crate) use csm_traits::unix_now_secs;
pub(crate) use csm_traits::{BinaryExportPayload, ExportConcept, ExportPayload};

// Wire types the production paths only ever construct via `BinaryExportPayload`;
// the unit tests exercise their metadata conversions directly.
#[cfg(test)]
pub(crate) use csm_traits::{BinaryConcept, BinaryMetadataValue};

use crate::singularity::Concept;

/// Stored concept → wire concept.
pub(crate) fn concept_to_export(concept: Concept) -> ExportConcept {
    ExportConcept {
        id: concept.id,
        vector: concept.vector,
        metadata: concept.metadata,
        created_at: concept.created_at,
        modified_at: concept.modified_at,
        expires_at: concept.expires_at,
        canonical_concept_ids: concept.canonical_concept_ids,
    }
}

/// Wire concept → stored concept.
pub(crate) fn export_to_concept(concept: ExportConcept) -> Concept {
    Concept {
        id: concept.id,
        vector: concept.vector,
        metadata: concept.metadata,
        created_at: concept.created_at,
        modified_at: concept.modified_at,
        expires_at: concept.expires_at,
        canonical_concept_ids: concept.canonical_concept_ids,
    }
}
