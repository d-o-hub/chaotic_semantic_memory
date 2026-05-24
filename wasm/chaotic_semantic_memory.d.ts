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
  persist_ops_total: number;
  avg_persist_latency_ms: number;
}

export interface FrameworkStats {
  concept_count: number;
  db_size_bytes: number | null;
}

export class WasmFramework {
  static new(): Promise<WasmFramework>;
  inject_concept(id: string, vector: Uint8Array): Promise<void>;
  probe(vector: Uint8Array, top_k: number): Promise<ProbeResult[]>;
  associate(from: string, to: string, strength: number): Promise<void>;
  delete_concept(id: string): Promise<void>;
  get_associations(id: string): Promise<AssociationResult[]>;
  metrics_snapshot(): Promise<FrameworkMetrics>;
  stats(): Promise<FrameworkStats>;
  processSequence(sequence: Float32Array[]): Promise<Uint8Array>;
  exportToBytes(): Promise<Uint8Array>;
  importFromBytes(data: Uint8Array, merge: boolean): Promise<number>;
}

export function random_hypervector(): Uint8Array;
export function cosine_similarity(a: Uint8Array, b: Uint8Array): number;
