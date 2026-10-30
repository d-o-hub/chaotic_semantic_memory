//! Persistence-first expiration purging for framework namespaces.

use crate::framework::ChaoticSemanticFramework;
use crate::framework_events_ce::ChaoticEvent;
use crate::singularity::unix_now_secs;
use csm_core_lib::error::{MemoryError, Result};
use std::collections::HashSet;
use tracing::instrument;

impl ChaoticSemanticFramework {
    /// Purge all expired concepts. Returns the count of concepts removed.
    #[instrument(err, skip(self))]
    pub async fn purge_expired(&self) -> Result<usize> {
        let result = self.purge_expired_inner().await;
        #[cfg(feature = "prometheus")]
        {
            let (outcome, concepts) = match &result {
                Ok(count) => ("success", *count),
                Err(_) => ("failure", 0),
            };
            crate::observability::prom::record_ttl_purge(outcome, concepts);
        }
        result
    }

    async fn purge_expired_inner(&self) -> Result<usize> {
        #[cfg(not(target_arch = "wasm32"))]
        let start = std::time::Instant::now();
        #[cfg(target_arch = "wasm32")]
        let start = js_sys::Date::now();

        let _mutation = self.mutation_coordinator.lock().await;
        let ns = self.namespace().await;
        let now = unix_now_secs();
        let cascading = self.config.ttl_config.cascading_purge;
        let mut to_remove = {
            let sing = self.singularity.read().await;
            let Some(state) = sing.get_namespace(&ns) else {
                return Ok(0);
            };
            let mut expired: HashSet<String> = state
                .concepts
                .iter()
                .filter(|(_, concept)| concept.expires_at.is_some_and(|expires| expires <= now))
                .map(|(id, _)| id.clone())
                .collect();

            if cascading && !expired.is_empty() {
                let mut changed = true;
                while changed {
                    changed = false;
                    for (from_id, neighbors) in &state.associations {
                        if expired.contains(from_id) {
                            for to_id in neighbors.keys() {
                                if state.concepts.contains_key(to_id)
                                    && expired.insert(to_id.clone())
                                {
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }
            let expired_ids = expired.into_iter().collect::<Vec<_>>();
            drop(sing);
            expired_ids
        };
        if to_remove.is_empty() {
            return Ok(0);
        }
        to_remove.sort_unstable();

        if let Some(persistence) = &self.persistence {
            persistence.delete_concepts(&ns, &to_remove).await?;
        }
        let apply_result = {
            let mut sing = self.singularity.write().await;
            to_remove.iter().try_for_each(|id| sing.delete(&ns, id))
        };
        if let Err(error) = apply_result {
            if self.persistence.is_some() {
                if let Err(reload) = self.reload_namespace_from_rows(&ns).await {
                    return Err(MemoryError::database(format!(
                        "post-commit expiration purge failed ({error}); namespace reconciliation failed ({reload})"
                    )));
                }
                return Err(MemoryError::database(format!(
                    "post-commit expiration purge failed ({error}); namespace was reloaded from durable rows"
                )));
            }
            return Err(error);
        }
        self.invalidate_text_index(&ns).await;
        let count = to_remove.len();
        drop(_mutation);

        #[cfg(not(target_arch = "wasm32"))]
        let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        #[cfg(target_arch = "wasm32")]
        let duration_ms = (js_sys::Date::now() - start) as u64;
        self.emit_chaotic_event(ChaoticEvent::MemoryConsolidated {
            episode_count: count,
            duration_ms,
        })
        .await;
        Ok(count)
    }
}
