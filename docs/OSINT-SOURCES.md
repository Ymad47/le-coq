# OSINT Data Sources for Geopolitical Intelligence

Curated list of open-source intelligence datasources for Le Coq plugins.

---

## Satellite & Imagery

| Source | Data | API | Cost | Notes |
|--------|------|-----|------|-------|
| **Sentinel Hub** | Satellite imagery (Sentinel-1/2/3) | REST API | Free tier | 10m resolution, 5-day revisit |
| **NASA FIRMS** | Active fires, thermal anomalies | API / FTP | Free | Real-time fire detection |
| **USGS EarthExplorer** | Landsat, MODIS, aerial photos | Web portal | Free | Historical archive (1970s+) |
| **Maxar OpenData** | High-res satellite (disaster response) | Web portal | Free (select events) | 30cm-50cm resolution |
| **Google Earth Engine** | Multi-source satellite + analysis | Python API | Free (research) | Massive compute platform |
| **EOSDA LandViewer** | Satellite imagery search | Web / API | Freemium | Easy search, download |

**Use cases:** Infrastructure changes, crop monitoring, disaster assessment, military activity, environmental tracking

---

## Shipping & Maritime

| Source | Data | API | Cost | Notes |
|--------|------|-----|------|-------|
| **MarineTraffic** | AIS ship positions, port arrivals | REST API | Freemium | 30-min delay free, real-time paid |
| **VesselFinder** | Ship tracking, fleet data | REST API | Freemium | Similar to MarineTraffic |
| **OpenSeaMap** | Nautical charts, maritime data | Web / OSM | Free | Community-maintained |
| **Global Fishing Watch** | Fishing vessel activity | Web / API | Free | Focus on fishing industry |
| **IMB Piracy Report** | Piracy incidents, maritime security | PDF reports | Free | ICC Commercial Crime Services |

**Use cases:** Trade flow tracking, sanctions evasion, naval movements, port activity, illegal fishing

---

## Corporate & Financial

| Source | Data | API | Cost | Notes |
|--------|------|-----|------|-------|
| **OpenCorporates** | Company registries (140M+ entities) | REST API | Freemium | Largest open database |
| **ICIJ Offshore Leaks** | Offshore companies, beneficial owners | Search / DB dump | Free | Panama Papers, Pandora Papers |
| **OpenSanctions** | PEPs, sanctions, watchlists | API / DB dump | Free | Aggregated from 50+ sources |
| **World Bank Open Data** | Economic indicators, development data | REST API | Free | GDP, poverty, trade stats |
| **IMF Data** | Macroeconomic data, debt, reserves | REST API | Free | International financial stats |
| **UN Comtrade** | International trade statistics | REST API | Free | Import/export by country/product |
| **OpenOwnership Register** | Beneficial ownership data | API | Free | Corporate transparency |
| **SEC EDGAR** | US public company filings | Web / API | Free | 10-K, 10-Q, insider trades |

**Use cases:** Corporate networks, sanctions screening, trade analysis, beneficial ownership, corruption tracking

---

## Conflict & Security

| Source | Data | API | Cost | Notes |
|--------|------|-----|------|-------|
| **ACLED** | Conflict events (protests, battles, violence) | REST API | Free (research) | Global coverage, daily updates |
| **GDELT** | Global news events, sentiment | BigQuery / API | Free | Massive scale, 100+ languages |
| **Crisis Group** | Conflict analysis, reports | Web / RSS | Free | Expert analysis |
| **ISS Africa** | African security research | Web / RSS | Free | Focus on African conflicts |
| **SIPRI** | Arms transfers, military spending | Database | Free | Stockholm International Peace Research Institute |
| **Oryx** | Military equipment losses (visual confirmation) | Web / Blog | Free | Ukraine war tracking |
| **Conflict Intelligence Team** | Casualty tracking, conflict zones | Web | Free | Russia/Ukraine focus |

**Use cases:** Conflict monitoring, protest tracking, arms flow analysis, casualty estimation, early warning

---

## Social Media & News

| Source | Data | API | Cost | Notes |
|--------|------|-----|------|-------|
| **X/Twitter API v2** | Tweets, users, trends | REST API | Freemium | $100/mo basic, limited free |
| **Telegram** | Channels, groups (public) | MTProto API | Free | Requires scraping libs |
| **Facebook Graph API** | Pages, posts (limited) | REST API | Restricted | Heavily limited post-2018 |
| **Reddit API** | Posts, comments, communities | REST API | Free | Rate limited |
| **NewsAPI** | News articles from 80k+ sources | REST API | Freemium | $50/mo developer |
| **GDELT News** | Global news monitoring | BigQuery | Free | 100+ languages, sentiment |
| **Wayback Machine** | Archived web pages | API | Free | Historical web content |
| **CrowdTangle** (deprecated) | Social media analytics | API | Closed | Facebook tool, now shut down |

**Use cases:** Sentiment analysis, event detection, disinformation tracking, protest monitoring, influence operations

---

## Government & Institutional

