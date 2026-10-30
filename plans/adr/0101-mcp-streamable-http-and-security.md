# ADR-0101: MCP Streamable HTTP Compatibility and Security

## Status

Accepted

## Context and Problem Statement

The repository's MCP server exposed an `Sse` transport name even though HTTP+SSE is deprecated by the audited 2026-07-28 protocol. The current Streamable HTTP protocol is stateless and adds `server/discover`; older clients may still negotiate a legacy session-based protocol. The HTTP listener also needs explicit host, origin, and authentication policy, deterministic tool ordering, cancellation, and trace-context handling.

## Decision Drivers

- Advertise the current MCP protocol while retaining supported legacy clients.
- Avoid misleading public transport names and hidden session requirements for current clients.
- Require authentication for non-loopback network binds.
- Reject unexpected Host and Origin values, and compare bearer tokens without data-dependent equality.
- Keep tool-list order stable and honor cancellation and W3C trace context.

## Considered Options

- Keep the old SSE-labelled API and session behavior.
- Drop legacy protocol support and implement only current stateless requests.
- Rename the transport to Streamable HTTP, use rmcp's current stateless path, and retain its session manager only for negotiated legacy requests.

## Decision Outcome

Chosen option: **current Streamable HTTP with bounded legacy compatibility**, because current clients get stateless requests by default while supported older clients keep negotiated compatibility.

The public transport variants are `Stdio` and `StreamableHttp`; the removed `Sse` name is not retained as an alias. rmcp handles current stateless Streamable HTTP requests, including discovery, and the local session manager remains available only for negotiated legacy protocol versions. The server negotiates the latest rmcp protocol version it supports and returns deterministic tool ordering.

For Streamable HTTP, an empty bearer token is rejected; a non-loopback bind requires a token. The default Host allow-list is limited to loopback/bind addresses, callers may configure explicit hosts and origins, and Origin validation is enforced. Authorization uses constant-time token comparison. Cancellation is returned as the protocol cancellation error. Incoming W3C `traceparent` is extracted when OTLP tracing is enabled. Stdio remains the local-process transport and does not use HTTP auth/Host/Origin checks.

### Positive Consequences

- Current clients use stateless requests without a session id.
- Supported legacy clients retain rmcp-managed session compatibility.
- Network exposure requires explicit authentication and host/origin policy.
- Stable tool ordering supports client-side caching; cancellation and traceparent have explicit conformance tests.

### Negative Consequences

- Renaming `Transport::Sse` to `Transport::StreamableHttp` is a breaking Rust API change.
- Adopting the `2026-07-28` revision inherits SEP-2243 header enforcement: from that version rmcp requires
  `Mcp-Method` on every HTTP request (plus `Mcp-Name` for `tools/call`) and must see the body method echoed
  there, answering a missing or mismatched value with JSON-RPC `-32020` over HTTP 400
  (`rmcp::transport::common::mcp_headers::validate_request_headers`). Clients that predate SEP-2243 can only
  reach this server through the negotiated legacy session path, and one test file therefore holds both
  behaviours — the `2024-11-05` session test sends no such header and must not.
- Legacy protocol behavior remains in rmcp and must not be confused with the current stateless path.
- Operators exposing non-loopback HTTP must provision and rotate bearer tokens.

## Pros and Cons of the Options

### Keep the SSE label and session shape

- Good, because it minimizes immediate source changes for existing callers.
- Bad, because it misrepresents the current protocol and advertises deprecated transport terminology.

### Remove legacy support

- Good, because the server would have one protocol path.
- Bad, because rmcp already supports negotiated legacy compatibility and removing it would unnecessarily break older clients.

### Current stateless protocol plus negotiated legacy support

- Good, because it is current by default while retaining a bounded compatibility path.
- Bad, because tests must distinguish stateless and legacy requests explicitly.

## Follow-up and Verification

- Keep protocol negotiation, discovery, list ordering, no-session-id, legacy session, cancellation, auth, host/origin, and trace-context tests.
- Document the breaking transport rename and the bearer-token environment variable.
- Do not treat stdio-only tests as evidence for HTTP lifecycle or security behavior.
