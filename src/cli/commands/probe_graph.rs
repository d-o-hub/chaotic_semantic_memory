use crate::error::Result;
use crate::cli::args::*;
use crate::hyperdim::HVec10240;

pub async fn run_probe_graph(ns: &str, _args: ProbeGraphArgs) -> Result<()> {
    let _framework = crate::cli::commands::create_framework_with_namespace::<HVec10240>(ns).await?;
    println!("Command run on namespace '{}'", ns);
    Ok(())
}
