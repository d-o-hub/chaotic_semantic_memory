-- Generic SQLite/libSQL checks for lifecycle verification.
-- Adapt table names if target codebase differs.

-- Concept existence checks
SELECT COUNT(*) AS concept_count
FROM concepts
WHERE id IN ('test::lifecycle::alpha', 'test::lifecycle::beta');

-- Association existence check
SELECT COUNT(*) AS association_count
FROM associations
WHERE source_id = 'test::lifecycle::alpha'
  AND target_id = 'test::lifecycle::beta';

-- Archive marker check
SELECT COUNT(*) AS archive_marker_count
FROM concepts
WHERE id = 'archive::test::lifecycle::alpha';

-- Orphan association check
SELECT COUNT(*) AS orphan_associations
FROM associations a
LEFT JOIN concepts c1 ON c1.id = a.source_id
LEFT JOIN concepts c2 ON c2.id = a.target_id
WHERE c1.id IS NULL OR c2.id IS NULL;
