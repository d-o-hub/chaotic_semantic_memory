#![no_main]

use chaotic_semantic_memory::reservoir::Reservoir;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let input_size = usize::from(data[0]).clamp(1, 64);
    let reservoir_size = usize::from(data[1]).clamp(1, 256);
    let input_len = usize::from(data[2]).clamp(0, 128);
    let seed = u64::from(data[3]);

    let mut reservoir = match Reservoir::new_seeded(input_size, reservoir_size, seed) {
        Ok(r) => r,
        Err(_) => return,
    };

    let mut input = vec![0.0f32; input_len];
    for (i, val) in input.iter_mut().enumerate() {
        if let Some(byte) = data.get(4 + i) {
            let normalized = ((*byte as i16 - 128) as f32) / 128.0;
            *val = normalized;
        }
    }

    let _ = reservoir.step(&input);
});
