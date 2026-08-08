#![no_main]

// Fuzzes the product's real JSON import path: bytes are fed to
// `ChaoticSemanticFramework::import_json`, which decodes the exact export/import
// payload schema (`ExportPayload` + `Concept` + associations), validates each
// concept, and injects it into an in-memory framework. It never deserializes a
// generic `serde_json::Value` in place of the product schema.

use chaotic_semantic_memory::ChaoticSemanticFramework;
use libfuzzer_sys::fuzz_target;
use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonic counter used only to derive a unique temp-file path per call.
/// No cross-call state is shared: the framework and file are discarded after
/// each call.
static IMPORT_SEQ: AtomicU64 = AtomicU64::new(0);

fuzz_target!(|data: &[u8]| {
    let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(_) => return,
    };

    runtime.block_on(async {
        let seq = IMPORT_SEQ.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "csm_fuzz_import_{}_{}.json",
            std::process::id(),
            seq
        ));
        if std::fs::write(&path, data).is_err() {
            return;
        }
        let Some(path_str) = path.to_str() else {
            let _ = std::fs::remove_file(&path);
            return;
        };

        let framework = match ChaoticSemanticFramework::builder().build().await {
            Ok(fw) => fw,
            Err(_) => {
                let _ = std::fs::remove_file(&path);
                return;
            }
        };

        // merge=false exercises validation + inject of every concept.
        let _ = framework.import_json(path_str, false).await;

        drop(framework);
        let _ = std::fs::remove_file(&path);
    });
});