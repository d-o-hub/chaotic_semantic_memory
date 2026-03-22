# WASM Bindings

## Building

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --no-default-features --features wasm
```

Using wasm-pack:

```bash
wasm-pack build --target web --no-default-features --features wasm
```

## JavaScript API

```javascript
import init, { WasmFramework } from 'chaotic_semantic_memory';

await init();

// Create framework
const framework = new WasmFramework();

// Inject concept
const vector = WasmFramework.random_vector();
await framework.inject_concept('my-concept', vector);

// Query similar
const hits = await framework.probe(vector, 10);
console.log(hits); // [{id: 'my-concept', score: 1.0}]

// Create association
await framework.associate('cat', 'dog', 0.8);

// Process sequence
const sequence = [
    new Float32Array([0.1, 0.2, ...]),
    new Float32Array([0.3, 0.4, ...])
];
const resultVector = await framework.process_sequence(sequence);

// Export/Import
const exported = await framework.export_to_bytes();
await framework.import_from_bytes(exported);
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
    score: number;
}

declare class WasmFramework {
    inject_concept(id: string, vector: Uint8Array): Promise<void>;
    get_concept(id: string): Promise<Concept | null>;
    delete_concept(id: string): Promise<void>;
    probe(vector: Uint8Array, topK: number): Promise<SimilarityHit[]>;
    associate(from: string, to: string, strength: number): Promise<void>;
    get_associations(id: string): Promise<SimilarityHit[]>;
    disassociate(from: string, to: string): Promise<void>;
    update_concept(id: string, vector: Uint8Array): Promise<void>;
    process_sequence(inputs: Float32Array[]): Promise<Uint8Array>;
    export_to_bytes(): Promise<Uint8Array>;
    import_from_bytes(data: Uint8Array): Promise<number>;
    neighbors(id: string, minStrength: number): Promise<string[]>;
    bfs(start: string): Promise<string[]>;
    shortest_path(from: string, to: string): Promise<string[] | null>;
}

declare class WasmFramework {
    static random_vector(): Uint8Array;
    static bundle(vectors: Uint8Array[]): Uint8Array;
    static bind(a: Uint8Array, b: Uint8Array): Uint8Array;
    static cosine_similarity(a: Uint8Array, b: Uint8Array): number;
}
```

## Limitations

- **No persistence** - WASM build excludes filesystem access
- **No threading** - Rayon parallelization is gated
- **Memory only** - All data is in-memory for browser context

## NPM Package

```bash
npm install @d-o-hub/chaotic_semantic_memory
```

Or use directly with npx:

```bash
npx @d-o-hub/chaotic_semantic_memory
```
