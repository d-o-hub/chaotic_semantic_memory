#![no_main]

use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::singularity::Concept;
use chaotic_semantic_memory::HVec10240;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(_) => return,
    };

    runtime.block_on(async {
        let id = if data.is_empty() {
            "fuzz-empty".to_string()
        } else {
            format!("fuzz-{:02x}", data[0])
        };

        let metadata = serde_json::from_slice::<serde_json::Value>(data)
            .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(data).to_string()));

        let path = std::env::temp_dir().join("csm_fuzz_persistence.db");
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
        };

        let _ = persistence.save_concept(&concept).await;
    });
});
