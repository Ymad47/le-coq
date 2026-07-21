# Fact-Checking Engine

**Vérification automatique de la véracité des données OSINT** — Critique pour la crédibilité de Le Coq.

> Mieux vaut moins d'intel, mais de l'intel **fiable**.

---

## Pourquoi

| Risque sans fact-check | Impact |
|------------------------|--------|
| Fake news relayée | Perte de confiance immédiate |
| Source non vérifiée | Intel pourrie = décisions pourries |
| Data obsolète | Analyse fausse |
| Manipulation détectée | Réputation détruite |

---

## Architecture

```
Foundry (data ingestion)
    ↓
FactCheck Engine (scoring)
    ↓
    ├─ ✅ Verified (85+) → Gotham (intel)
    ├─ ⚠️ Unverified (50-84) → Queue for review
    └─ ❌ Fake (<50) → Rejected + log
```

---

## Checks Implémentés

### 1. Cross-Reference

**Objectif :** Vérifier que l'info est corroborée par 3+ sources indépendantes.

```python
def cross_reference_check(claim: str, sources: list) -> dict:
    """
    Score: 0-100
    - 3+ sources indépendantes = 100
    - 2 sources = 60
    - 1 source = 20
    - 0 source = 0
    """
    independent_count = count_independent_sources(sources)
    
    if independent_count >= 3:
        return {"score": 100, "status": "verified"}
    elif independent_count == 2:
        return {"score": 60, "status": "unverified"}
    elif independent_count == 1:
        return {"score": 20, "status": "unverified"}
    else:
        return {"score": 0, "status": "rejected"}
```

---

### 2. Timestamp Check

**Objectif :** Data fraîche ou obsolète ?

```python
def freshness_check(timestamp: datetime) -> dict:
    """
    Score: 0-100
    - < 24h = 100
    - < 7 jours = 80
    - < 30 jours = 50
    - > 30 jours = 20
    - > 1 an = 0
    """
    age = datetime.now() - timestamp
    
    if age.days == 0:
        return {"score": 100, "status": "fresh"}
    elif age.days < 7:
        return {"score": 80, "status": "recent"}
    elif age.days < 30:
        return {"score": 50, "status": "aging"}
    elif age.days < 365:
        return {"score": 20, "status": "old"}
    else:
        return {"score": 0, "status": "obsolete"}
```

---

### 3. Source Reputation

**Objectif :** Qui publie cette info ?

| Type de source | Score |
|----------------|-------|
| Media indépendant (AFP, Reuters) | 90 |
| Media public (France24, RFI) | 70 |
| Media state-controlled (RT, Xinhua) | 30 |
| Blog/Anonymous | 20 |
| Social media (non vérifié) | 10 |

```python
SOURCE_REPUTATION = {
    "reuters": 90,
    "afp": 90,
    "ap": 90,
    "france24": 70,
    "rfi": 70,
    "rt": 30,
    "sputnik": 30,
    "xinhua": 30,
    "blog": 20,
    "twitter": 10,
    "telegram": 10,
}

def source_reputation_check(source: str) -> dict:
    score = SOURCE_REPUTATION.get(source.lower(), 20)  # Default: low trust
    return {"score": score, "source": source}
```

---

### 4. Geolocation Verification

**Objectif :** Photo/vidéo vraiment de cet endroit ?

**Outils :**
- Reverse image search (Google, TinEye)
- Exif GPS coords
- Landmark matching

```python
def geolocation_check(image_path: str, claimed_location: tuple) -> dict:
    """
    Vérifie coords GPS + landmarks
    Score: 0-100
    """
    # Extract EXIF
    exif_gps = extract_gps_from_exif(image_path)
    
    if exif_gps:
        distance = haversine(exif_gps, claimed_location)
        if distance < 1:  # < 1km
            return {"score": 100, "status": "verified"}
        elif distance < 10:
            return {"score": 60, "status": "approximate"}
        else:
            return {"score": 0, "status": "mismatch"}
    
    # Fallback: reverse image search
    reverse_results = reverse_image_search(image_path)
    if reverse_results.match_location(claimed_location):
        return {"score": 80, "status": "likely"}
    
    return {"score": 50, "status": "unknown"}
```

---

### 5. Conflict Detection

**Objectif :** Deux sources disent le contraire → flag pour review humaine.

