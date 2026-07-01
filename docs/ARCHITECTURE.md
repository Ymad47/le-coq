# Le Coq — Architecture Document

## Vision

**Open-source geopolitical intelligence platform** — alternative to Palantir, built for journalists, researchers, NGOs, and sovereign states.

**Core principle:** OSINT-first, transparent, accessible, privacy-preserving.

---

## Palantir Comparison

| Palantir Does | Palantir CAN'T | Le Coq Edge |
|--------------|----------------|-------------|
| Enterprise/gov only | Individuals, journalists, NGOs can't access | **Open-source, accessible** |
| Proprietary, black box | No transparency on algorithms | **Explainable AI, auditable** |
| Classified + licensed data | Can't ingest all public OSINT automatically | **OSINT-first, real-time** |
| Centralized architecture | Hard to deploy in contested environments | **Decentralized, offline-capable** |
| Expensive ($100M+ contracts) | Small orgs, researchers priced out | **Free/cheap, modular** |
| Surveillance reputation | Privacy concerns limit adoption | **Privacy-preserving by design** |
| Structured data focus | Struggles with unstructured (social, news, images) | **Multi-modal ingestion** |
| Closed ecosystem | Can't community-contribute sources | **Plugin architecture** |

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         LE COQ                              │
├─────────────────────────────────────────────────────────────┤
│  DATA LAYER (Plugins)                                       │
│  ├── Satellite (Sentinel, Landsat, NASA FIRMS)              │
│  ├── Shipping (AIS, MarineTraffic, VesselFinder)            │
│  ├── Corporate (OpenCorporates, ICIJ, company registries)   │
│  ├── Conflict (ACLED, GDELT, news APIs)                     │
│  ├── Finance (OpenSanctions, World Bank, IMF, UN Comtrade)  │
│  ├── Social (X/Twitter, Telegram, Facebook scrapers)        │
│  ├── Energy (oil/gas fields, pipelines, power grids)        │
│  ├── Trade (customs data, port manifests, shipping routes)  │
│  └── Custom sources (user-contributed plugins)              │
├─────────────────────────────────────────────────────────────┤
│  PROCESSING LAYER                                           │
│  ├── Entity Extraction (NER: people, orgs, places, events)  │
│  ├── Relationship Inference (co-occurrence, flows, links)   │
│  ├── Temporal Analysis (trend detection, change points)     │
│  ├── Geospatial Clustering (hotspot detection, patterns)    │
│  ├── Network Analysis (centrality, communities, brokers)    │
│  ├── Sentiment Analysis (media tone, propaganda detection)  │
│  └── Multi-modal Fusion (text + image + satellite + video)  │
├─────────────────────────────────────────────────────────────┤
│  STORAGE LAYER                                              │
│  ├── Graph DB (Neo4j / ArangoDB) — relationships, networks  │
│  ├── Document DB (MongoDB / PostgreSQL JSONB) — raw sources │
│  ├── Search Index (Meilisearch / Elasticsearch)             │
│  ├── Geospatial (PostGIS) — maps, regions, coordinates      │
│  └── Time-series (InfluxDB / TimescaleDB) — trends          │
├─────────────────────────────────────────────────────────────┤
│  INTERFACE LAYER                                            │
│  ├── Graph Visualization (D3.js, Cytoscape.js, KeyLines)    │
│  ├── Timeline View (temporal evolution, event sequences)    │
│  ├── Map View (Leaflet, MapLibre, Cesium for 3D)            │
│  ├── Query Language (natural + structured, SQL-like)        │
│  ├── Report Export (PDF, DOCX, API, dashboards)             │
│  └── Alert System (threshold triggers, anomaly detection)   │
└─────────────────────────────────────────────────────────────┘
```

---

## Killer Features

### 1. Automated OSINT Pipelines
- Continuous ingestion from 50+ public sources
- Configurable refresh rates (real-time to weekly)
- Automatic deduplication, source scoring

### 2. Explainable Connections
- Every link shows: source + timestamp + confidence score
- "Why are these connected?" — click to see evidence
- Chain of reasoning visible, auditable

### 3. Plugin Marketplace
- Users contribute data source plugins
- Rating system, community maintenance
- One-click install new sources

### 4. Offline/Field Mode
- Full functionality without internet
- Local SQLite + embedded graph engine
- Sync when connected, conflict resolution

### 5. Privacy Modes
- **Local-only:** nothing leaves your machine
- **Tor mode:** anonymized queries
- **Air-gapped:** manual data import, no network

### 6. Temporal Reasoning
- "Show me how this network evolved over 6 months"
- Animation, change detection, anomaly flags
- Predictive modeling (optional ML)

### 7. Multi-modal Analysis
- Text + images + satellite + video in same graph
- Object detection in satellite imagery
- OCR for documents, face blur for privacy

### 8. Adversarial Mode
- "How would someone hide this relationship?"
- Counter-analysis: find gaps, blind spots
- Red-team your own intelligence

---

## Tech Stack

### Backend
```yaml
Primary: Python 3.11+ (FastAPI)
  - Rapid development, rich OSINT libraries
  - spaCy, transformers for NLP
  - Celery + Redis for async pipelines

