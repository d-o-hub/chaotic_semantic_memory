use super::{BINARY_EXPORT_HEADER_LEN, BoundedPostcardWriter, encode_binary_export_with_limit};
use csm_traits::BinaryExportPayload;
use std::io::Write;

fn payload() -> BinaryExportPayload {
    BinaryExportPayload {
        version: "v".to_string(),
        exported_at: 1,
        concepts: Vec::new(),
        associations: Vec::new(),
    }
}

#[test]
fn bounded_writer_accepts_exact_limit_and_rejects_overflow_without_partial_write() {
    let mut writer = BoundedPostcardWriter {
        bytes: Vec::new(),
        limit: 3,
    };
    writer.write_all(b"abc").unwrap();
    assert!(writer.write_all(b"d").is_err());
    assert_eq!(writer.bytes, b"abc");
}

#[test]
fn binary_export_limit_includes_the_envelope_header() {
    let limit = BINARY_EXPORT_HEADER_LEN + 1;
    let result = encode_binary_export_with_limit(&payload(), limit);
    assert!(
        result.is_err(),
        "payload bytes must fit within the total envelope limit"
    );
}
