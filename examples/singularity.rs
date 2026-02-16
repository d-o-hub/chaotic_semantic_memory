use chaotic_semantic_memory::{ChaoticSemanticFramework, HVec10240};

const RESERVOIR_SIZE: usize = 50_000;
const PROBE_TOP_K: usize = 2;
const RUST_SEED: u64 = 7;
const MEMORY_SEED: u64 = 9;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mem = ChaoticSemanticFramework::singularity()
        .with_reservoir_size(RESERVOIR_SIZE)
        .build()
        .await?;
    let _ = mem.inject_concept("rust", HVec10240::from_seed(RUST_SEED));
    let _ = mem.inject_concept("memory", HVec10240::from_seed(MEMORY_SEED));
    let top = mem.singularity_probe(HVec10240::from_seed(RUST_SEED), PROBE_TOP_K);
    println!("{top:?}");
    Ok(())
}