Alternative: Rust (for performance-critical modules)
  - Data ingestion, graph algorithms
  - FFI bindings for Python
```

### Database
```yaml
Graph: Neo4j (community edition) or ArangoDB
  - Relationship storage, path queries
  - Graph algorithms (PageRank, community detection)

Document: PostgreSQL + JSONB
  - Raw source storage, metadata
  - ACID compliance, easy backup

Search: Meilisearch
  - Full-text, typo-tolerant
  - Lighter than Elasticsearch, easier ops

Geospatial: PostGIS (PostgreSQL extension)
  - Spatial queries, geocoding
  - Map tile generation

Time-series: TimescaleDB (PostgreSQL extension)
  - Trend analysis, anomaly detection
  - No separate infrastructure
```

### Frontend
```yaml
Framework: Svelte or React
  - Svelte: lighter, faster
  - React: larger ecosystem

Graph Viz: Cytoscape.js or D3.js
  - Interactive networks, custom layouts
  - Zoom, pan, node/edge styling

Maps: MapLibre GL (open-source Mapbox)
  - Vector tiles, custom styles
  - 3D terrain (optional)

Timeline: Vis.js or custom D3
  - Event sequences, zoomable time axis
```

### NLP / ML
```yaml
Entity Extraction: spaCy + custom models
  - People, orgs, locations, dates
  - Multi-language support

Embeddings: Sentence Transformers
  - Semantic search, similarity
  - All-MiniLM-L6-v2 (fast, small)

Summarization: Local LLM (Ollama)
  - Optional, for report generation
  - Qwen2.5-Coder or similar

Classification: scikit-learn / PyTorch
  - Source type detection
  - Sentiment, propaganda flags
```

### Deployment
```yaml
Dev: Docker Compose
  - Single command: `docker compose up`
  - All services on one machine

Prod: Kubernetes (optional)
  - Horizontal scaling
  - Multi-region deployment

Desktop: Tauri (Rust + web frontend)
  - Native app, small binary
  - Offline-first by design

Server: Systemd + Docker
  - Simple VPS deployment
  - Auto-restart, logging
```

---

## Data Model

### Core Entities
```
Entity
├── id: UUID
├── name: String
├── type: Enum [Person, Organization, Location, Event, Asset]
├── aliases: String[]
├── properties: JSON
├── sources: SourceRef[]
├── created_at: Timestamp
└── updated_at: Timestamp

Relationship
├── id: UUID
├── from_entity: UUID
├── to_entity: UUID
├── type: Enum [owns, works_for, located_in, participated_in, trades_with, ...]
├── properties: JSON
├── confidence: Float (0-1)
├── sources: SourceRef[]
├── start_date: Date (optional)
├── end_date: Date (optional)
└── is_temporal: Boolean

Source
├── id: UUID
├── url: String (or file path)
├── type: Enum [news, report, satellite, social, corporate, ...]
├── fetched_at: Timestamp
├── content_hash: String
├── metadata: JSON
└── raw_content: Binary/Text

