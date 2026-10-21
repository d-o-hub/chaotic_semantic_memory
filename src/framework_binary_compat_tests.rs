#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::export_payload::{BinaryExportPayload, decode_binary_export, encode_binary_export};
use bincode::Options;

fn payload() -> BinaryExportPayload {
    BinaryExportPayload {
        version: "legacy-version".to_string(),
        exported_at: 17,
        concepts: Vec::new(),
        associations: Vec::new(),
    }
}

fn assert_empty_legacy_payload(actual: BinaryExportPayload) {
    assert_eq!(actual.version, "legacy-version");
    assert_eq!(actual.exported_at, 17);
    assert!(actual.concepts.is_empty());
    assert!(actual.associations.is_empty());
}

#[test]
fn binary_export_envelope_roundtrips_and_reads_legacy_codecs() {
    let current = encode_binary_export(&payload()).unwrap();
    assert_eq!(&current[..8], b"CSMBIN01");
    assert_eq!(current[8], 1);
    assert_eq!(current[9], 2, "new envelopes identify the Postcard codec");
    assert_empty_legacy_payload(decode_binary_export(&current).unwrap());

    let mut unsupported_codec = current;
    unsupported_codec[9] = u8::MAX;
    assert!(decode_binary_export(&unsupported_codec).is_err());

    let fixed = bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .serialize(&payload())
        .unwrap();
    let mut versioned_legacy = b"CSMBIN01".to_vec();
    versioned_legacy.extend_from_slice(&[1, 1]);
    versioned_legacy.extend_from_slice(&fixed);
    assert_empty_legacy_payload(decode_binary_export(&versioned_legacy).unwrap());

    assert_eq!(decode_binary_export(&fixed).unwrap().exported_at, 17);

    let variable = bincode::DefaultOptions::new()
        .serialize(&payload())
        .unwrap();
    assert_eq!(decode_binary_export(&variable).unwrap().exported_at, 17);
}
