# Le Coq — Vision Document

**Geopolitical Intelligence Platform — Open-source alternative to Palantir**

> Fabriqué en France 🇫🇷  
> Souverain. Transparent. Puissant. Complet.

---

## Mission

> "People see the truth information"

Build the world's most complete open-source intelligence platform — combining **all data sources** (clear, deep, dark web) with **sovereign European infrastructure** and **deep knowledge graph analysis**.

---

## Core Philosophy

| Principle | Stance |
|-----------|--------|
| **Data Sources** | ✅ All sources worldwide — US, EU, CN, RU, whatever works |
| **Platform Control** | ✅ Sovereign — EU hosting, AGPL, user control |
| **Infrastructure** | ✅ Independent — Hetzner/OVH/Scaleway, not AWS/Vercel |
| **License** | ✅ AGPL — can never be acquired and closed |
| **Dark Web** | ✅ Read-only integration — Tor, .onion, markets, forums |

> **"Take the best data. Keep sovereign control."**

---

## Complete Intelligence Layers

```
┌─────────────────────────────────────────────────────────┐
│              LE COQ — FULL INTELLIGENCE                 │
├─────────────────────────────────────────────────────────┤
│  1. CLEAR WEB                                           │
│  • News, corporate, gov, academic                       │
│  • Social media, public databases                       │
│  • Live feeds (flights, ships, CCTV, seismic)           │
├─────────────────────────────────────────────────────────┤
│  2. DEEP WEB                                            │
│  • Academic journals (paywalled)                        │
│  • Corporate registries, court records                  │
│  • Satellite imagery, maritime data                     │
│  • Professional databases                               │
├─────────────────────────────────────────────────────────┤
│  3. DARK WEB                                            │
│  • Tor .onion sites (forums, leaks, markets)            │
│  • Darknet paste monitors (creds, docs)                 │
│  • Encrypted channels (Telegram, IRC)                   │
│  • Marketplace listings (read-only)                     │
├─────────────────────────────────────────────────────────┤
│  4. KNOWLEDGE GRAPH                                     │
│  • Entity relationships (Neo4j)                         │
│  • Network analysis (centrality, communities)           │
│  • Historical pattern detection                         │
│  • Inference engine (hidden connections)                │
├─────────────────────────────────────────────────────────┤
│  5. SOVEREIGN INFRA                                     │
│  • EU hosting (Hetzner, OVH, Scaleway)                  │
│  • AGPL license (stays open forever)                    │
│  • Offline mode (Zig CLI + local cache)                 │
│  • Full user control                                    │
└─────────────────────────────────────────────────────────┘
```

---

## Data Sources (Worldwide — Best Available)

### Clear Web

| Domain | Source | Country | Status |
|--------|--------|---------|--------|
| **Flights** | OpenSky Network | 🇨🇭 Switzerland | ✅ |
| **Maritime** | AIS streams | 🇮🇳 India | ✅ |
| **Earthquakes** | USGS | 🇺🇸 USA | ✅ |
| **Fires** | NASA FIRMS | 🇺🇸 USA | ✅ |
| **Weather** | NOAA + EUMETSAT | 🇺🇸 + 🇪🇺 | ✅ |
| **Satellites** | N2YO + ESA | 🇺🇸 + 🇪🇺 | ✅ |
| **Conflict** | ACLED | 🇬🇧 UK | ✅ |
| **Corporate** | OpenCorporates | 🇬🇧 UK | ✅ |
| **News** | GDelt + RSS feeds | 🇺🇸 + Worldwide | ✅ |
| **Sanctions** | OFAC + EU Consolidated | 🇺🇸 + 🇪🇺 | ✅ |

### Deep Web

| Domain | Source | Access |
|--------|--------|--------|
| **Academic** | JSTOR, ScienceDirect | API/Subscription |
| **Court Records** | PACER, EU e-Justice | API |
| **Corporate** | Orbis, Bureau van Dijk | API |
| **Satellite** | Maxar, Planet Labs | API |

