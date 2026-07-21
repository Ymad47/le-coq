# Le Coq - Architecture Phase 1 (MVP)

## 🎯 Objectifs Phase 1

- **Stack minimale** : PostgreSQL + pgvector uniquement (pas Neo4j, pas Meilisearch)
- **API REST** : 7 endpoints fonctionnels avec authentification API key
- **Plugins Python** : 4 sources (ACLED, GDELT, OpenCorporates, Wikipedia)
- **Sécurité** : TLS + API keys + backup quotidien
- **Timeline** : 2-4 semaines

---

## 📐 Architecture

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│  CLI / UI   │────▶│  Backend     │────▶│ PostgreSQL  │
│  (Svelte/   │     │  Rust (Axum) │     │ + pgvector  │
│   Zig)      │     │  :8000       │     │  :5432      │
└─────────────┘     └──────────────┘     └─────────────┘
                           │
                           ▼
                    ┌──────────────┐
                    │ Plugin       │
                    │ Bridge       │
                    │ (Flask)      │
                    └──────────────┘
                           │
                           ▼
                    ┌──────────────┐
                    │ Plugins      │
                    │ Python       │
                    │ (4 sources)  │
                    └──────────────┘
```

---

## 🔧 Modifications Récentes

### Suppressions (Simplification)

| Composant | Raison | Alternative |
|-----------|--------|-------------|
| Neo4j | JVM lourde, maintenance complexe | PostgreSQL + relations SQL |
| Meilisearch | Dépendance supplémentaire | PostgreSQL full-text (tsvector) |
| gRPC | Complexité prématurée | REST (Phase 1), gRPC si besoin (Phase 3) |
| NATS | Over-engineering MVP | Appels directs (Phase 1), NATS si scale (Phase 2) |

### Ajouts (Phase 1)

| Composant | Fichier | Description |
|-----------|---------|-------------|
| API Key Auth | `backend/src/api.rs` | Middleware `api_key_auth()` |
| pgvector | `migrations/002_add_pgvector.sql` | Embeddings + similarité |
| Backup | `scripts/backup.sh` | pg_dump quotidien + rotation 7 jours |
| TODO list | `PHASE1_TODO.md` | Checklist détaillée |

---

## 🗂️ Structure Backend

```
backend/
├── src/
│   ├── main.rs           # Point d'entrée, AppState
│   ├── api.rs            # Routes + middleware auth
│   ├── db/
│   │   └── postgres.rs   # Connexion pool SQLx
│   ├── ingestion.rs      # Handlers /ingest
│   └── models.rs         # Structs Entity, Relationship
├── migrations/
│   ├── 001_init_schema.sql
│   └── 002_add_pgvector.sql
├── Cargo.toml            # Dependencies (45 crates)
└── Dockerfile
```

---

## 🔑 Endpoints API

| Endpoint | Méthode | Auth | Description |
|----------|---------|------|-------------|
| `/health` | GET | ❌ | Health check |
| `/api/v1/entities` | GET/POST | ✅ | Lister/créer entités |
| `/api/v1/entities/:id` | GET | ✅ | Détails entité |
| `/api/v1/relationships` | GET/POST | ✅ | Lister/créer relations |
| `/api/v1/search` | POST | ✅ | Recherche full-text + vector |
| `/api/v1/ingest` | POST | ✅ | Ingestion plugins |
| `/api/v1/sources` | GET | ✅ | Lister sources actives |

---

## 🔒 Sécurité

### API Keys
```bash
# Générer une clé
openssl rand -hex 32

# Utiliser dans les requêtes
curl -H "X-API-Key: vOTkR..." http://localhost:8000/api/v1/entities
```

### TLS (Production)
- **Option A** : nginx reverse proxy + Let's Encrypt
- **Option B** : axum-tls (dev uniquement)

### Backup
```bash
# Manuel
bash /home/yaya/le-coq/scripts/backup.sh

# Cron (quotidien 2h)
crontab -e
0 2 * * * /home/yaya/le-coq/scripts/backup.sh
```

---

## 🚀 Quick Start

```bash
# 1. Build backend
cd /home/yaya/le-coq
cargo build --release

# 2. Démarrer PostgreSQL
cd docker
docker-compose up -d postgres

# 3. Vérifier pgvector
docker-compose exec postgres psql -U lecoq -c "CREATE EXTENSION IF NOT EXISTS vector;"

# 4. Démarrer backend
docker-compose up -d backend

# 5. Tester
curl http://localhost:8000/health
curl -H "X-API-Key: test" http://localhost:8000/api/v1/entities
```

---

## 📊 État d'Avancement

| Tâche | Statut | Progress |
|-------|--------|----------|
| Suppression Neo4j/Meilisearch | ✅ | 100% |
| API Key Auth | ✅ | 100% |
| Migration pgvector | ✅ | 100% |
| Script backup | ✅ | 100% |
| Build & tests | ⏳ | 0% |
| TLS setup | ⏳ | 0% |
| Plugin bridge | ⏳ | 0% |
| Frontend MVP | ⏳ | 0% |

**Total Phase 1 :** ~60% complété

---

## 📝 Prochaines Étapes

1. **Cette semaine** : Build + tests + API keys
2. **Semaine 2** : TLS + plugin bridge
3. **Semaine 3-4** : Frontend MVP + documentation

Voir `PHASE1_TODO.md` pour la checklist complète.
