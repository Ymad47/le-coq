"""
OpenCorporates Plugin - Company registry data

API: https://api.opencorporates.com/documentation/API-Reference
"""

import aiohttp
from typing import Dict, List, Any
from plugins import DataSourcePlugin, Entity, register_plugin


class OpenCorporatesPlugin(DataSourcePlugin):
    """OpenCorporates company registry plugin"""
    
    name = "opencorporates"
    version = "1.0.0"
    description = "Company registry data from OpenCorporates (140M+ entities)"
    
    BASE_URL = "https://api.opencorporates.com/v0.4"
    
    async def validate(self) -> bool:
        """Test API connection"""
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(
                    f"{self.BASE_URL}/companies/search",
                    params={"q": "test", "format": "json", "api_token": self.api_key},
                    timeout=aiohttp.ClientTimeout(total=10)
                ) as resp:
                    return resp.status == 200
        except Exception:
            return False
    
    async def fetch(self, query: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Search for companies"""
        results = []
        
        search_query = query.get("q", "")
        jurisdiction = query.get("jurisdiction", "")
        
        async with aiohttp.ClientSession() as session:
            params = {
                "q": search_query,
                "format": "json",
                "api_token": self.api_key,
                "per_page": 100,
            }
            if jurisdiction:
                params["jurisdiction"] = jurisdiction
            
            async with session.get(
                f"{self.BASE_URL}/companies/search",
                params=params,
                timeout=aiohttp.ClientTimeout(total=30)
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    for company in data.get("results", {}).get("companies", []):
                        entity = Entity(
                            id=f"oc_{company['company']['opencorporates_url']}",
                            name=company["company"]["name"],
                            entity_type="Organization",
                            aliases=company["company"].get("previous_names", []),
                            properties={
                                "jurisdiction": company["company"].get("jurisdiction_code"),
                                "company_number": company["company"].get("company_number"),
                                "incorporation_date": company["company"].get("incorporation_date"),
                                "company_type": company["company"].get("company_type"),
                                "status": company["company"].get("current_status"),
                                "address": company["company"].get("registered_address_in_full"),
                                "source_url": company["company"]["opencorporates_url"],
                            },
                            sources=[company["company"]["opencorporates_url"]]
                        )
                        results.append(entity.model_dump())
        
        return results
    
    def get_schema(self) -> Dict[str, Any]:
        return {
            "entities": ["Organization"],
            "relationships": ["owns", "subsidiary_of", "officer_of"],
            "update_frequency": "weekly",
            "rate_limit": "50 requests/minute (free tier)"
        }


# Auto-register
register_plugin(OpenCorporatesPlugin({"api_key": ""}))
