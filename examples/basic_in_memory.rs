use chaotic_semantic_memory::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await?;

    framework
        .inject_concept("rust", HVec10240::random())
        .await?;
    framework
        .inject_concept("memory", HVec10240::random())
        .await?;

    framework.associate("rust", "memory", 0.8).await?;

    let query = HVec10240::random();
    let hits = framework.probe(query, 3).await?;
    println!("top hits: {hits:?}");

    Ok(())
}
