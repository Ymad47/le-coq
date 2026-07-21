# Le Coq — Gap Analysis & Points Non Abordés

**Date :** Juillet 2026  
**Objectif :** Identifier les zones d'ombre entre la vision et l'architecture API proposée  
**Statut :** À valider avant Phase 1

---

## Synthèse des Docs Existantes

| Document | Sujet | Statut |
|----------|-------|--------|
| `VISION.md` | Vision produit, sources (clear/deep/dark) | ✅ Complet |
| `ARCHITECTURE.md` | Architecture système globale | ✅ Complet |
| `VECTOR_STORAGE.md` | Truth Engine, vectors, corrélation | ✅ Complet |
| `FACT-CHECKING.md` | Scoring de véracité, checks | ✅ Complet |
| `BUSINESS_PLAN.md` | Modèle économique open-core | ✅ Complet |
| `API_ARCHITECTURE_DECISION.md` | Choix API (REST + NATS) | ✅ Nouveau |

---

## 🔴 Gaps Identifiés (Non Abordés)

### 1. Truth Engine vs API REST — Compatibilité ?

**Problème :**
- `VECTOR_STORAGE.md` décrit un pipeline **asynchrone** (embed → vector DB → corrélation)
- `API_ARCHITECTURE_DECISION.md` propose du **REST synchrone** (Phase 1)

**Question non résolue :**
```
Plugin POST /ingest → Backend → ???
  - Synchrone : embed + store + correlate avant de répondre ? (lent)
  - Asynchrone : 202 Accepted + job queue ? (complexe)
```

**Impact :**
- Si synchrone : timeout HTTP (embeddings = 100ms-1s par source)
- Si asynchrone : besoin d'une queue (NATS) dès la Phase 1

**Recommandation :**
- **Phase 1 :** REST synchrone pour le MVP (valider le concept)
- **Phase 2 :** NATS + workers async pour le pipeline Truth Engine
- **Documenter :** Le endpoint `/ingest` retourne `202 Accepted` + job_id en Phase 2

---

### 2. Fact-Checking Engine — Où s'intègre-t-il ?

**Problème :**
- `FACT-CHECKING.md` décrit 6 checks (cross-reference, freshness, reputation, etc.)
- Aucun endpoint API dédié dans `API_ARCHITECTURE_DECISION.md`

**Question non résolue :**
```
Le fact-checking est-il :
  - Un endpoint séparé ? POST /api/v1/fact-check
  - Intégré dans le pipeline ingest ? (automatique)
  - Un worker async ? (après ingestion)
```

**Impact :**
- Si automatique : ralentit l'ingestion (6 checks par source)
- Si séparé : besoin d'un endpoint dédié + UI pour review humaine

**Recommandation :**
- **Phase 1 :** Fact-checking basique (2 checks : freshness + source reputation)
- **Phase 2 :** Fact-checking complet (6 checks) + endpoint `/fact-check`
- **Documenter :** Le scoring est stocké avec la source (`confidence` field)

---

### 3. Dark Web / Tor — Implications Architecture ?

**Problème :**
- `VISION.md` prévoit un module Tor (stem library, .onion scrapers)
- Aucune mention dans `API_ARCHITECTURE_DECISION.md`

**Question non résolue :**
```
Le module Tor est-il :
  - Un plugin Python isolé ? (recommandé)
  - Intégré au backend Rust ? (risqué)
  - Un service séparé ? (complexe)
```

**Impact :**
- Sécurité : Tor ne doit pas être sur le même réseau que le backend principal
- Performance : Scraping .onion = lent (circuit rotation)
- Légal : Audit logging requis

**Recommandation :**
- **Plugin Python isolé** (dans un container/VM séparé)
- **Endpoint API :** POST /api/v1/ingest/darkweb (rate-limited)
- **Documenter :** Security boundaries (read-only, no interaction)

---

### 4. Offline Mode / Zig CLI — Comment parle-t-il au backend ?

**Problème :**
- `README.md` mentionne un CLI Zig pour offline mode
- `API_ARCHITECTURE_DECISION.md` ne spécifie pas le protocole CLI ↔ Backend

