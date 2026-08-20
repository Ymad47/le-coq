# Le Coq — Vision Produit

> **Build a microscope, not an oracle.**

Une plateforme OSINT géopolitique qui aide les analystes à investiguer des événements complexes — sans prétendre dire "la vérité".

---

## 1. Le Problème

Quand un événement géopolitique arrive, 5 médias = 5 récits différents.

L'analyste doit reconstruire manuellement :
- Quoi ? Quand ? Où ? Qui ?
- Quelles sources s'accordent / se contredisent ?
- Qu'est-ce qu'on savait à 10h00 vs 15h00 ?
- Quelles preuves pour chaque affirmation ?

**Le Coq = un microscope.** Pas un oracle. On montre les preuves, pas les conclusions.

---

## 2. Architecture Principale

```
Sources Publiques → Ingestion → Normalisation → Entités/Événements
                                              ↓
                                    Relations → Timeline / Carte / Graphe
                                              ↓
                                         Preuves → Analyste Humain
```

**Principe :** Complexité en dessous, simplicité en surface.

---

## 3. Stack Technique (Phase 1)

| Couche | Techno | Pourquoi |
|--------|--------|----------|
| Frontend | Svelte + TS | UI réactive, légère |
| Backend | Rust (axum) | Perf, sécurité, type-safe |
| Plugins | Python | Flexibilité, écosystème OSINT |
| Bridge | Flask + PyO3 | Rust ↔ Python |
| Database | PostgreSQL + pgvector | 1 seul dep (graph + vector + text) |
| CLI | Zig (deferred) | Binaires natifs, stable |

**Décision Phase 1 :** Pas de Neo4j, Meilisearch, Kafka, gRPC. PostgreSQL suffit.

---

## 4. Plugin Architecture

Chaque source = plugin indépendant avec contrat commun :

```
Rust → Plugin Interface → GDELT/ACLED/News/etc. → API Externe
```

**Avantages :**
- Ajouter/supprimer des sources sans toucher le core
- Rate limiter indépendant par source
- Cache TTL variable par source
- Auth spécifique par source

**Voir :** `docs/PLUGIN_API.md` (à écrire)

---

## 5. Performance Strategy

**Règle :** Mesurer avant d'optimiser.

Exemple de profil :
```
Requête totale : 1.7s
├── External API : 620ms (36%)
├── Normalization : 80ms (5%)
├── Database : 90ms (5%)
├── Graph gen : 45ms (3%)
└── Rust API : 12ms (1%)
```

**Optimiser Rust = inutile.** Le goulot = API externe.

---

## 6. Cache + Rate Limiting

### Cache

| Type de donnée | TTL |
|----------------|-----|
| Métadonnées statiques | Heures/jours |
| Recherche news | Minutes |
| Données historiques | Jours/semaines |

### Request Deduplication

20 utilisateurs demandent la même requête simultanément :
- **Sans dedup :** 20 appels API
- **Avec dedup :** 1 appel API, résultat partagé

**Impact :** Réduction quotas API de 20x. Critique pour GDELT/ACLED.

---

## 7. Modèle de Données

### Global vs Workspace

**Global Data (public, partagé) :**
- Événements, Acteurs, Organisations, Lieux, Sources
- Un événement = 1 entrée, pas dupliqué par utilisateur

**Workspace Data (privé, par utilisateur) :**
- Investigations, Notes, Hypothèses, Relations personnalisées
- Référence les événements globaux

```
                Événement Global #92831
                   /                  \
                  /                    \
        Investigation A          Investigation B
```

**Avantages :**
- Pas de duplication
- Scalable multi-tenant
- Données sources cohérentes

---

## 8. Multi-Tenant Architecture

```
Organisation
├── Utilisateur A
│   ├── Investigation 1
│   └── Investigation 2
├── Utilisateur B
└── Utilisateur C
```

**Isolation :** Backend enforced, pas frontend trust.

**Déploiement :**
- Community : Self-hosted
- SaaS : Vendor-hosted
- Enterprise : Customer infrastructure

---

## 9. 5 Timestamps par Événement

| Champ | Signification |
|-------|---------------|
| `occurred_at` | Quand l'événement s'est produit |
| `published_at` | Quand la source a publié |
| `observed_at` | Quand la source a observé |
| `collected_at` | Quand la plateforme a collecté |
| `updated_at` | Quand la source a mis à jour |

**Pourquoi :** Permet le **Historical Replay** : "Qu'est-ce qu'on savait à 10h00 ?"

---

## 10. Evidence & Provenance

Chaque affirmation = traçable :

```
Claim : "Acteur X a organisé l'Événement Y"
├── Source A → supporte
├── Source B → supporte
├── Source C → contredit
└── Official statement → nie
```

**L'analyste voit :**
- Source originale
- Heure de publication
- Identité de la source
- Contradictions
- Historique des changements

---

## 11. Confidence Levels

La plateforme montre l'incertitude honnêtement :

