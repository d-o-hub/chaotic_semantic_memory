#![no_main]
use chaotic_semantic_memory::metadata_filter::MetadataFilter;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_json::from_str::<MetadataFilter>(s);
    }
});
