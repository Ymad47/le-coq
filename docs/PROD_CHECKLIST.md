# Le Coq — Pre-Production Checklist

**Current state:** Dev-grade backend. Solid for solo local use.

**Before going to production (any environment reachable by others — VPS, public demo, homelab with port-forward), complete every item below.**

⚠️ Never expose port 8000 without ALL of sections 1-3 done.

---

## 1. Security (BLOCKER — do these first)

### 1.1 Remove DISABLE_AUTH kill-switch
**File:** `backend/.env`
- [ ] Delete `DISABLE_AUTH=true` line entirely
- [ ] Verify: `curl http://localhost:8000/api/v1/entities` returns 401 without header

### 1.2 Real API key validation
**Currently:** any non-empty key passes auth. **Required:** keys validated against a table.

- [ ] Migration: create `api_keys` table:
  ```sql
  CREATE TABLE api_keys (
      id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
      key_hash TEXT NOT NULL UNIQUE,           -- bcrypt or sha256 of key
      label VARCHAR(100) NOT NULL,             -- "laptop", "mobile", "alice"
      created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
      created_by VARCHAR(100),
      revoked_at TIMESTAMP WITH TIME ZONE,
      rate_limit_per_min INTEGER DEFAULT 100,
      last_used_at TIMESTAMP WITH TIME ZONE
  );
  -- Hash keys, never store plaintext.
  -- SELECT * FROM api_keys WHERE key_hash = crypt($1, key_hash) AND revoked_at IS NULL;
  ```
- [ ] Replace `api_key_auth` in `api.rs` to query DB on each request (with 30s in-memory cache)
- [ ] Add `last_used_at` update (audit trail)
- [ ] Document key rotation procedure in `docs/SECURITY.md`

