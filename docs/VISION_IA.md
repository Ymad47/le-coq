# Le Coq — Vision IA Autonome

**Document de vision technique — Multi-agents IA embarqués**

*Auteur: Y'mIT*  
*Date: 10 August 2026*  
*Statut: Vision en raffinement*

---

## Mission

> **"Un système qui s'auto-gère, s'auto-répare, et traite les données massives sans intervention humaine."**

Le Coq n'est pas juste une plateforme OSINT. C'est un **système autonome intelligent** qui intègre des agents IA spécialisés pour:
- Surveiller sa propre santé
- Se maintenir et se réparer automatiquement
- Traiter des masses de données en background
- Offrir des analystes IA dédiés aux clients premium

**Mantra:** *Simple mais puissant.*

---

## Architecture Multi-Agents

```
+-----------------------------------------------------------+
|              SUPERVISOR AGENT (Global)                    |
|  - Monitoring santé système (CPU, RAM, Disk, Network)     |
|  - Détection anomalies temps réel                         |
|  - Coordination des sous-agents                           |
|  - Alertes proactives avant incident                      |
+---------------------------+-------------------------------+
                            |
        +-------------------+-------------------+
        |                   |                   |
        v                   v                   v
+---------------+   +---------------+   +---------------+
| MAINTENANCE   |   |   DATA        |   |   PREMIUM     |
| AGENT         |   |   AGENT       |   |   AGENTS      |
|               |   |               |   |   (Par client)|
| - Reçoit      |   | - Traitement  |   | - Analyse     |
|   rapports    |   |   masse       |   |   personnalisée|
| - Diagnostique|   | - Filtrage    |   | - Insights    |
| - Auto-fix    |   |   pertinence  |   |   dédiés      |
| - Rollback si |   | - Génération  |   | - Scalabilité |
|   échec       |   |   insights    |   |   infinie     |
+-------+-------+   +-------+-------+   +-------+-------+
        |                   |
        +---------+---------+
                  |
                  v
        +-------------------+
        |   CLI DEBUG       |
        |   (Y'mIT Tool)    |
        |                   |
        | - Override agents |
        | - Debug manuel    |
        | - Force fix       |
        | - Audit logs      |
        +-------------------+
```

---

## Détail des Agents

### 1. SUPERVISOR AGENT (Global)

**Rôle:** Gardien global du système

**Tâches:**
- Health checks toutes les 5 secondes
- Métriques: CPU, RAM, Disk, Network
- Détection de patterns anormaux
- Coordination des autres agents
- Alertes proactives avant incident

**Implémentation (vision):**
- Modèle lightweight embarqué (TinyLLM ou fine-tuned spécialisé)
- Pas d'API cloud — tout tourne local
- Règles simples en premier (thresholds), IA pour anomalies complexes

**Exemple de détection:**
```
CPU > 90% pendant 5min → Alerte Niveau 1
CPU > 95% + pattern anormal → Alerte Niveau 2 + Maintenance Agent notifié
Process down → Redémarrage auto immédiat
```

---

### 2. MAINTENANCE AGENT

**Rôle:** Mécanicien autonome

**Tâches:**
- Reçoit les alertes du Supervisor
- Analyse les logs pour identifier la root cause
- Applique patch / redémarrage / rollback
- Notifie si échec

**Sécurité:**
- Sandboxée (container éphémère)
- Rollback automatique si fix échoue
- Jamais de changement breaking sans validation humaine

**Implémentation (vision):**
- Fine-tuned sur dataset: erreurs Rust, Docker, nginx, DB
- Inférence locale (llama.cpp / ollama)
- Actions limitées et réversibles

**Exemple de fix:**
```
Erreur: nginx 502 Bad Gateway
Diagnostic: Backend Rust down
Action: systemctl restart le-coq-backend
Vérification: Health check endpoint
Rollback: Si health check échoue → restore previous config
```

---

### 3. DATA AGENT

**Rôle:** Analyste de données massives

**Tâches:**
- Traitement en background des flux OSINT, deep web, dark web
- Filtrage du bruit
- Extraction d'entités
- Corrélation entre sources
- Génération d'insights
- Mise à jour du graphe de connaissances

**Scalabilité:**
- Batch processing
- Queue-based (Redis / RabbitMQ)
- Parallélisable à l'infini (un agent par source/type)

**Implémentation (vision):**
- Modèle spécialisé NLP (BERT fine-tuned ou lightweight LLM)
- Embeddings locaux (sentence-transformers all-MiniLM-L6-v2 = 80MB)
- Filtrage en amont (règles simples avant IA)

**Exemple de pipeline:**
```
1. Collecte: 10,000 articles news (clear web)
2. Filtrage règles: Mots-clés géopolitiques → 2,000 articles
3. Embedding: all-MiniLM-L6-v2 → vecteurs 384 dimensions
4. Clustering: Regroupement par thème
5. Extraction: Entités (pays, personnes, orgs)
6. Corrélation: Lien avec événements passés
7. Insight: "Tension diplomatique X-Y en hausse"
8. Graphe: Mise à jour noeuds + relations
```

