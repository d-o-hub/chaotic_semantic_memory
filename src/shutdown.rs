//! Shared OS-signal shutdown primitive for long-running commands (ADR-0099).
//!
//! A foreground operator stops `csm watch` / `csm mcp serve` with Ctrl+C (SIGINT),
//! but a service manager stops them with SIGTERM (`systemctl stop`, `docker stop`).
//! Only SIGINT was handled, so a service-managed server died on the default
//! SIGTERM disposition — exit 143, 128+15 — and never ran the bounded TTL-cleanup
//! stop ADR-0099 promises. One primitive serves both signals so no command gets to
//! wire only half of them again.

use tracing::{info, warn};

/// Resolves when the process is asked to stop by an operator (SIGINT) or a
/// service manager (SIGTERM).
///
/// Both arms deliberately park forever rather than resolving when a handler
/// cannot be installed: a failure to install is not a request to shut down, and
/// resolving would exit the process the instant it started. Parking leaves the
/// default signal disposition as the fallback, which is the pre-existing
/// behaviour.
pub(crate) async fn operator_shutdown() {
    let sigint = sigint();

    #[cfg(unix)]
    {
        let sigterm = sigterm();
        tokio::select! {
            () = sigint => {}
            () = sigterm => {}
        }
    }

    #[cfg(not(unix))]
    {
        sigint.await;
    }
}

/// The foreground operator's Ctrl+C.
async fn sigint() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => info!("received SIGINT; stopping the process"),
        Err(e) => {
            warn!(
                error = %e,
                "could not install the SIGINT handler; the process keeps running and \
                 will be stopped by the default signal disposition"
            );
            std::future::pending::<()>().await;
        }
    }
}

/// A service manager's SIGTERM.
#[cfg(unix)]
async fn sigterm() {
    use tokio::signal::unix::{SignalKind, signal};

    match signal(SignalKind::terminate()) {
        Ok(mut stream) => {
            // `recv` yields `None` only if the sender is dropped, which is not a
            // shutdown request either — park for the same reason as the error arm.
            if stream.recv().await.is_some() {
                info!("received SIGTERM; stopping");
            } else {
                warn!(
                    "the SIGTERM stream closed without a signal; the process keeps \
                     running and will be stopped by the default signal disposition"
                );
                std::future::pending::<()>().await;
            }
        }
        Err(e) => {
            warn!(
                error = %e,
                "could not install the SIGTERM handler; the process keeps running and \
                 will be stopped by the default signal disposition"
            );
            std::future::pending::<()>().await;
        }
    }
}

#[cfg(test)]
#[path = "shutdown_tests.rs"]
mod tests;
