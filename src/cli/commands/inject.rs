use crate::error::Result;
use crate::cli::args::InjectArgs;
use crate::hyperdim::HVec10240;

pub async fn run_inject(ns: &str, args: InjectArgs) -> Result<()> {
    let framework = crate::cli::commands::create_framework_with_namespace::<HVec10240>(ns).await?;
    if let Some(text) = args.text {
        framework.inject_text(&args.concept_id, &text).await?;
        println!("✓ Concept '{}' injected into {}", args.concept_id, ns);
    }
    Ok(())
}
