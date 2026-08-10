"""
Wikipedia Plugin - Free encyclopedia data + relations from internal links

API: https://en.wikipedia.org/api/rest_v1/
Extracts entities + creates 'related_to' relations from internal Wikipedia links.
"""

import aiohttp
import asyncio
import re
from typing import Dict, List, Any, Set, Tuple
from bs4 import BeautifulSoup
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
        """Search for Wikipedia pages + extract internal links as relations"""
        results = []
        relations = []
        
        # Support both 'q' and 'query' parameter names
        search_query = query.get("q") or query.get("query", "")
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
                                            "source": "Wikipedia",
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
                                    
                                    # Extract internal links from full page HTML to create relations
                                    relations.extend(await self._extract_links(session, title, entity))
                                    
                                else:
                                    # If we can't get the summary, create a basic entity from search data
                                    entity = Entity(
                                        id=f"wiki_{title.replace(' ', '_')}",
                                        name=title,
                                        entity_type="Topic",
                                        aliases=[],
                                        properties={
                                            "source": "Wikipedia",
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
                                    "source": "Wikipedia",
                                    "description": page.get("snippet", ""),
                                    "url": f"https://en.wikipedia.org/wiki/{title.replace(' ', '_')}",
                                    "wordcount": page.get("wordcount", 0),
                                    "size": page.get("size", 0)
                                },
                                sources=[f"https://en.wikipedia.org/wiki/{title.replace(' ', '_')}"]
                            )
                            results.append(entity.model_dump())
        
        # Append relations at the end
        results.extend(relations)
        return results
    
    async def _extract_links(self, session: aiohttp.ClientSession, title: str, source_entity: Entity) -> List[Dict[str, Any]]:
        """Extract internal Wikipedia links using API (more reliable than HTML parsing)"""
        relations = []
        seen_links: Set[str] = set()
        
        try:
            # Use Wikipedia links API (no HTML parsing needed)
            api_url = "https://en.wikipedia.org/w/api.php"
            params = {
                "action": "query",
                "format": "json",
                "titles": title,
                "prop": "links",
                "plnamespace": "0",  # Only main article namespace
                "pllimit": "max"  # Get max links (500 for bots)
            }
            
            async with session.get(
                api_url,
                params=params,
                timeout=aiohttp.ClientTimeout(total=15)
            ) as resp:
                if resp.status != 200:
                    return relations
                data = await resp.json()
                
                # Extract links from API response
                pages = data.get("query", {}).get("pages", {})
                for page_id, page_data in pages.items():
                    links = page_data.get("links", [])
                    
                    for link in links:
                        link_title = link.get("title", "")
                        if not link_title or link_title in seen_links:
                            continue
                        seen_links.add(link_title)
                        
                        # Skip disambiguation pages (check title)
                        if '(disambiguation)' in link_title.lower():
                            continue
                        
                        # Create target entity (minimal, will be enriched if user clicks)
                        target_id = f"wiki_{link_title.replace(' ', '_')}"
                        
                        # Create relation
                        relation = {
                            "id": f"rel_wiki_{self._slug(source_entity.name)}_related_{self._slug(link_title)}",
                            "entity_type": "Relation",
                            "from_entity": source_entity.id,
                            "to_entity": target_id,
                            "relationship_type": "related_to",
                            "properties": {
                                "source": "Wikipedia",
                                "link_context": "internal_link",
                            },
                            "confidence": 0.7,  # Lower than Wikidata (less structured)
                            "sources": [f"https://en.wikipedia.org/wiki/{title.replace(' ', '_')}"]
                        }
                        relations.append(relation)
                        
                        # Limit to 20 relations per page to avoid overload
                        if len(relations) >= 20:
                            break
                    break  # Only process first page
                    
        except Exception as e:
            print(f"Error extracting links from {title}: {e}")
        
        return relations
    
    def _slug(self, name: str) -> str:
        """Create URL-safe slug from name"""
        return re.sub(r'[^a-zA-Z0-9]+', '_', name).strip('_')[:80]
    
    def get_schema(self) -> Dict[str, Any]:
        return {
            "entities": ["Topic", "Person", "Organization", "Location"],
            "relationships": ["related_to", "mentioned_in"],
            "update_frequency": "realtime",
            "rate_limit": "Unlimited (public API)"
        }


# Auto-register
register_plugin(WikipediaPlugin({}))