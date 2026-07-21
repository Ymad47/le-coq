# Le Coq

**Geopolitical Intelligence Platform** — Open-source alternative to Palantir.

> Fabriqué en France 🇫🇷  
> Souverain. Transparent. Puissant.

---

## Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| **Backend** | Rust (axum) | High-performance API, graph engine |
| **Plugins** | Python | OSINT data sources, NLP, ML |
| **CLI** | Zig | Static binary, offline mode |
| **Frontend** | Svelte | Graph/map/timeline visualization |
| **Database** | PostgreSQL + PostGIS | Entities, geospatial |
| **Graph** | Neo4j | Relationships, network analysis |
| **Search** | Meilisearch | Full-text search |

---

## Quick Start

### Prerequisites

- Rust 1.75+
- Python 3.11+
- Zig 0.11+
- Node.js 18+
- Docker + Docker Compose

### Development Setup

```bash
# Clone
git clone https://codeberg.org/ymad/le-coq.git
cd le-coq

# Start databases
docker compose -f docker/docker-compose.yml up -d postgres neo4j meilisearch

# Backend (Rust)
cd backend
cargo run

# Plugins (Python)
cd ../plugins
pip install -e .

# CLI (Zig)
cd ../cli
zig build run -- search "test query"

# Frontend (Svelte)
cd ../frontend
npm install
npm run dev
```

Open http://localhost:3000

---

## Project Structure

```
le-coq/
├── backend/          # Rust API server
│   ├── src/
│   │   ├── main.rs   # Entry point
│   │   ├── api.rs    # HTTP routes
│   │   ├── db/       # Database modules
│   │   ├── graph.rs  # Graph algorithms
│   │   └── models.rs # Data models
│   └── Cargo.toml
├── plugins/          # Python OSINT plugins
│   ├── __init__.py   # Plugin system
│   └── sources/      # Data source plugins
│       ├── opencorporates.py
│       ├── acled.py
│       └── gdelt.py
├── cli/              # Zig CLI tool
│   ├── src/
│   │   ├── main.zig  # CLI commands
│   │   └── lib.zig   # Shared types
│   └── build.zig
├── frontend/         # Svelte web UI
│   ├── src/
│   │   └── App.svelte
│   └── package.json
├── docker/           # Docker Compose
│   └── docker-compose.yml
├── docs/             # Documentation
│   ├── ARCHITECTURE.md
│   └── OSINT-SOURCES.md
├── Cargo.toml        # Rust workspace
└── pyproject.toml    # Python package
```

---

## Features (Planned)

### Phase 1 (MVP)
- [ ] Entity search (PostgreSQL + Meilisearch)
- [ ] Basic graph visualization
- [ ] 3 OSINT plugins (OpenCorporates, ACLED, GDELT)
- [ ] Export to JSON/PDF

### Phase 2
- [ ] Map view (Leaflet + PostGIS)
- [ ] Timeline view (temporal analysis)
- [ ] More OSINT sources (satellite, shipping, social)
- [ ] Network analysis (centrality, communities)

### Phase 3
- [ ] Offline mode (Zig CLI + local cache)
- [ ] Plugin marketplace
- [ ] Multi-user collaboration
- [ ] Desktop app (Tauri)

---

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/api/v1/entities` | List entities |
| POST | `/api/v1/entities` | Create entity |
| GET | `/api/v1/entities/:id` | Get entity by ID |
| GET | `/api/v1/relationships` | List relationships |
| POST | `/api/v1/relationships` | Create relationship |
| POST | `/api/v1/search` | Full-text search |
| POST | `/api/v1/ingest` | Trigger data ingestion |

---

## Configuration

Set environment variables:

```bash
# Backend
export DATABASE_URL=postgresql://user:pass@localhost:5432/lecoq
export NEO4J_URL=bolt://user:pass@localhost:7687
export MEILISEARCH_URL=http://localhost:7700
export MEILISEARCH_KEY=master_key

# Plugins
export OPENCORPORATES_API_KEY=your_key
export ACLED_API_KEY=your_key
```

---

## License

**AGPL-3.0** — Strong copyleft. Derivatives must stay open-source.

---

## Contributing

1. Fork on Codeberg
2. Create feature branch
3. Add tests
4. Submit PR

**Note:** Repository is currently private. Contact maintainer for access.

---

## Contact

- **Repo:** https://github.com/Ymad47/le-coq
- **Issues:** Codeberg issue tracker
- **Email:** ymad@ymad.fr

---

**Built with** 🦀 **Rust**, 🐍 **Python**, ⚡ **Zig**, 🎯 **Svelte**
