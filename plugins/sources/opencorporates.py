"""
OpenCorporates Plugin - Company registry data

API: https://api.opencorporates.com/documentation/API-Reference
"""

import os

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
        if not self.api_key:
            raise RuntimeError(
                "OpenCorporates: no API token (set OPENCORPORATES_API_TOKEN)"
            )
        results = []

        search_query = query.get("q") or query.get("query") or ""
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
                if resp.status != 200:
                    raise RuntimeError(f"OpenCorporates: API returned HTTP {resp.status}")
                data = await resp.json()

        for item in data.get("results", {}).get("companies", []):
            company = item["company"]
            entity = Entity(
                id=f"oc_{company['opencorporates_url']}",
                name=company["name"],
                entity_type="Organization",
                aliases=_previous_names(company),
                properties={
                    "jurisdiction": company.get("jurisdiction_code"),
                    "company_number": company.get("company_number"),
                    "incorporation_date": company.get("incorporation_date"),
                    "company_type": company.get("company_type"),
                    "status": company.get("current_status"),
                    "address": company.get("registered_address_in_full"),
                    "source_url": company["opencorporates_url"],
                },
                sources=[company["opencorporates_url"]]
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


def _previous_names(company: Dict[str, Any]) -> List[str]:
    """previous_names is a list of {"company_name": ...} objects, not strings."""
    names = []
    for prev in company.get("previous_names") or []:
        name = prev.get("company_name") if isinstance(prev, dict) else prev
        if isinstance(name, str) and name:
            names.append(name)
    return names


# Auto-register
register_plugin(OpenCorporatesPlugin({"api_key": os.getenv("OPENCORPORATES_API_TOKEN", "")}))
