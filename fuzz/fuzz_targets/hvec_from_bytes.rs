#![no_main]

use chaotic_semantic_memory::hyperdim::HVec10240;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = HVec10240::from_bytes(data);
});
