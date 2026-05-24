/* tslint:disable */
/* eslint-disable */

export interface ProbeResult {
    id: string;
    score: number;
}

export interface AssociationResult {
    to: string;
    strength: number;
}

export interface FrameworkMetrics {
    concepts_injected_total: number;
    associations_created_total: number;
    probes_total: number;
    avg_probe_latency_ms: number;
    cache_hits_total: number;
    cache_misses_total: number;
    cache_evictions_total: number;
    reservoir_steps_total: number;
    avg_reservoir_step_latency_us: number;
    reservoir_nodes_active: number;
}

export interface FrameworkStats {
    concept_count: number;
    db_size_bytes: number | null;
}

export interface Concept {
    id: string;
    vector: Uint8Array;
    metadata: Record<string, any>;
    created_at: number;
    modified_at: number;
    expires_at: number | null;
    canonical_concept_ids: string[];
}

export interface VersionInfo {
    version: number;
    timestampUnix: number;
    vectorChanged: boolean;
    metadataChanged: boolean;
}

export interface GraphProbeResult {
    id: string;
    score: number;
    similarity: number;
    anchor_id: string | null;
    hop_distance: number;
    assoc_strength: number;
}

export interface TraversalResult {
    id: string;
    depth: number;
}

export type MemoryEvent =
| { type: "ConceptInjected"; id: string; timestamp: number }
| { type: "ConceptUpdated"; id: string; timestamp: number }
| { type: "ConceptDeleted"; id: string; timestamp: number }
| { type: "Associated"; from: string; to: string; strength: number }
| { type: "Disassociated"; from: string; to: string };



/**
 * WASM-friendly wrapper for the framework
 */
export class WasmFramework {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Associate two concepts
     */
    associate(from: string, to: string, strength: number): Promise<void>;
    /**
     * Create multiple associations in batch
     */
    associate_many(associations: Array<{ from: string; to: string; strength: number }>): Promise<void>;
    /**
     * Breadth-first traversal from a starting concept.
     *
     * Returns an Array of `{id: string, depth: number}` objects.
     * Uses default `TraversalConfig`.
     */
    bfs(start: string): Promise<TraversalResult[]>;
    /**
     * Clear all outbound associations for a concept.
     */
    clear_associations(id: string): Promise<void>;
    /**
     * Get concept count (convenience method)
     */
    concept_count(): Promise<number>;
    /**
     * Delete concept by ID
     */
    delete_concept(id: string): Promise<void>;
    /**
     * Remove an association between two concepts
     */
    disassociate(from: string, to: string): Promise<void>;
    /**
     * Export all concepts and associations to bytes for in-browser storage.
     */
    export_to_bytes(): Promise<Uint8Array>;
    /**
     * Get associations for a concept
     */
    get_associations(id: string): Promise<AssociationResult[]>;
    /**
     * Get a concept by ID
     */
    get_concept(id: string): Promise<Concept | null>;
    /**
     * Get the current namespace
     */
    get_namespace(): Promise<string>;
    /**
     * Load a specific concept version.
     */
    get_version(id: string, version: number): Promise<Concept | null>;
    /**
     * Import state from bytes previously produced by `export_to_bytes`.
     */
    import_from_bytes(data: Uint8Array, merge: boolean): Promise<number>;
    /**
     * Inject a concept
     */
    inject_concept(id: string, vector: Uint8Array): Promise<void>;
    /**
     * Inject multiple concepts in batch
     */
    inject_concepts(ids: string[], vectors: Uint8Array[]): Promise<void>;
    /**
     * Inject a concept from text
     */
    inject_text(id: string, text: string): Promise<void>;
    /**
     * List all historical versions of a concept.
     */
    list_versions(id: string): Promise<VersionInfo[]>;
    /**
     * Get framework metrics snapshot
     */
    metrics_snapshot(): Promise<FrameworkMetrics>;
    /**
     * Get direct neighbors of a concept with edge strengths.
     *
     * Returns an Array of `{to: string, strength: number}` objects.
     */
    neighbors(id: string, min_strength: number): Promise<AssociationResult[]>;
    /**
     * Create a new framework instance (no persistence in WASM)
     */
    static new(): Promise<WasmFramework>;
    /**
     * Register a callback for memory events.
     */
    on_event(callback: (event: MemoryEvent) => void): void;
    /**
     * Query for similar concepts
     */
    probe(vector: Uint8Array, top_k: number): Promise<ProbeResult[]>;
    /**
     * Probe for similar concepts with multiple queries in batch
     */
    probe_batch(vectors: Array<any>, top_k: number): Promise<ProbeResult[][]>;
    /**
     * Probe for similar concepts with metadata filtering.
     */
    probe_filtered(vector: Uint8Array, top_k: number, filter_json: string): Promise<ProbeResult[]>;
    /**
     * Probe for similar concepts using text
     */
    probe_text(query: string, top_k: number): Promise<Array<{ id: string, similarity: number }>>;
    /**
     * GraphRAG retrieval: similarity + graph traversal hybrid using text query.
     */
    probe_text_with_graph(text: string, anchor_top_k: number, max_hops: number, min_assoc_strength: number, similarity_weight: number, graph_weight: number, final_top_k: number): Promise<GraphProbeResult[]>;
    /**
     * GraphRAG retrieval: similarity + graph traversal hybrid using vector query.
     */
    probe_with_graph(vector: Uint8Array, anchor_top_k: number, max_hops: number, min_assoc_strength: number, similarity_weight: number, graph_weight: number, final_top_k: number): Promise<GraphProbeResult[]>;
    /**
     * Process a temporal sequence and return the resulting hypervector bytes.
     */
    process_sequence(sequence: Float32Array[]): Promise<Uint8Array>;
    /**
     * Roll back a concept to a historical version.
     */
    rollback_to_version(id: string, version: number): Promise<Concept>;
    /**
     * Set the current namespace
     */
    set_namespace(ns: string): Promise<void>;
    /**
     * Find the minimum-cost path between two concepts (weighted Dijkstra).
     *
     * Returns an Array of concept ID strings, or an empty Array if no path exists.
     * Uses default `TraversalConfig`.
     */
    shortest_path(from: string, to: string): Promise<string[]>;
    /**
     * Get framework stats
     */
    stats(): Promise<FrameworkStats>;
    /**
     * Breadth-first traversal from a starting concept with custom config.
     */
    traverse(start: string, max_depth: number, min_strength: number): Promise<TraversalResult[]>;
    /**
     * Update a concept's vector
     */
    update_concept(id: string, vector: Uint8Array): Promise<void>;
    /**
     * Update a concept's metadata from a JSON string.
     *
     * The `metadata_json` argument must be a valid JSON object string,
     * e.g. `{"category":"science","score":0.9}`.
     *
     * Note: In WASM, persistence is in-memory only. Use `export_to_bytes` to
     * snapshot state to IndexedDB or other storage.
     */
    update_concept_metadata(id: string, metadata_json: string): Promise<void>;
}

