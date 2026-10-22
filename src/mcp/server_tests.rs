//! Tests for the MCP server entry points and their exit path.
//!
//! Extracted from the parent module to stay under the 500-line gate
//! (AGENTS.md step 9: child-module extraction over comment stripping).
//!
//! These are crate-internal on purpose: the ADR-0099 guarantee is a field of the
//! framework the handler owns, so observing it needs `pub(crate)` access. The
//! public exit path itself is asserted end-to-end in
//! `tests/mcp_streamable_http_integration.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::framework_ttl_advanced::TtlConfig;

/// Reserve an ephemeral port, read it, then release it so the server under test
/// binds that exact address and the test knows where to connect.
async fn reserve_ephemeral_port() -> std::net::SocketAddr {
    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("an ephemeral port should be available");
    let addr = probe
        .local_addr()
        .expect("a bound listener has a local address");
    drop(probe);
    addr
}

/// A handler whose framework has a live shared TTL cleanup task (interval 1 s).
async fn handler_with_live_reaper() -> Arc<McpHandler> {
    let handler = Arc::new(McpHandler::new(None).with_ttl_cleanup_interval(1));
    let framework = crate::framework::ChaoticSemanticFramework::builder()
        .with_ttl_config(TtlConfig {
            cleanup_interval_seconds: 1,
            ..Default::default()
        })
        .without_persistence()
        .build()
        .await
        .expect("a framework with a cleanup interval should build");
    // `SetError` is not `Debug` (the framework is), so `is_ok()` rather than
    // `expect` — same shape as the handler shutdown test in `tools_tests.rs`.
    assert!(handler.framework.set(framework).is_ok());
    handler
}

/// The Streamable HTTP transport must resolve when its shutdown signal does,
/// and the exit path must then stop the shared TTL cleanup task (ADR-0099).
///
/// This asserts the effect, not the config echo:
///
/// - the server really binds and is confirmed listening by a successful connect,
///   so readiness is a sync point rather than a `sleep`;
/// - the signal is a `tokio::sync::oneshot`, i.e. the same
///   `Future<Output = ()> + Send + 'static` mechanism [`serve`] drives with
///   Ctrl+C through [`serve_with_shutdown`] — production and test go through one
///   function, [`serve_with_handler`];
/// - the transport future is *awaited*: there is no `abort()` anywhere, which is
///   precisely how an unobserved serve failure used to hide a shutdown leak;
/// - the final assertion reads the cleanup task's handle slot, which only
///   `McpHandler::shutdown` can empty.
#[tokio::test]
async fn streamable_http_exit_stops_the_ttl_cleanup_task() {
    let handler = handler_with_live_reaper().await;
    let framework = handler
        .framework
        .get()
        .expect("the handler's framework is initialized");
    let task = framework
        .cleanup
        .as_ref()
        .expect("cleanup_interval_seconds > 0 spawns the shared task");
    assert!(
        task.handle.lock().await.is_some(),
        "the cleanup task must be live before the exit"
    );

    let bind = reserve_ephemeral_port().await;
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server_handler = handler.clone();
    let running = tokio::spawn(async move {
        serve_with_handler(
            server_handler,
            Transport::StreamableHttp { bind },
            None,
            None,
            Vec::new(),
            async move {
                let _ = shutdown_rx.await;
            },
        )
        .await
    });

    // Readiness without a sleep: while the server has not bound, the connect
    // fails and `yield_now` hands the runtime back to the server task. A server
    // that died during startup surfaces its own error here instead of turning
    // the test into a timeout.
    loop {
        if tokio::net::TcpStream::connect(bind).await.is_ok() {
            break;
        }
        if running.is_finished() {
            let outcome = running.await.expect("the server task must not panic");
            panic!("the Streamable HTTP server stopped before it was listening: {outcome:?}");
        }
        tokio::task::yield_now().await;
    }

    // The exit: signal, then await. `Serve` resolves `Ok(())` because
    // `with_graceful_shutdown` exists; before the fix this await never finished.
    shutdown_tx
        .send(())
        .expect("the shutdown receiver is still held by the transport");
    running
        .await
        .expect("the server task must not panic")
        .expect("serve must return Ok(()) once the shutdown signal resolves");

    assert!(
        task.handle.lock().await.is_none(),
        "the Streamable HTTP exit path must cancel and await the shared TTL cleanup task (ADR-0099)"
    );
}

