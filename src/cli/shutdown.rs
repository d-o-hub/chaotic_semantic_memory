//! Non-wasm shutdown signal primitive handling both SIGINT (Ctrl+C) and SIGTERM.

use tracing::{info, warn};

/// Wait for a graceful shutdown signal (SIGINT / Ctrl+C or SIGTERM).
///
/// Returns when either signal is received by the process. If signal handler
/// registration fails, the error is logged and that handler arm parks forever,
/// allowing the default OS signal disposition or remaining signal handlers
/// to take effect.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        match tokio::signal::ctrl_c().await {
            Ok(()) => info!("Received SIGINT (Ctrl+C); stopping"),
            Err(e) => {
                warn!(
                    error = %e,
                    "could not install SIGINT (Ctrl+C) handler; parking"
                );
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
                info!("Received SIGTERM; stopping");
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "could not install SIGTERM handler; parking"
                );
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shutdown_signal_compiles() {
        // Simple sanity check that the future can be constructed.
        // Execution of real signals is tested via integration tests.
        let _fut = shutdown_signal();
    }
}
