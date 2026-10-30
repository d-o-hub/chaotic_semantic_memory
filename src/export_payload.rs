//! Export/import payload adapter over the owner types in `csm-traits` (ADR-0094).
//!
//! The payload and wire types live in `csm-traits`; this module only re-exports
//! them for the root facade and converts between the stored `Concept` and its
//! wire form. Both conversions are infallible: `ExportConcept` carries the
//! vector directly, and the binary path decodes bytes inside
//! `csm_traits::BinaryConcept::to_export_concept()`.

#[cfg(test)]
#[path = "export_payload/binary_codec_tests.rs"]
mod binary_codec_tests;
#[cfg(test)]
mod export_payload_tests;

use bincode::Options;
use csm_core_lib::error::{MemoryError, Result};
pub(crate) use csm_traits::unix_now_secs;
pub(crate) use csm_traits::{BinaryExportPayload, ExportConcept, ExportPayload};
use std::io::{self, Write};

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
const BINARY_EXPORT_MAGIC: &[u8; 8] = b"CSMBIN01";
const BINARY_EXPORT_HEADER_LEN: usize = 10;
const BINARY_EXPORT_FORMAT_VERSION: u8 = 1;
const BINARY_EXPORT_CODEC_BINCODE_FIXED: u8 = 1;
const BINARY_EXPORT_CODEC_POSTCARD: u8 = 2;

struct BoundedPostcardWriter {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for BoundedPostcardWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let next_len = self.bytes.len().checked_add(buffer.len()).ok_or_else(|| {
            io::Error::new(io::ErrorKind::WriteZero, "binary export is too large")
        })?;
        if next_len > self.limit {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "binary export exceeds the maximum size",
            ));
        }
        if next_len > self.bytes.capacity() {
            let target_capacity = self
                .bytes
                .capacity()
                .saturating_mul(2)
                .max(next_len)
                .min(self.limit);
            self.bytes
                .try_reserve_exact(target_capacity - self.bytes.len())
                .map_err(io::Error::other)?;
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Encode the bounded, versioned binary export envelope with the maintained
/// Postcard codec. Codec 1 remains readable for existing version-1 envelopes.
pub(crate) fn encode_binary_export(payload: &BinaryExportPayload) -> Result<Vec<u8>> {
    let limit =
        usize::try_from(csm_traits::MAX_IMPORT_SIZE).map_err(|_| MemoryError::InvalidInput {
            field: "export_data".to_string(),
            reason: "maximum binary export size does not fit this platform".to_string(),
        })?;
    encode_binary_export_with_limit(payload, limit)
}

fn encode_binary_export_with_limit(payload: &BinaryExportPayload, limit: usize) -> Result<Vec<u8>> {
    if limit < BINARY_EXPORT_HEADER_LEN {
        return Err(MemoryError::InvalidInput {
            field: "export_data".to_string(),
            reason: "maximum binary export size is smaller than its header".to_string(),
        });
    }

    let mut writer = BoundedPostcardWriter {
        bytes: Vec::with_capacity(BINARY_EXPORT_HEADER_LEN),
        limit,
    };
    writer.bytes.extend_from_slice(BINARY_EXPORT_MAGIC);
    writer
        .bytes
        .extend_from_slice(&[BINARY_EXPORT_FORMAT_VERSION, BINARY_EXPORT_CODEC_POSTCARD]);
    postcard::to_io(payload, writer)
        .map(|writer| writer.bytes)
        .map_err(|error| MemoryError::InvalidInput {
            field: "export_data".to_string(),
            reason: format!("binary export encoding failed: {error}"),
        })
}

/// Decode the versioned envelope or either legacy un-enveloped bincode format.
pub(crate) fn decode_binary_export(data: &[u8]) -> Result<BinaryExportPayload> {
    if data.len() as u64 > csm_traits::MAX_IMPORT_SIZE {
        return Err(MemoryError::InvalidInput {
            field: "import_data".to_string(),
            reason: "binary import exceeds the maximum size".to_string(),
        });
    }

    if data.starts_with(BINARY_EXPORT_MAGIC) {
        if data.len() < BINARY_EXPORT_HEADER_LEN {
            return Err(MemoryError::InvalidInput {
                field: "import_data".to_string(),
                reason: "truncated binary export header".to_string(),
            });
        }
        if data[8] != BINARY_EXPORT_FORMAT_VERSION {
            return Err(MemoryError::InvalidInput {
                field: "import_data".to_string(),
                reason: format!("unsupported binary export version: {}", data[8]),
            });
        }
        let payload = &data[BINARY_EXPORT_HEADER_LEN..];
        return match data[9] {
            BINARY_EXPORT_CODEC_BINCODE_FIXED => bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .with_limit(csm_traits::MAX_IMPORT_SIZE)
                .deserialize(payload)
                .map_err(|error| MemoryError::InvalidInput {
                    field: "import_data".to_string(),
                    reason: format!("legacy versioned binary export decoding failed: {error}"),
                }),
            BINARY_EXPORT_CODEC_POSTCARD => {
                postcard::from_bytes(payload).map_err(|error| MemoryError::InvalidInput {
                    field: "import_data".to_string(),
                    reason: format!("Postcard binary export decoding failed: {error}"),
                })
            }
            codec => Err(MemoryError::InvalidInput {
                field: "import_data".to_string(),
                reason: format!("unsupported binary export codec: {codec}"),
            }),
        };
    }

    let fixed = bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_limit(csm_traits::MAX_IMPORT_SIZE)
        .deserialize(data);
    match fixed {
        Ok(payload) => Ok(payload),
        Err(fixed_error) => bincode::DefaultOptions::new()
            .with_limit(csm_traits::MAX_IMPORT_SIZE)
            .deserialize(data)
            .map_err(|variable_error| MemoryError::InvalidInput {
                field: "import_data".to_string(),
                reason: format!(
                    "legacy binary export decoding failed (fixed-int: {fixed_error}; varint: {variable_error})"
                ),
            }),
    }
}