/**
 * Compute cosine similarity between two hypervectors
 */
export function cosine_similarity(a: Uint8Array, b: Uint8Array): number;

/**
 * Encode text to a hypervector using HDC encoding
 */
export function encode_text(text: string): Uint8Array;

export function initialize_wasm(): void;

/**
 * Create a random hypervector (1280 bytes)
 */
export function random_hypervector(): Uint8Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmframework_free: (a: number, b: number) => void;
    readonly cosine_similarity: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly encode_text: (a: number, b: number, c: number) => void;
    readonly random_hypervector: (a: number) => void;
    readonly wasmframework_associate: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly wasmframework_associate_many: (a: number, b: number) => number;
    readonly wasmframework_bfs: (a: number, b: number, c: number) => number;
    readonly wasmframework_clear_associations: (a: number, b: number, c: number) => number;
    readonly wasmframework_concept_count: (a: number) => number;
    readonly wasmframework_delete_concept: (a: number, b: number, c: number) => number;
    readonly wasmframework_disassociate: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly wasmframework_export_to_bytes: (a: number) => number;
    readonly wasmframework_get_associations: (a: number, b: number, c: number) => number;
    readonly wasmframework_get_concept: (a: number, b: number, c: number) => number;
    readonly wasmframework_get_namespace: (a: number) => number;
    readonly wasmframework_get_version: (a: number, b: number, c: number, d: number) => number;
    readonly wasmframework_import_from_bytes: (a: number, b: number, c: number) => number;
    readonly wasmframework_inject_concept: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly wasmframework_inject_concepts: (a: number, b: number, c: number) => number;
    readonly wasmframework_inject_text: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly wasmframework_list_versions: (a: number, b: number, c: number) => number;
    readonly wasmframework_metrics_snapshot: (a: number) => number;
    readonly wasmframework_neighbors: (a: number, b: number, c: number, d: number) => number;
    readonly wasmframework_new: () => number;
    readonly wasmframework_on_event: (a: number, b: number) => void;
    readonly wasmframework_probe: (a: number, b: number, c: number, d: number) => number;
    readonly wasmframework_probe_batch: (a: number, b: number, c: number) => number;
    readonly wasmframework_probe_filtered: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly wasmframework_probe_text: (a: number, b: number, c: number, d: number) => number;
    readonly wasmframework_probe_text_with_graph: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => number;
    readonly wasmframework_probe_with_graph: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => number;
    readonly wasmframework_process_sequence: (a: number, b: number) => number;
    readonly wasmframework_rollback_to_version: (a: number, b: number, c: number, d: number) => number;
    readonly wasmframework_set_namespace: (a: number, b: number, c: number) => number;
    readonly wasmframework_shortest_path: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly wasmframework_stats: (a: number) => number;
    readonly wasmframework_traverse: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly wasmframework_update_concept: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly wasmframework_update_concept_metadata: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly initialize_wasm: () => void;
    readonly __wasm_bindgen_func_elem_833: (a: number, b: number, c: number, d: number) => void;
    readonly __wasm_bindgen_func_elem_850: (a: number, b: number, c: number, d: number) => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number) => void;
    readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export5: (a: number, b: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
