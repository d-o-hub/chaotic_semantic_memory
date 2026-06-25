#![no_main]
use chaotic_semantic_memory::retrieval::bm25::Bm25Index;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let tokens: Vec<&str> = s.split_whitespace().collect();
        let mut index = Bm25Index::new();
        index.add_document("fuzz-doc", &tokens);
        let _ = index.search(&tokens, 5);
    }
});
