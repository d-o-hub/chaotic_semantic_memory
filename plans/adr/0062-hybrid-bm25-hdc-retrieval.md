# ADR-0062: Hybrid BM25+HDC Retrieval with Query-Length-Dependent Scoring

## Status
Proposed

## Context

Community feedback from [github-template-ai-agents#121](https://github.com/d-o-hub/github-template-ai-agents/issues/121) highlights a gap in the current retrieval pipeline: for short queries under 5 tokens (function names, error strings, constants), exact keyword matching consistently outperforms semantic similarity search. Users searching for `MAX_CONTEXT_TOKENS`, `get_user_by_id`, or `UTF-8 truncation panic` get poor results from HDC embeddings alone.

### Current Retrieval Path

```
query.rs::run_query()
    → TextEncoder.encode(text)
    → framework.probe(vector, top_k)
    → singularity.find_similar(vector, top_k)
    → cosine_similarity for all candidates
    → filter by min_score
```

**Problem**: Single encoding path, no keyword fallback, no hybrid scoring.

### Observed Behavior

- **Short queries** (<5 tokens): Exact identifiers return irrelevant results because HDC cosine similarity dilutes exact matches
- **Long natural language queries**: Work well with HDC but would benefit from keyword reinforcement
- **No mechanism** to combine the strengths of both approaches

## Decision

### 1. Add BM25 Keyword Index Alongside HDC Store

Implement a lightweight BM25 index that runs in parallel with the HDC probe:

- Index tokens at ingest time (`index-jsonl`, `index-dir`) into a keyword index
- Store in the same SQLite database (separate table, no schema migration for existing data)
- Zero external dependencies — pure Rust implementation
- Feature-gated: `#[cfg(feature = "bm25")]` with default enabled

### 2. Query-Length-Dependent Scoring

Combine keyword and semantic scores with weights that shift based on query length:

| Query Tokens | Keyword Weight | Semantic Weight | Rationale |
|-------------|---------------|-----------------|-----------|
| 1-2 | 0.9 | 0.1 | Exact match dominates |
| 3-4 | 0.7 | 0.3 | Keyword still strong |
| 5-8 | 0.4 | 0.6 | Semantic takes over |
| 9+ | 0.2 | 0.8 | Full semantic mode |

**Score Formula**:
```
final_score = w_kw * normalize(bm25_score) + w_sem * normalize(hdc_score)
```

**Normalization**: Min-max normalization across result set:
```
normalize(score) = (score - min) / (max - min + ε)
```

### 3. BM25 Algorithm Parameters

- **k1**: 1.2 (term frequency saturation parameter)
- **b**: 0.75 (document length normalization)
- **Average document length**: Computed dynamically from index

**BM25 Score**:
```
score(D, Q) = Σ IDF(qi) * (f(qi, D) * (k1 + 1)) / (f(qi, D) + k1 * (1 - b + b * |D| / avgdl))
```

Where:
- `IDF(qi) = ln((N - n(qi) + 0.5) / (n(qi) + 0.5) + 1)`
- `f(qi, D)` = frequency of term qi in document D
- `|D|` = document length
- `avgdl` = average document length

### 4. CLI Interface

```bash
# Default: hybrid mode (auto-weighted by query length)
csm query "get_user_by_id" --top-k 5

# Force semantic-only (current behavior)
csm query "how to handle git worktree cleanup" --semantic-only

# Force keyword-only
csm query "MAX_CONTEXT_TOKENS" --keyword-only

# Custom weights
csm query "something in between" --keyword-weight 0.5
```

### 5. Module Structure

| Module | Purpose | LOC |
|--------|---------|-----|
| `src/retrieval/mod.rs` | Re-exports | ~10 |
| `src/retrieval/bm25.rs` | BM25 implementation | ~250 |
| `src/retrieval/hybrid.rs` | Score combination, normalization | ~150 |
| Changes to `src/cli/args.rs` | New flags | ~20 |
| Changes to `src/cli/commands/query.rs` | Hybrid logic | ~80 |

Total: ~500 LOC (under limit)

## Implementation Phases

### Phase 60: BM25 Index Module (cost: 8)
- Create `src/retrieval/mod.rs` and `src/retrieval/bm25.rs`
- Implement `Bm25Index` with `add_document`, `search`, `clear`
- Tokenization using `TextEncoder::tokenize_code` patterns
- In-memory index (persistence optional)

### Phase 61: Hybrid Scoring (cost: 4)
- Create `src/retrieval/hybrid.rs`
- Implement `compute_weights(token_count)` and `normalize_scores(scores)`
- Implement `merge_results(bm25_results, hdc_results, weights)`

### Phase 62: CLI Integration (cost: 6)
- Add flags to `QueryArgs`: `--hybrid`, `--semantic-only`, `--keyword-only`, `--keyword-weight`
- Update `query.rs` to run parallel searches
- Deduplicate by ID with max score

### Phase 63: Tests (cost: 4)
- Unit tests for BM25: IDF, TF, normalization
- Integration test: `get_user_by_id` returns exact match at #1
- Benchmark: hybrid vs pure HDC latency

## Consequences

### Positive
- Improved recall for short queries (exact matches)
- No loss of semantic capability for long queries
- Configurable via CLI flags
- No external dependencies
- WASM compatible (feature-gated)

### Negative
- Increased memory usage for keyword index
- Additional latency for parallel searches (mitigated by async)
- More complex query path

### Risks
- BM25 index memory growth with large corpora
- Score normalization edge cases (all scores equal)
- Tokenization drift between ingest and query

## Alternatives Considered

1. **External embedding model**: Introduces drift, vendor lock-in, compute cost
2. **BM25 only**: Loses semantic understanding for long queries
3. **Learned weights**: Too complex, requires training data
4. **Tantivy dependency**: Overkill, adds heavy dependency

## References

- [Robertson & Zaragoza, 2009: The Probabilistic Relevance Framework: BM25 and Beyond](https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/SigIRForum2009.pdf)
- [github-template-ai-agents#121](https://github.com/d-o-hub/github-template-ai-agents/issues/121)
- Related: ADR-0059 (Retrieval Optimization)