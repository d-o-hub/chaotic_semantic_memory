#![no_main]
use chaotic_semantic_memory::metadata_filter::MetadataFilter;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Product schema: real MetadataFilter deserialization, then the
        // product-side depth/parameter validation and evaluation code.
        if let Ok(filter) = serde_json::from_str::<MetadataFilter>(s) {
            let _ = filter.validate();
            let _ = filter.matches(&std::collections::HashMap::new());
        }
    }
});