| Niveau | Signification |
|--------|---------------|
| `UNKNOWN` | Pas assez de données |
| `UNVERIFIED` | 1 source, pas confirmé |
| `CORROBORATED` | 2+ sources indépendantes |
| `CONFIRMED` | Source primaire + vérification |
| `DISPUTED` | Sources se contredisent |
| `RETRACTED` | Source a retiré |

**Principe :** "Insufficient evidence" = affiché clairement.

---

## 12. Historical Provenance

**Jamais d'overwrite silencieux.**

Exemple :
```
18:00 → Source rapporte X
19:30 → Source modifie rapport
20:00 → Source retire X
```

**Les 3 versions restent accessibles.**

**Pourquoi :** L'analyste peut reconstruire l'évolution de l'information.

---

## 13. Timeline (Killer Feature)

Même modèle d'événement = 3 vues :

| Vue | Question |
|-----|----------|
| **Timeline** | Quand ? |
| **Carte** | Où ? |
| **Graphe** | Qui/quoi est connecté ? |

**Exemple Timeline :**
```
27 MAR
├── 08:14 Source A rapporte incident
├── 08:32 Source B publie preuve
├── 09:05 Official statement
├── 10:41 Source A modifie rapport
├── 12:20 Confirmation indépendante
└── 15:03 Événement lié
```

---

## 14. Historical Replay (Killer Feature)

Slider temporel :

```
08:00 ─────────────────────────── 20:00
        ▲
        │
      Slider
```

**Question :** "Quelle information était disponible à 10h00 ?"

**Usage :** Étudier comment une situation a évolué, pas juste le résultat final.

---

## 15. Narrative Analysis (Différenciateur)

Même événement, 5 sources :

```
                    MÊME ÉVÉNEMENT
                         │
        ┌────────────────┼────────────────┐
        ↓                ↓                ↓
     Source A         Source B         Source C
        │                │                │
   Narrative A      Narrative B      Narrative C
```

**La plateforme montre :**
- Descriptions différentes
- Informations manquantes
- Contradictions
- Acteurs mis en avant
- Terminologie différente

**Pas de :** "Source A a raison."

---

## 16. UX Principles

**Problème :** Des millions de records ≠ interface overload.

**Solutions :**
- **Progressive disclosure** — Commence simple, approfondis sur demande
- **View = Question** — Timeline (Quand?), Carte (Où?), Graphe (Qui?)
- **Complexité en dessous** — L'analyste ne voit pas le moteur

---

## 17. Investigation UI

```
┌────────────────────────────────────────────────────┐
│ Investigation : Incident Golfe d'Aden              │
├──────────────┬─────────────────────────────────────┤
│ FILTRES      │ VUE PRINCIPALE                      │
│              │                                     │
│ Acteurs      │ Timeline / Carte / Graphe           │
│ Lieux        │                                     │
│ Dates        │                                     │
│ Sources      │                                     │
│ Type         │                                     │
├──────────────┴──────────────────────┬──────────────┤
│ Timeline / Événements               │ Preuves      │
│                                     │              │
│ 14:32 Incident A                    │ Source A     │
│ 15:01 Incident B                    │ Source B     │
│ 16:43 Incident C                    │ Conflit      │
└─────────────────────────────────────┴──────────────┘
```

---

## 18. Core Data Model

**Entités :**
- User, Organization, Workspace, Investigation
- Actor, Organization, Event, Location, Source
- Observation, Claim, Evidence, Relationship

**Relations :**
```
Actor ──participated_in──► Event
Actor ──member_of────────► Organization
Event ──occurred_at──────► Location
Event ──supported_by─────► Evidence
Claim ──contradicted_by──► Source
Event ──related_to───────► Event
```

---

## 19. AI Governance

**AI = accélérateur, pas architecte.**

**Humain possède :**
- Architecture
- Sécurité
- Authorization
- Data model
- Product direction

**Règle :** "If the AI produces code you cannot explain, don't merge it."

**Workflow :**
```
AI implementation → Tests → AI review → Human review 
                  → Understand → Benchmark → Commit
```

---

## 20. Security Principles

| Principe | Implémentation |
|----------|----------------|
| Auth forte | Sessions cryptographiquement sûres |
| Isolation workspace | Backend enforced (DB level) |
| Secrets | Jamais exposés au frontend |
| Audit logging | Toutes actions critiques |
| Rate limiting | Par utilisateur + par source |
| TLS | nginx (prod) ou axum-tls (dev) |
| Backup | Daily pg_dump + 7-day rotation |

**Voir :** `docs/SECURITY.md`

---

## 21. Scalability

**MVP :** ~1,000 records

**Cible :** 1,000,000+ records

**Technos (si besoin) :**
- PostgreSQL + PostGIS + pgvector
- Redis (cache)
- Object storage (preuves, médias)
- Search engine (si full-text PostgreSQL insuffisant)
- Background workers (ingestion async)

**Principe :** Commence simple, ajoute quand profiling montre le besoin.

---

## 22. Entity Resolution (Phase 3)

Même entité, noms différents :

```
                    "Donald Trump"
                    /     |     \
             "Trump"  "President Trump"  "Donald J. Trump"
```

