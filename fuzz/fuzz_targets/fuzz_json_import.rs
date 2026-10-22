#![no_main]
use libfuzzer_sys::fuzz_target;

use std::cell::RefCell;
use std::io::Write;

thread_local! {
    static RUNTIME: tokio::runtime::Runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("fuzz Tokio runtime");
    static FRAMEWORK: RefCell<Option<chaotic_semantic_memory::ChaoticSemanticFramework>> =
        const { RefCell::new(None) };
}

fuzz_target!(|data: &[u8]| {
    const MAX_INPUT_SIZE: usize = 100 * 1024 * 1024;
    if data.len() > MAX_INPUT_SIZE {
        return;
    }

    let Ok(mut input) = tempfile::NamedTempFile::new() else {
        return;
    };
    if input.write_all(data).is_err() {
        return;
    }
    let path = input.path().to_string_lossy().into_owned();

    RUNTIME.with(|runtime| {
        FRAMEWORK.with(|framework_slot| {
            let mut framework_slot = framework_slot.borrow_mut();
            if framework_slot.is_none() {
                let framework = runtime.block_on(
                    chaotic_semantic_memory::ChaoticSemanticFramework::builder()
                        .without_persistence()
                        .build(),
                );
                let Ok(framework) = framework else {
                    return;
                };
                *framework_slot = Some(framework);
            }

            if let Some(framework) = framework_slot.as_ref() {
                let _ = runtime.block_on(framework.import_json(&path, false));
            }
        });
    });
});
