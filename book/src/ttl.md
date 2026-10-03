# Concept TTL

Concepts can be given a **time-to-live (TTL)** so they automatically expire. Expired concepts are filtered from probe results and can be bulk-purged from memory.

## Why It Exists

- Ephemeral context (chat turns, session data) should not persist forever
- Avoids unbounded memory growth without manual cleanup
- Expired concepts are still stored until explicitly purged, keeping deletion costs amortised

## Setting TTL on a Concept

### With `ConceptBuilder`

```rust,no_run
use chaotic_semantic_memory::ConceptBuilder;
use chaotic_semantic_memory::HVec10240;

let concept = ConceptBuilder::new("session-ctx-42")
    .with_vector(HVec10240::random())
    .with_ttl(3600) // expires in 1 hour
    .build()
    .unwrap();

assert!(concept.expires_at.is_some());
```

If `with_ttl` is omitted the concept never expires (`expires_at` is `None`).

### Framework Convenience APIs

```rust,no_run
use chaotic_semantic_memory::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await?;

    // Inject a raw vector with TTL
    framework
        .inject_concept_with_ttl("vec-1", HVec10240::random(), 600)
        .await?;

    // Inject text with TTL (encodes via TextEncoder)
    framework
        .inject_text_with_ttl("note-1", "meeting notes from standup", 1800)
        .await?;

    Ok(())
}
```

## How Expiry Works

A concept is considered **expired** when `expires_at <= now` (UNIX seconds). The `Singularity` layer exposes helpers to inspect and act on expiry:

| Method | Description |
|---|---|
| `is_expired(id)` | Check whether a single concept has expired |
| `active_concept_ids()` | List all concept IDs that are still alive |
| `purge_expired()` | Delete all expired concepts and invalidate the similarity cache |

### Purging Expired Concepts

```rust,no_run
use chaotic_semantic_memory::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await?;

    // ... inject concepts with TTL ...

    let removed = framework.purge_expired().await?;
    println!("Purged {removed} expired concepts");
    Ok(())
}
```

`purge_expired` iterates stored concepts, collects those past their deadline, deletes them, and invalidates the similarity cache so subsequent probes reflect the new state.

## Background Cleanup in Long-Running Processes

A process that stays up — `csm watch`, `csm mcp serve` — can run `purge_expired` on a timer instead of waiting for a caller to invoke it. Both commands accept:

```console
$ csm watch --ttl-cleanup-interval 60
$ csm mcp serve --ttl-cleanup-interval 60
```

The value is **seconds between cleanup passes, and `0` (the default) disables the reaper entirely**, so upgrading does not start deleting expired concepts from your store: the flag has to be asked for. Expiry is still filtered at probe time either way, so leaving it at `0` only defers reclaiming the rows.

The flag is threaded into `TtlConfig::cleanup_interval_seconds` (library users can set it directly through `ChaoticSemanticFramework::builder().with_ttl_config(..)`) and gates the task spawned by `framework_cleanup::spawn_cleanup_task` (ADR-0099). One task serves every clone of the framework, and it is stopped on the command's exit path by `ChaoticSemanticFramework::shutdown()` — `csm watch` on Ctrl+C, `csm mcp serve` when the transport ends. For a library embedding, call `shutdown()` yourself when the server stops; dropping the last framework handle also stops the task, but cooperatively at its next check.

## Behaviour Notes

- **Probe filtering** — expired concepts are automatically excluded during probe operations so stale results are never returned.
- **Lazy deletion** — expiry does not remove data unless something asks for it: an explicit `purge_expired` call, or a long-running command started with a nonzero `--ttl-cleanup-interval`. One-shot CLI commands never run a reaper.
- **Persistence** — when persistence is enabled, `inject_concept_with_ttl` and `inject_text_with_ttl` save the concept (including its `expires_at` timestamp) to the database. Purging only removes concepts from the in-memory `Singularity`; see [Configuration](./configuration.md) for persistence details.