---

### 4. PREMIUM AGENTS (Future)

**Rôle:** Analystes dédiés par client premium

**Tâches:**
- Requêtes en langage naturel
- Rapports personnalisés
- Alertes thématiques
- Veille sectorielle

**Business:**
- Justifie pricing €999/mo
- Équivalent à une équipe d'analystes pour 1% du coût
- Scalabilité: un processus par client

**Implémentation (vision):**
- Contexte isolé par client (stocké en DB, pas en RAM partagée)
- Modèle fine-tuned sur domaine du client (défense, journalisme, finance...)
- Rate limiting par client

---

## Philosophie Technique

### Mantra

> **"Un truc simple mais puissant."**

### Principes

1. **Zéro dépendance cloud**
   - Tout tourne local
   - Pas d'API cloud (coûts, latence, vendor lock-in)
   - Données restent sur l'infrastructure client (GDPR compliant)

2. **Pas de prompts = tout en background**
   - Scalabilité maximale
   - Pas de latence utilisateur
   - Coûts compute prévisibles

3. **CLI debug pour override manuel**
   - Human in the loop pour cas critiques
   - Debug post-mortem possible
   - Confiance des enterprise clients

4. **Ressources minimisées**
   - CPU/RAM optimisés
   - Modèles quantifiés (Q4_K_M)
   - Inférence efficient (llama.cpp)

5. **Open-source**
   - Communauté peut améliorer les agents
   - Auditabilité totale
   - Confiance (pas de backdoor, pas de fuite de données)

---

## Options d'Implémentation

### Option 1: Hermes Agent

**Pros:**
- Utiliser l'existant
- Rapide à déployer
- Déjà intégré à l'écosystème

**Cons:**
- Dépendance à Hermes
- Peut être overkill pour certaines tâches
- Moins de contrôle sur l'optimisation

---

### Option 2: Fine-tuning spécialisé

**Pros:**
- Modèle optimisé pour tâche spécifique
- Meilleur ratio perf/ressources
- Contrôle total sur le dataset d'entraînement

**Cons:**
- Effort de fine-tuning initial
- Maintenance des datasets
- Expertise requise

**Modèles candidats:**
- Mistral 7B v0.3 Q4_K_M (4GB RAM, bon perf, français OK)
- Phi-3 mini 3.8B (2GB RAM, Microsoft, très efficient)

---

### Option 3: Modèle lightweight embarqué

**Pros:**
- Contrôle total
- Optimisé pour le hardware cible
- Indépendance totale

**Cons:**
- Effort de développement important
- Maintenance à long terme
- Expertise ML requise

**Modèles candidats:**
- TinyLLM
- Phi-3 mini
- Gemma 2B
- Qwen 1.5B

---

## CLI Debug — Y'mIT Tool

### Commandes

```bash
# Supervision manuelle
le-coq supervisor status
le-coq supervisor logs --agent maintenance
le-coq supervisor metrics --last 24h

# Override agent
le-coq agent maintenance disable
le-coq agent maintenance enable
le-coq agent maintenance force-fix <incident_id>
le-coq agent maintenance rollback <incident_id>

# Audit
le-coq agent audit --last 24h
le-coq agent metrics --all
le-coq agent actions --agent maintenance --status failed

# Emergency
le-coq emergency rollback --to <timestamp>
le-coq emergency safe-mode --enable
le-coq emergency safe-mode --disable
```

### Usage

- Quand les agents échouent
- Pour audit manuel
- Debug post-mortem
- Force fix en cas de blocage
- Rollback d'urgence

**Principe:** Interface simple, commandes courtes, puissance maximale.

---

## Roadmap — Phasage

### Phase 1 (MVP — Month 1-2)

**Objectif:** Supervision basique sans IA

```
Supervisor Agent = Règles simples + seuils
- CPU > 90% pendant 5min → alerte
- Disk > 85% → alerte
- Process down → redémarrage auto

PAS d'IA encore. Juste du code Rust + thresholds.
```

**Livrables:**
- Health checks basiques
- Alertes logging
- Redémarrage auto processes
- CLI debug (commandes de base)

---

### Phase 2 (Month 3-6)

**Objectif:** IA pour Data Agent + détection anomalies

```
Data Agent = Fine-tuned BERT/small LLM
- Mistral 7B Q4_K_M (4GB RAM)
- Fine-tune sur: logs OSINT, entités géopolitiques
- Batch processing nocturne

Supervisor = Détection patterns anormaux
- Même modèle, fine-tune différent
- Entraîne sur: historiques incidents, métriques
```

**Livrables:**
- Pipeline data processing
- Embeddings locaux
- Détection anomalies (IA)
- CLI debug (audit logs)

---

### Phase 3 (Month 6-12)

**Objectif:** Maintenance Agent auto-healing

