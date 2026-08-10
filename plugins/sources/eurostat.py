"""
Eurostat Plugin — official EU statistical data

API: https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data
Free, no key required. JSON-stat format.

Data model:
- Country-level indicators (GDP, unemployment, population, etc.)
- Returns country entities with economic/demographic metadata
"""

import aiohttp
from typing import Dict, List, Any
from plugins import DataSourcePlugin, Entity, register_plugin


class EurostatPlugin(DataSourcePlugin):
    """Eurostat EU official statistics plugin"""

    name = "eurostat"
    version = "1.0.0"
    description = "Official European statistics (GDP, unemployment, population, trade)"

    BASE_URL = "https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data"

    # High-value datasets
    DATASETS = {
        "gdp": "nama_10_gdp",                     # GDP and main aggregates
        "population": "demo_pjan",                # Population on 1 January
        "unemployment": "une_rt_a",               # Unemployment rate
        "govt_debt": "gov_10dd_edpt1",            # Government debt
    }

    COUNTRY_MAP = {
        "france": "FR", "germany": "DE", "spain": "ES", "italy": "IT",
        "belgium": "BE", "netherlands": "NL", "poland": "PL",
        "portugal": "PT", "greece": "EL", "austria": "AT", "sweden": "SE",
        "denmark": "DK", "finland": "FI", "ireland": "IE",
        "czechia": "CZ", "czech republic": "CZ", "romania": "RO",
        "hungary": "HU", "slovakia": "SK", "croatia": "HR",
        "bulgaria": "BG", "estonia": "EE", "latvia": "LV", "lithuania": "LT",
        "luxembourg": "LU", "malta": "MT", "cyprus": "CY", "slovenia": "SI",
    }

    async def validate(self) -> bool:
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(
                    f"{self.BASE_URL}/{self.DATASETS['gdp']}",
                    params={"format": "JSON", "geo": "FR", "time": "2023", "na_item": "B1GQ", "unit": "CP_MEUR"},
                    timeout=aiohttp.ClientTimeout(total=10),
                ) as resp:
                    return resp.status == 200
        except Exception:
            return False

    async def fetch(self, query: Dict[str, Any]) -> List[Dict[str, Any]]:
        """
        Fetch economic indicators for a European country.

        query:
          q / query: country name (e.g. "France")
          indicator: gdp | population | unemployment | govt_debt (default: all)
        """
        results: List[Dict[str, Any]] = []

        raw = (query.get("q") or query.get("query") or "").strip().lower()
        indicator_filter = query.get("indicator")

        geo = self.COUNTRY_MAP.get(raw)
        if not geo:
            return results

        country_name = raw.title()
        country_id = f"eurostat_country_{geo}"

        # Emit country entity once
        results.append(Entity(
            id=country_id,
            name=country_name,
            entity_type="Country",
            properties={"source": "Eurostat", "geo_code": geo},
            sources=["https://ec.europa.eu/eurostat"],
        ).model_dump())

        indicators_to_fetch = [indicator_filter] if indicator_filter else list(self.DATASETS.keys())

        async with aiohttp.ClientSession() as session:
            for ind in indicators_to_fetch:
                dataset = self.DATASETS.get(ind)
                if not dataset:
                    continue

                params = {
                    "format": "JSON",
                    "geo": geo,
                    "time": "2023",
                    "lang": "EN",
                }
                # Dataset-specific filters (minimal to get latest aggregate)
                if ind == "gdp":
                    params.update({"na_item": "B1GQ", "unit": "CP_MEUR"})
                elif ind == "unemployment":
                    params.update({"age": "TOTAL", "sex": "T", "unit": "PC_POP"})
                elif ind == "population":
                    params.update({"age": "TOTAL", "sex": "T"})

                try:
                    async with session.get(
                        f"{self.BASE_URL}/{dataset}",
                        params=params,
                        timeout=aiohttp.ClientTimeout(total=15),
                    ) as resp:
                        if resp.status != 200:
                            continue
                        data = await resp.json()
                except Exception:
                    continue

                # JSON-stat: extract a simple scalar (latest value)
                value = self._extract_scalar(data)
                if value is None:
                    continue

                # Attach as a separate property entity (light pattern: one entity per indicator)
                ind_id = f"{country_id}_{ind}_2023"
                results.append(Entity(
                    id=ind_id,
                    name=f"{country_name} — {ind.replace('_', ' ').title()} 2023",
                    entity_type="Indicator",
                    properties={
                        "source": "Eurostat",
                        "indicator": ind,
                        "year": 2023,
                        "value": value,
                        "country": country_name,
                        "geo_code": geo,
                    },
                    sources=[f"https://ec.europa.eu/eurostat/databrowser/view/{dataset}/default/table"],
                ).model_dump())

                # Relationship country → indicator
                results.append({
                    "from_entity": country_id,
                    "to_entity": ind_id,
                    "relationship_type": "has_indicator",
                    "properties": {"year": 2023, "indicator": ind, "value": value},
                    "confidence": 1.0,
                    "sources": ["https://ec.europa.eu/eurostat"],
                })

        return results

    def _extract_scalar(self, jsonstat: Dict[str, Any]) -> float | None:
        """Extract first numeric value from JSON-stat v2 response."""
        try:
            values = jsonstat.get("value")
            if isinstance(values, list) and values:
                return values[0]
            if isinstance(values, dict) and values:
                # sorted keys — take first
                k = sorted(values.keys(), key=lambda x: int(x) if str(x).isdigit() else 0)
                if k:
                    return values[k[0]]
        except Exception:
            return None
        return None

    def get_schema(self) -> Dict[str, Any]:
        return {
            "entities": ["Country", "Indicator"],
            "relationships": ["has_indicator"],
            "update_frequency": "annual",
            "coverage": "EU-27 + EFTA + candidates",
        }


register_plugin(EurostatPlugin({}))
