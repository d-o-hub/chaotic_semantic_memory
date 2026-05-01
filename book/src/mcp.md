# Model Context Protocol (MCP)

`chaotic_semantic_memory` supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/), allowing it to be used as a first-class memory provider for LLM agents like Claude Desktop, Cursor, and VS Code Copilot.

## Usage

You can start the MCP server using the `csm mcp serve` command. By default, it uses the **stdio** transport, which is suitable for desktop integrations.

```bash
csm mcp serve
```

For hosted deployments, you can use the **SSE** (Server-Sent Events) transport:

```bash
csm mcp serve --transport sse --bind 127.0.0.1:3030
```

## Configuration

The MCP server uses the same database resolution logic as the CLI:
1. Explicit `--database PATH`
2. Git-local storage if in a git repository
3. In-memory mode otherwise

### Claude Desktop Integration

To use CSM with Claude Desktop, add the following to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "csm": {
      "command": "csm",
      "args": ["mcp", "serve"]
    }
  }
}
```

## Tools

The MCP server exposes 12 tools for interacting with memory:

| Tool | Description |
|------|-------------|
| `memory_inject` | Store a concept with vector and metadata |
| `memory_inject_text` | Store a concept from text |
| `memory_probe` | Top-K similarity search with vector |
| `memory_probe_text` | Top-K similarity search with text query |
| `memory_probe_filtered` | Filtered similarity search with text query |
| `memory_get` | Fetch concept by ID |
| `memory_delete` | Remove a concept |
| `memory_associate` | Create directed association |
| `memory_traverse` | BFS from concept |
| `memory_shortest_path` | Path between concepts |
| `memory_stats` | DB stats snapshot |
| `memory_export` | Export to JSON |

## Resources

Three resource types are available:

- `concept://{id}`: JSON representation of a single concept.
- `stats://current`: Live framework statistics.
- `health://current`: Persistence health check status.
