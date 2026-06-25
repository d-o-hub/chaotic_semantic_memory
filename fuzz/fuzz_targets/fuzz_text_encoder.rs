#![no_main]
use chaotic_semantic_memory::encoder::TextEncoder;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let encoder = TextEncoder::new();
        let _ = encoder.encode(s);
    }
});