| Source | Data | API | Cost | Notes |
|--------|------|-----|------|-------|
| **US Gov Data** | data.gov (federal datasets) | Web / API | Free | 300k+ datasets |
| **EU Open Data** | European Union data portal | Web / API | Free | EU institutions data |
| **UN Data** | UN statistics (population, trade, etc.) | Web / API | Free | Multiple UN agencies |
| **Transparency Intl** | Corruption indices (CPI) | Web / Downloads | Free | Corruption Perceptions Index |
| **V-Dem Institute** | Democracy indices | Web / Downloads | Free | Varieties of Democracy |
| **Freedom House** | Freedom ratings (political/civil) | Web / Downloads | Free | Freedom in the World report |
| **EITI** | Extractive industries transparency | API | Free | Oil, gas, mining payments |

**Use cases:** Governance tracking, corruption analysis, democracy monitoring, resource governance

---

## Energy & Infrastructure

| Source | Data | API | Cost | Notes |
|--------|------|-----|------|-------|
| **Global Energy Monitor** | Coal, oil, gas, solar plants | API / DB dump | Free | Power plant tracker |
| **IEA Data** | Energy statistics, balances | Web / API | Freemium | International Energy Agency |
| **OPEC** | Oil production, reserves | Web / Reports | Free | Monthly oil market report |
| **US EIA** | Energy data (global) | API | Free | US Energy Information Administration |
| **Open Infrastructure Map** | Power grids, pipelines | Web / OSM | Free | Community-maintained |
| **Energy Charter** | Energy investment, transit | Reports | Free | International energy treaty |

**Use cases:** Energy security, infrastructure mapping, resource tracking, pipeline monitoring

---

## Humanitarian & Demographics

| Source | Data | API | Cost | Notes |
|--------|------|-----|------|-------|
| **UNHCR Data** | Refugee statistics, displacement | API / Downloads | Free | Global refugee data |
| **IOM Displacement** | Internal displacement tracking | Reports / API | Free | International Organization for Migration |
| **WorldPop** | Population density maps | Downloads | Free | 100m resolution grids |
| **HRW** | Human rights reports | Web / RSS | Free | Human Rights Watch |
| **Amnesty Intl** | Human rights documentation | Web / RSS | Free | Amnesty International |

**Use cases:** Migration tracking, displacement monitoring, humanitarian crises, population analysis

---

## Tools & Libraries

### Python
```python
# OSINT frameworks
- ohsome API (OpenStreetMap history)
- spiderfoot (automated OSINT collection)
- maltego (visual link analysis - commercial)
- theHarvester (email/subdomain enumeration)
- recon-ng (web reconnaissance)

# Data processing
- spaCy (NLP, entity extraction)
- transformers (HuggingFace - embeddings, classification)
- geopandas (geospatial analysis)
- networkx (graph analysis)
```

### JavaScript/Node
```javascript
// Scraping
- puppeteer (headless browser)
- cheerio (HTML parsing)
- node-telegram-bot-api (Telegram scraping)

// Visualization
- Cytoscape.js (graph viz)
- D3.js (custom charts)
- Leaflet / MapLibre (maps)
```

### CLI Tools
```bash
# Data download
- wget, curl (HTTP downloads)
- rsync (file sync)
- aria2c (parallel downloads)

# Processing
- jq (JSON parsing)
- xsv (CSV parsing)
- sqlite3 (local database)
```

---

## Plugin Template

```python
# plugins/example_source.py
from typing import Dict, List, Optional
from datetime import datetime

class DataSourcePlugin:
    """Base class for Le Coq data source plugins"""
    
    name = "example_source"
    version = "1.0.0"
    description = "Example data source plugin"
    
    def __init__(self, config: Dict):
        self.config = config
        self.api_key = config.get("api_key")
    
    def fetch(self, query: Dict) -> List[Dict]:
        """
        Fetch data from source
        
        Args:
            query: Dict with search parameters
            
        Returns:
            List of entity/relationship dicts
        """
        raise NotImplementedError
    
    def validate(self) -> bool:
        """Validate API connection / credentials"""
        raise NotImplementedError
    
    def get_schema(self) -> Dict:
        """Return expected data schema"""
        return {
            "entities": ["Person", "Organization"],
            "relationships": ["works_for", "owns"],
            "update_frequency": "daily"
        }
```

---

## Best Practices

### 1. Respect Terms of Service
- Read API ToS before scraping
- Implement rate limiting
- Cache responses to reduce API calls

### 2. Data Quality
- Score sources by reliability
- Track data freshness
- Flag unverified information
- Cross-reference multiple sources

### 3. Privacy & Ethics
- Don't collect personal data without purpose
- Anonymize where possible
- Consider harm potential
- Follow GDPR/local regulations

### 4. Security
- Store API keys in env vars, not code
- Use HTTPS for all requests
- Validate input data
- Sanitize output (prevent XSS in reports)

### 5. Attribution
- Always cite sources
- Include timestamps
- Link to original data
- Note license restrictions

---

## Quick Start: Add a Source

1. **Choose source** from list above
2. **Create plugin** in `plugins/sources/`
3. **Implement** `fetch()`, `validate()`, `get_schema()`
4. **Test** with sample queries
5. **Document** in `plugins/SOURCES.md`
6. **Submit PR** to main repo

---

**Last updated:** July 2026
**Contributors:** Add your name when submitting plugins
