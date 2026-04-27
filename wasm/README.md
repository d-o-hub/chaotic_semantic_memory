# @d-o-hub/chaotic_semantic_memory

WASM bindings for chaotic_semantic_memory - AI memory systems with hyperdimensional vectors and chaotic reservoirs.

## Installation

```bash
npm install @d-o-hub/chaotic_semantic_memory
```

## Quick Start

```javascript
import init, { ChaoticSemanticFramework, HVec10240 } from '@d-o-hub/chaotic_semantic_memory';

// Initialize WASM module
await init();

// Create framework
const framework = new ChaoticSemanticFramework();
await framework.buildWithoutPersistence();

// Inject concepts
const catVector = HVec10240.random();
const dogVector = HVec10240.random();
await framework.injectConcept('cat', catVector);
await framework.injectConcept('dog', dogVector);

// Create association
await framework.associate('cat', 'dog', 0.8);

// Query similar concepts
const hits = await framework.probe(catVector, 5);
console.log(hits); // [{id: 'cat', similarity: 1.0}, {id: 'dog', similarity: 0.7}]

// Process temporal sequence
const sequence = [
    new Float32Array(10240).fill(0.1),
    new Float32Array(10240).fill(0.2)
];
const temporalVector = await framework.processSequence(sequence);
```

## API

### ChaoticSemanticFramework

```typescript
class ChaoticSemanticFramework {
    // Build
    buildWithoutPersistence(): Promise<void>;
    buildWithLocalDb(path: string): Promise<void>;

    // Concepts
    injectConcept(id: string, vector: Uint8Array): Promise<void>;
    getConcept(id: string): Promise<Concept | null>;
    deleteConcept(id: string): Promise<void>;

    // Queries
    probe(vector: Uint8Array, topK: number): Promise<SimilarityHit[]>;

    // Associations
    associate(from: string, to: string, strength: number): Promise<void>;
    getAssociations(id: string): Promise<SimilarityHit[]>;

    // Temporal
    processSequence(inputs: Float32Array[]): Promise<Uint8Array>;

    // Export/Import
    exportToBytes(): Promise<Uint8Array>;
    importFromBytes(data: Uint8Array): Promise<number>;
}
```

### HVec10240

```typescript
class HVec10240 {
    static random(): Uint8Array;
    static zero(): Uint8Array;
    static bundle(vectors: Uint8Array[]): Uint8Array;
    static bind(a: Uint8Array, b: Uint8Array): Uint8Array;
    static permute(v: Uint8Array, shift: number): Uint8Array;
    static cosineSimilarity(a: Uint8Array, b: Uint8Array): number;
    static hammingDistance(a: Uint8Array, b: Uint8Array): number;
}
```

## Limitations

- **In-memory only** - No persistence in WASM build
- **No threading** - Parallelization is disabled
- **Browser/Node** - Works in both environments

## License

MIT
