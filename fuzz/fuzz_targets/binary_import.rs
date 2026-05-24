#![no_main]

use chaotic_semantic_memory::ChaoticSemanticFramework;
use chaotic_semantic_memory::export_payload::BinaryExportPayload;
use libfuzzer_sys::fuzz_target;
use bincode::Options;

fuzz_target!(|data: &[u8]| {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return,
    };

    runtime.block_on(async {
        // Test 1: Direct deserialization of BinaryExportPayload
        let options = bincode::DefaultOptions::new().with_limit(10 * 1024 * 1024); // 10MB limit for fuzzing
        let binary_payload: Result<BinaryExportPayload, _> = options.deserialize(data);

        if let Ok(payload) = binary_payload {
            // If deserialization succeeds, try converting to internal ExportPayload
            if let Ok(_export_payload) = payload.to_export_payload() {
                // Exercise the framework's import logic (I/O free)
                let fw = ChaoticSemanticFramework::builder()
                    .without_persistence()
                    .build()
                    .await
                    .unwrap();

                // Re-serializing ensures we have a valid, well-formed byte buffer for the framework.
                if let Ok(well_formed_bytes) = options.serialize(&payload) {
                    let _ = fw.import_binary_from_bytes(&well_formed_bytes, false).await;
                }
            }
        }

        // Test 2: Try import_binary_from_bytes directly with arbitrary data
        let fw = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        let _ = fw.import_binary_from_bytes(data, false).await;
    });
});
