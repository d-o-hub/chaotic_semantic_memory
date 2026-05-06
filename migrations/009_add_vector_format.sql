-- Migration 009: Add vector_format column to csm_concepts
-- Supports multiple hypervector formats (f32 default, binary opt-in)

ALTER TABLE csm_concepts ADD COLUMN vector_format TEXT NOT NULL DEFAULT 'f32';
