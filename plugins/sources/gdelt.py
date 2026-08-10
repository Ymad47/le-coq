"""
GDELT Plugin - Global news event monitoring

API: BigQuery or GDELT 2.0 API
"""

import asyncio
import aiohttp
from typing import Dict, List, Any
from plugins import DataSourcePlugin, Entity, register_plugin


class GDELTPlugin(DataSourcePlugin):
    """GDELT global news monitoring plugin
    
    Note: GDELT free API has strict rate limits (1 req per 5s, max ~100/day).
    For production use, consider:
    - GDELT BigQuery dataset (free, no rate limits)
    - Caching layer with Redis/Memcached
    - Exponential backoff with long delays
    """
    
    name = "gdelt"
    version = "1.0.0"
    description = "Global Database of Events, Language, and Tone (rate-limited free API)"
    
    BASE_URL = "https://api.gdeltproject.org/api/v2/doc/doc"
    
    async def _rate_limited_request(self, session: aiohttp.ClientSession, url: str, params: dict, max_retries: int = 3) -> dict | None:
        """Make rate-limited request with exponential backoff"""
        for attempt in range(max_retries):
            try:
                await asyncio.sleep(5 * (2 ** attempt))  # 5s, 10s, 20s between retries
                async with session.get(
                    url,
                    params=params,
                    timeout=aiohttp.ClientTimeout(total=60)
                ) as resp:
                    content = await resp.text()
                    if resp.status == 429 or "Please limit requests" in content:
                        print(f"GDELT rate limited, retry {attempt + 1}/{max_retries}...")
                        continue
                    if resp.status != 200:
                        print(f"GDELT error: {resp.status}")
                        return None
                    if "articles" not in content:
                        print("GDELT: Invalid response format")
                        return None
                    import json as _json
                    return _json.loads(content)
            except asyncio.TimeoutError:
                print(f"GDELT timeout, retry {attempt + 1}/{max_retries}...")
                continue
            except Exception as e:
                print(f"GDELT error: {e}")
                return None
        return None

    async def validate(self) -> bool:
        """Test API connection"""
        try:
            async with aiohttp.ClientSession() as session:
                result = await self._rate_limited_request(
                    session,
                    self.BASE_URL,
                    {"query": "test", "format": "json", "maxrecords": "1"}
                )
                return result is not None
        except Exception as e:
            print(f"Validate error: {e}")
            return False
    
    async def fetch(self, query: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Search GDELT for news events"""
        results = []
        
        search_query = query.get("q") or query.get("query") or ""
        start_date = query.get("start_date", "-30d")
        end_date = query.get("end_date", "now")
        
        params = {
            "query": search_query,
            "format": "json",
            "startdatetime": start_date,
            "enddatetime": end_date,
            "maxrecords": 100,
        }
        
        async with aiohttp.ClientSession() as session:
            data = await self._rate_limited_request(session, self.BASE_URL, params)
            if not data:
                print("GDELT: No data returned")
                return results
            
            for article in data.get("articles", []):
                # Extract organizations
                for org in article.get("organizations", [])[:5]:
                    entity = Entity(
                        id=f"gdelt_org_{org}",
                        name=org,
                        entity_type="Organization",
                        properties={"source": "GDELT"},
                        sources=[article.get("url")]
                    )
                    results.append(entity.model_dump())
                
                # Extract persons
                for person in article.get("persons", [])[:5]:
                    entity = Entity(
                        id=f"gdelt_person_{person}",
                        name=person,
                        entity_type="Person",
                        properties={"source": "GDELT"},
                        sources=[article.get("url")]
                    )
                    results.append(entity.model_dump())
                
                # Extract locations
                for location in article.get("locations", [])[:5]:
                    entity = Entity(
                        id=f"gdelt_loc_{location}",
                        name=location,
                        entity_type="Location",
                        properties={"source": "GDELT"},
                        sources=[article.get("url")]
                    )
                    results.append(entity.model_dump())
        
        return results
    
    def get_schema(self) -> Dict[str, Any]:
        return {
            "entities": ["Person", "Organization", "Location"],
            "relationships": ["mentioned_with", "located_in"],
            "update_frequency": "real-time (15min delay)",
            "coverage": "Global news in 100+ languages"
        }


# Auto-register
register_plugin(GDELTPlugin({}))
