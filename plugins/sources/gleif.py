"""
GLEIF Plugin — Global Legal Entity Identifier Foundation

API: https://api.gleif.org/api/v1/lei-lookup
Free, no key required.
Golden copy of 2M+ Legal Entity Identifiers (ISO 17442) + ownership relations.

Data model:
- Organization entities (name, LEI, jurisdiction, address)
- Relationships: parent_company / ultimate_parent (corporate ownership)
"""

import aiohttp
from typing import Dict, List, Any
from plugins import DataSourcePlugin, Entity, register_plugin


class GLEIFPlugin(DataSourcePlugin):
    """GLEIF Legal Entity Identifier plugin — corporate ownership graph"""

    name = "gleif"
    version = "1.0.0"
    description = "Global LEI registry — corporate ownership relations (parent ↔ child)"

    BASE_URL = "https://api.gleif.org/api/v1"

    async def validate(self) -> bool:
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(
                    f"{self.BASE_URL}/lei-records",
                    params={"filter[fulltext]": "TotalEnergies", "page[size]": 1},
                    timeout=aiohttp.ClientTimeout(total=10),
                ) as resp:
                    return resp.status == 200
        except Exception:
            return False

    async def fetch(self, query: Dict[str, Any]) -> List[Dict[str, Any]]:
        """
        Search companies by name, return LEI records + ownership relationships.

        query:
          q / query: company name (e.g. "TotalEnergies", "LVMH")
          limit: max records (default 5)
        """
        results: List[Dict[str, Any]] = []

        raw = (query.get("q") or query.get("query") or "").strip()
        limit = int(query.get("limit", 5))

        if not raw:
            return results

        async with aiohttp.ClientSession() as session:
            # Full-text search on name
            try:
                async with session.get(
                    f"{self.BASE_URL}/lei-records",
                    params={"filter[fulltext]": raw, "page[size]": limit},
                    timeout=aiohttp.ClientTimeout(total=15),
                ) as resp:
                    if resp.status != 200:
                        return results
                    data = await resp.json()
            except Exception:
                return results

            records = data.get("data") or []
            for rec in records:
                attrs = rec.get("attributes") or {}
                lei = rec.get("id")
                entity_attrs = attrs.get("entity") or {}

                legal_name = entity_attrs.get("legalName", {}).get("name")
                if not legal_name:
                    continue

                jurisdiction = entity_attrs.get("jurisdiction") or ""
                entity_id = f"gleif_{lei}"

                results.append(Entity(
                    id=entity_id,
                    name=legal_name,
                    entity_type="Organization",
                    aliases=[a.get("name") for a in (entity_attrs.get("otherEntityNames") or []) if a.get("name")][:3],
                    properties={
                        "source": "GLEIF",
                        "lei": lei,
                        "jurisdiction": jurisdiction,
                        "country": (entity_attrs.get("legalAddress") or {}).get("country") or jurisdiction,
                        "legal_form": (entity_attrs.get("legalForm") or {}).get("id"),
                        "status": attrs.get("registration", {}).get("registrationStatus"),
                    },
                    sources=[f"https://search.gleif.org/#/record/{lei}"],
                ).model_dump())

                # Ownership relationships via consolidation endpoint
                try:
                    parents = await self._fetch_parents(session, lei)
                    for parent in parents:
                        parent_lei = parent.get("lei")
                        parent_name = parent.get("name") or parent_lei
                        if not parent_lei:
                            continue
                        parent_id = f"gleif_{parent_lei}"

                        results.append(Entity(
                            id=parent_id,
                            name=parent_name,
                            entity_type="Organization",
                            properties={"source": "GLEIF", "lei": parent_lei},
                            sources=[f"https://search.gleif.org/#/record/{parent_lei}"],
                        ).model_dump())

                        results.append({
                            "from_entity": parent_id,
                            "to_entity": entity_id,
                            "relationship_type": "parent_of",
                            "properties":{
                                "source": "GLEIF",
                                "ownership": parent.get("relationship_type"),
                            },
                            "confidence": 1.0,
                            "sources":[f"https://search.gleif.org/#/record/{lei}"],
                        })
                except Exception:
                    pass

        return results

    async def _fetch_parents(self, session: aiohttp.ClientSession, lei: str) -> List[Dict[str, Any]]:
        """Get direct + ultimate parents for a given LEI."""
        parents: List[Dict[str, Any]] = []
        for kind, rt in (("direct_parent", "IS_DIRECTLY_CONSOLIDATED_BY"),
                          ("ultimate_parent", "IS_ULTIMATELY_CONSOLIDATED_BY")):
            try:
                async with session.get(
                    f"{self.BASE_URL}/lei-records/{lei}/{kind}",
                    timeout=aiohttp.ClientTimeout(total=10),
                ) as resp:
                    if resp.status != 200:
                        continue
                    data = await resp.json()
            except Exception:
                continue

            rec = (data.get("data") or {})
            attrs = rec.get("attributes") or {}
            ent = attrs.get("entity") or {}
            name = (ent.get("legalName") or {}).get("name")
            if name:
                parents.append({
                    "lei": rec.get("id"),
                    "name": name,
                    "relationship_type": rt,
                })
        return parents

    def get_schema(self) -> Dict[str, Any]:
        return {
            "entities": ["Organization"],
            "relationships": ["parent_of"],
            "update_frequency": "quarterly (GLEIF Golden Copy)",
            "coverage": "2M+ LEI records worldwide, ISO-certified",
        }


register_plugin(GLEIFPlugin({}))
