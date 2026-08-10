"""
GDELT Plugin — news articles from GDELT 2.0 doc API

API: https://api.gdeltproject.org/api/v2/doc/doc
- Returns recent articles matching a query (title, url, domain, sourcecountry)
- Rate limit: 1 req / 5s. Requires User-Agent header to avoid 429.
- This plugin extracts country from `sourcecountry` and article title, links the
  queried entity to the article entity.
"""

import asyncio
import aiohttp
from typing import Dict, List, Any
from plugins import DataSourcePlugin, Entity, register_plugin


class GDELTPlugin(DataSourcePlugin):
    name = "gdelt"
    version = "2.0.0"
    description = "GDELT 2.0 — global news articles in 100+ languages (doc API)"

    BASE_URL = "https://api.gdeltproject.org/api/v2/doc/doc"
    USER_AGENT = "LeCoqOSINT/1.0 (gdelt-fetch; research)"

    # Country name → entity id helper (aligned with uncomtrade naming)
    COUNTRY_TO_ISO = {
        "France": "FRA", "United States": "USA", "United States of America": "USA",
        "China": "CHN", "Germany": "DEU", "United Kingdom": "GBR", "UK": "GBR",
        "Russia": "RUS", "Brazil": "BRA", "India": "IND", "Japan": "JPN",
        "South Africa": "ZAF", "Canada": "CAN", "Mexico": "MEX", "Australia": "AUS",
        "South Korea": "KOR", "Korea": "KOR", "Spain": "ESP", "Italy": "ITA",
        "Belgium": "BEL", "Netherlands": "NLD", "Poland": "POL", "Turkey": "TUR",
        "Saudi Arabia": "SAU", "United Arab Emirates": "ARE", "Qatar": "QAT",
        "Egypt": "EGY", "Morocco": "MAR", "Algeria": "DZA", "Nigeria": "NGA",
        "Kenya": "KEN", "Ghana": "GHA", "Ethiopia": "ETH", "Switzerland": "CHE",
        "Austria": "AUT", "Denmark": "DNK", "Finland": "FIN", "Greece": "GRC",
        "Ireland": "IRL", "Norway": "NOR", "Portugal": "PRT", "Sweden": "SWE",
        "Czech Republic": "CZE", "Hungary": "HUN", "Romania": "ROU", "Bulgaria": "BGR",
        "Croatia": "HRV", "Slovakia": "SVK", "Slovenia": "SVN", "Estonia": "EST",
        "Latvia": "LVA", "Lithuania": "LTU", "Ukraine": "UKR", "Israel": "ISR",
        "Iran": "IRN", "Iraq": "IRQ", "Jordan": "JOR", "Lebanon": "LBN",
        "Syria": "SYR", "Yemen": "YEM", "Oman": "OMN", "Bahrain": "BHR",
        "Kuwait": "KWT", "New Zealand": "NZL", "Argentina": "ARG", "Chile": "CHL",
        "Colombia": "COL", "Peru": "PER", "Venezuela": "VEN", "Pakistan": "PAK",
        "Bangladesh": "BGD", "Philippines": "PHL", "Indonesia": "IDN",
        "Malaysia": "MYS", "Singapore": "SGP", "Thailand": "THA", "Vietnam": "VNM",
        "Taiwan": "TWN", "Hong Kong": "HKG", "Georgia": "GEO", "Belarus": "BLR",
        "Angola": "AGO", "Mozambique": "MOZ", "Zimbabwe": "ZWE", "Zambia": "ZMB",
        "Tanzania": "TZA", "Uganda": "UGA", "Rwanda": "RWA", "Senegal": "SEN",
        "Mali": "MLI", "Tunisia": "TUN", "Sudan": "SDN", "Somalia": "SOM",
        "Cameroon": "CMR", "Ivory Coast": "CIV",
    }

    async def _rate_limited_request(
        self, session: aiohttp.ClientSession, url: str, params: dict, max_retries: int = 3
    ) -> dict | None:
        """Rate-limited GET with exponential backoff. GDELT: 1 req / 5s."""
        import json as _json
        headers = {"User-Agent": self.USER_AGENT, "Accept": "application/json"}
        for attempt in range(max_retries):
            try:
                await asyncio.sleep(5 * (2 ** attempt))
                async with session.get(
                    url, params=params, headers=headers, timeout=aiohttp.ClientTimeout(total=60)
                ) as resp:
                    content = await resp.text()
                    if resp.status == 429 or "Please limit requests" in content:
                        continue
                    if resp.status != 200:
                        print(f"GDELT HTTP {resp.status}")
                        return None
                    try:
                        return _json.loads(content)
                    except _json.JSONDecodeError:
                        print(f"GDELT invalid JSON: {content[:200]}")
                        return None
            except asyncio.TimeoutError:
                continue
            except Exception as e:
                print(f"GDELT error: {e}")
                return None
        return None

    def _slug(self, s: str) -> str:
        import re
        return re.sub(r"[^\w]+", "_", s.strip().lower()).strip("_")

    async def validate(self) -> bool:
        try:
            async with aiohttp.ClientSession() as session:
                result = await self._rate_limited_request(
                    session,
                    self.BASE_URL,
                    {"query": "france", "format": "json", "maxrecords": "1", "mode": "artlist"},
                )
                return result is not None and "articles" in result
        except Exception:
            return False

    async def fetch(self, query: Dict[str, Any]) -> List[Dict[str, Any]]:
        """
        Search recent news articles matching query.
        Returns: 1 entity per article + relation to the queried entity (if country detected).
        """
        results: List[Dict[str, Any]] = []
        search_query = query.get("q") or query.get("query") or ""
        if not search_query.strip():
            return results

        params = {
            "query": search_query,
            "format": "json",
            "mode": "artlist",
            "maxrecords": str(query.get("limit", 50)),
            "sourcelang": query.get("lang", ""),  # optional filter: 'french', 'english', ...
        }
        params = {k: v for k, v in params.items() if v}

        async with aiohttp.ClientSession() as session:
            data = await self._rate_limited_request(session, self.BASE_URL, params)
        if not data or "articles" not in data:
            return results

        # Detect queried country to link articles back to it
        detected_country_iso = self.COUNTRY_TO_ISO.get(search_query.title())
        detected_country_id = (
            f"uncomtrade_country_{search_query.replace(' ', '_')}" if detected_country_iso else None
        )

        # Emit the queried country as entity if matched (so links can land)
        if detected_country_iso and detected_country_id:
            results.append(
                Entity(
                    id=detected_country_id,
                    name=search_query.title(),
                    entity_type="Country",
                    properties={"iso3": detected_country_iso, "source": "GDELT"},
                    sources=["https://www.gdeltproject.org/"],
                ).model_dump()
            )

        seen_article_ids = set()
        for article in data["articles"]:
            url = article.get("url")
            title = (article.get("title") or "").strip()
            seendate = article.get("seendate")  # e.g. "20260805T213000Z"
            domain = article.get("domain") or ""
            sourcecountry = article.get("sourcecountry") or ""
            language = article.get("language") or ""

            if not url or not title:
                continue

            article_id = f"gdelt_article_{self._slug(url)[:80]}"
            if article_id in seen_article_ids:
                continue
            seen_article_ids.add(article_id)

            # Extract year from seendate (first 4 chars: YYYYMMDDT...)
            year = None
            if seendate and len(seendate) >= 4 and seendate[:4].isdigit():
                year = int(seendate[:4])

            # Article entity
            results.append(
                Entity(
                    id=article_id,
                    name=title[:200],
                    entity_type="Article",
                    properties={
                        "source": "GDELT",
                        "url": url,
                        "domain": domain,
                        "source_country": sourcecountry,
                        "language": language,
                        "seendate": seendate,
                        **({"year": year} if year else {}),
                        "query": search_query,
                    },
                    sources=[url, "https://www.gdeltproject.org/"],
                ).model_dump()
            )

            # Link: query country —mentions→ article
            if detected_country_id:
                results.append(
                    {
                        "from_entity": detected_country_id,
                        "to_entity": article_id,
                        "relationship_type": "mentioned_in",
                        "properties": {"source": "GDELT", "domain": domain, **({"year": year} if year else {})},
                        "confidence": 0.8,
                        "sources": [url, "https://www.gdeltproject.org/"],
                    }
                )

        return results

    def get_schema(self) -> Dict[str, Any]:
        return {
            "entities": ["Article", "Country"],
            "relationships": ["mentioned_in"],
            "update_frequency": "real-time (15 min)",
            "coverage": "Global news in 100+ languages",
        }


register_plugin(GDELTPlugin({}))
