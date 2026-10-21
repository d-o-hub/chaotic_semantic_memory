# ADR-0100: Versioned Postcard Memory Export

## Status

Accepted

## Context and Problem Statement

Public memory exports are read and written by native file APIs, namespace byte APIs, and the WASM API. They previously depended on bincode 1.x, which is unmaintained (RUSTSEC-2025-0141), while separate bincode option paths made compatibility dependent on which API produced the bytes. Changing the public format without a compatibility reader would strand existing snapshots. Exports and imports must remain bounded by the existing 100 MiB limit.

## Decision Drivers

- Use a maintained binary codec for newly written public memory snapshots.
- Keep native, namespace, and WASM bytes interoperable.
- Bound output during serialization, not only after a potentially large allocation.
- Preserve versioned and historical un-enveloped snapshots.
- Validate a replacement import completely before committing any state.

## Considered Options

- Keep writing bincode and document its maintenance risk.
- Replace binary snapshots with JSON, trading compactness for human readability.
- Add an explicit envelope and write Postcard while retaining bounded legacy readers.

## Decision Outcome

Chosen option: **versioned Postcard writes with bounded legacy readers**, because it gives new snapshots an explicit maintained codec contract without stranding existing exports.

The shared wire envelope is `CSMBIN01`, followed by format version `1`, codec identifier `2` (Postcard), then the payload. The writer streams through a size-limited `std::io::Write` implementation and caps the entire envelope at `MAX_IMPORT_SIZE`. The decoder continues to accept version-1 fixed-int bincode envelopes and the historical fixed-int and varint un-enveloped bincode payloads. New native binary exports, namespace byte exports, and WASM `exportToBytes` share this encoder; all corresponding imports share the decoder.

Replacement imports stage and validate concepts and associations before changing memory or durable rows. Invalid association endpoints reject the whole replacement. Derived ANN snapshots and scoped absence-cache entries are rebuilt or invalidated on restore rather than treated as authoritative data. Bincode remains only for legacy public readers and internal ANN/index persistence until those internal formats receive a separate migration.

### Positive Consequences

- New public snapshots use a maintained codec and have one explicit version/codec boundary.
- Native and WASM clients exchange the same bytes.
- Serialization cannot grow beyond the envelope limit before the check.
- Existing exported files remain readable without continuing to write bincode.
- Restore rebuilds derived state from authoritative rows and drops revision-scoped absences.

### Negative Consequences

- The decoder retains legacy bincode code until the support window for existing exports is deliberately ended.
- The format is not human-readable; JSON remains available for inspectable exports.
- Adding Postcard is an additional dependency and the codec id becomes a compatibility contract.

## Pros and Cons of the Options

### Continue writing bincode

- Good, because it avoids format migration and dependency changes.
- Bad, because the public format remains on an unmaintained crate and still needs explicit compatibility rules.

### Switch public snapshots to JSON

- Good, because users can inspect and hand-edit snapshots.
- Bad, because vector-heavy snapshots are much larger and it does not provide one compact browser/native binary contract.

### Versioned Postcard with compatibility readers

- Good, because it makes compatibility explicit while avoiding a silent break.
- Bad, because the old-reader code and historical fixtures must be maintained during migration.

## Follow-up and Verification

- Keep codec version and identifier assertions in `framework_binary_compat_tests.rs`.
- Exercise native/WASM shared paths and malformed replacement behavior.
- Keep internal ANN bincode use out of the public memory export contract and revisit it separately.
