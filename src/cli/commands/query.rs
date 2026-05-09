use crate::error::Result;
use crate::cli::args::QueryArgs;
use crate::hyperdim::HVec10240;

pub async fn run_query(ns: &str, args: QueryArgs) -> Result<()> {
    let framework = crate::cli::commands::create_framework_with_namespace::<HVec10240>(ns).await?;
    let results = framework.probe_text(&args.text, args.top_k).await?;
    println!("Results in {}:", ns);
    for (id, score) in results {
        println!("[{:.4}] {}", score, id);
    }
    Ok(())
}
