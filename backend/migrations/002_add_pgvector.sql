-- Migration 002: Enable pgvector extension

-- Enable pgvector extension for vector similarity search
CREATE EXTENSION IF NOT EXISTS vector;

-- Add embedding column to entities table for semantic search
ALTER TABLE entities ADD COLUMN IF NOT EXISTS embedding vector(768);

-- Create index for vector similarity search (cosine distance)
CREATE INDEX IF NOT EXISTS idx_entities_embedding ON entities USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

-- Add metadata for fact-checking scores
ALTER TABLE entities ADD COLUMN IF NOT EXISTS fact_check_score REAL DEFAULT 0.0 CHECK (fact_check_score >= 0.0 AND fact_check_score <= 1.0);
ALTER TABLE entities ADD COLUMN IF NOT EXISTS fact_check_status VARCHAR(50) DEFAULT 'pending';

-- Add indexes for fact-checking queries
CREATE INDEX IF NOT EXISTS idx_entities_fact_check_status ON entities(fact_check_status);
CREATE INDEX IF NOT EXISTS idx_entities_fact_check_score ON entities(fact_check_score);