### Dark Web

| Domain | Source | Method | Risk |
|--------|--------|--------|------|
| **.onion forums** | Leak sites, discussion boards | Tor scraper | Medium |
| **Markets** | Marketplace listings | Read-only scraper | Medium |
| **Paste sites** | Darknet pastebins | Monitor | Low |
| **Telegram** | Darknet channels | Bot/scraper | Low |
| **IRC** | Darknet IRC networks | Bot | Low |

---

## Technical Architecture

### Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| **Backend** | Rust (axum) | High-performance API, graph engine |
| **Plugins** | Python | OSINT data sources, NLP, ML, Tor integration |
| **CLI** | Zig | Static binary, offline mode |
| **Frontend** | Svelte + MapLibre GL | Graph/map/timeline visualization |
| **Database** | PostgreSQL + PostGIS | Entities, geospatial |
| **Graph** | Neo4j | Relationships, network analysis |
| **Search** | Meilisearch | Full-text search |
| **Cache** | Redis | Session, rate limiting |
| **Tor** | stem + socks5 | Dark web integration |

### Dark Web Module (Python)

```python
# plugins/sources/darkweb/onion_scraper.py
import requests
import stem
from stem import Signal
from stem.control import Controller

def rotate_identity():
    """Rotate Tor circuit for anonymity"""
    with Controller.from_port(port=9051) as controller:
        controller.authenticate()
        controller.signal(Signal.NEWNYM)

def fetch_onion(url: str) -> dict:
    """Fetch .onion page via Tor"""
    proxies = {
        'http': 'socks5h://127.0.0.1:9050',
        'https': 'socks5h://127.0.0.1:9050'
    }
    response = requests.get(url, proxies=proxies, timeout=30)
    rotate_identity()
    return {
        'url': url,
        'status': response.status_code,
        'content': response.text[:10000],  # Limit size
        'timestamp': datetime.utcnow().isoformat()
    }

def monitor_marketplace(base_url: str) -> list:
    """Monitor darknet marketplace listings (read-only)"""
    listings = []
    # Implementation: parse HTML, extract product info
    # NO interaction, NO login, NO purchase
    return listings
```

### Security Boundaries (Dark Web)

| Do ✅ | Don't ❌ |
|-------|---------|
| Read-only scraping | Never post/interact |
| Tor isolation (VM/container) | Never run from main machine |
| No credentials stored | Never login to darknet services |
| Legal sources only | No illegal content storage |
| Automated circuit rotation | No persistent connections |
| Log access for audit | No anonymization of logs |

---

## Feature Roadmap

### Phase 1 (MVP) — Clear Web Foundation

- [ ] Entity search (PostgreSQL + Meilisearch)
- [ ] Basic graph visualization (Neo4j)
- [ ] 3 OSINT plugins (OpenCorporates, ACLED, GDELT)
- [ ] Export to JSON/PDF
- [ ] API endpoints (REST)

### Phase 2 — Live Intelligence (Osiris-inspired)

- [ ] Map view (MapLibre GL + PostGIS)
- [ ] Timeline view (temporal analysis)
- [ ] Live feeds (flights, seismic, fires, news)
- [ ] Network analysis (centrality, communities)
- [ ] Telegram OSINT (geoparsed channels)

### Phase 3 — Deep Web Integration

- [ ] Academic journal connectors
- [ ] Court records integration
- [ ] Corporate registry APIs
- [ ] Satellite imagery pipeline
- [ ] Maritime tracking (AIS)

### Phase 4 — Dark Web Module

- [ ] Tor integration (stem library)
- [ ] .onion scraper (forums, leaks)
- [ ] Darknet paste monitor
- [ ] Marketplace listing tracker (read-only)
- [ ] IRC network monitor
- [ ] Security isolation (container/VM)

### Phase 5 — Sovereign Features