Event
├── id: UUID
├── title: String
├── date: Timestamp
├── location: UUID (→ Entity)
├── participants: UUID[] (→ Entity)
├── type: Enum [conflict, election, protest, meeting, disaster, ...]
├── description: Text
├── sources: SourceRef[]
└── severity: Enum [low, medium, high, critical]
```

---

## MVP Scope (Phase 1)

**Goal:** Single-machine, local-first intelligence tool

### Ingest (3-5 sources)
- [ ] News API (GDELT or NewsAPI.org)
- [ ] OpenCorporates (company registry)
- [ ] OpenSanctions (PEPs, sanctions lists)
- [ ] ACLED (conflict events)
- [ ] Manual upload (CSV, JSON)

### Process
- [ ] Entity extraction (spaCy)
- [ ] Basic relationship inference (co-occurrence in same document)
- [ ] Geocoding (location names → coordinates)

### Store
- [ ] PostgreSQL + JSONB (entities, relationships)
- [ ] Meilisearch (full-text search)

### Visualize
- [ ] Graph view (Cytoscape.js)
- [ ] Search bar
- [ ] Entity detail panel (sources, properties)

### Export
- [ ] PDF report generation
- [ ] JSON export

**Timeline:** 4-6 weeks for usable MVP

---

## Phase 2 (Months 2-4)

### More Sources
- [ ] Satellite imagery (Sentinel Hub API)
- [ ] Shipping data (MarineTraffic AIS)
- [ ] Social media (X/Twitter API v2, Telegram scrapers)
- [ ] Trade data (UN Comtrade)

### Advanced Processing
- [ ] Temporal analysis (trend detection)
- [ ] Network analysis (centrality, communities)
- [ ] Image analysis (OCR, object detection)

### Better UI
- [ ] Map view (MapLibre)
- [ ] Timeline view
- [ ] Saved searches, alerts

---

## Phase 3 (Months 5-8)

### Collaboration
- [ ] Multi-user sync (PostgreSQL replication)
- [ ] Shared graphs, comments
- [ ] Access control (RBAC)

### Plugin System
- [ ] Plugin API (Python interface)
- [ ] Plugin marketplace (web UI)
- [ ] Rating, review system

### Desktop App
- [ ] Tauri wrapper
- [ ] Offline mode (SQLite + embedded search)
- [ ] Auto-updates

---

## Security Considerations

### Data Protection
- Encrypt database at rest (PostgreSQL TDE or filesystem encryption)
- TLS for all API communication
- No telemetry, no phone-home

### User Safety
- Tor integration for anonymous queries
- Plausible deniability modes (deniable encryption)
- Panic button (quick data wipe)

### Source Security
- Rate limiting, respectful scraping
- API key management (env vars, secrets manager)
- Source anonymization (remove tracking params)

---

## Success Metrics

| Metric | Target (Year 1) |
|--------|-----------------|
| Active users | 500+ (researchers, journalists, NGOs) |
| Data sources | 30+ plugins |
| Performance | <1s query response (100k entities) |
| Accuracy | 85%+ entity extraction F1 |
| Community | 10+ external plugin contributors |

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| API rate limits / blocks | Caching, multiple API keys, fallback sources |
| Data quality issues | Source scoring, user flagging, manual curation |
| Performance at scale | Indexing, query optimization, sharding strategy |
| Legal liability (scraping) | Respect robots.txt, TOS compliance, legal review |
| Misuse (surveillance, doxxing) | Code of conduct, feature restrictions, community moderation |

---

## Getting Started (Developers)

```bash
# Clone
git clone https://codeberg.org/ymad/le-coq.git
cd le-coq

# Backend setup
cd backend
python -m venv venv
source venv/bin/activate
pip install -r requirements.txt

# Database (Docker)
docker compose up -d postgres meilisearch

# Run backend
uvicorn app.main:app --reload

# Frontend
cd ../frontend
npm install
npm run dev

# Tests
pytest tests/
```

---

## License

**AGPL-3.0** — strong copyleft, ensures derivatives stay open-source.

---

## Contact

- **Repo:** https://codeberg.org/ymad/le-coq
- **Issues:** Codeberg issue tracker
- **Discussion:** Matrix/Telegram community (TBD)

---

**Fabriqué en France 🇫🇷**
**Souverain. Transparent. Puissant.**
