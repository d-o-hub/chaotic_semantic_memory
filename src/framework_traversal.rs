//! Graph traversal operations for the framework.

use crate::framework::ChaoticSemanticFramework;
use crate::graph_traversal::TraversalConfig;
use csm_core_lib::error::Result;
use tracing::instrument;

impl ChaoticSemanticFramework {
    /// Traverse graph using breadth-first search.
    #[instrument(err, skip(self, config))]
    pub async fn traverse(
        &self,
        start: &str,
        config: TraversalConfig,
    ) -> Result<Vec<(String, u32)>> {
        Self::validate_concept_id(start)?;
        Self::validate_traversal_config(&config)?;
        let sing = self.singularity.read().await;
        let ns = self.namespace.read().await;
        self.metrics.inc_traversals();
        sing.bfs(&ns, start, &config)
    }

    /// Find shortest weighted path between two concepts.
    #[instrument(err, skip(self))]
    pub async fn shortest_path(&self, from: &str, to: &str) -> Result<Option<Vec<String>>> {
        Self::validate_concept_id(from)?;
        Self::validate_concept_id(to)?;
        let sing = self.singularity.read().await;
        let ns = self.namespace.read().await;
        self.metrics.inc_shortest_path();
        sing.shortest_path(&ns, from, to, &TraversalConfig::default())
    }
}