- [ ] Offline mode (Zig CLI + local cache)
- [ ] Multi-user collaboration
- [ ] Plugin marketplace
- [ ] Desktop app (Tauri)
- [ ] EU hosting deployment scripts
- [ ] GDPR compliance tools

---

## Competitive Positioning

| Feature | Palantir | Osiris | Le Coq |
|---------|----------|--------|--------|
| **Data Sources** | ✅ All | ✅ All | ✅ All |
| **Clear Web** | ✅ | ✅ | ✅ |
| **Deep Web** | ✅ | ❌ | ✅ |
| **Dark Web** | ✅ (gov) | ❌ | ✅ |
| **Graph Engine** | ✅ | ❌ | ✅ |
| **Live Feeds** | ✅ | ✅ | ✅ |
| **US Control** | ❌ Yes | ❌ Yes | ✅ No |
| **EU Hosting** | ❌ | ❌ | ✅ |
| **Offline Mode** | ❌ | ❌ | ✅ |
| **Open Source** | ❌ | ✅ MIT | ✅ AGPL |
| **Sovereign** | ❌ | ❌ | ✅ |

### Unique Value Proposition

> **Le Coq = Best data (worldwide) + Sovereign control (EU) + Complete coverage (clear/deep/dark) + Knowledge graph**

---

## License Strategy

**AGPL-3.0** — Strong copyleft

| Why AGPL | Benefit |
|----------|---------|
| Network use = distribution | SaaS must open-source modifications |
| Can absorb MIT code | Osiris (MIT) → Le Coq (AGPL) compatible |
| Can't be closed by acquirer | Stays open forever |
| European-friendly | Aligns with EU open-source values |

---

## Infrastructure

### Hosting (EU Only)

| Provider | Location | Use |
|----------|----------|-----|
| **Hetzner** | 🇩🇪 Germany | Primary (VPS, GPU) |
| **OVH** | 🇫🇷 France | Backup, redundancy |
| **Scaleway** | 🇫🇷 France | Development |

### Deployment

```bash
# Production (Hetzner)
docker compose -f docker/docker-compose.prod.yml up -d

# Development (local)
docker compose -f docker/docker-compose.yml up -d

# Offline mode (Zig CLI)
./le-coq-cli sync    # Download latest data
./le-coq-cli search  # Query offline
```

---

## Legal & Ethics

### Dark Web Guidelines

1. **Read-only access** — No interaction, no posting
2. **No illegal content storage** — Metadata only, content filtered
3. **Tor isolation** — Run in dedicated VM/container
4. **Audit logging** — All access logged for compliance
5. **Legal review** — Consult EU law on darknet scraping

### Data Compliance

- **GDPR** — EU data protection compliance
- **Terms of Service** — Respect source
- **Rate limiting** — Don't overload sources
- **Attribution** — Credit data sources where required

---

## The Vision

> Build the world's most complete open-source intelligence platform.
>
> Not American. Not Chinese. **European. Sovereign. Independent.**
>
> All data sources. All layers. Full control.
>
> For journalists. Investigators. Researchers. Citizens.
>
> **So people can see the truth.**

---

## Next Steps

1. ✅ Fork Osiris code (MIT = free to use)
2. ✅ Adapt data source integrations (Python plugins)
3. ✅ Build knowledge graph (Neo4j + Rust)
4. ✅ Add MapLibre GL frontend (GPU rendering)
5. ✅ Implement Tor integration (dark web module)
6. ✅ Deploy on Hetzner (EU hosting)
7. ✅ Release AGPL (stay open forever)

---

**Built with** 🦀 **Rust**, 🐍 **Python**, ⚡ **Zig**, 🎯 **Svelte**, 🧅 **Tor**

**Fabriqué en France** 🇫🇷

**AGPL-3.0** — Stays open forever

---

*Last updated: 2026-07-04*  
*Author: Ymad (yaya47)*  
*Contact: ymad@ymad.fr*
