//! Accessor and query methods for ChaoticSemanticFramework.

use crate::framework::ChaoticSemanticFramework;
use crate::framework_builder::FrameworkStats;
use crate::framework_metrics::FrameworkMetricsSnapshot;
use crate::graph_traversal::TraversalConfig;
use crate::singularity::Concept;
use csm_core::error::Result;
use tracing::instrument;

impl ChaoticSemanticFramework {
    /// Find the fewest-hop path between two concepts (unweighted BFS).
    ///
    /// Returns the path with the minimum number of hops, ignoring edge strengths.
    #[instrument(err, skip(self))]
    pub async fn shortest_path_hops(&self, from: &str, to: &str) -> Result<Option<Vec<String>>> {
        Self::validate_concept_id(from)?;
        Self::validate_concept_id(to)?;
        let sing = self.singularity.read().await;
        let ns = self.namespace.read().await;
        sing.shortest_path_hops(&ns, from, to, &TraversalConfig::default())
    }

    /// Get a concept by ID.
    #[instrument(err, skip(self))]
    pub async fn get_concept(&self, id: &str) -> Result<Option<Concept>> {
        Self::validate_concept_id(id)?;
        let sing = self.singularity.read().await;
        let ns = self.namespace.read().await;
        Ok(sing.get(&ns, id).cloned())
    }

    /// Backward-compatible alias for replace semantics.
    ///
    /// Delegates to [`load_replace`](Self::load_replace).
    pub async fn load(&self) -> Result<()> {
        self.load_replace().await
    }

    /// Get a snapshot of framework metrics.
    pub async fn metrics_snapshot(&self) -> FrameworkMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Get framework statistics.
    pub async fn stats(&self) -> Result<FrameworkStats> {
        let concept_count = {
            let sing = self.singularity.read().await;
            let ns = self.namespace.read().await;
            sing.len(&ns)
        };

        let db_size = if let Some(ref persistence) = self.persistence {
            Some(persistence.size().await.unwrap_or(0))
        } else {
            None
        };

        Ok(FrameworkStats {
            concept_count,
            db_size_bytes: db_size,
        })
    }
}
