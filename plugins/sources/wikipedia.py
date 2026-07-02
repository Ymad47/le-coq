"""
Wikipedia Plugin - Free encyclopedia data

API: https://en.wikipedia.org/api/rest_v1/
"""

import aiohttp
import asyncio
from typing import Dict, List, Any
from plugins import DataSourcePlugin, Entity, register_plugin


class WikipediaPlugin(DataSourcePlugin):
    """Wikipedia free encyclopedia plugin"""
    
    name = "wikipedia"
    version = "1.0.0"
    description = "Free encyclopedia data from Wikipedia"
    
    BASE_URL = "https://en.wikipedia.org/api/rest_v1"
    HEADERS = {
        "User-Agent": "Le-Coq-OSINT/1.0 (https://github.com/Ymad47/le-coq) python-aiohttp/3.14"
    }
    
    async def validate(self) -> bool:
        """Test API connection"""
        try:
            async with aiohttp.ClientSession(headers=self.HEADERS) as session:
                async with session.get(
                    f"{self.BASE_URL}/page/summary/Wikipedia",
                    timeout=aiohttp.ClientTimeout(total=10)
                ) as resp:
                    return resp.status == 200
        except Exception:
            return False
    
    async def fetch(self, query: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Search for Wikipedia pages using the correct API endpoint"""
        results = []
        
        search_query = query.get("q", "")
        if not search_query:
            return results
            
        async with aiohttp.ClientSession(headers=self.HEADERS) as session:
            # Use the correct Wikipedia search API endpoint
            search_url = "https://en.wikipedia.org/w/api.php"
            params = {
                "action": "query",
                "format": "json",
                "list": "search",
                "srsearch": search_query,
                "srlimit": 10
            }
            
            async with session.get(
                search_url,
                params=params,
                timeout=aiohttp.ClientTimeout(total=30)
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    search_results = data.get("query", {}).get("search", [])
                    
                    # For each search result, get detailed information
                    for page in search_results:
                        title = page.get("title", "")
                        if not title:
                            continue
                            
                        # Get page summary using the REST API
                        try:
                            summary_url = f"{self.BASE_URL}/page/summary/{title.replace(' ', '_')}"
                            async with session.get(
                                summary_url,
                                timeout=aiohttp.ClientTimeout(total=10)
                            ) as summary_resp:
                                if summary_resp.status == 200:
                                    summary_data = await summary_resp.json()
                                    
                                    entity = Entity(
                                        id=f"wiki_{title.replace(' ', '_')}",
                                        name=title,
                                        entity_type="Topic",
                                        aliases=[],
                                        properties={
                                            "description": summary_data.get("extract", ""),
                                            "url": summary_data.get("content_urls", {}).get("desktop", {}).get("page", ""),
                                            "thumbnail": summary_data.get("thumbnail", {}).get("source", ""),
                                            "page_id": str(summary_data.get("pageid", "")),
                                            "wordcount": page.get("wordcount", 0),
                                            "size": page.get("size", 0),
                                            "snippet": page.get("snippet", ""),
                                        },
                                        sources=[f"https://en.wikipedia.org/wiki/{title.replace(' ', '_')}"]
                                    )
                                    results.append(entity.model_dump())
                                else:
                                    # If we can't get the summary, create a basic entity from search data
                                    entity = Entity(
                                        id=f"wiki_{title.replace(' ', '_')}",
                                        name=title,
                                        entity_type="Topic",
                                        aliases=[],
                                        properties={
                                            "description": page.get("snippet", ""),
                                            "url": f"https://en.wikipedia.org/wiki/{title.replace(' ', '_')}",
                                            "wordcount": page.get("wordcount", 0),
                                            "size": page.get("size", 0),
                                        },
                                        sources=[f"https://en.wikipedia.org/wiki/{title.replace(' ', '_')}"]
                                    )
                                    results.append(entity.model_dump())
                        except Exception as e:
                            print(f"Error fetching summary for {title}: {e}")
                            # Create a basic entity from search data
                            entity = Entity(
                                id=f"wiki_{title.replace(' ', '_')}",
                                name=title,
                                entity_type="Topic",
                                aliases=[],
                                properties={
                                    "description": page.get("snippet", ""),
                                    "url": f"https://en.wikipedia.org/wiki/{title.replace(' ', '_')}",
                                    "wordcount": page.get("wordcount", 0),
                                    "size": page.get("size", 0),
                                },
                                sources=[f"https://en.wikipedia.org/wiki/{title.replace(' ', '_')}"]
                            )
                            results.append(entity.model_dump())
        
        return results
    
    def get_schema(self) -> Dict[str, Any]:
        return {
            "entities": ["Topic", "Person", "Organization", "Location"],
            "relationships": ["related_to", "mentioned_in"],
            "update_frequency": "realtime",
            "rate_limit": "Unlimited (public API)"
        }


# Auto-register
register_plugin(WikipediaPlugin({}))