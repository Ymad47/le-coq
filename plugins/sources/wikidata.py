"""
Wikidata Plugin - Linked open data via SPARQL

Query entities and their relationships from Wikidata.
Free API, no key required, rate limit: 5 req/sec (respectful use).
"""

import asyncio
import aiohttp
import re
from typing import Dict, List, Any, Optional
from plugins import DataSourcePlugin, Entity, register_plugin


class WikidataPlugin(DataSourcePlugin):
    """Query Wikidata for entities and relationships via SPARQL endpoint"""

    name = "wikidata"
    version = "1.0.0"
    description = "Wikidata linked open data (SPARQL, free, no key)"

    SPARQL_URL = "https://query.wikidata.org/sparql"
    SEARCH_URL = "https://www.wikidata.org/w/api.php"

    # Map Wikidata property IDs to human-readable relation types
    RELATIONS = {
        "P17": "country",
        "P27": "citizenship",
        "P39": "position_held",
        "P102": "member_of_party",
        "P108": "employer",
        "P159": "headquarters_location",
        "P112": "founded_by",
        "P127": "owned_by",
        "P355": "subsidiary",
        "P749": "parent_organization",
        "P19": "born_in",
        "P20": "died_in",
        "P463": "member_of",
        "P607": "conflict",
        "P361": "part_of",
    }

    INSTANCE_TYPES = {
        "Q5": "Person",
        "Q43229": "Organization",
        "Q4830453": "Organization",
        "Q7188": "Organization",
        "Q6256": "Country",
        "Q515": "City",
        "Q15284": "City",
        "Q16521": "Topic",
        "Q7315158": "Event",
        "Q1190554": "Event",
    }

    async def validate(self) -> bool:
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(
                    self.SPARQL_URL,
                    params={"query": "SELECT * WHERE { wd:Q42 wdt:P31 ?t } LIMIT 1", "format": "json"},
                    headers={"Accept": "application/sparql-results+json", "User-Agent": "LeCoq/1.0 (osint research)"},
                    timeout=aiohttp.ClientTimeout(total=15),
                ) as resp:
                    return resp.status == 200
        except Exception:
            return False

    async def _sparql(self, session: aiohttp.ClientSession, query: str) -> List[Dict[str, Any]]:
        async with session.get(
            self.SPARQL_URL,
            params={"query": query, "format": "json"},
            headers={"Accept": "application/sparql-results+json", "User-Agent": "LeCoq/1.0 (osint research)"},
            timeout=aiohttp.ClientTimeout(total=90),
        ) as resp:
            if resp.status != 200:
                print(f"Wikidata SPARQL error: {resp.status}")
                return []
            data = await resp.json()
            return data.get("results", {}).get("bindings", [])

    async def _search_entity(self, session: aiohttp.ClientSession, name: str, limit: int = 3) -> List[str]:
        """Resolve a name like 'Emmanuel Macron' to QIDs like ['Q3052772']"""
        async with session.get(
            self.SEARCH_URL,
            params={
                "action": "wbsearchentities",
                "search": name,
                "language": "en",
                "format": "json",
                "limit": limit,
                "type": "item",
            },
            headers={"User-Agent": "LeCoq/1.0 (osint research)"},
            timeout=aiohttp.ClientTimeout(total=15),
        ) as resp:
            if resp.status != 200:
                return []
            data = await resp.json()
            return [r["id"] for r in data.get("search", [])[:limit]]

    def _qid_label_type(self, binding: Dict[str, Any]) -> tuple:
        """Extract (qid, label, type_hint) from a SPARQL binding."""
        qid = binding.get("item", {}).get("value", "").rsplit("/", 1)[-1]
        label = binding.get("itemLabel", {}).get("value", qid)
        type_uri = binding.get("instanceType", {}).get("value", "")
        type_q = type_uri.rsplit("/", 1)[-1] if type_uri else ""
        return qid, label, type_q

    def _entity_type_from_qid(self, type_qid: str) -> str:
        return self.INSTANCE_TYPES.get(type_qid, "Topic")

    def _entity_id(self, prefix: str, qid: str) -> str:
        return f"wd_{prefix}_{qid}"

    def _slug(self, name: str) -> str:
        return re.sub(r"[^a-zA-Z0-9]+", "_", name or "").strip("_")[:80]

    async def _resolve_labels(self, session: aiohttp.ClientSession, qids: set) -> Dict[str, str]:
        """Batch-resolve QIDs to English labels via wbgetentities (chunks of 50)."""
        labels: Dict[str, str] = {}
        qs = sorted(qids)
        for i in range(0, len(qs), 50):
            chunk = qs[i : i + 50]
            for attempt in range(2):
                try:
                    async with session.get(
                        self.SEARCH_URL,
                        params={
                            "action": "wbgetentities",
                            "ids": "|".join(chunk),
                            "props": "labels|descriptions",
                            "languages": "en",
                            "format": "json",
                        },
                        headers={"User-Agent": "LeCoq/1.0 (osint research; contact@ymad.fr)"},
                        timeout=aiohttp.ClientTimeout(total=20),
                    ) as resp:
                        if resp.status == 429:
                            await asyncio.sleep(2 + attempt * 3)
                            continue
                        data = await resp.json()
                        for q, ent in data.get("entities", {}).items():
                            lbl = ent.get("labels", {}).get("en", {}).get("value")
                            if lbl:
                                labels[q] = lbl
                        break
                except Exception as e:
                    print(f"Wikidata label resolve error: {e}")
                    await asyncio.sleep(2)
            await asyncio.sleep(0.5)  # reste poli avec l'API
        return labels

    async def fetch(self, query: Dict[str, Any]) -> List[Dict[str, Any]]:
        """
        Fetch entity + direct relations for a search term.

        query:
          q: search text (resolved to Wikidata item)
          qid: direct QID (skip search resolution)
          depth: 1 (default) — direct relations only
          limit: max results (default 5)
        """
        results: List[Dict[str, Any]] = []
        search_q = query.get("q") or query.get("query") or ""
        qid_direct = query.get("qid")
        limit = int(query.get("limit", 5))

        async with aiohttp.ClientSession() as session:
            # Resolve QIDs
            if qid_direct:
                qids = [qid_direct if qid_direct.startswith("Q") else f"Q{qid_direct}"]
            elif search_q:
                qids = await self._search_entity(session, search_q, limit=min(limit, 5))
            else:
                return results

            if not qids:
                return results

            # 1. Fetch basic info + instance type for each QID
            values = " ".join(f"wd:{q}" for q in qids)
            basic_query = f"""
SELECT ?item ?itemLabel ?instanceType ?desc WHERE {{
  VALUES ?item {{ {values} }}
  OPTIONAL {{ ?item wdt:P31 ?instanceType. }}
  OPTIONAL {{ ?item schema:description ?desc. FILTER(LANG(?desc) = 'en') }}
  SERVICE wikibase:label {{ bd:serviceParam wikibase:language 'en'. }}
}}
"""
            bindings = await self._sparql(session, basic_query)
            # Group per item: keep first type seen
            items: Dict[str, Dict[str, str]] = {}
            for b in bindings:
                qid, label, type_q = self._qid_label_type(b)
                if qid not in items:
                    items[qid] = {
                        "label": label,
                        "type": self._entity_type_from_qid(type_q),
                        "desc": b.get("desc", {}).get("value", ""),
                    }

            # Fallback for items not returned by SPARQL (e.g. redirect)
            for q in qids:
                if q not in items:
                    items[q] = {"label": q, "type": "Topic", "desc": ""}

            # Add entities
            for q, info in items.items():
                entity = Entity(
                    id=self._entity_id("item", q),
                    name=info["label"],
                    entity_type=info["type"],
                    properties={
                        "qid": q,
                        "description": info["desc"],
                        "source": "Wikidata",
                    },
                    sources=[f"https://www.wikidata.org/wiki/{q}"],
                )
                results.append(entity.model_dump())

            # 2. Fetch direct relations for each QID
            # VALUES wdt: direct — filter on specific property URIs, fast + reliable
            values_props = " ".join(f"wdt:{p}" for p in self.RELATIONS.keys())
            rel_query = f"""
SELECT DISTINCT ?item ?prop ?value ?valueType WHERE {{
  VALUES ?item {{ {values} }}
  VALUES ?prop {{ {values_props} }}
  ?item ?prop ?value.
  OPTIONAL {{ ?value wdt:P31 ?valueType. }}
}}
LIMIT 300
"""
            rel_bindings = await self._sparql(session, rel_query)

            # Collect all QIDs needing labels (sources + targets)
            label_qids: set = set()
            for b in rel_bindings:
                for key in ("item", "value"):
                    u = b.get(key, {}).get("value", "")
                    q = u.rsplit("/", 1)[-1]
                    if q.startswith("Q"):
                        label_qids.add(q)
            labels = await self._resolve_labels(session, label_qids)
            # Track created target entities to avoid duplicates
            created_targets: set = set()
            seen_rels: set = set()
            for b in rel_bindings:
                src_qid = b.get("item", {}).get("value", "").rsplit("/", 1)[-1]
                prop_uri = b.get("prop", {}).get("value", "")
                prop_pid = prop_uri.rsplit("/", 1)[-1]
                rel_type = self.RELATIONS.get(prop_pid, prop_pid)
                val_qid = b.get("value", {}).get("value", "").rsplit("/", 1)[-1]
                src_label = labels.get(src_qid, src_qid)
                val_label = labels.get(val_qid, val_qid)
                val_type_q = b.get("valueType", {}).get("value", "").rsplit("/", 1)[-1]
                val_type = self._entity_type_from_qid(val_type_q)

                if not src_qid or not val_qid or not val_qid.startswith("Q"):
                    continue

                src_eid = self._entity_id("item", src_qid)
                tgt_eid = self._entity_id("item", val_qid)
                rel_key = (src_eid, rel_type, tgt_eid)
                if rel_key in seen_rels:
                    continue
                seen_rels.add(rel_key)

                # Create target entity if not already present
                if tgt_eid not in created_targets and val_qid not in items:
                    entity = Entity(
                        id=tgt_eid,
                        name=val_label,
                        entity_type=val_type,
                        properties={"qid": val_qid, "source": "Wikidata", "related_to": src_label},
                        sources=[f"https://www.wikidata.org/wiki/{val_qid}"],
                    )
                    results.append(entity.model_dump())
                    created_targets.add(tgt_eid)

                # Create relationship record
                rel = {
                    "id": f"rel_{self._slug(src_label)}_{rel_type}_{self._slug(val_label)}",
                    "from_entity": src_eid,
                    "to_entity": tgt_eid,
                    "relationship_type": rel_type,
                    "properties": {"source": "Wikidata", "prop": prop_pid},
                    "confidence": 0.9,
                    "sources": [f"https://www.wikidata.org/wiki/{src_qid}"],
                }
                results.append(rel)

            return results


register_plugin(WikidataPlugin({}))
