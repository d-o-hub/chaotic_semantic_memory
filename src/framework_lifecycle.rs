//! Manual `Clone`/`Drop` for [`ChaoticSemanticFramework`].
//!
//! The struct cannot derive `Clone` because it owns a `tokio::task::JoinHandle`
//! (the background TTL cleanup task). Cloning shares every other field — and
//! the cancellation token, so cancelling any copy stops the loop for all of
//! them — but never clones the task handle.
//!
//! `Drop` cancels the loop so no orphaned purge task can outlive the
//! framework. When dropped from a thread without a runtime context the join is
//! bounded to 5s (then aborted); when dropped inside a runtime we cannot
//! `block_on` (it would panic), so we only cancel: the deadline is best-effort
//! in that case, which is acceptable because the loop observes the cancelled
//! token at its next `select!` wakeup and exits by itself.

use crate::ChaoticSemanticFramework;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

impl Clone for ChaoticSemanticFramework {
    fn clone(&self) -> Self {
        Self {
            singularity: self.singularity.clone(),
            persistence: self.persistence.clone(),
            reservoir: self.reservoir.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            event_sender: self.event_sender.clone(),
            emitters: self.emitters.clone(),
            namespace: self.namespace.clone(),
            embedding_provider: self.embedding_provider.clone(),
            projection: self.projection.clone(),
            #[cfg(not(target_arch = "wasm32"))]
            ttl_cleanup_shutdown: self.ttl_cleanup_shutdown.clone(),
            #[cfg(not(target_arch = "wasm32"))]
            ttl_cleanup_task: None,
            #[cfg(test)]
            cleanup_loop_exited: self.cleanup_loop_exited.clone(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for ChaoticSemanticFramework {
    fn drop(&mut self) {
        // Always cancel: the loop's select! wakes and the task exits
        // normally, so no orphaned purge task can outlive the framework.
        self.ttl_cleanup_shutdown.cancel();
        if let Some(handle) = self.ttl_cleanup_task.take() {
            if tokio::runtime::Handle::try_current().is_err() {
                // Dropped outside a Tokio runtime: we may block. Give the task
                // a bounded window to observe cancellation and exit, aborting
                // as a last resort so Drop never hangs.
                if let Ok(runtime) = tokio::runtime::Runtime::new() {
                    let _ = runtime.block_on(async {
                        if tokio::time::timeout(Duration::from_secs(5), &mut handle)
                            .await
                            .is_err()
                        {
                            handle.abort();
                        }
                    });
                }
            }
            // Inside a runtime context block_on would panic, so we only
            // cancel; see the module docs for why the deadline is best-effort.
        }
    }
}