#![no_main]

use chaotic_semantic_memory::ChaoticSemanticFramework;
use chaotic_semantic_memory::export_payload::BinaryExportPayload;
use libfuzzer_sys::fuzz_target;
use bincode::Options;

// Target 2: Structured fuzzing of BinaryExportPayload (tests logic only, bypassing deserialization)
fuzz_target!(|payload: BinaryExportPayload| {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return,
    };

    runtime.block_on(async {
        // Test 1: Conversion logic
        if let Ok(_export_payload) = payload.to_export_payload() {
             let fw = ChaoticSemanticFramework::builder()
                .without_persistence()
                .build()
                .await
                .unwrap();

            // Test 2: Framework logic using re-serialized well-formed bytes
            let options = bincode::DefaultOptions::new().with_limit(10 * 1024 * 1024);
            if let Ok(well_formed_bytes) = options.serialize(&payload) {
                let _ = fw.import_binary_from_bytes(&well_formed_bytes, false).await;
            }
        }
    });
});
