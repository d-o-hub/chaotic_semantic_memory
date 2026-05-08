-- Migration v9: Vector format support for binary hypervectors
ALTER TABLE csm_concepts ADD COLUMN vector_format TEXT NOT NULL DEFAULT 'f32';
ALTER TABLE csm_versions ADD COLUMN vector_format TEXT NOT NULL DEFAULT 'f32';
