# Le Coq

**Sovereign OSINT / geopolitical intelligence platform.**

> Fabriqué en France 🇫🇷
> Souverain. Transparent. Puissant.

Le Coq collects open-source data (GDELT, ACLED, Wikidata, Wikipedia, company
registries, trade statistics…) through Python plugins, stores entities and
relationships in PostgreSQL, and explores them through a list, graph, map and
timeline UI.

**Status:** early development, intended for local / homelab use. Not hardened
for public exposure — see [Security](#security).

---

## Stack

| Component | Technology | Notes |
|-----------|------------|-------|
| **Backend** | Rust, Axum 0.7, SQLx 0.7 | REST API, embedded SQL migrations |
| **Plugins** | Python 3.11, embedded via PyO3 | OSINT data sources |
| **Database** | PostgreSQL 16 (pgvector image) | Entities, relationships, full-text search |
| **Frontend** | Svelte 4, Vite 5, TypeScript | List, graph (Cytoscape), map (jsvectormap), timeline |
| **CLI** | Zig | Experimental |

Not implemented yet (planned): Neo4j, Meilisearch, RAG / vector search, graph
analytics (`backend/src/graph.rs` is a stub).

---

## Quick start

### Prerequisites

- Rust (stable)
- Python **3.11** — required: PyO3 is built against `abi3-py311`
- Node.js 18+
- Docker + Docker Compose
- Zig (CLI only)

### 1. Configure secrets

```bash
cp .env.example .env
export LE_COQ_API_KEY=$(openssl rand -hex 32)
export POSTGRES_PASSWORD=$(openssl rand -hex 16)
```

### 2a. Run everything with Docker

```bash
docker compose -f docker/docker-compose.yml up -d --build
curl http://127.0.0.1:8000/health
```

Postgres and the backend are published on **loopback only**
(`127.0.0.1:5432`, `127.0.0.1:8000`).

### 2b. Or run the backend locally

```bash
# Database only
docker compose -f docker/docker-compose.yml up -d postgres

# Python plugins
python3.11 -m venv plugins/.venv
plugins/.venv/bin/pip install -r plugins/requirements.txt

# Backend (migrations run automatically at startup)
export DATABASE_URL=postgresql://postgres:$POSTGRES_PASSWORD@127.0.0.1:5432/le_coq
export PYO3_PYTHON=$(command -v python3.11)
cargo run -p le-coq-backend
```

`.cargo/config.toml` sets a machine-specific `PYO3_PYTHON`; an exported value
overrides it.

### 3. Frontend

```bash
cd frontend
npm install
LE_COQ_API_KEY=$LE_COQ_API_KEY npm run dev
```

Open http://127.0.0.1:3001. The Vite dev proxy forwards `/api` and `/health` to
the backend and injects the `X-API-Key` header server-side — the key is never
shipped in the JS bundle.

---

## Configuration

Full list with comments: [`.env.example`](.env.example).

| Variable | Default | Purpose |
|----------|---------|---------|
| `DATABASE_URL` | — (required) | PostgreSQL connection string |
| `LE_COQ_API_KEY` | — (required) | API key, checked against the `X-API-Key` header |
| `BIND_ADDRESS` | `127.0.0.1:8000` | Backend listen address |
| `DISABLE_AUTH` | unset | Dev only; refused unless `BIND_ADDRESS` is loopback |
| `RUST_LOG` | `le_coq=info,tower_http=debug` | Log level |
| `LE_COQ_ROOT` | repo path | Project root |
| `LE_COQ_PLUGINS_DIR` | `$LE_COQ_ROOT/plugins` | Plugin directory |
| `LE_COQ_PLUGINS_VENV` | `$LE_COQ_PLUGINS_DIR/.venv/lib/python3.11/site-packages` | Plugin dependencies |
| `POLL_INTERVAL_SECS` | `300` | Poller interval |
| `ACLED_EMAIL`, `ACLED_PASSWORD` (or `ACLED_ACCESS_TOKEN` / `ACLED_REFRESH_TOKEN`) | — | ACLED plugin credentials |

Frontend dev server: `LE_COQ_BACKEND_URL` (default `http://127.0.0.1:8000`),
`LE_COQ_UI_HOST` (default `127.0.0.1`).

---

## API

All `/api/v1/*` routes require the `X-API-Key` header. `/health` is public.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET, POST | `/api/v1/entities` | List / create entities |
| GET | `/api/v1/entities/:id` | Get an entity |
| GET | `/api/v1/entities/:id/summary` | Entity summary |
| GET, POST | `/api/v1/relationships` | List / create relationships |
| GET | `/api/v1/search` | Smart search (database + optional online sources) |
| POST | `/api/v1/search` | Full-text search |
| GET | `/api/v1/topic` | Topic search |
| POST | `/api/v1/graph/neighborhood` | Graph neighborhood of an entity |
| GET | `/api/v1/timeline` | Timeline events *(in progress, `feat/timeline`)* |
| POST | `/api/v1/ingest` | Run ingestion from a plugin |
| GET | `/api/v1/sources` | List data sources |
| GET | `/api/v1/plugins/health` | Plugin status |
| POST | `/api/v1/plugins/reload` | Reload plugins |

```bash
curl -H "X-API-Key: $LE_COQ_API_KEY" http://127.0.0.1:8000/api/v1/entities
```

---

## Plugins

Located in `plugins/sources/`:

| Plugin | Source |
|--------|--------|
| `gdelt` | GDELT 2.0 events |
| `acled` | ACLED conflict data (credentials required) |
| `wikidata` | Wikidata |
| `wikipedia` | Wikipedia |
| `opencorporates` | OpenCorporates company registry |
| `gleif` | GLEIF legal entity identifiers |
| `eurostat` | Eurostat statistics |
| `uncomtrade` | UN Comtrade trade data |

---

## Project structure

```
le-coq/
├── backend/            # Rust API (crate le-coq-backend)
│   ├── migrations/     # SQLx migrations, applied at startup
│   └── src/
│       ├── main.rs, lib.rs, config.rs
│       ├── api.rs          # Routes and handlers
│       ├── auth.rs         # X-API-Key middleware
│       ├── db/             # entities, relationships, search, summary, timeline
│       ├── ingestion.rs, dedup.rs
│       ├── plugin_bridge.rs # PyO3 bridge to Python plugins
│       ├── poller.rs       # Background poller (not started yet)
│       └── graph.rs        # Graph algorithms (stub)
├── plugins/            # Python plugins + tests
├── frontend/           # Svelte UI (+ nginx Dockerfile)
├── cli/                # Zig CLI
├── docker/             # docker-compose.yml
├── docs/               # Architecture, security, deployment, ethics
├── scripts/            # start-backend.sh, systemd unit, backup.sh
└── .github/workflows/  # CI
```

---

## Development

Same checks as CI:

```bash
# Rust
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked

# Python plugins
pip install -r plugins/requirements-dev.txt
PYTHONPATH=. pytest plugins/tests/

# Frontend
cd frontend
npx svelte-check --tsconfig ./tsconfig.json --fail-on-warnings
npm run build
```

Note: some plugin tests hit live APIs (e.g. Wikipedia) and need network access.

---

## Security

- The API key is compared in constant time; the backend refuses to start
  without one unless `DISABLE_AUTH` is set on a loopback bind.
- Ports are bound to `127.0.0.1` by default. To expose Le Coq, put it behind a
  reverse proxy with TLS, or a VPN (WireGuard).
- Anyone who can reach the Vite dev server gets authenticated API access —
  keep it on loopback.

More: [`docs/SECURITY.md`](docs/SECURITY.md),
[`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md),
[`docs/PROD_CHECKLIST.md`](docs/PROD_CHECKLIST.md).

---

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [OSINT sources](docs/OSINT-SOURCES.md)
- [Network modes](docs/NETWORK_MODES.md)
- [Ethics charter](docs/ETHICS_CHARTER.md)
- [Vision](docs/VISION.md)

---

## License

**AGPL-3.0** — derivatives must stay open source.

## Contact

- **Repo:** https://github.com/Ymad47/le-coq (private — contact maintainer for access)
- **Email:** ymad@ymad.fr
