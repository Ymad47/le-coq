-- Migration 004: Five-timestamp model + confidence infrastructure
--
-- Per VISION.md §17: events should not have a single timestamp.
-- Adding temporal dimensions to entities enables historical replay ("what did we know at 10:00?").

-- Five timestamps on entities
ALTER TABLE entities
    ADD COLUMN IF NOT EXISTS occurred_at TIMESTAMP WITH TIME ZONE,
    ADD COLUMN IF NOT EXISTS published_at TIMESTAMP WITH TIME ZONE,
    ADD COLUMN IF NOT EXISTS observed_at TIMESTAMP WITH TIME ZONE,
    ADD COLUMN IF NOT EXISTS collected_at TIMESTAMP WITH TIME ZONE;

-- Backfill: created_at becomes the initial collected_at if not set
UPDATE entities SET collected_at = created_at WHERE collected_at IS NULL;

-- Indexes for time-based queries (replay slider, timeline view)
CREATE INDEX IF NOT EXISTS idx_entities_occurred_at ON entities(occurred_at) WHERE occurred_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_entities_published_at ON entities(published_at) WHERE published_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_entities_observed_at ON entities(observed_at) WHERE observed_at IS NOT NULL;

-- Track source reliability characteristics per observed item (Phase 3 prep, VISION §38)
ALTER TABLE entities
    ADD COLUMN IF NOT EXISTS source_attributes JSONB;

COMMENT ON COLUMN entities.source_attributes IS
    'Source reliability signals: {independence: "primary"|"secondary", publication_delay: "fast"|"slow", transparency: "high"|"low", ...}';
