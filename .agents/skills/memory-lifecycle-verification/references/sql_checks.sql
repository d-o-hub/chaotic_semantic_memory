-- SQLite/libSQL checks for lifecycle verification.
-- NOTE: Table names vary per codebase. This crate uses csm_ prefix.
-- Override TABLE_PREFIX env var or edit if adapting to another codebase.

-- Concept existence checks
SELECT COUNT(*) AS concept_count
FROM csm_concepts
WHERE id IN ('test::lifecycle::alpha', 'test::lifecycle::beta');

-- Association existence check
SELECT COUNT(*) AS association_count
FROM csm_associations
WHERE from_id = 'test::lifecycle::alpha'
  AND to_id = 'test::lifecycle::beta';

-- Archive marker check
SELECT COUNT(*) AS archive_marker_count
FROM csm_concepts
WHERE id = 'archive::test::lifecycle::alpha';

-- Orphan association check
SELECT COUNT(*) AS orphan_associations
FROM csm_associations a
LEFT JOIN csm_concepts c1 ON c1.id = a.from_id
LEFT JOIN csm_concepts c2 ON c2.id = a.to_id
WHERE c1.id IS NULL OR c2.id IS NULL;
