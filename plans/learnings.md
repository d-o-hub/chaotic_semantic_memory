# Learnings

## 2026-05-02 — PR #159: MCP Server Implementation

### What was fixed
- MCP Server now remains active on stdio by awaiting `.waiting()` on the service handle.
- Missing tool arguments from clients now default to an empty JSON object instead of `null`, preventing deserialization failures.
- `concept://{id}` is now correctly exposed as a resource template for discovery.
- `stats://current` and `health://current` resources are now fully implemented and return live framework data.
- Fixed CLI build failure by correctly gating the `mcp` subcommand and its imports.

### Patterns to remember
- Always await the service handle returned by `rmcp::service::serve_server` to prevent the process from exiting immediately.
- Use resource templates for parameterized URIs in MCP to allow clients to discover valid parameter names.
- Ensure all new CLI subcommands are feature-gated to maintain standard build compatibility.
