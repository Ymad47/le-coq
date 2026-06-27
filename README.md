# La Coq 🐓

**Software souverain de data integration & analytics**

Alternative open source à Palantir — écrite en Zig, pour la France.

---

## 🎯 Vision

Quatre plateformes unifiées :

| Produit | Cible | Statut |
|---------|-------|--------|
| **Apollo** | CI/CD multi-env | 🚧 En dev |
| **Foundry** | Intégration données entreprises | 📋 Planifié |
| **Gotham** | Renseignement, défense | 📋 Planifié |
| **AIP** | IA générative + données | 📋 Planifié |

---

## 🦀 Pourquoi Zig ?

- **Memory safety** sans garbage collector
- **Perf C** avec modern syntax
- **Binaire statique** — déploiement simple
- **Open source** — code auditable par l'État
- **Souveraineté** — pas de dépendance US

---

## 📁 Structure

```
la-coq/
├── apollo/       # CI/CD, déploiements, rollbacks
├── foundry/      # Data integration, ETL, analytics
├── gotham/       # Intel, defense, link analysis
├── aip/          # AI Platform, LLM integration
├── core/         # Shared libs (crypto, http, db)
├── docs/         # Documentation, specs
└── scripts/      # Build, deploy, tests
```

---

## 🚀 Quick Start

```bash
# Build Apollo
cd apollo
zig build

# Run
./zig-out/bin/apollo
```

---

## 📋 Roadmap

| Phase | Produit | Timeline |
|-------|---------|----------|
| 1 | Apollo MVP | Q3 2026 |
| 2 | Foundry Core | Q1 2027 |
| 3 | Gotham Lite | Q3 2027 |
| 4 | AIP Integration | Q1 2028 |

---

## 🔐 Security

- Code 100% auditable
- Cryptographie nationale (ANSSI)
- Certifications visées : SecNumCloud, HDS

---

**Fabriqué en France 🇫🇷**
