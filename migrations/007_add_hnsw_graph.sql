-- Migration 005: Add HNSW graph table for index persistence
CREATE TABLE IF NOT EXISTS csm_hnsw_graph (
    id TEXT PRIMARY KEY,
    data BLOB NOT NULL,
    modified_at INTEGER NOT NULL
);
