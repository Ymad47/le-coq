# Le Coq — Vector Storage & Correlation Engine

## Vision

**Le Coq = Truth Engine.**

Not just aggregation. Not just dashboards.

**Cross-reference → Correlate → Verify → Surface truth.**

Multiple independent sources converging on same reality = signal. Single source = noise until proven otherwise.

---

## Architecture Overview

```
┌─────────────┐
│ Raw Sources │ (news, leaks, trackers, APIs, dark web)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Tokenizer   │ → Entity extraction, geohash, temporal
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Embedder    │ → 768-dim vectors (local model)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Vector DB   │ → pgvector / Qdrant (HNSW index)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Correlation │ → Cosine similarity + temporal weighting
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Truth Score │ → Confidence output + evidence chain
└─────────────┘
```

---

## Hybrid Storage Strategy

**Vectors + Raw** — best of both worlds.

| Store | What | Why |
|-------|------|-----|
| **Vectors** | Embeddings (768-dim) | Fast correlation, similarity search |
| **Raw** | Original text (compressed) | User display, audit, verification |

### Why Not Vectors Only?

- User wants to **see the source** ("Show me the article")
- Legal/compliance (**audit trail**)
- Debugging ("Why did this correlate?")
- Re-embedding later (better models come out)

### Why Not Raw Only?

