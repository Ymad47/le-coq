-- Migration 003: Prevent duplicate relationships + confidence defaults
--
-- Bug prevention:
--   1. Same (from, to, type) can currently be inserted many times → graph noise
--   2. Confidence default 1.0 means "fully certain" — should be NULL (unknown)

-- 1) Remove existing duplicates (keep highest-confidence row per triple)
DELETE FROM relationships a
USING relationships b
WHERE a.id < b.id
  AND a.from_entity = b.from_entity
  AND a.to_entity = b.to_entity
  AND a.relationship_type = b.relationship_type;

-- 2) Add unique constraint on the logical identity of a relationship
CREATE UNIQUE INDEX IF NOT EXISTS uniq_relationship_triple
    ON relationships (from_entity, to_entity, relationship_type);

-- 3) Confidence: 1.0 was "certain by default" — wrong semantics
--    NULL = unknown; CHECK still enforces range when set
ALTER TABLE relationships ALTER COLUMN confidence DROP DEFAULT;