### 1.3 TLS / HTTPS
- [ ] Terminate with **Caddy** (easiest — auto Let's Encrypt):
  ```caddyfile
  # /etc/caddy/Caddyfile
  api.yourdomain.com {
      reverse_proxy localhost:8000
  }
  ```
- [ ] Or nginx + certbot if you prefer
- [ ] Update `BIND_ADDRESS=127.0.0.1:8000` in `.env` (no longer listen on public iface)
- [ ] Verify: `curl https://api.yourdomain.com/health` works, `http://` redirects

### 1.4 Body size limits
- [ ] Add to `create_router` in `api.rs`:
  ```rust
  .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024))  // 10MB
  ```

### 1.5 CORS lockdown
- [ ] Restrict to known frontend origin:
  ```rust
  let cors = tower_http::cors::CorsLayer::new()
      .allow_origin("https://app.yourdomain.com".parse().unwrap())
      .allow_methods([Method::GET, Method::POST])
      .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);
  ```
- [ ] Remove `Access-Control-Allow-Origin: *` if anywhere

---

## 2. Robustness (HIGH PRIORITY)

### 2.1 Pagination on relationships
- [ ] Add `offset` to `RelQuery` in `api.rs`:
  ```rust
  use axum::extract::Query;
  #[derive(Deserialize)]
  struct RelQuery {
      #[serde(default)] limit: Option<u32>,
      #[serde(default)] offset: Option<u32>,
  }
  // LIMIT $1 OFFSET $2
  ```

### 2.2 Statement timeouts (PostgreSQL)
- [ ] Configure per-connection in `db/postgres.rs`:
  ```rust
  .after_connect(|conn, _meta| Box::pin(async move {
      sqlx::query("SET statement_timeout = '10s'").execute(conn).await?;
      Ok(())
  }))
  ```
- [ ] Prevents one slow query from hanging a connection

### 2.3 Graceful shutdown
- [ ] Add to `main.rs`:
  ```rust
  let shutdown = async {
      tokio::signal::ctrl_c().await.expect("ctrl_c handler");
      tracing::info!("Shutdown signal received");
  };
  axum::serve(listener, app)
      .with_graceful_shutdown(shutdown)
      .await?;
  ```

### 2.4 Indexes for hot paths
- [ ] Migration 004:
  ```sql
  -- Trigram index for substring search (replaces full seq scan)
  CREATE EXTENSION IF NOT EXISTS pg_trgm;
  CREATE INDEX idx_entities_name_trgm ON entities USING gin (name gin_trgm_ops);

  -- JSONB properties search
  CREATE INDEX idx_entities_properties_gin ON entities USING gin (properties);

  -- Composite index for graph traversal
  CREATE INDEX idx_rel_from_to ON relationships(from_entity, to_entity);
  CREATE INDEX idx_rel_to_from ON relationships(to_entity, from_entity);
  ```

### 2.5 Rate limiting
- [ ] Add `tower-governor` crate:
  ```toml
  tower-governor = "0.4"
  ```
- [ ] Per-IP burst + sustained limits in `main.rs`:
  ```rust
  let governor_conf = Box::new(
      tower_governor::governor::GovernorConfigBuilder::default()
          .per_second(2)
          .burst_size(5)
          .finish().unwrap()
  );
  ```

---

## 3. Observability (IMPORTANT for ops)

### 3.1 Structured logging with request ID
- [ ] Add `tower-http` RequestId:
  ```rust
  .layer(tower_http::request_id::SetRequestIdLayer::x_request_id(...))
  .layer(tower_http::request_id::PropagateRequestIdLayer::x_request_id(...))
  ```
- [ ] Include in every `tracing::info!` via span

### 3.2 Metrics endpoint
- [ ] Add `metrics` + `metrics-exporter-prometheus` crates
- [ ] Expose `/metrics` (localhost only — not public):
  - Request count by route
  - Request latency p50/p95/p99
  - DB pool connections in use
  - Plugin call counts by source

### 3.3 Health check that actually checks
Replace static "Tout fonctionne Bg !" with real health:
- [ ] DB ping: `SELECT 1` on pool
- [ ] Plugin bridge: `list_plugins()` works
- [ ] Return 503 if anything is degraded

### 3.4 Error tracking
- [ ] Optional: Sentry self-hosted or GlitchTip
- [ ] `tracing` → Sentry via `sentry-tracing` crate

---

## 4. Data Integrity (Phase 2 priorities)

### 4.1 Confidence model alignment
- [ ] Decide: confidence in DB = `UNKNOWN(0.0) | UNVERIFIED(0.2) | CORROBORATED(0.5) | CONFIRMED(0.8) | DISPUTED(0.1)`
- [ ] Migration 005: map existing REAL values to enum
- [ ] Update `models.rs` `Severity`/`Confidence` to match vision doc

### 4.2 Five-timestamp tracking on entities
Add to `entities` table:
- [ ] `occurred_at` (when the event actually happened)
- [ ] `published_at` (when source published it)
- [ ] `observed_at` (when our plugin saw it)
- [ ] `collected_at` (when ingest picked it up)
- [ ] `updated_at` (already exists)

This enables "what did we know at 10:00" historical replay.

### 4.3 Soft delete
- [ ] Add `deleted_at TIMESTAMP WITH TIME ZONE` to entities + relationships
- [ ] Change SELECTs to `WHERE deleted_at IS NULL`
- [ ] Never hard-delete — audit trail

---

## 5. Code Quality

### 5.1 Separation of concerns (the violation we noted)
**Currently:** SQL in `api.rs` handlers. **Target:** SQL in `db/*.rs`, handlers stay thin.
- [ ] Move entity queries to `db/entities.rs`
- [ ] Move relationship queries to `db/relationships.rs`
- [ ] `api.rs` becomes HTTP-only (extract → call db fn → format response)
- [ ] Add integration tests in `tests/rust/api_integration.rs`

### 5.2 Language consistency
- [ ] All user-facing API strings in English (entity_summary, health check)
- [ ] Logs in English
- [ ] Comments bilingual OK, but errors consistent

### 5.3 Error responses with shape
Standardize on:
```json
{"error": {"code": "VALIDATION_FAILED", "message": "...", "details": {...}}}
```
- [ ] Replace `StatusCode::BAD_REQUEST` returns with typed error struct
- [ ] Add error codes enum in `api.rs`

---

## 6. Deployment

### 6.1 Production systemd hardening
Update `/etc/systemd/system/le-coq-backend.service`:
```ini
[Service]
# ... existing config ...

# Hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/var/lib/le-coq /var/log/le-coq
PrivateTmp=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes
MemoryMax=512M
CPUQuota=50%
Restart=on-failure
RestartSec=10
```

### 6.2 Backup automation
- [ ] Daily `pg_dump` cron:
  ```bash
  0 3 * * * pg_dump postgresql://user:***@localhost/lecoq | gzip > /var/backups/lecoq-$(date +\%Y\%m\%d).sql.gz
  ```
- [ ] Retention: keep 7 daily, 4 weekly, 12 monthly
- [ ] Test restore monthly: `pg_restore` to a scratch DB

### 6.3 Secret management
- [ ] Move all secrets out of `.env` → use systemd `LoadCredential=` or `pass`
- [ ] Rotate DB password
- [ ] Rotate API keys

---

## 7. Pre-Go-Live Verification

Run this end-to-end before flipping any DNS:

- [ ] `cargo clippy -- -D warnings` — zero warnings
- [ ] `cargo test --all-targets` — all pass
- [ ] `cargo audit` — zero high/critical vulns
- [ ] `sqlfluff lint backend/migrations/` — clean
- [ ] Smoke test all 11 routes with curl
- [ ] Load test with `oha -z 30s -c 20 http://localhost:8000/api/v1/entities` — no 500s
- [ ] Kill postgres mid-request → API returns 503 (not panic)
- [ ] Kill python subprocess → next request still works
- [ ] Full disk → app doesn't corrupt DB
- [ ] Backup restore test — restore latest dump to scratch DB, run queries

---

## Sign-off

When all sections checked:
- Version: _______________
- Date: _______________
- Reviewer: _______________
- Deployed to: _______________

---

**Note:** None of this matters until Phase 1 is actually finished (per `PHASE1_TODO.md`: smart search end-to-end, frontend build, TLS, ingest pipeline). Don't prod-harden incomplete features.