/// An HTTP server that never served a request has no framework, so the exit
/// path must return `Ok(())` rather than build one just to stop it.
#[tokio::test]
async fn streamable_http_exit_is_ok_without_a_request() {
    let bind = reserve_ephemeral_port().await;
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        serve_with_handler(
            Arc::new(McpHandler::new(None)),
            Transport::StreamableHttp { bind },
            None,
            None,
            Vec::new(),
            async move {
                let _ = shutdown_rx.await;
            },
        )
        .await
    });

    shutdown_tx
        .send(())
        .expect("the shutdown receiver is still held by the transport");
    running
        .await
        .expect("the server task must not panic")
        .expect("an idle Streamable HTTP server must still exit cleanly");
}

/// The public entry point must not invent a framework either: `serve` is
/// `serve_with_shutdown(config, ctrl_c)`, and with `ttl_cleanup_interval: 0` the
/// transport exit still resolves without the reaper ever starting. This pins the
/// wiring so `serve` cannot silently drop the shutdown hop.
///
/// Readiness is asserted before the signal is sent, exactly as in
/// [`streamable_http_exit_stops_the_ttl_cleanup_task`]: without it a
/// `serve_with_shutdown` body that returned `Ok(())` immediately would satisfy
/// this test, because the signal would merely resolve a future nothing ever
/// waited on.
#[tokio::test]
async fn serve_with_shutdown_returns_when_the_signal_resolves() {
    let bind = reserve_ephemeral_port().await;
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let config = McpConfig {
        transport: Transport::StreamableHttp { bind },
        bind: Some(bind.to_string()),
        database: None,
        auth_token: None,
        allowed_hosts: None,
        allowed_origins: Vec::new(),
        ttl_cleanup_interval: 0,
    };
    let running = tokio::spawn(async move {
        serve_with_shutdown(config, async move {
            let _ = shutdown_rx.await;
        })
        .await
    });

    loop {
        if tokio::net::TcpStream::connect(bind).await.is_ok() {
            break;
        }
        if running.is_finished() {
            let outcome = running.await.expect("the server task must not panic");
            panic!("serve_with_shutdown stopped before it was listening: {outcome:?}");
        }
        tokio::task::yield_now().await;
    }

    shutdown_tx
        .send(())
        .expect("the shutdown receiver is still held by the transport");
    running
        .await
        .expect("the server task must not panic")
        .expect("serve_with_shutdown must return Ok(()) once the signal resolves");
}

#[tokio::test]
async fn non_loopback_bind_requires_authentication() {
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let result = serve_with_shutdown(
        McpConfig {
            transport: Transport::StreamableHttp {
                bind: "0.0.0.0:0".parse().expect("valid socket address"),
            },
            auth_token: None,
            ..McpConfig::default()
        },
        async move {
            let _ = shutdown_rx.await;
        },
    )
    .await;
    drop(shutdown_tx);
    assert!(
        result.is_err(),
        "unauthenticated public binds must be rejected"
    );
}

#[test]
fn debug_output_redacts_bearer_token() {
    let config = McpConfig {
        auth_token: Some("secret-token".into()),
        ..McpConfig::default()
    };
    let rendered = format!("{config:?}");
    assert!(!rendered.contains("secret-token"));
    assert!(rendered.contains("[REDACTED]"));
}

#[cfg(feature = "otlp")]
#[test]
fn http_traceparent_is_extracted_as_a_remote_parent() {
    use opentelemetry::trace::TraceContextExt as _;

    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "traceparent",
        axum::http::HeaderValue::from_static(
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
        ),
    );
    let parent = extract_trace_parent(&headers).span().span_context().clone();
    assert!(parent.is_valid());
    assert_eq!(
        parent.trace_id().to_string(),
        "4bf92f3577b34da6a3ce929d0e0e4736"
    );
    assert_eq!(parent.span_id().to_string(), "00f067aa0ba902b7");
    assert_eq!(parent.trace_flags().to_u8(), 1);

    headers.insert(
        "traceparent",
        axum::http::HeaderValue::from_static("invalid"),
    );
    assert!(
        !extract_trace_parent(&headers)
            .span()
            .span_context()
            .is_valid()
    );
}