```
Maintenance Agent = Fine-tuned sur erreurs + fixes
- Dataset: erreurs Rust, Docker, nginx, PostgreSQL
- Sandbox containerisée
- Rollback auto si échec
```

**Livrables:**
- Diagnostic automatique pannes
- Auto-fix (sandboxée)
- Rollback automatique
- CLI debug (force-fix, rollback manuel)

---

### Phase 4 (Year 2)

**Objectif:** Premium Agents multi-tenant

```
Premium Agents = Architecture multi-tenant
- Un processus par client premium
- Contexte isolé en DB
- Rate limiting par client
```

**Livrables:**
- Isolation contextuelle par client
- Requêtes langage naturel
- Rapports personnalisés
- Alertes thématiques

---

### Phase 5 (Year 3+)

**Objectif:** IA lightweight embarquée

```
Modèle propriétaire = Optimisé pour Le Coq
- Entraîné sur données réelles du système
- Quantifié pour hardware cible
- Intégré nativement (pas de dépendance externe)
```

**Livrables:**
- Modèle embarqué Le Coq
- Optimisation hardware
- Indépendance totale

---

## Stack Technique Recommandée

| Composant | Choix | Pourquoi |
|-----------|-------|----------|
| **Modèle de base** | Mistral 7B v0.3 Q4_K_M | 4GB RAM, bon perf, français OK |
| **Alternative tiny** | Phi-3 mini 3.8B | 2GB RAM, Microsoft, très efficient |
| **Embeddings** | all-MiniLM-L6-v2 | 80MB, rapide, bon pour similarité |
| **Inference engine** | llama.cpp / ollama | Local, quantifié, API simple |
| **Queue** | Redis | Léger, rapide, persistant |
| **Sandbox** | Docker (container éphémère) | Isolation, rollback facile |
| **Orchestration** | Systemd + Rust CLI | Simple, robuste, pas de Kubernetes |

---

## Pièges à Éviter

### ❌ Trop d'IA trop tôt

**Risque:** Complexité inutile, bugs, coûts compute

**Solution:** Commencer avec des règles simples, ajouter IA progressivement

---

### ❌ Un seul modèle pour tout

**Risque:** Modèle jack-of-all-trades, master of none

**Solution:** Spécialiser par tâche (Supervisor ≠ Maintenance ≠ Data)

---

### ❌ Pas de rollback

**Risque:** Agent casse quelque chose, pas de retour arrière

**Solution:** Toujours un plan B, rollback automatique si health check échoue

---

### ❌ Entraîner sur données synthétiques

**Risque:** Modèle performe mal en production

**Solution:** Utiliser de vrais logs, vraies erreurs, vraies données

---

### ❌ Oublier le monitoring des agents

**Risque:** Qui surveille le surveillant?

**Solution:** Supervisor se monitor lui-même + alertes externes (email, webhook)

---

### ❌ IA prend des actions irréversibles

**Risque:** Agent supprime des données, change config critique

**Solution:** Actions limitées et réversibles, validation humaine pour breaking changes

---

## Critères de Succès

### Technique

- [ ] Zéro dépendance cloud (tout local)
- [ ] Inférence < 100ms pour anomalies simples
- [ ] Batch processing 10K documents / heure
- [ ] Rollback automatique < 30 secondes
- [ ] CLI debug fonctionnel pour tous les agents

### Business

- [ ] Justifie pricing Premium €999/mo
- [ ] Réduit coûts support de 50%
- [ ] Uptime 99.9% garanti (SLA enterprise)
- [ ] Différenciation claire vs Palantir

### Philosophie

- [ ] Simple mais puissant
- [ ] Souverain (UE, AGPL, local)
- [ ] Open-source (communauté peut améliorer)
- [ ] Contrôle humain préservé (CLI debug)

---

## Vision Long Terme

```
Le Coq = Palantir + Autonomous AI + Souveraineté UE

Construire un business durable à €1M+/an.

Pas de one-time exit. Pas de pression VC. Pas de vente.

Souverain. Open. Profitable. Indépendant.

Aider journalistes, chercheurs, entreprises, gouvernements à voir la vérité.

Rester Européen. Rester AGPL. Rester libre.

Devenir riche via cash-flow, pas exit.

INTÉGRER IA AUTONOME:
- Multi-agents spécialisés (Supervisor, Maintenance, Data)
- 100% background, pas de prompts, scalabilité infinie
- Zéro dépendance cloud (modèles locaux, fine-tuned)
- CLI debug pour contrôle humain si besoin
- Simple mais puissant
```

---

## Références

- [BUSINESS_PLAN.md](./BUSINESS_PLAN.md) — Modèle économique, projections
- [PHASE1_ARCHITECTURE.md](./PHASE1_ARCHITECTURE.md) — Architecture technique de base
- [VISION.md](./VISION.md) — Vision globale du projet

---

*Document vivant — à revisiter trimestriellement pour ajuster la vision aux évolutions technologiques.*

**Fabriqué en France**  
**AGPL-3.0**  
**IA Locale — Zéro dépendance cloud**
