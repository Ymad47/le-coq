# Le Coq — API Architecture Decision

**Décision Architecturale Validée — Juillet 2026**

> Philosophie : **"Simple d'abord, optimise après"**  
> Objectif : Outil puissant, léger, fiable, sans dépendance technique lourde

---

## 📋 Contexte

Cette décision fait suite à l'analyse de 12 architectures API (REST, GraphQL, gRPC, etc.) et à la revue complète des gaps documentaires. **12 questions architecturales ont été tranchées** avec l'utilisateur.

**Critères de décision :**
- ✅ Solide et fiable (pas de fragilité)
- ✅ Léger (pas de JVM, pas de dépendances lourdes)
- ✅ Maintenance simple (1 personne, pas d'équipe ops)
- ✅ Time-to-market rapide (semaines, pas mois)
- ✅ Souverain (self-hosted, EU, pas de lock-in cloud US)

---

## 🎯 Architecture Recommandée

### Stack Technique

| Composant | Technologie | Justification |
|-----------|-------------|---------------|
| **Backend** | Rust (axum) | Perfo, sécurité mémoire, binaire statique |
| **API** | REST HTTP | Simple, debug facile (curl), écosystème mature |
| **Database** | PostgreSQL + pgvector | 1 source de vérité, ACID, vectors natifs |
| **Cache** | Redis | Rate limiting, sessions, léger |
| **Message Queue** | NATS (Phase 2) | Léger (Go), pas de JVM, découplage async |
| **Search** | Meilisearch | Full-text, Rust, léger |
| **Frontend** | Svelte | Léger, réactif, pas de virtual DOM lourd |
| **CLI** | Zig | Binaire statique, offline-friendly |
| **Plugins** | Python | Écosystème OSINT/ML, facile à étendre |

### Pourquoi pas gRPC dès le début ?

| Critère | REST | gRPC |
|---------|------|------|
| **Complexité** | Faible (HTTP + JSON) | Élevée (.proto, code généré) |
| **Debug** | curl, logs lisibles | binaire, besoin d'outils |
| **Recrutement** | Développeurs HTTP = partout | Rust + gRPC + .proto = rare |
| **Time-to-market** | Semaines | Mois |
| **Maintenance** | 1 personne | Équipe dédiée |

**Verdict :** REST → gRPC = refactor localisé possible. gRPC dès le début = 6 mois d'infra avant de valider le produit.

---

## ✅ Décisions Validées (12 Gaps)

### GAP 1 — Truth Engine : Pipeline d'ingestion

**Décision :** **Hybride — synchrone MVP (Phase 1), NATS async (Phase 2)**

```
Phase 1 (REST synchrone) :
  Plugin → POST /ingest → Backend → embed + store → 200 OK
  ⚠️ Lent (100ms-1s par source) mais simple

Phase 2 (NATS asynchrone) :
  Plugin → POST /ingest → Backend → 202 Accepted + job_id
         → NATS publish → Worker async → embed + store + correlate
         → WebSocket/SSE → Frontend (job completed)
  ✅ Scalable, backpressure, retry
```

**Endpoints :**
```
POST /api/v1/ingest           # Phase 1: synchrone
POST /api/v1/ingest/async     # Phase 2: 202 Accepted + job_id
GET  /api/v1/jobs/:id/status  # Phase 2: polling status
```

---

### GAP 2 — Fact-Checking : Intégration

**Décision :** **Hybride — 2 checks auto à l'ingest, 6 checks complets en async**

```
Phase 1 (auto, synchrone) :
  - Freshness (date de la source)
  - Source reputation (score pré-calculé)
  → Score basique stocké avec la source

Phase 2 (async, worker) :
  - Cross-reference (autres sources)
  - Entity consistency (cohérence entités)
  - Temporal coherence (timeline)
  - Source bias detection (biais politique)
  - Content analysis (NLP)
  - Network propagation (viralité)
  → Score complet mis à jour en async
```

**Schema :**
```sql
CREATE TABLE sources (
    id UUID PRIMARY KEY,
    url TEXT,
    content TEXT,
    -- Fact-checking scores
    freshness_score FLOAT,          -- Phase 1
    reputation_score FLOAT,         -- Phase 1
    cross_reference_score FLOAT,    -- Phase 2
    consistency_score FLOAT,        -- Phase 2
    temporal_score FLOAT,           -- Phase 2
    bias_score FLOAT,               -- Phase 2
    nlp_score FLOAT,                -- Phase 2
    propagation_score FLOAT,        -- Phase 2
    overall_confidence FLOAT,       -- Moyenne pondérée
    fact_check_status TEXT,         -- 'pending', 'verified', 'disputed'
    fact_checked_at TIMESTAMPTZ,
    fact_check_version INTEGER      -- 1 = Phase 1, 2 = Phase 2
);
```

---

### GAP 3 — Dark Web / Tor : Isolation

**Décision :** **Plugin Python isolé dans son propre container**

**Architecture :**
```
┌─────────────────────────────────────────────┐
│  Tor Plugin Container (isolated network)    │
│  ┌─────────────┐                            │
│  │ Python      │                            │
│  │ stem lib    │─── socks5 ───> Tor         │
│  │ scraper     │                            │
│  └─────────────┘                            │
│         │                                   │
│         │ POST /ingest (HTTP)               │
└─────────┼───────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────┐
│  Backend Container (main network)           │
│  ┌─────────────┐                            │
│  │ Rust        │                            │
│  │ axum        │<─── HTTP /ingest           │
│  │ API         │                            │
│  └─────────────┘                            │
└─────────────────────────────────────────────┘
```

**Security boundaries :**
- ✅ Read-only (no POST/interaction)
- ✅ Tor circuit rotation (every request)
- ✅ Container isolation (pas d'accès direct au backend)
- ✅ Audit logging (qui a scrapé quoi, quand)
- ❌ Jamais de login sur les services darknet
- ❌ Jamais de téléchargement de contenu illégal

---

### GAP 4 — CLI Zig : Protocole

**Décision :** **REST HTTP — CLI appelle l'API comme un client normal**

**Usage :**
```bash
# CLI Zig → HTTP → Backend
./le-coq search "query"
./le-coq ingest file.json
./le-coq entity get 123
```

**Implémentation :**
```zig
// cli/src/main.zig
const std = @import("std");
const http = std.http;

pub fn search(query: []const u8) !void {
    var client = http.Client{ .allocator = std.heap.page_allocator };
    defer client.deinit();
    
    const url = try std.fmt.allocPrint(
        std.heap.page_allocator,
        "{s}/api/v1/search?q={s}",
        .{ base_url, query }
    );
    
    const response = try client.get(.{
        .url = url,
        .headers = .{
            .user_agent = "le-coq-cli/0.1.0",
            .authorization = try std.fmt.allocPrint(
                std.heap.page_allocator,
                "Bearer {s}",
                .{api_key}
            ),
        },
    });
    
    // Parse JSON response, display to user
}
```

**Pourquoi pas gRPC ?**
- CLI = usage intermittent, pas de perf critique
- REST = facile à debugger (curl, logs)
- gRPC = code généré, binaire, overkill

---

### GAP 5 — Plugins Python → Backend : Protocole

**Décision :** **REST HTTP — plugins appellent POST /ingest**

**Implémentation :**
```python
# plugins/sources/opencorporates.py
import requests

def ingest_company_data(company_id: str) -> dict:
    """Fetch from OpenCorporates, POST to backend"""
    
    # Fetch from source
    response = requests.get(
        f"https://api.opencorporates.com/companies/{company_id}",
        params={'api_token': API_KEY}
    )
    data = response.json()
    
    # POST to backend
    ingest_response = requests.post(
        "http://localhost:8080/api/v1/ingest",
        json={
            'source': 'opencorporates',
            'source_id': company_id,
            'content': data,
            'entity_type': 'company'
        },
        headers={'X-API-Key': BACKEND_API_KEY}
    )
    
    return ingest_response.json()
```

**Phase 2 (NATS) :**
```python
# plugins/sources/opencorporates.py (Phase 2)
import nats

async def ingest_company_data(company_id: str):
    nc = await nats.connect("nats://localhost:4222")
    
    # Fetch from source
    data = await fetch_from_opencorporates(company_id)
    
    # Publish to NATS (découplé)
    await nc.publish("ingest.opencorporates", json.dumps({
        'source': 'opencorporates',
        'source_id': company_id,
        'content': data,
        'entity_type': 'company'
    }))
    
    await nc.close()
```

---

### GAP 6 — Multi-Tenancy : Isolation

**Décision :** **Single-tenant only — ton instance perso (Phase 1), multi-tenant plus tard**

**Phase 1 :**
- 1 instance = 1 utilisateur (toi)
- Pas de multi-tenancy à gérer
- Database unique, pas d'isolation nécessaire

**Phase 2 (SaaS) :**
- Row-level security (PostgreSQL RLS)
- `tenant_id` dans chaque table
- Policies RLS pour isoler les données

```sql
-- Phase 2: RLS
ALTER TABLE entities ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON entities
    USING (tenant_id = current_setting('app.current_tenant')::uuid);

-- Backend set tenant_id per request
SET app.current_tenant = 'uuid-du-tenant';
```

---

### GAP 7 — Graph DB : Source de vérité

**Décision :** **PostgreSQL only (entities + relationships + pgvector)**

**Pourquoi pas Neo4j ?**
- 1 source de vérité (pas de sync, pas de divergence)
- pgvector = vectors natifs dans PostgreSQL
- PostGIS = géospatial natif
- Cypher (Neo4j) vs SQL (PostgreSQL) = SQL plus connu
- Ops plus simple (1 DB au lieu de 2)

**Schema :**
```sql
-- Entities
CREATE TABLE entities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    type TEXT NOT NULL,  -- 'person', 'company', 'event', etc.
    metadata JSONB,
    embedding vector(1536),  -- pgvector
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Relationships
CREATE TABLE relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID REFERENCES entities(id),
    target_id UUID REFERENCES entities(id),
    type TEXT NOT NULL,  -- 'owns', 'works_for', 'located_in', etc.
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index for graph traversal
CREATE INDEX ON relationships (source_id, type);
CREATE INDEX ON relationships (target_id, type);

-- Index for vector search
CREATE INDEX ON entities USING ivfflat (embedding vector_cosine_ops);
```

**Graph algorithms (Phase 2) :**
- PageRank, centrality, communities = SQL queries ou extension PostgreSQL (Apache Age)
- Si besoin critique : Neo4j en cache de lecture (sync unidirectionnelle)

---

### GAP 8 — Live Feeds : Streaming

**Décision :** **Polling HTTP (GET /events?since=timestamp)**

**Phase 1 :**
```
GET /api/v1/events?since=2026-07-21T10:00:00Z
→ 200 OK + JSON array of events
```

**Implémentation frontend :**
```javascript
// frontend/src/stores/events.js
let lastTimestamp = new Date().toISOString();

async function pollEvents() {
    const response = await fetch(
        `/api/v1/events?since=${lastTimestamp}`
    );
    const events = await response.json();
    
    if (events.length > 0) {
        // Update UI
        lastTimestamp = events[events.length - 1].timestamp;
    }
    
    // Poll again in 5 seconds
    setTimeout(pollEvents, 5000);
}

pollEvents();
```

**Phase 2 (SSE) :**
```
GET /api/v1/events/stream
→ Server-Sent Events (natif navigateur)
```

```javascript
// Phase 2: SSE
const eventSource = new EventSource('/api/v1/events/stream');

eventSource.onmessage = (event) => {
    const newEvent = JSON.parse(event.data);
    // Update UI
};
```

---

### GAP 9 — Sécurité API : Authentification

**Décision :** **Hybride — API keys Phase 1, JWT Phase 2, OAuth2 Phase 3**

**Phase 1 (API keys) :**
```bash
curl -H "X-API-Key: your-secret-key" https://api.lecoq.fr/api/v1/entities
```

**Implémentation Rust :**
```rust
// backend/src/auth.rs
pub async fn auth_middleware(
    headers: HeaderMap,
    next: Next,
) -> Result<Response, StatusCode> {
    let api_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    // Validate against database
    let valid = db.validate_api_key(api_key).await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    Ok(next.run(valid.into_request()).await)
}
```

**Phase 2 (JWT) :**
```
POST /api/v1/auth/login → JWT access + refresh tokens
GET /api/v1/entities → Bearer <JWT>
```

**Phase 3 (OAuth2/SAML) :**
- SSO enterprise
- SAML 2.0, OIDC
- Requis pour vendre aux gouvernements/entreprises

---

### GAP 10 — Backup : Stratégie

**Décision :** **pg_dump daily + cron (simple, manuel, suffit Phase 1)**

**Script :**
```bash
#!/bin/bash
# /home/yaya/scripts/backup-lecoq.sh

DATE=$(date +%Y-%m-%d_%H-%M-%S)
BACKUP_DIR="/backup/lecoq"
DB_NAME="lecoq"
DB_USER="lecoq"

# Dump
pg_dump -U $DB_USER -d $DB_NAME -F c -f "$BACKUP_DIR/lecoq_$DATE.dump"

# Compress
gzip "$BACKUP_DIR/lecoq_$DATE.dump"

# Upload to Hetzner Storage Box
scp "$BACKUP_DIR/lecoq_$DATE.dump.gz" storage-box.hetzner.com:/backup/

# Delete local (keep 7 days)
find $BACKUP_DIR -name "*.dump.gz" -mtime +7 -delete

echo "Backup done: lecoq_$DATE.dump.gz"
```

**Cron :**
```bash
# Crontab (every day at 3am)
0 3 * * * /home/yaya/scripts/backup-lecoq.sh >> /var/log/lecoq-backup.log 2>&1
```

**Phase 2 (WAL archiving) :**
- Point-in-time recovery (PITR)
- RPO = 1h (perte max 1h de données)
- pgBackRest ou WAL-G

---

### GAP 11 — Monitoring : Stack

**Décision :** **Hybride — logs + /health Phase 1, Prometheus Phase 2, ELK Phase 3**

**Phase 1 :**
```rust
// backend/src/main.rs
#[tokio::main]
async fn main() {
    // Setup logging
    tracing_subscriber::fmt::init();
    
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/*path", post(api_handler));
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "timestamp": Utc::now().iso8601(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}
```

**Logs :**
```
2026-07-21T10:30:00Z INFO  Request: POST /api/v1/ingest - 200 OK (150ms)
2026-07-21T10:30:05Z ERROR Database connection failed: timeout
```

**Phase 2 (Prometheus + Grafana) :**
- Métriques : latence, erreurs, queue size, DB connections
- Dashboard : Grafana
- Alerts : Slack/email

**Phase 3 (ELK/Loki) :**
- Logs centralisés
- Recherche full-text
- Corrélation events

---

### GAP 12 — CI/CD : Pipeline

**Décision :** **Hybride — GitHub Actions (tests) + deploy manuel Phase 1, auto-deploy Phase 2**

**Phase 1 (GitHub Actions) :**
```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Build
        run: cargo build --release
      
      - name: Test
        run: cargo test
      
      - name: Security Audit
        run: cargo audit

  docker:
    needs: test
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v3
      
      - name: Build Docker image
        run: docker build -t lecoq-backend .
      
      - name: Push to registry
        run: |
          docker tag lecoq-backend registry.hetzner.com/lecoq/backend:latest
          docker push registry.hetzner.com/lecoq/backend:latest
```

**Deploy manuel (Phase 1) :**
```bash
# SSH to Hetzner server
ssh yaya@api.lecoq.fr

# Pull latest image
docker pull registry.hetzner.com/lecoq/backend:latest

# Restart container
docker compose restart backend
```

**Phase 2 (auto-deploy) :**
```yaml
# .github/workflows/deploy.yml
name: Deploy

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Deploy to Hetzner
        uses: appleboy/ssh-action@master
        with:
          host: ${{ secrets.HETZNER_HOST }}
          username: ${{ secrets.HETZNER_USER }}
          key: ${{ secrets.HETZNER_SSH_KEY }}
          script: |
            cd /opt/lecoq
            docker pull registry.hetzner.com/lecoq/backend:latest
            docker compose up -d --no-deps backend
```

---

## 📊 Roadmap par Phase

### Phase 1 (MVP — 2-4 semaines)

**Objectif :** Valider le concept (ingest → search → visualize)

| Composant | Techno | Statut |
|-----------|--------|--------|
| Backend API | Rust + axum (REST) | ✅ À coder |
| Database | PostgreSQL + pgvector | ✅ À setup |
| Plugins | Python (1 source POC) | ✅ À coder |
| CLI | Zig (HTTP client) | ✅ À coder |
| Frontend | Svelte (search + detail) | ✅ À coder |
| Auth | API keys | ✅ À implémenter |
| TLS | Certbot + Let's Encrypt | ✅ À setup |
| Backup | pg_dump daily (cron) | ✅ À scripter |
| Monitoring | Logs fichier + /health | ✅ À ajouter |
| CI/CD | GitHub Actions (tests) | ✅ À configurer |

**Endpoints REST :**
```
GET  /health                    # Health check
POST /api/v1/ingest             # Ingest data (synchrone)
GET  /api/v1/entities           # List entities
GET  /api/v1/entities/:id       # Get entity by ID
POST /api/v1/entities           # Create entity
GET  /api/v1/relationships      # List relationships
POST /api/v1/relationships      # Create relationship
POST /api/v1/search             # Full-text search
GET  /api/v1/events?since=:ts   # Polling live events
```

---

### Phase 2 (Découplage — 2-3 mois)

**Objectif :** Résilience, backpressure, multi-tenancy

| Composant | Techno | Statut |
|-----------|--------|--------|
| Message Queue | NATS | ⏳ Phase 2 |
| Async Ingest | Workers + NATS | ⏳ Phase 2 |
| Fact-Checking | 6 checks complets | ⏳ Phase 2 |
| Auth | JWT + refresh tokens | ⏳ Phase 2 |
| Multi-Tenancy | PostgreSQL RLS | ⏳ Phase 2 |
| Live Feeds | SSE (Server-Sent Events) | ⏳ Phase 2 |
| Monitoring | Prometheus + Grafana | ⏳ Phase 2 |
| CI/CD | Auto-deploy | ⏳ Phase 2 |

---

### Phase 3 (Scale — 6+ mois)

**Objectif :** Performance à grande échelle, enterprise features

| Composant | Techno | Statut |
|-----------|--------|--------|
| gRPC | Inter-services (si besoin) | ⏳ Phase 3 |
| Neo4j | Graph avancé (PageRank, etc.) | ⏳ Phase 3 |
| Dark Web | Tor plugin isolé | ⏳ Phase 3 |
| Auth | OAuth2/SAML (SSO enterprise) | ⏳ Phase 3 |
| Audit Logs | Compliance (GDPR, SOC2) | ⏳ Phase 3 |
| Monitoring | ELK/Loki (logs centralisés) | ⏳ Phase 3 |
| SaaS | Multi-tenant (Premium tier) | ⏳ Phase 3 |

---

## 🔒 Sécurité

Voir **SECURITY.md** pour :
- Authentification (API keys, JWT, OAuth2)
- Rate limiting (Redis, tier-based)
- TLS/HTTPS (Certbot, headers)
- Audit logs (qui a fait quoi, quand)
- Dark Web isolation (container, network)
- Secrets management (.env, Vault)
- Security testing (OWASP ZAP, cargo audit)

---

## 📚 Documents Liés

- **VISION.md** — Vision produit, sources (clear/deep/dark)
- **ARCHITECTURE.md** — Architecture système globale
- **VECTOR_STORAGE.md** — Truth Engine, vectors, corrélation
- **FACT-CHECKING.md** — Scoring de véracité
- **BUSINESS_PLAN.md** — Modèle économique open-core
- **GAP_ANALYSIS.md** — 12 gaps identifiés + recommandations
- **SECURITY.md** — Sécurité par phase
- **DEPLOYMENT.md** — Backup, monitoring, CI/CD (à créer)

---

## 🎯 Verdict

**Architecture : REST + PostgreSQL (Phase 1) → NATS (Phase 2)**

**Respect des objectifs Le Coq :**
- ✅ **Souverain** — Self-hosted (Hetzner/OVH), pas de cloud US
- ✅ **Léger** — Rust + NATS (0 JVM), binaire statique
- ✅ **Fiable** — PostgreSQL (ACID, 20+ ans), NATS (épis en prod)
- ✅ **Peu de dépendances** — < 50 crates, écosystèmes matures
- ✅ **Maintenance simple** — 1 personne, ops léger
- ✅ **Time-to-market** — Semaines (MVP), pas mois (infra)
- ✅ **Open-source** — AGPL (stays open forever)

**Prochaine étape :**
1. ✅ Gap analysis validée (12 décisions)
2. ✅ API_ARCHITECTURE_DECISION.md mise à jour
3. ⏳ **Créer DEPLOYMENT.md** (backup, monitoring, CI/CD détaillés)
4. ⏳ **Coder Phase 1** (PostgreSQL + REST API + 1 plugin POC)

---

*Fabriqué en France 🇫🇷*  
*Souverain. Transparent. Puissant.*  
*AGPL-3.0 — Stays open forever*
