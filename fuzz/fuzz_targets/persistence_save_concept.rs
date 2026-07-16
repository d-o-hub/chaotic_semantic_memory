#![no_main]

use std::collections::HashMap;

use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::singularity::Concept;
use chaotic_semantic_memory::HVec10240;
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return,
    };

    runtime.block_on(async {
        let id = if data.is_empty() {
            "fuzz-empty".to_string()
        } else {
            format!("fuzz-{:02x}", data[0])
        };

        // Concept metadata is HashMap<String, Value>, not a bare Value.
        let metadata: HashMap<String, Value> =
            match serde_json::from_slice::<HashMap<String, Value>>(data) {
                Ok(m) => m,
                Err(_) => {
                    let mut m = HashMap::new();
                    m.insert(
                        "raw".to_string(),
                        Value::String(String::from_utf8_lossy(data).to_string()),
                    );
                    m
                }
            };

        // Process-scoped unique temp dir (auto-cleaned on drop).
        let Ok(tmp) = tempfile::tempdir() else {
            return;
        };
        let path = tmp.path().join("csm_fuzz_persistence.db");
        let Some(path_str) = path.to_str() else {
            return;
        };

        let persistence = match Persistence::new_local(path_str).await {
            Ok(p) => p,
            Err(_) => return,
        };

        let concept = Concept {
            id,
            vector: HVec10240::random(),
            metadata,
            created_at: 0,
            modified_at: 0,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        };

        // Ignore persistence errors on adversarial inputs; never panic.
        let _ = persistence.save_concept("fuzz", &concept).await;

        // Drop connection before tempdir cleanup.
        drop(persistence);
    });
});
