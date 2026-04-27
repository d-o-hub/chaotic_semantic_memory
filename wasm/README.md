# @d-o-hub/chaotic_semantic_memory

WASM bindings for chaotic_semantic_memory - AI memory systems with hyperdimensional vectors and chaotic reservoirs.

## Installation

```bash
npm install @d-o-hub/chaotic_semantic_memory
```

## Quick Start

```javascript
import init, { WasmFramework, random_hypervector } from '@d-o-hub/chaotic_semantic_memory';

await init();
const framework = await WasmFramework.new();

// Inject concepts
const catVector = random_hypervector();
const dogVector = random_hypervector();
await framework.inject_concept('cat', catVector);
await framework.inject_concept('dog', dogVector);

// Create association
await framework.associate('cat', 'dog', 0.8);

// Query similar concepts
const hits = await framework.probe(catVector, 5);
console.log(hits);

// Export state for persistence
const exported = await framework.exportToBytes();
```

## API

### WasmFramework

`WasmFramework` mirrors the async Rust API. Method names follow snake_case to match the generated bindings.

```typescript
class WasmFramework {
    static new(): Promise<WasmFramework>;

    inject_concept(id: string, vector: Uint8Array): Promise<void>;
    get_concept(id: string): Promise<Concept | null>;
    delete_concept(id: string): Promise<void>;

    probe(vector: Uint8Array, topK: number): Promise<SimilarityHit[]>;
    probe_filtered(vector: Uint8Array, topK: number, filterJson: string): Promise<SimilarityHit[]>;
    probe_text(query: string, topK: number): Promise<SimilarityHit[]>;

    associate(from: string, to: string, strength: number): Promise<void>;
    get_associations(id: string): Promise<Array<{ to: string; strength: number }>>;

    processSequence(inputs: Float32Array[]): Promise<Uint8Array>;

    exportToBytes(): Promise<Uint8Array>;
    importFromBytes(data: Uint8Array, merge: boolean): Promise<number>;

    metrics_snapshot(): Promise<any>;
    stats(): Promise<any>;
}
```

### Hypervector helpers

```typescript
random_hypervector(): Uint8Array;
cosine_similarity(a: Uint8Array, b: Uint8Array): number;
encode_text(text: string): Uint8Array;
```

## Limitations

- **In-memory only** - No persistence in WASM build
- **No threading** - Parallelization is disabled
- **Browser/Node** - Works in both environments

## License

MIT
