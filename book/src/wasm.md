# WASM Bindings

## Building

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --features wasm
```

Using wasm-pack:

```bash
wasm-pack build --target web --features wasm
```

## JavaScript API

```javascript
import init, { 
    ChaoticSemanticFramework, 
    HVec10240 
} from 'chaotic_semantic_memory';

await init();

// Create framework
const framework = new ChaoticSemanticFramework();
await framework.buildWithoutPersistence();

// Inject concept
const vector = HVec10240.random();
await framework.injectConcept('my-concept', vector);

// Query similar
const hits = await framework.probe(vector, 10);
console.log(hits); // [{id: 'my-concept', similarity: 1.0}]

// Create association
await framework.associate('cat', 'dog', 0.8);

// Process sequence
const sequence = [
    new Float32Array([0.1, 0.2, ...]),
    new Float32Array([0.3, 0.4, ...])
];
const resultVector = await framework.processSequence(sequence);

// Export/Import
const exported = await framework.exportToBytes();
await framework.importFromBytes(exported);
```

## TypeScript Types

```typescript
interface Concept {
    id: string;
    vector: Uint8Array;
    metadata?: string;
}

interface SimilarityHit {
    id: string;
    similarity: number;
}

declare class ChaoticSemanticFramework {
    buildWithoutPersistence(): Promise<void>;
    injectConcept(id: string, vector: Uint8Array): Promise<void>;
    getConcept(id: string): Promise<Concept | null>;
    deleteConcept(id: string): Promise<void>;
    probe(vector: Uint8Array, topK: number): Promise<SimilarityHit[]>;
    associate(from: string, to: string, strength: number): Promise<void>;
    getAssociations(id: string): Promise<SimilarityHit[]>;
    processSequence(inputs: Float32Array[]): Promise<Uint8Array>;
    exportToBytes(): Promise<Uint8Array>;
    importFromBytes(data: Uint8Array): Promise<number>;
}

declare class HVec10240 {
    static random(): Uint8Array;
    static bundle(vectors: Uint8Array[]): Uint8Array;
    static bind(a: Uint8Array, b: Uint8Array): Uint8Array;
    static cosineSimilarity(a: Uint8Array, b: Uint8Array): number;
}
```

## Limitations

- **No persistence** - WASM build excludes filesystem access
- **No threading** - Rayon parallelization is gated
- **Memory only** - All data is in-memory for browser context

## NPM Package

```bash
npm install @d-o-hub/chaotic-semantic-memory
```

Or use directly with npx:

```bash
npx @d-o-hub/chaotic-semantic-memory
```