**Question non résolue :**
```
Le CLI Zig utilise-t-il :
  - REST HTTP ? (simple, mais lourd pour CLI)
  - gRPC ? (perfo, mais complexe)
  - JSON-RPC over stdin/stdout ? (léger, offline-friendly)
```

**Impact :**
- REST = dépendances HTTP dans le CLI (curl-like)
- gRPC = code généré (.proto), binaire
- JSON-RPC = minimal, mais moins documenté

**Recommandation :**
- **Phase 1 :** REST (le CLI appelle l'API comme un client normal)
- **Phase 2 :** gRPC si besoin (perf critique)
- **Documenter :** Le CLI est un client HTTP normal (pas de protocole spécial)

---

### 5. Plugins Python → Backend Rust — Communication ?

**Problème :**
- `ARCHITECTURE.md` décrit des plugins Python pour OSINT
- `API_ARCHITECTURE_DECISION.md` propose REST pour tout

**Question non résolue :**
```
Les plugins Python appellent-ils :
  - L'API REST HTTP ? (simple, mais overhead)
  - gRPC ? (perfo, mais .proto)
  - NATS pub/sub ? (découplé, mais Phase 2)
```

**Impact :**
- REST = facile à debugger (curl, logs)
- gRPC = perfo mais complexité prématurée
- NATS = découplage mais ops supplémentaire

**Recommandation :**
- **Phase 1 :** REST (plugins → POST /ingest)
- **Phase 2 :** NATS (plugins → publish event → backend consume)
- **Documenter :** Les plugins sont des clients HTTP normaux

---

### 6. Multi-Tenancy (SaaS) — Isolation des Données ?

**Problème :**
- `BUSINESS_PLAN.md` prévoit 3 tiers (Community, Premium, Enterprise)
- Aucune mention de multi-tenancy dans `API_ARCHITECTURE_DECISION.md`

**Question non résolue :**
```
Comment isoler les données :
  - Database par tenant ? (lourd, mais isolation forte)
  - Schema par tenant ? (moyen)
  - Row-level security (RLS) ? (léger, PostgreSQL native)
```

**Impact :**
- Si DB par tenant : ops lourd (100 tenants = 100 DB)
- Si RLS : 1 DB, mais risque de fuite si bug
- Si Schema par tenant : compromis

**Recommandation :**
- **Phase 1 :** Single-tenant (ton instance perso)
- **Phase 2 :** Row-level security (PostgreSQL RLS)
- **Documenter :** `tenant_id` dans chaque table + RLS policies

---

### 7. Graph DB (Neo4j) vs PostgreSQL — Comment Coexistent-ils ?

**Problème :**
- `ARCHITECTURE.md` prévoit Neo4j + PostgreSQL
- `VECTOR_STORAGE.md` propose pgvector (PostgreSQL only)
- `API_ARCHITECTURE_DECISION.md` ne mentionne pas Neo4j

**Question non résolue :**
```
Quelle est la source de vérité :
  - PostgreSQL pour tout (entities + vectors) ?
  - Neo4j pour les relationships uniquement ?
  - Les deux synchronisés ? (risque de divergence)
```

**Impact :**
- Si deux DB : sync complexe, risque d'incohérence
- Si PostgreSQL only : perte des algo graph (PageRank, communities)
- Si Neo4j only : perte de pgvector + PostGIS

**Recommandation :**
- **Phase 1 :** PostgreSQL only (entities + relationships + pgvector)
- **Phase 2 :** Neo4j si besoin (algo graph avancés)
- **Documenter :** PostgreSQL = source de vérité, Neo4j = cache de lecture

---

### 8. Live Feeds (16+ Layers) — Comment Gérer le Streaming ?

**Problème :**
- `VISION.md` prévoit 16+ live feeds (flights, ships, seismic, fires)
- `API_ARCHITECTURE_DECISION.md` propose SSE pour le frontend

**Question non résolue :**
```
Les live feeds sont-ils :
  - Polling HTTP ? (simple, mais inefficace)
  - WebSocket ? (bidirectionnel, mais lourd)
  - SSE ? (unidirectionnel, suffisant pour dashboard)
  - NATS stream ? (interne, pas pour frontend)
```

**Impact :**
- Polling = inefficace (1000 clients = 1000 requêtes/s)
- WebSocket = perfo mais complexe (gestion connexions)
- SSE = simple, natif navigateur, suffisant pour updates

**Recommandation :**
- **Phase 1 :** Polling HTTP (GET /events?since=timestamp)
- **Phase 2 :** SSE (GET /events/stream)
- **Documenter :** SSE pour dashboard, polling pour CLI

---

### 9. Sécurité / Encryption — Missing from API Discussion

**Problème :**
- `VISION.md` mentionne encryption, Tor isolation
- `API_ARCHITECTURE_DECISION.md` ne parle pas de sécurité API

**Question non résolue :**
```
Sécurité API :
  - Auth ? (API keys, JWT, OAuth2)
  - Rate limiting ? (par tenant, par endpoint)
  - Encryption ? (TLS obligatoire, data at rest)
  - Audit logs ? (qui a queryé quoi)
```

**Impact :**
- Sans auth : n'importe qui peut appeler l'API
- Sans rate limiting : abuse, DDoS
- Sans TLS : interception des données
- Sans audit logs : pas de traçabilité (requis pour dark web)

**Recommandation :**
- **Phase 1 :** API keys (simple) + TLS (obligatoire)
- **Phase 2 :** JWT + rate limiting + audit logs
- **Documenter :** Security headers (CORS, CSP, HSTS)

---

### 10. Backup / Recovery — Non Abordé

**Problème :**
- Aucune doc ne mentionne la stratégie de backup

**Question non résolue :**
```
Backup :
  - PostgreSQL : pg_dump ? WAL archiving ?
  - Neo4j : neo4j-admin dump ?
  - Frequency : hourly, daily, weekly ?
  - RTO/RPO : combien de temps de perte acceptable ?
```

**Impact :**
- Sans backup : perte de données = fin du projet
- Sans test de restore : backup inutile

**Recommandation :**
- **Phase 1 :** pg_dump daily + WAL archiving (Hetzner Storage Box)
- **Phase 2 :** Backup automatisé + test de restore mensuel
- **Documenter :** RTO = 4h, RPO = 1h (objectif)

---

### 11. Monitoring / Alerting — Non Abordé

**Problème :**
- Aucune doc ne mentionne le monitoring

**Question nonresolved :**
```
Monitoring :
  - Métriques : Prometheus ? Grafana ?
  - Logs : ELK ? Loki ?
  - Alerts : Slack, email, PagerDuty ?
  - Health checks : endpoints dédiés ?
```

**Impact :**
- Sans monitoring : tu ne sais pas quand ça casse
- Sans alerts : downtime non détecté

**Recommandation :**
- **Phase 1 :** Logs fichier + health endpoint (/health)
- **Phase 2 :** Prometheus + Grafana + alerts Slack
- **Documenter :** Métriques clés (latence, erreurs, queue size)

---

### 12. CI/CD — Non Abordé

**Problème :**
- Aucune doc ne mentionne le déploiement automatisé

**Question non résolue :**
```
CI/CD :
  - GitHub Actions ? GitLab CI ?
  - Tests auto avant merge ?
  - Déploiement auto sur main ?
  - Rollback en cas d'erreur ?
```

**Impact :**
- Sans CI : bugs en prod
- Sans CD : déploiement manuel = erreur humaine

**Recommandation :**
- **Phase 1 :** GitHub Actions (build + tests)
- **Phase 2 :** Déploiement auto + rollback
- **Documenter :** Pipeline CI/CD dans le repo

---

## 🟡 Points à Clarifier (Décisions à Prendre)

| Sujet | Option A | Option B | Recommandation |
|-------|----------|----------|----------------|
| **Truth Engine** | Synchrone (REST) | Asynchrone (NATS) | Phase 1: REST, Phase 2: NATS |
| **Fact-Checking** | Automatique | Manuel | Phase 1: Auto (2 checks), Phase 2: Review humaine |
| **Dark Web** | Plugin isolé | Backend Rust | Plugin Python isolé (container) |
| **CLI Protocol** | REST | gRPC | REST (Phase 1), gRPC (si besoin) |
| **Multi-Tenancy** | RLS PostgreSQL | DB par tenant | RLS (Phase 2) |
| **Graph DB** | PostgreSQL only | Neo4j + PostgreSQL | PostgreSQL only (Phase 1) |
| **Live Feeds** | Polling HTTP | SSE | Polling (Phase 1), SSE (Phase 2) |
| **Auth API** | API keys | JWT | API keys (Phase 1), JWT (Phase 2) |
| **Backup** | pg_dump manual | WAL archiving auto | pg_dump daily (Phase 1) |
| **Monitoring** | Logs fichier | Prometheus + Grafana | Logs + /health (Phase 1) |

---

## ✅ Points Alignés avec la Vision

| Vision Le Coq | Architecture Proposée | Alignement |
|---------------|----------------------|------------|
| **Souverain** | Self-hosted (Hetzner/OVH) | ✅ |
| **Léger** | REST + NATS (pas de JVM) | ✅ |
| **Fiable** | PostgreSQL (ACID, éprouvé) | ✅ |
| **Peu de dépendances** | Rust + Python (écosystèmes matures) | ✅ |
| **Open-source** | AGPL (stays open) | ✅ |
| **EU hosting** | Hetzner/OVH/Scaleway | ✅ |
| **Offline mode** | Zig CLI + REST API | ✅ |
| **Dark web** | Plugin Python isolé | ✅ |
| **Truth Engine** | pgvector + corrélation | ✅ |
| **Fact-checking** | Scoring automatique | ✅ |

---

## 📋 Checklist — Prochaines Étapes

### Avant Phase 1 (MVP)

- [ ] **Valider cette gap analysis** (relire, ajuster)
- [ ] **Décider des options** (tableau "Points à Clarifier")
- [ ] **Mettre à jour API_ARCHITECTURE_DECISION.md** avec les décisions
- [ ] **Créer un document SECURITY.md** (auth, TLS, rate limiting)
- [ ] **Créer un document DEPLOYMENT.md** (backup, monitoring, CI/CD)

### Phase 1 (MVP — 2-4 semaines)

- [ ] **Setup PostgreSQL + pgvector** (Hetzner AX52)
- [ ] **Implémenter endpoints REST de base** (/ingest, /entities, /search)
- [ ] **Plugin Python POC** (1 source : OpenCorporates ou ACLED)
- [ ] **CLI Zig POC** (appel API REST, affichage JSON)
- [ ] **Frontend Svelte POC** (search + entity detail)
- [ ] **TLS + API keys** (sécurité minimale)
- [ ] **Backup pg_dump daily** (cron + Storage Box)

### Phase 2 (2-3 mois)

- [ ] **NATS pour events** (plugins → backend)
- [ ] **Fact-checking complet** (6 checks)
- [ ] **SSE pour dashboard** (live updates)
- [ ] **Row-level security** (multi-tenancy)
- [ ] **Monitoring Prometheus + Grafana**
- [ ] **CI/CD GitHub Actions**

### Phase 3 (6+ mois)

- [ ] **Dark web module** (Tor, .onion scrapers)
- [ ] **Neo4j pour graph avancé** (PageRank, communities)
- [ ] **JWT + OAuth2** (auth enterprise)
- [ ] **Audit logs** (compliance)
- [ ] **SaaS multi-tenant** (Premium tier)

---

## 🎯 Verdict

**L'architecture proposée (REST + NATS) est alignée avec la vision Le Coq.**

**Mais :**
- ❌ **Trop simplifiée** pour la Truth Engine (besoin d'async dès Phase 2)
- ❌ **Manque sécurité** (auth, rate limiting, audit logs)
- ❌ **Manque ops** (backup, monitoring, CI/CD)
- ❌ **Manque multi-tenancy** (requis pour SaaS)

**Recommandation :**
1. **Valider cette gap analysis** avec Ymad
2. **Mettre à jour API_ARCHITECTURE_DECISION.md** avec les décisions
3. **Créer SECURITY.md + DEPLOYMENT.md** avant de coder
4. **Phase 1 : MVP simple** (valider le concept)
5. **Phase 2 : Ajouter complexité** (NATS, fact-checking, multi-tenancy)

---

**Document de travail — À valider avant implémentation**

*Fabriqué en France 🇫🇷*  
*Souverain. Transparent. Puissant.*