** requis :**
- Similarité temporelle
- Similarité géographique
- Overlap d'entités
- Similarité texte
- Relations entre sources

---

## 23. Event Correlation (Phase 3)

5 sources = même événement réel :

```
Source A ─────┐
Source B ─────┼──► Même Événement
Source C ─────┤
Source D ─────┘
```

**Au lieu de :** 4 événements indépendants.

---

## 24. Roadmap 6 Phases

| Phase | Focus | Durée | Livrables |
|-------|-------|-------|-----------|
| **1 — MVP** | Maintenant | 4 semaines | Auth, isolation, cache, rate limit, dedup, sécurité |
| **2 — UX** | +2-3 mois | Timeline, preuves, pages acteur/événement/source, filtres, search |
| **3 — Intelligence** | +3-6 mois | Entity resolution, correlation, semantic search, clustering |
| **4 — Live** | +6-12 mois | Event streams, SSE/WebSocket, live map, alerts, replay slider |
| **5 — SaaS** | +12-18 mois | Orgs, billing, RBAC, SSO, audit logs, collaboration |
| **6 — Enterprise** | +18-24 mois | Self-hosted, enterprise auth, private networking |

---

## 25. Métriques de Succès (Phase 1)

| Métrique | Cible | Pourquoi |
|----------|-------|----------|
| p95 latency | <200ms | UX fluide |
| Cache hit rate | >60% | Réduction API calls |
| API quota reduction | >10x | Dedup + cache |
| Uptime | >99% | Confiance utilisateur |
| Time to ingest 100 events | <5 min | Performance ingestion |

**Validation utilisateur :**
- 1 analyste réel utilise la plateforme sur 1 événement
- Tu observes où il bloque (sans parler)
- Tu fixes ces points avant d'ajouter des features

---

## 26. Différenciation Produit

**Pas :** "On a le plus de données."

**Mais :** "On rend l'information géopolitique complexe plus facile à investiguer."

**Différenciateurs :**
- Spécialisation géopolitique
- Architecture event-first
- Timeline-centric
- Transparence des preuves
- Source provenance
- Narrative comparison
- Historical replay
- Open-source core
- Self-hosting possible

---

## 27. Business Model

**Open Source :**
- Core backend
- Data model
- Plugin SDK
- Connecteurs de base
- UI de base
- Self-hosting

**Premium SaaS :**
- Infrastructure managée
- Sources premium
- Ingestion à grande échelle
- Monitoring continu
- Alerts
- Advanced correlation
- Semantic search
- Collaboration
- Backups
- Intégrations enterprise

**Principe :** Logiciel open + infrastructure/data/scale payants.

---

## 28. Utilisateurs Cibles

- Researchers OSINT
- Journalistes d'investigation
- Analystes géopolitiques
- Researchers conflits
- Analystes sécurité
- Universités

**Validation :**
- Pas : "Tu aimes mon produit ?"
- Mais : "Investigue cet événement et dis-moi où tu bloques"

---

## 29. Core Product Philosophy

La plateforme doit être :
- Puissante en dessous
- Simple en surface
- Evidence-driven
- Transparente
- Source-aware
- Timeline-oriented
- Géographiquement aware
- Relationship-aware
- Contrôlée par l'humain

**Peut contenir des millions de records. L'utilisateur doit comprendre ce qu'il voit.**

---

## 30. Final Principle

> **Build a microscope, not an oracle.**

La plateforme aide les humains à investiguer sans prétendre juger à leur place.

**Ambition :** Grande.
**Implémentation :** Incrémentale.

```
Build → Test → Measure → Observe → Improve → Repeat
```

---

## Appendix A : Ce qui est Phase 1 vs Future

| Feature | Phase | Priorité |
|---------|-------|----------|
| Auth + isolation | 1 | 🔴 Critique |
| Cache + rate limit | 1 | 🔴 Critique |
| Request dedup | 1 | 🔴 Critique |
| Timeline basique | 2 | 🟠 High |
| Evidence panel | 2 | 🟠 High |
| Entity resolution | 3 | 🟡 Medium |
| Event correlation | 3 | 🟡 Medium |
| Live SSE/WebSocket | 4 | 🟡 Medium |
| SaaS billing | 5 | ⚪ Low (now) |
| Enterprise SSO | 6 | ⚪ Low (now) |

**Ne pas construire Phases 3-6 maintenant.** Valider Phase 1 d'abord.

---

## Appendix B : Documents Requis

| Doc | Statut | Priorité |
|-----|--------|----------|
| `PLUGIN_API.md` | ❌ À écrire | 🔴 High |
| `DATA_MODEL.md` | ❌ À écrire | 🔴 High |
| `SECURITY.md` | ⚠️ Partiel | 🔴 High |
| `DEPLOYMENT.md` | ⚠️ Partiel | 🟠 Medium |
| `PHASE1_METRICS.md` | ❌ À écrire | 🟠 Medium |

---

*Version : 1.0 — Août 2026*
*Basé sur vision document 50 sections — consolidé en 30 sections*
