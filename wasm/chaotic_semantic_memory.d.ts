/* tslint:disable */
/* eslint-disable */

export interface ProbeResult { id: string; score: number; }
export interface AssociationResult { to: string; strength: number; }



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
    associate_many(associations: Array<any>): Promise<void>;
    /**
     * Breadth-first traversal from a starting concept.
     *
     * Returns an Array of `{id: string, depth: number}` objects.
     * Uses default `TraversalConfig`.
     */
    bfs(start: string): Promise<Array<any>>;
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
     * Export a namespace to bytes for in-browser storage.
     */
    exportNamespaceToBytes(ns: string): Promise<Uint8Array>;
    /**
     * Export all concepts and associations to bytes for in-browser storage.
     */
    exportToBytes(): Promise<Uint8Array>;
    /**
     * Get the current namespace
     */
    getNamespace(): Promise<string>;
    /**
     * Load a specific concept version.
     */
    getVersion(id: string, version: number): Promise<any>;
    /**
     * Get associations for a concept
     */
    get_associations(id: string): Promise<Array<any>>;
    /**
     * Get a concept by ID
     */
    get_concept(id: string): Promise<any>;
    /**
     * Import state from bytes previously produced by `exportToBytes`.
     */
    importFromBytes(data: Uint8Array, merge: boolean): Promise<number>;
    /**
     * Inject a concept
     */
    inject_concept(id: string, vector: Uint8Array): Promise<void>;
    /**
     * Inject multiple concepts in batch
     */
    inject_concepts(ids: Array<any>, vectors: Array<any>): Promise<void>;
    /**
     * Inject a concept from text
     */
    inject_text(id: string, text: string): Promise<void>;
    /**
     * List all historical versions of a concept.
     */
    listVersions(id: string): Promise<Array<any>>;
    /**
     * Get framework metrics snapshot
     */
    metrics_snapshot(): Promise<any>;
    /**
     * Get direct neighbors of a concept with edge strengths.
     *
     * Returns an Array of `{to: string, strength: number}` objects.
     */
    neighbors(id: string, min_strength: number): Promise<Array<any>>;
    /**
     * Create a new framework instance (no persistence in WASM)
     */
    static new(): Promise<WasmFramework>;
    /**
     * Register a callback for memory events.
     */
    on_event(callback: Function): void;
    /**
     * Query for similar concepts
     */
    probe(vector: Uint8Array, top_k: number): Promise<Array<any>>;
    /**
     * Probe for similar concepts with multiple queries in batch
     */
    probe_batch(vectors: Array<any>, top_k: number): Promise<Array<any>>;
    /**
     * Probe for similar concepts with metadata filtering.
     */
    probe_filtered(vector: Uint8Array, top_k: number, filter_json: string): Promise<Array<any>>;
    /**
     * Probe for similar concepts using text
     */
    probe_text(query: string, top_k: number): Promise<Array<any>>;
    /**
     * GraphRAG retrieval: similarity + graph traversal hybrid using text query.
     */
    probe_text_with_graph(text: string, anchor_top_k: number, max_hops: number, min_assoc_strength: number, similarity_weight: number, graph_weight: number, final_top_k: number): Promise<Array<any>>;
    /**
     * GraphRAG retrieval: similarity + graph traversal hybrid using vector query.
     */
    probe_with_graph(vector: Uint8Array, anchor_top_k: number, max_hops: number, min_assoc_strength: number, similarity_weight: number, graph_weight: number, final_top_k: number): Promise<Array<any>>;
    /**
     * Process a temporal sequence and return the resulting hypervector bytes.
     */
    processSequence(sequence: Array<any>): Promise<Uint8Array>;
    /**
     * Roll back a concept to a historical version.
     */
    rollbackToVersion(id: string, version: number): Promise<any>;
    /**
     * Set the current namespace
     */
    setNamespace(ns: string): Promise<void>;
    /**
     * Find the minimum-cost path between two concepts (weighted Dijkstra).
     *
     * Returns an Array of concept ID strings, or an empty Array if no path exists.
     * Uses default `TraversalConfig`.
     */
    shortest_path(from: string, to: string): Promise<Array<any>>;
    /**
     * Get framework stats
     */
    stats(): Promise<any>;
    /**
     * Breadth-first traversal from a starting concept with custom config.
     */
    traverse(start: string, max_depth: number, min_strength: number): Promise<Array<any>>;
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
     * Note: In WASM, persistence is in-memory only. Use `exportToBytes` to
     * snapshot state to IndexedDB or other storage.
     */
    update_concept_metadata(id: string, metadata_json: string): Promise<void>;
}

