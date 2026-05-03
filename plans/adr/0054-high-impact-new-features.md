# ADR-0054: High-Impact New Features

## Status

Proposed (backfilled 2026-05-01) - Wave 15

## Context

High-impact features not implemented:
- Graph-based retrieval (HNSW)
- Embedding model bridge
- MCP server integration
- Advanced reranking

## Decision

Propose **high-impact new features**.

**Proposed Scope:**
- HNSW approximate nearest neighbor index
- Embedding model bridge (external models)
- MCP server for AI integration
- MMR reranking pipeline
- GraphRAG hybrid retrieval

## Consequences

### Positive
- Significant capability expansion
- AI integration ready
- Advanced retrieval options
- Competitive features

### Negative
- Large implementation scope
- Additional dependencies
- Complexity increase
- Feature maintenance

## Implementation

- Phase: 37-41 (Wave 15)
- Dependencies: Wave 14 completion
- Files: see existing ADRs 0068-0071

## Sources

- ADR_REGISTRY.md: High-Impact New Features (Proposed)
- ACTIONS.md lines 2017-2199
- Existing ADRs: 0066-0075 cover these features