```python
def conflict_detection(claims: list) -> dict:
    """
    Détecte contradictions entre claims
    """
    if len(claims) < 2:
        return {"conflict": False}
    
    # NLP similarity + contradiction detection
    for i, claim1 in enumerate(claims):
        for claim2 in claims[i+1:]:
            if are_contradictory(claim1, claim2):
                return {
                    "conflict": True,
                    "claims": [claim1, claim2],
                    "status": "review_required"
                }
    
    return {"conflict": False, "status": "consistent"}
```

---

### 6. Bot Detection

**Objectif :** Tweet viral = real humans ou bot farm ?

**Signaux :**
- Account age < 30 jours
- > 100 posts/jour
- 0 followers, 0 following
- Copy-paste content

```python
def bot_detection(account: dict) -> dict:
    score = 100
    
    if account["age_days"] < 30:
        score -= 30
    if account["posts_per_day"] > 100:
        score -= 30
    if account["followers"] == 0 and account["following"] == 0:
        score -= 20
    if account["content_similarity"] > 0.9:
        score -= 20
    
    if score < 50:
        return {"score": score, "status": "likely_bot"}
    else:
        return {"score": score, "status": "likely_human"}
```

---

## Scoring Global

```python
def calculate_final_score(checks: dict) -> dict:
    """
    Poids des checks:
    - Cross-reference: 30%
    - Freshness: 15%
    - Source reputation: 25%
    - Geolocation: 15%
    - Conflict: 10%
    - Bot detection: 5%
    """
    weights = {
        "cross_reference": 0.30,
        "freshness": 0.15,
        "source_reputation": 0.25,
        "geolocation": 0.15,
        "conflict": 0.10,
        "bot_detection": 0.05,
    }
    
    final_score = sum(checks[k]["score"] * weights[k] for k in weights)
    
    if final_score >= 85:
        status = "verified"
    elif final_score >= 50:
        status = "unverified"
    else:
        status = "rejected"
    
    return {
        "score": round(final_score, 2),
        "status": status,
        "details": checks
    }
```

---

## Intégration Foundry

```python
# plugins/foundry/ingest.py

from factcheck.engine import FactCheckEngine

def ingest_data(source: str, data: dict) -> dict:
    # Step 1: Parse data
    claim = extract_claim(data)
    
    # Step 2: Run fact-check
    engine = FactCheckEngine()
    checks = engine.run_all(claim, source, data)
    result = engine.calculate_final_score(checks)
    
    # Step 3: Route based on score
    if result["status"] == "verified":
        store_in_gothem(data, result)
    elif result["status"] == "unverified":
        queue_for_human_review(data, result)
    else:  # rejected
        log_rejected(data, result)
        return {"status": "rejected", "reason": "low_confidence"}
    
    return {"status": "ingested", "score": result["score"]}
```

---

## APIs Externes

| Service | Usage | Coût |
|---------|-------|------|
| Google Fact Check API | Claim verification | Gratuit |
| InVID | Video verification | Gratuit |
| TinEye | Reverse image | Freemium |
| Google Vision | Landmark detection | Pay-per-use |
| Botometer | Bot detection | Gratuit |

---

## Roadmap

| Phase | Features | Priority |
|-------|----------|----------|
| **MVP** | Cross-reference + Timestamp + Source reputation | 🔴 High |
| **V2** | Geolocation + Image verification | 🟡 Medium |
| **V3** | Bot detection + Network analysis | 🟢 Low |

---

## Exemple de Sortie

```json
{
  "claim": "Russian troops withdrew from Kherson",
  "timestamp": "2026-07-11T10:30:00Z",
  "sources": ["reuters", "afp", "france24"],
  "checks": {
    "cross_reference": {"score": 100, "status": "verified"},
    "freshness": {"score": 100, "status": "fresh"},
    "source_reputation": {"score": 90, "status": "trusted"},
    "geolocation": {"score": 80, "status": "likely"},
    "conflict": {"conflict": false, "status": "consistent"},
    "bot_detection": {"score": 95, "status": "likely_human"}
  },
  "final_score": 94.5,
  "status": "verified"
}
```

---

## Notes

- **Seuil de confiance :** 85+ pour auto-verify
- **Review humaine :** Obligatoire pour 50-84
- **Rejet automatique :** <50
- **Log :** Tous les rejets sont loggés pour audit

---

**Crédibilité > Quantité.**

Le Coq ne relaie que de l'intel vérifiée. Point.