export function cosine_similarity(a: Uint8Array, b: Uint8Array): number;

export function encode_text(text: string): Uint8Array;

export function initialize_wasm(): void;

export function random_hypervector(): Uint8Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly wasmframework_bfs: (a: number, b: number, c: number) => any;
    readonly wasmframework_probe_text_with_graph: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => any;
    readonly wasmframework_probe_with_graph: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => any;
    readonly wasmframework_shortest_path: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmframework_traverse: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly __wbg_wasmframework_free: (a: number, b: number) => void;
    readonly cosine_similarity: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly encode_text: (a: number, b: number) => [number, number];
    readonly initialize_wasm: () => void;
    readonly random_hypervector: () => [number, number];
    readonly wasmframework_associate: (a: number, b: number, c: number, d: number, e: number, f: number) => any;
    readonly wasmframework_associate_many: (a: number, b: any) => any;
    readonly wasmframework_delete_concept: (a: number, b: number, c: number) => any;
    readonly wasmframework_disassociate: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmframework_exportToBytes: (a: number) => any;
    readonly wasmframework_getNamespace: (a: number) => any;
    readonly wasmframework_get_associations: (a: number, b: number, c: number) => any;
    readonly wasmframework_get_concept: (a: number, b: number, c: number) => any;
    readonly wasmframework_importFromBytes: (a: number, b: any, c: number) => any;
    readonly wasmframework_inject_concept: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmframework_inject_concepts: (a: number, b: any, c: any) => any;
    readonly wasmframework_new: () => any;
    readonly wasmframework_probe: (a: number, b: number, c: number, d: number) => any;
    readonly wasmframework_probe_batch: (a: number, b: any, c: number) => any;
    readonly wasmframework_setNamespace: (a: number, b: number, c: number) => any;
    readonly wasmframework_update_concept: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmframework_clear_associations: (a: number, b: number, c: number) => any;
    readonly wasmframework_concept_count: (a: number) => any;
    readonly wasmframework_exportNamespaceToBytes: (a: number, b: number, c: number) => any;
    readonly wasmframework_getVersion: (a: number, b: number, c: number, d: number) => any;
    readonly wasmframework_inject_text: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmframework_listVersions: (a: number, b: number, c: number) => any;
    readonly wasmframework_metrics_snapshot: (a: number) => any;
    readonly wasmframework_neighbors: (a: number, b: number, c: number, d: number) => any;
    readonly wasmframework_on_event: (a: number, b: any) => void;
    readonly wasmframework_probe_filtered: (a: number, b: number, c: number, d: number, e: number, f: number) => any;
    readonly wasmframework_probe_text: (a: number, b: number, c: number, d: number) => any;
    readonly wasmframework_processSequence: (a: number, b: any) => any;
    readonly wasmframework_rollbackToVersion: (a: number, b: number, c: number, d: number) => any;
    readonly wasmframework_stats: (a: number) => any;
    readonly wasmframework_update_concept_metadata: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasm_bindgen__convert__closures_____invoke__hebf7812b93ae332a: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h0493d3f6782368f7: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h68634416af742c52: (a: number, b: number) => number;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
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
