#![no_main]

use chaotic_semantic_memory::ChaoticSemanticFramework;
use libfuzzer_sys::fuzz_target;

// Target 1: Binary import from raw bytes (tests deserialization + logic)
fuzz_target!(|data: &[u8]| {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return,
    };

    runtime.block_on(async {
        let fw = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        // Exercise the framework's import logic with arbitrary raw bytes
        let _ = fw.import_binary_from_bytes(data, false).await;
    });
});
