# ADR-0090: MCP SSE Transport and Integration Tests

## Status

Proposed

## Context and Problem Statement

The MCP server module (`src/mcp/`) documents support for both stdio and SSE
transports, but only stdio is implemented. The `Transport` enum in
`src/mcp/server.rs` has a single `Stdio` variant, and `McpConfig::bind` is
unused. Additionally, the MCP module has zero integration tests — only 5 unit
tests in `src/mcp/handler.rs` testing `parse_hvec`.

This leaves two gaps:
1. **SSE transport**: Documented but unimplemented. Users expecting HTTP-based
   MCP access will hit "SSE is currently unsupported" errors.
2. **Test coverage**: No integration tests exercise tool execution
   (memory_inject, memory_probe, etc.), resource reads, server initialization,
   or error handling paths.

## Decision

### Phase 1: MCP Integration Tests (cost: 4)

Add `tests/mcp_integration.rs` covering:
- Tool execution: memory_inject, memory_probe, memory_associate
- Resource read operations
- Server initialization flow
- Error handling paths (invalid tool names, malformed params)
- Handler dispatch for all registered tools

### Phase 2: SSE Transport (cost: 12, delegate to Jules)

Implement SSE transport variant:
- Add `Sse { bind: SocketAddr }` variant to `Transport` enum
- Implement HTTP server with SSE endpoint using hyper
- Wire `McpConfig::bind` to start the SSE listener
- Add CLI `--mcp-transport sse --mcp-bind 127.0.0.1:8080` support
- Integration test with HTTP client

## Consequences

- Phase 1 catches regressions in MCP tool dispatch without transport changes.
- Phase 2 enables HTTP-based MCP clients (VS Code extensions, web UIs).
- SSE transport (cost 12) should be delegated to Jules per AGENTS.md policy.

## References

- `src/mcp/handler.rs` — MCP request handler
- `src/mcp/server.rs` — Transport enum (currently stdio-only)
- `src/mcp/schema.rs` — JSON-RPC schema types
- ADR-0067 — Original MCP server decision
