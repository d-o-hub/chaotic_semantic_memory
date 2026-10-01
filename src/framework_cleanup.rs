//! Ownership and shutdown of the background TTL cleanup task (ADR-0099).
//!
//! One task serves every clone of a [`ChaoticSemanticFramework`], so
//! cancellation is cooperative (a `watch` channel) and never `abort()`:
//! aborting from one clone's `Drop` would stop cleanup for the others, and the
//! loop also ends by itself once the last handle is dropped.
//!
//! This lives in its own module so `framework.rs` stays under the 500-line
//! gate (AGENTS.md step 9: child-module extraction over comment stripping).

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

use csm_core_lib::error::Result;

use crate::framework::ChaoticSemanticFramework;

/// Shared ownership of the background TTL cleanup task.
///
/// The struct deliberately has no inherent methods: the lifecycle is field
/// access from the framework, builder and namespace-clone paths, which keeps
/// the generated public API documents free of an `impl` block for a
/// crate-private type.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub(crate) struct CleanupTask {
    /// `true` asks the loop to stop; dropping this sender stops it as well.
    pub(crate) cancel: tokio::sync::watch::Sender<bool>,
    /// Taken by `shutdown()` so the task can be awaited exactly once.
    pub(crate) handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

/// Upper bound for waiting on the cleanup task during `shutdown()`.
#[cfg(not(target_arch = "wasm32"))]
const CLEANUP_SHUTDOWN_GRACE: std::time::Duration = std::time::Duration::from_secs(5);

/// Spawn the background cleanup loop when `cleanup_interval_seconds > 0`.
///
/// The loop works on a framework clone that does not own the stop handle:
/// holding it would keep the `watch` sender alive forever, so the loop could
/// never end after the last user handle was dropped.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn spawn_cleanup_task(fw: &mut ChaoticSemanticFramework) {
    let interval = fw.config.ttl_config.cleanup_interval_seconds;
    if interval == 0 {
        return;
    }
    let mut task_fw = fw.clone();
    task_fw.cleanup = None;
    let (cancel, mut cancelled) = tokio::sync::watch::channel(false);
    let handle = tokio::spawn(async move {
        let mut timer = tokio::time::interval(tokio::time::Duration::from_secs(interval));
        loop {
            tokio::select! {
                _ = timer.tick() => {
                    if let Err(e) = task_fw.purge_expired().await {
                        tracing::error!(error = %e, "background cleanup failed");
                    }
                }
                changed = cancelled.changed() => {
                    // Err = every handle dropped; Ok(true) = explicit stop.
                    if changed.is_err() || *cancelled.borrow_and_update() {
                        break;
                    }
                }
            }
        }
    });
    fw.cleanup = Some(Arc::new(CleanupTask {
        cancel,
        handle: tokio::sync::Mutex::new(Some(handle)),
    }));
}

/// `wasm32` has no background task, so there is nothing to spawn.
#[cfg(target_arch = "wasm32")]
pub(crate) fn spawn_cleanup_task(_fw: &mut ChaoticSemanticFramework) {}

impl ChaoticSemanticFramework {
    /// Stop the shared background TTL cleanup task and wait for it to finish.
    ///
    /// The cleanup task is shared by every clone of this framework, so calling
    /// `shutdown` on any clone stops it for all of them. Cancellation is
    /// cooperative and the wait is bounded by a five-second grace period; a
    /// task that panicked or refused to stop is reported as an error instead of
    /// being hidden. Idempotent — later calls return `Ok(())` immediately — and
    /// dropping the last framework handle stops the task even without a call.
    ///
    /// Built with `cleanup_interval_seconds == 0` (the default), there is no
    /// task and this is a no-op, as it is on `wasm32`.
    ///
    /// # Errors
    ///
    /// Returns a `MemoryError::External` if the cleanup task panicked or did
    /// not stop within the grace period.
    pub async fn shutdown(&self) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(task) = self.cleanup.as_ref() else {
                return Ok(());
            };
            let _ = task.cancel.send(true);
            if let Some(handle) = task.handle.lock().await.take() {
                return match tokio::time::timeout(CLEANUP_SHUTDOWN_GRACE, handle).await {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(join_error)) => Err(csm_core_lib::error::MemoryError::External(
                        format!("TTL cleanup task failed: {join_error}"),
                    )),
                    Err(_) => Err(csm_core_lib::error::MemoryError::External(
                        "TTL cleanup task did not stop within 5s".to_string(),
                    )),
                };
            }
        }
        Ok(())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::framework_ttl_advanced::TtlConfig;

    /// Build a framework with the given cleanup interval and no persistence.
    async fn framework_with_interval(interval: u64) -> ChaoticSemanticFramework {
        ChaoticSemanticFramework::builder()
            .with_ttl_config(TtlConfig {
                cleanup_interval_seconds: interval,
                ..Default::default()
            })
            .without_persistence()
            .build()
            .await
            .unwrap()
    }

    /// `interval == 0` means no task; any interval above zero must spawn one.
    #[tokio::test]
    async fn spawn_cleanup_task_starts_the_loop_only_when_configured() {
        let disabled = framework_with_interval(0).await;
        assert!(
            disabled.cleanup.is_none(),
            "interval 0 must not spawn a cleanup task"
        );

        let enabled = framework_with_interval(1).await;
        assert!(
            enabled.cleanup.is_some(),
            "interval > 0 must spawn a cleanup task"
        );
        enabled.shutdown().await.unwrap();
    }

    /// `shutdown()` cancels, awaits the task, and takes the handle exactly once.
    #[tokio::test]
    async fn shutdown_awaits_the_task_and_is_idempotent() {
        let fw = framework_with_interval(1).await;
        let task = fw.cleanup.as_ref().expect("cleanup task");

        assert!(
            task.handle.lock().await.is_some(),
            "the join handle must be owned before shutdown"
        );
        fw.shutdown().await.unwrap();
        assert!(
            task.handle.lock().await.is_none(),
            "shutdown must await the task and take its handle"
        );
        // Idempotent: the handle is gone, so a second call returns immediately.
        fw.shutdown().await.unwrap();
    }
}
