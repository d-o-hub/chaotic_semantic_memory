use chaotic_semantic_memory::{ChaoticSemanticFramework, HVec10240, TursoClient};

const RESERVOIR_SIZE: usize = 50_000;
const AGENT_SEED: u64 = 12;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("TURSO_DATABASE_URL")?;
    let token = std::env::var("TURSO_AUTH_TOKEN")?;
    let mem = ChaoticSemanticFramework::singularity()
        .with_turso(url.clone(), token.clone())
        .with_reservoir_size(RESERVOIR_SIZE)
        .build()
        .await?;
    let _ = mem.inject_concept("agent", HVec10240::from_seed(AGENT_SEED));
    #[cfg(not(target_arch = "wasm32"))]
    {
        let client = TursoClient::new(url, token)?;
        mem.persist_turso(&client).await?;
    }
    Ok(())
}
