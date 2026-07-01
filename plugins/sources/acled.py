"""
ACLED Plugin - Conflict event data

API: https://acleddata.com/api/
"""

import aiohttp
from typing import Dict, List, Any
from datetime import datetime
from plugins import DataSourcePlugin, Entity, Event, register_plugin


class ACLEDPlugin(DataSourcePlugin):
    """ACLED conflict event data plugin"""
    
    name = "acled"
    version = "1.0.0"
    description = "Armed Conflict Location & Event Data Project"
    
    BASE_URL = "https://api.acleddata.com"
    
    async def validate(self) -> bool:
        """Test API connection"""
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(
                    f"{self.BASE_URL}/events",
                    headers={"X-API-KEY": self.api_key},
                    params={"limit": 1},
                    timeout=aiohttp.ClientTimeout(total=10)
                ) as resp:
                    return resp.status == 200
        except Exception:
            return False
    
    async def fetch(self, query: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Fetch conflict events"""
        results = []
        
        params = {
            "limit": query.get("limit", 100),
        }
        
        if query.get("country"):
            params["country"] = query["country"]
        if query.get("event_type"):
            params["event_type"] = query["event_type"]
        if query.get("from_date"):
            params["from_date"] = query["from_date"]
        if query.get("to_date"):
            params["to_date"] = query["to_date"]
        
        async with aiohttp.ClientSession() as session:
            async with session.get(
                f"{self.BASE_URL}/events",
                headers={"X-API-KEY": self.api_key},
                params=params,
                timeout=aiohttp.ClientTimeout(total=60)
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    for event in data.get("data", []):
                        # Create location entity
                        location = Entity(
                            id=f"acled_loc_{event['location']}",
                            name=event["location"],
                            entity_type="Location",
                            properties={
                                "latitude": event.get("latitude"),
                                "longitude": event.get("longitude"),
                                "country": event.get("country"),
                                "region": event.get("region"),
                            },
                            sources=[event.get("source")]
                        )
                        results.append(location.model_dump())
                        
                        # Create event
                        acled_event = Event(
                            id=f"acled_{event['event_id_cnty']}",
                            title=f"{event['sub_event_type']} in {event['location']}",
                            date=datetime.strptime(event["event_date"], "%Y-%m-%d"),
                            location=location.id,
                            participants=[],
                            event_type=self._map_event_type(event["event_type"]),
                            description=event.get("notes", ""),
                            sources=[event.get("source")] if event.get("source") else [],
                            severity=self._map_severity(event.get("fatalities", 0))
                        )
                        results.append(acled_event.model_dump())
        
        return results
    
    def _map_event_type(self, acled_type: str) -> str:
        """Map ACLED event types to Le Coq event types"""
        mapping = {
            "Battles": "conflict",
            "Violence against civilians": "conflict",
            "Riots": "protest",
            "Protests": "protest",
            "Strategic developments": "meeting",
        }
        return mapping.get(acled_type, "conflict")
    
    def _map_severity(self, fatalities: int) -> str:
        """Map fatalities to severity level"""
        if fatalities == 0:
            return "low"
        elif fatalities < 10:
            return "medium"
        elif fatalities < 50:
            return "high"
        else:
            return "critical"
    
    def get_schema(self) -> Dict[str, Any]:
        return {
            "entities": ["Location", "Event"],
            "relationships": ["occurred_in", "involved"],
            "update_frequency": "daily",
            "coverage": "Global conflict events"
        }


# Auto-register
register_plugin(ACLEDPlugin({"api_key": ""}))
