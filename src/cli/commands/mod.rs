//! CLI subcommands implementation.

pub mod associate;
pub mod associations;
pub mod completions;
pub mod delete;
pub mod disassociate;
pub mod export;
pub mod get;
pub mod import;
pub mod index_dir;
pub mod index_jsonl;
pub mod inject;
pub mod metrics;
pub mod path;
pub mod probe;
pub mod probe_filtered;
pub mod probe_graph;
pub mod query;
pub mod stats;
pub mod traverse;
pub mod update;
pub mod watch;

pub use associate::run_associate;
pub use associations::run_associations;
pub use completions::run_completions;
pub use delete::run_delete;
pub use disassociate::run_disassociate;
pub use export::run_export;
pub use get::run_get;
pub use import::run_import;
pub use index_dir::run_index_dir;
pub use index_jsonl::run_index_jsonl;
pub use inject::run_inject;
pub use metrics::run_metrics;
pub use path::run_path;
pub use probe::run_probe;
pub use probe_filtered::run_probe_filtered;
pub use probe_graph::run_probe_graph;
pub use query::run_query;
pub use stats::run_stats;
pub use traverse::run_traverse;
pub use update::run_update;
pub use watch::run_watch;

use crate::error::Result;
use crate::framework::ChaoticSemanticFramework;
use crate::hyperdim::{HVec10240, Hypervector};

pub async fn create_framework_with_namespace<H: Hypervector + 'static>(ns: &str) -> Result<ChaoticSemanticFramework<H>> {
    let framework = ChaoticSemanticFramework::<H>::builder()
        .build()
        .await?;
    {
        let mut n = framework.namespace.write().await;
        *n = ns.to_string();
    }
    Ok(framework)
}

pub async fn create_framework<H: Hypervector + 'static>() -> Result<ChaoticSemanticFramework<H>> {
    ChaoticSemanticFramework::<H>::builder().build().await
}

pub fn print_success(msg: &str) {
    println!("✓ {}", msg);
}

pub fn print_warning(msg: &str) {
    println!("! {}", msg);
}

pub fn truncate_preview(text: &str, _len: usize) -> String {
    text.to_string()
}

pub fn validate_concept_id(id: &str) -> Result<()> {
    ChaoticSemanticFramework::<HVec10240>::validate_concept_id(id)
}

pub fn validate_top_k(top_k: usize) -> Result<()> {
    if top_k == 0 {
         return Err(crate::error::MemoryError::InvalidInput { field: "top_k".to_string(), reason: "must be > 0".to_string() });
    }
    Ok(())
}
