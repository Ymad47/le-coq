"""
GDELT Plugin - Global news event monitoring

API: BigQuery or GDELT 2.0 API
"""

import aiohttp
from typing import Dict, List, Any
from plugins import DataSourcePlugin, Entity, register_plugin


class GDELTPlugin(DataSourcePlugin):
    """GDELT global news monitoring plugin"""
    
    name = "gdelt"
    version = "1.0.0"
    description = "Global Database of Events, Language, and Tone"
    
    BASE_URL = "https://api.gdeltproject.org/api/v2/doc/doc"
    
    async def validate(self) -> bool:
        """Test API connection"""
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(
                    f"{self.BASE_URL}?query=test&format=json",
                    timeout=aiohttp.ClientTimeout(total=10)
                ) as resp:
                    return resp.status == 200
        except Exception:
            return False
    
    async def fetch(self, query: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Search GDELT for news events"""
        results = []
        
        search_query = query.get("q", "")
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
            async with session.get(
                self.BASE_URL,
                params=params,
                timeout=aiohttp.ClientTimeout(total=60)
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
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
