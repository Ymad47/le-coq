# Le Coq - Phase 1 TODO List

## ✅ Completed (Phase 1 Setup)

- [x] Remove Neo4j dependency (simplify to PostgreSQL only)
- [x] Remove Meilisearch dependency (use PostgreSQL full-text search)
- [x] Update docker-compose.yml (pgvector image, remove Neo4j/Meilisearch)
- [x] Update Cargo.toml (remove neo4rs, meilisearch-sdk)
- [x] Update main.rs (remove Neo4j/Meilisearch from AppState)
- [x] Update db.rs (remove neo4j module)
- [x] Add API key authentication middleware
- [x] Add pgvector migration (002_add_pgvector.sql)
- [x] Create backup script (scripts/backup.sh)

---

## 🔴 High Priority (This Week)

### 1. Build & Test Backend
- [x] Run `cargo build` in backend directory
- [x] Fix any compilation errors (chrono Datelike import dans poller.rs tests)
- [x] Run `cargo test` — 33/33 passent (PYO3_PYTHON=python3.11 requis, fixé dans .cargo/config.toml)
- [x] Verify backend image builds and runs (plugins load, non-root user) — 2026-11-08

### 2. Database Setup
- [x] PostgreSQL + pgvector running (docker-postgres-1, healthy)
- [x] pgvector extension active, migrations appliquées (entities, relationships, _sqlx_migrations)
- [x] Backend health check OK : curl http://localhost:8000/health

### 3. API Key Configuration
- [ ] Generate secure API key: `openssl rand -hex 32`
- [x] Key header is fixed to `X-API-Key` (`API_KEY_HEADER` is no longer read); set `LE_COQ_API_KEY` instead
- [ ] Document in README.md how to set API keys
- [ ] Test auth: `curl -H "X-API-Key: *** http://localhost:8000/api/v1/entities`

### 4. TLS/HTTPS Setup (reporté — dev local only pour l'instant)
- [ ] Option A: Add nginx reverse proxy (recommended for prod)
  - [ ] Create nginx Dockerfile
  - [ ] Configure SSL certificates (Let's Encrypt)
  - [ ] Update docker-compose.yml
- [ ] Option B: Use axum-tls (simpler, dev only)
  - [ ] Add axum-tls to Cargo.toml
  - [ ] Configure in main.rs
  - [ ] Generate self-signed cert for dev

---

## 🟠 Medium Priority (Next 2 Weeks)

### 5. Plugin Bridge Fix
- [ ] Update plugin_bridge.rs to support all 4 plugins (ACLED, GDELT, OpenCorporates, Wikipedia)
- [ ] Test each plugin individually
- [ ] Add error handling for plugin failures
- [ ] Add timeout for plugin calls (30s max)

### 6. Ingestion Pipeline
- [ ] Test `/api/v1/ingest` endpoint with ACLED plugin
- [ ] Verify data stored in PostgreSQL
- [ ] Add deduplication logic (check existing entities before insert)
- [ ] Add rate limiting (max 100 requests/min per API key)

### 7. Search Enhancement
- [ ] Test full-text search with tsvector
- [ ] Add vector similarity search with pgvector
- [ ] Create embedding generation function (Python script or Rust crate)
- [ ] Test hybrid search (text + vector)

### 8. Monitoring & Logging
- [ ] Add structured logging (JSON format)
- [ ] Add /metrics endpoint (Prometheus format)
- [ ] Add health check with DB connection test
- [ ] Set up log rotation (max 100MB per file)

---

## 🟢 Low Priority (Later)

### 9. Frontend MVP
- [ ] Update App.svelte to connect to backend
- [ ] Add entity list view
- [ ] Add entity detail view
- [ ] Add search with filters
- [ ] Add graph visualization (D3.js or similar)

### 11. Documentation
- [ ] Write API documentation (OpenAPI/Swagger)
- [ ] Add plugin development guide
- [ ] Write deployment guide (Hetzner/OVH)
- [ ] Create CONTRIBUTING.md

### 12. CI/CD
- [ ] Set up GitHub Actions for tests
- [ ] Add Docker build on push
- [ ] Add auto-deploy to staging
- [ ] Set up backup verification (test restore monthly)

---

## 📋 Quick Start Commands

```bash
# Start all services
cd /home/yaya/le-coq/docker
docker-compose up -d

# Check logs
docker-compose logs -f backend

# Test health endpoint
curl http://localhost:8000/health

# Test API (with auth)
curl -H "X-API-Key: your-key" http://localhost:8000/api/v1/entities

# Run backup manually
bash /home/yaya/le-coq/scripts/backup.sh

# Build backend
cd /home/yaya/le-coq
cargo build --release

# Run tests
cargo test
```

---

## 🎯 Phase 1 Success Criteria

- [ ] Backend compiles and runs without errors
- [ ] All 7 endpoints respond correctly
- [ ] API key authentication works
- [ ] PostgreSQL + pgvector functional
- [ ] At least 1 plugin (ACLED) ingests data successfully
- [ ] Backup script runs daily via cron
- [ ] Health check returns 200 OK
- [ ] Documentation complete for MVP

**Estimated Time:** 2-4 weeks
**Current Progress:** ~60% complete