- **Slow** (full-text search can't do semantic similarity)
- **Expensive** (10x more storage)
- **No correlation** (can't compute "closeness" of ideas)

---

## Storage Tiers (Scale Strategy)

### Phase 1: MVP (Now — €50/mo)

```
Single Hetzner server (AX52, ~€50/mo)
PostgreSQL + pgvector (built-in, no extra DB)
Raw data: local disk (4TB NVMe included)
Vectors: same DB (pgvector)
No tiering, no compression yet
```

**Capacity:** ~100K-500K sources easily.

**Cost:** €50/mo flat.

---

### Phase 2: Growth (€500/mo revenue → €150/mo infra)

```
3-Tier System:

┌─────────────────────────────────────────────────────┐
│ TIER 1: HOT (Vector Index)                          │
│ - Active embeddings (last 30 days)                  │
│ - High-correlation sources (high-credibility)       │
│ - In RAM or NVMe SSD                                │
│ - Qdrant/pgvector                                   │
│ Size: ~5-10 GB                                      │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│ TIER 2: WARM (Object Storage)                       │
│ - Raw data 30-365 days old                          │
│ - Compressed (zstd)                                 │
│ - Hetzner Storage Box / OVH Object Storage          │
│ - Retrieved on-demand for drill-down                │
│ Size: ~100-500 GB                                   │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│ TIER 3: COLD (Archive)                              │
│ - Raw data >1 year old                              │
│ - Heavy compression + dedup                         │
│ - Backup tapes or cheap object storage              │
│ - Rarely accessed (legal/compliance)                │
│ Size: ~1-5 TB (grows slowly)                        │
└─────────────────────────────────────────────────────┘
```

---

### Phase 3: Scale (€10K+/mo revenue → €500-1K/mo infra)

```
Multi-node Qdrant cluster
Tiered storage (hot/warm/cold)
CDN for API responses
Dedicated embedder workers
```

---

## Compression Strategy

| Data Type | Method | Ratio |
|-----------|--------|-------|
| Raw HTML/Text | zstd (level 3) | 5-10x |
| JSON | zstd + field pruning | 3-5x |
| Images | WebP + thumbnail only | 10-20x |
| Vectors | float32 → float16 | 2x |

**Result:** 1TB raw → ~100-200GB stored.

---

## Data Model

### Sources Table (PostgreSQL + pgvector)

```sql
CREATE TABLE sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    url TEXT NOT NULL,
    raw_content TEXT,                    -- Compressed (zstd)
    embedding vector(768),               -- pgvector
    entities JSONB,                      -- [FRA, MIL, SAHEL, ...]
    timestamp TIMESTAMPTZ NOT NULL,
    location GEOGRAPHY(POINT),           -- PostGIS
    confidence FLOAT DEFAULT 0.0,
    source_type TEXT,                    -- news, leak, tracker, etc.
    created_at TIMESTAMPTZ DEFAULT NOW(),
    tier TEXT DEFAULT 'hot'              -- hot, warm, cold
);

-- Index for vector similarity
CREATE INDEX ON sources USING hnsw (embedding vector_cosine_ops);

-- Index for temporal queries
CREATE INDEX ON sources (timestamp DESC);

-- Index for entity filtering
CREATE INDEX ON sources USING GIN (entities);
```

---

## Tokenization Scheme

```
Input: "French military deploys to Sahel region amid coup rumors"

Tokens:
  - Entities: [FRA, MIL, SAHEL, COUP]
  - Locations: [15.0, -5.0] (geohash)
  - Time: [2026-01-15T14:30:00]
  - Event Type: [MIL_DEPLOYMENT]
  - Confidence: [0.7] (source credibility)

Embedding: 768-dim vector (sentence-transformers or custom)
```

---

## Correlation Engine

### LLM Analogy

| LLM Architecture | Le Coq Equivalent |
|-----------------|-------------------|
| Input tokens | Raw data sources (news, leaks, OSINT, sensors) |
| Token embeddings | Source embeddings (credibility, bias, type, history) |
| Attention weights | Source weighting (independent confirmation > repetition) |
| Transformer layers | Correlation engine (cross-reference, temporal, spatial) |
| Softmax output | Confidence score (0-100% truth probability) |
| Next token | Verified truth claim |

---

### Truth Score Algorithm

```python
def compute_truth_score(sources: List[Source]) -> float:
    """
    Compute confidence score based on:
    - Number of independent sources
    - Source credibility weights
    - Temporal consistency
    - Entity agreement
    """
    if len(sources) == 0:
        return 0.0
    
    # Independence check (penalize copies)
    independence_score = compute_independence(sources)
    
    # Credibility weighting
    credibility_score = sum(s.credibility for s in sources) / len(sources)
    
    # Temporal alignment
    temporal_score = compute_temporal_consistency(sources)
    
    # Entity agreement
    entity_score = compute_entity_agreement(sources)
    
    # Weighted combination
    score = (
        0.3 * independence_score +
        0.3 * credibility_score +
        0.2 * temporal_score +
        0.2 * entity_score
    )
    
    return min(1.0, score)
```

---

### Example Correlation

```
Input: 50 sources mentioning "Country X mobilizing troops"
  → 10x News (varying credibility)
  → 15x Social media (geolocated?)
  → 5x Official statements
  → 3x Satellite imagery
  → 2x Dark web intel
  → 15x Network/flight/ship trackers

Processing:
  - Entity resolution (same event?)
  - Temporal alignment (same timeframe?)
  - Independence check (sources copying each other?)
  - Credibility weighting (satellite > rumor)

Output:
  "Country X troop mobilization: 87% confidence"
  + Timeline
  + Source breakdown
  + Contradictions flagged
```

---

## Query Patterns

### Similarity Search

```sql
-- Find sources similar to query
SELECT id, url, confidence, entities
FROM sources
ORDER BY embedding <-> '[...768-dim query vector...]'
LIMIT 100;
```

### Temporal Correlation

```sql
-- Find sources mentioning same entities within time window
SELECT s1.id, s2.id, s1.confidence, s2.confidence
FROM sources s1
JOIN sources s2 ON s1.entities && s2.entities  -- Array overlap
WHERE s1.timestamp BETWEEN '2026-01-01' AND '2026-01-07'
  AND s2.timestamp BETWEEN '2026-01-01' AND '2026-01-07'
  AND s1.id != s2.id
  AND s1.embedding <-> s2.embedding < 0.3;  -- Similar vectors
```

### Confidence Aggregation

```sql
-- Aggregate confidence by event/entity
SELECT 
    entity,
    COUNT(*) as source_count,
    AVG(confidence) as avg_confidence,
    MAX(confidence) as max_confidence
FROM sources,
     JSONB_ARRAY_ELEMENTS(entities) as entity
GROUP BY entity
HAVING COUNT(*) >= 3  -- At least 3 sources
ORDER BY avg_confidence DESC;
```

---

## Tech Stack (Sovereign EU)

| Component | Option |
|-----------|--------|
| Vector DB | pgvector (MVP) → Qdrant (scale) |
| Embedder | sentence-transformers (local, no API) |
| Tokenizer | spaCy + custom NER (French/English/Russian/Chinese) |
| Index | HNSW (approximate nearest neighbor, fast) |
| Backend | Rust (axum + sqlx) |
| Compression | zstd (fast, high ratio) |
| Object Storage | Hetzner Storage Box / OVH |

---

## Efficiency Gains

For 1M sources:

| Metric | Raw Search | Vector Search |
|--------|------------|---------------|
| Query Time | ~100ms-1s | ~1-10ms |
| Storage | 10-100 GB | 1-5 GB (768-dim float32) |
| Scalability | O(n) | O(log n) with HNSW |

**100x speedup. 20x compression.**

---

## Retention Policy (Auto-Cleanup)

```sql
-- Auto-age after 30 days
UPDATE sources 
SET tier = 'warm' 
WHERE created_at < NOW() - INTERVAL '30 days'
  AND confidence < 0.8;

-- Delete low-value data after 2 years
DELETE FROM sources 
WHERE tier = 'cold' 
  AND created_at < NOW() - INTERVAL '2 years'
  AND confidence < 0.5;
```

Not everything deserves infinite storage. Low-confidence, low-correlation data expires.

---

## Rule of Thumb

**Infra cost should be <10% of revenue.**

- €0 revenue → €50/mo max (your pocket)
- €500/mo revenue → €50/mo infra OK
- €10K/mo revenue → €1K/mo infra fine

**Don't build Phase 3 architecture at Phase 0.**

---

## Implementation Priority

### Now (MVP)

1. [ ] PostgreSQL + pgvector setup
2. [ ] Simple ingestion pipeline (fetch → embed → store)
3. [ ] Basic cosine similarity query
4. [ ] One Premium feature (dark web monitoring)

### Later (Scale)

1. [ ] Qdrant cluster (when pgvector too slow)
2. [ ] Object storage for raw (when disk full)
3. [ ] Tiered storage automation
4. [ ] Advanced correlation (temporal, entity resolution)

---

## Sovereign EU Providers

| Tier | Provider | Cost/mo |
|------|----------|---------|
| Hot (NVMe) | Hetzner AX52 (dedicated) | ~€50 |
| Warm (Object) | Hetzner Storage Box | ~€10/1TB |
| Cold (Archive) | OVH Cold Archive | ~€3/1TB |

**Total for 10TB:** ~€100-150/mo vs €3000+ on AWS.

---

**Fabriqué en France 🇫🇷**
**Souverain. Transparent. Puissant.**
