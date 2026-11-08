"""
ACLED Plugin - Conflict event data

API: https://acleddata.com/api/
Auth: OAuth 2.0 token-based (email/password → bearer token)
"""

import aiohttp
import os
from typing import Dict, List, Any, Optional
from datetime import datetime, timedelta
from plugins import DataSourcePlugin, Entity, Relationship, register_plugin


class ACLEDPlugin(DataSourcePlugin):
    """ACLED conflict event data plugin with OAuth token auth"""
    
    name = "acled"
    version = "2.0.0"
    description = "Armed Conflict Location & Event Data Project (OAuth auth)"
    
    BASE_URL = "https://api.acleddata.com"
    TOKEN_URL = "https://acleddata.com/oauth/token"
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.email = os.getenv("ACLED_EMAIL", "")
        self.password = os.getenv("ACLED_PASSWORD", "")
        self._access_token: Optional[str] = os.getenv("ACLED_ACCESS_TOKEN")
        self._refresh_token: Optional[str] = os.getenv("ACLED_REFRESH_TOKEN")
        self._token_expires: Optional[datetime] = None
        
        # Token expires in 24h from issue time (check if still valid)
        # If token is from env, assume it's valid for now (will refresh when needed)
        if self._access_token and not self._token_expires:
            self._token_expires = datetime.now() + timedelta(hours=23)
    
    async def _get_token(self) -> Optional[str]:
        """Get or refresh OAuth access token"""
        # Return cached token if still valid (with 5min buffer)
        if self._access_token and self._token_expires:
            if datetime.now() < self._token_expires - timedelta(minutes=5):
                return self._access_token
        
        # Try refresh token first if available
        if self._refresh_token:
            token = await self._refresh_token_flow()
            if token:
                return token
        
        # Request new token with credentials
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    self.TOKEN_URL,
                    data={
                        "username": self.email,
                        "password": self.password,
                        "grant_type": "password",
                        "client_id": "acled",
                        "scope": "authenticated",
                    },
                    timeout=aiohttp.ClientTimeout(total=10)
                ) as resp:
                    if resp.status == 200:
                        data = await resp.json()
                        self._access_token = data.get("access_token")
                        self._refresh_token = data.get("refresh_token")
                        expires_in = data.get("expires_in", 86400)
                        self._token_expires = datetime.now() + timedelta(seconds=expires_in)
                        return self._access_token
                    else:
                        print(f"ACLED token request failed: {resp.status}")
                        return None
        except Exception as e:
            print(f"ACLED token error: {e}")
            return None
    
    async def _refresh_token_flow(self) -> Optional[str]:
        """Refresh access token using refresh token"""
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    self.TOKEN_URL,
                    data={
                        "refresh_token": self._refresh_token,
                        "grant_type": "refresh_token",
                        "client_id": "acled",
                    },
                    timeout=aiohttp.ClientTimeout(total=10)
                ) as resp:
                    if resp.status == 200:
                        data = await resp.json()
                        self._access_token = data.get("access_token")
                        self._refresh_token = data.get("refresh_token")
                        expires_in = data.get("expires_in", 86400)
                        self._token_expires = datetime.now() + timedelta(seconds=expires_in)
                        return self._access_token
                    else:
                        print(f"ACLED token refresh failed: {resp.status}")
                        self._refresh_token = None  # Clear invalid refresh token
                        return None
        except Exception as e:
            print(f"ACLED refresh error: {e}")
            self._refresh_token = None
            return None
    
    async def validate(self) -> bool:
        """Test API connection with OAuth token"""
        token = await self._get_token()
        if not token:
            return False
        
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(
                    f"{self.BASE_URL}/api/acled/read?limit=1",
                    headers={"Authorization": f"Bearer {token}"},
                    timeout=aiohttp.ClientTimeout(total=10)
                ) as resp:
                    return resp.status == 200
        except Exception:
            return False
    
    async def fetch(self, query: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Fetch conflict events with OAuth token"""
        if not (self._access_token or self._refresh_token or (self.email and self.password)):
            raise RuntimeError(
                "ACLED: no credentials (set ACLED_EMAIL and ACLED_PASSWORD, "
                "or ACLED_ACCESS_TOKEN)"
            )
        token = await self._get_token()
        if not token:
            raise RuntimeError("ACLED: authentication failed (check credentials)")

        params = {
            "limit": query.get("limit", 100),
        }
        for key in ("country", "event_type", "from_date", "to_date"):
            if query.get(key):
                params[key] = query[key]

        async with aiohttp.ClientSession() as session:
            async with session.get(
                f"{self.BASE_URL}/api/acled/read",
                headers={"Authorization": f"Bearer {token}"},
                params=params,
                timeout=aiohttp.ClientTimeout(total=60)
            ) as resp:
                if resp.status != 200:
                    raise RuntimeError(f"ACLED: API returned HTTP {resp.status}")
                data = await resp.json()

        results: List[Dict[str, Any]] = []
        for event in data.get("data", []):
            results.extend(self._event_to_items(event))
        return results

    def _event_to_items(self, event: Dict[str, Any]) -> List[Dict[str, Any]]:
        """One ACLED event -> [location entity, event entity, occurred_in relation].

        The backend stores entities and relationships only, so the event is an
        entity of type "Event" whose published_at (event date) feeds the timeline.
        """
        source = event.get("source")
        sources = [source] if source else []

        location = Entity(
            id=f"acled_loc_{event['location']}",
            name=event["location"],
            entity_type="Location",
            properties={
                "latitude": event.get("latitude"),
                "longitude": event.get("longitude"),
                "country": event.get("country"),
                "region": event.get("region"),
                "source": "ACLED",
            },
            sources=sources,
        )

        fatalities = int(event.get("fatalities") or 0)
        event_entity = Entity(
            id=f"acled_{event['event_id_cnty']}",
            name=f"{event['sub_event_type']} in {event['location']}",
            entity_type="Event",
            properties={
                "event_type": self._map_event_type(event["event_type"]),
                "acled_event_type": event["event_type"],
                "sub_event_type": event["sub_event_type"],
                "severity": self._map_severity(fatalities),
                "fatalities": fatalities,
                "actor1": event.get("actor1"),
                "actor2": event.get("actor2"),
                "description": event.get("notes", ""),
                "country": event.get("country"),
                "source": "ACLED",
            },
            sources=sources,
            # ACLED dates are days; midnight UTC keeps them on the right day.
            published_at=f"{event['event_date']}T00:00:00Z",
        )

        occurred_in = Relationship(
            id=f"acled_rel_{event['event_id_cnty']}",
            from_entity=event_entity.id,
            to_entity=location.id,
            relationship_type="occurred_in",
            sources=sources,
        )

        return [location.model_dump(), event_entity.model_dump(), occurred_in.model_dump()]

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


# Auto-register (credentials from env vars)
register_plugin(ACLEDPlugin({}))
