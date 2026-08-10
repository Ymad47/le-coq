"""
UN Comtrade Plugin — bilateral trade flows between countries

API: https://comtradeapi.un.org/public/v1/preview/C/A/HS
Free tier with email registration: 10,000 requests/day.
No key required for preview endpoint (limited to 500 records).

Data model:
- Reporter country → Partner country
- Trade flow: exports / imports (in USD)
- Returns country entities + trade relationships
"""

import aiohttp
from typing import Dict, List, Any
from plugins import DataSourcePlugin, Entity, register_plugin


class UNComtradePlugin(DataSourcePlugin):
    """UN Comtrade bilateral trade flows plugin"""

    name = "uncomtrade"
    version = "1.0.0"
    description = "UN Commodity Trade Statistics — country↔country trade flows (geopolitical relations)"

    BASE_URL = "https://comtradeapi.un.org/public/v1/preview"

    # ISO3 country codes commonly queried
    COUNTRY_CODES = {
        "france": "FRA", "germany": "DEU", "usa": "USA", "united states": "USA",
        "china": "CHN", "russia": "RUS", "uk": "GBR", "united kingdom": "GBR",
        "spain": "ESP", "italy": "ITA", "belgium": "BEL", "netherlands": "NLD",
        "poland": "POL", "turkey": "TUR", "japan": "JPN", "india": "IND",
        "brazil": "BRA", "south africa": "ZAF", "saudi arabia": "SAU",
        "ukraine": "UKR", "israel": "ISR", "egypt": "EGY", "morocco": "MAR",
        "algeria": "DZA", "nigeria": "NGA", "canada": "CAN", "mexico": "MEX",
        "australia": "AUS", "south korea": "KOR", "korea": "KOR",
    }

    # M49 numeric code → country name (main trading partners; preview API returns codes only)
    M49_NAMES = {
        "0": "World", "156": "China", "842": "United States", "276": "Germany",
        "251": "France", "826": "United Kingdom", "380": "Italy", "724": "Spain",
        "528": "Netherlands", "56": "Belgium", "616": "Poland", "792": "Turkey",
        "392": "Japan", "699": "India", "76": "Brazil", "710": "South Africa",
        "682": "Saudi Arabia", "804": "Ukraine", "376": "Israel", "818": "Egypt",
        "504": "Morocco", "554": "New Zealand", "36": "Australia", "410": "South Korea",
        "124": "Canada", "484": "Mexico", "566": "Nigeria", "12": "Algeria",
        "756": "Switzerland", "40": "Austria", "208": "Denmark", "246": "Finland",
        "300": "Greece", "372": "Ireland", "578": "Norway", "620": "Portugal",
        "752": "Sweden", "203": "Czech Republic", "348": "Hungary", "642": "Romania",
        "100": "Bulgaria", "191": "Croatia", "703": "Slovakia", "705": "Slovenia",
        "233": "Estonia", "428": "Latvia", "440": "Lithuania", "643": "Russia",
        "784": "United Arab Emirates", "634": "Qatar", "414": "Kuwait", "364": "Iran",
        "368": "Iraq", "400": "Jordan", "422": "Lebanon", "760": "Syria",
        "887": "Yemen", "760": "Syria", "512": "Oman", "48": "Bahrain",
    }

    async def validate(self) -> bool:
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(
                    f"{self.BASE_URL}/C/A/HS",
                    params={"reporterCode": "251", "period": "2023", "partnerCode": "0"},
                    timeout=aiohttp.ClientTimeout(total=10),
                ) as resp:
                    return resp.status in (200, 400, 401)  # endpoint reachable
        except Exception:
            return False

    def _to_country_code(self, name: str) -> str | None:
        """Normalize to ISO3 code."""
        n = name.strip().lower()
        if n.upper() in self.COUNTRY_CODES.values():
            return n.upper()
        return self.COUNTRY_CODES.get(n)

    async def fetch(self, query: Dict[str, Any]) -> List[Dict[str, Any]]:
        """
        Fetch trade flows for a country.

        query:
          q / query: country name or ISO3 code (e.g. "France", "FRA")
          year: e.g. 2023 (default 2023)
          flow: exports= X, imports= M (default both summarized)
        """
        results: List[Dict[str, Any]] = []

        raw = query.get("q") or query.get("query") or ""
        year = query.get("year", 2023)

        country_code = self._to_country_code(raw)
        print(f"[uncomtrade] raw={raw!r} -> iso3={country_code}")
        if not country_code:
            # Unknown country — return empty
            return results

        # UN Comtrade numeric reporter codes (subset; preview endpoint accepts ISO3 for reporterArea?)
        # Preview endpoint accepts numeric M49 codes. Map ISO3 → M49 for top reporters.
        # For simplicity & robustness, we skip fetch unless mapping exists.
        M49 = {
            "FRA": "251", "DEU": "276", "USA": "842", "CHN": "156", "RUS": "643",
            "GBR": "826", "ESP": "724", "ITA": "380", "BEL": "56", "NLD": "528",
            "POL": "616", "TUR": "792", "JPN": "392", "IND": "699", "BRA": "76",
            "ZAF": "710", "SAU": "682", "UKR": "804", "ISR": "376", "EGY": "818",
            "MAR": "504", "DZA": "12", "NGA": "566", "CAN": "124", "MEX": "484",
            "AUS": "36", "KOR": "410",
        }
        reporter = M49.get(country_code)
        if not reporter:
            return results

        async with aiohttp.ClientSession() as session:
            for flow_code, flow_label in (("X", "exports_to"), ("M", "imports_from")):
                params = {
                    "reporterCode": reporter,
                    "period": str(year),
                    # partnerCode omitted → all bilateral partners (each record = one partner)
                    "flowCode": flow_code,
                    "partner2Code": "0",
                    "customsCode": "C00",
                    "motCode": "0",
                    "maxRecords": "500",
                    "format": "json",
                }
                try:
                    async with session.get(
                        f"{self.BASE_URL}/C/A/HS",
                        params=params,
                        timeout=aiohttp.ClientTimeout(total=20),
                    ) as resp:
                        if resp.status != 200:
                            continue
                        data = await resp.json(content_type=None)  # API returns empty Content-Type
                except Exception:
                    continue

                records = data.get("data") or []
                for rec in records[:30]:
                    # Preview API returns codes only — map M49 code to country name
                    partner_code = str(rec.get("partnerCode") or "")
                    partner_name = self.M49_NAMES.get(partner_code)
                    reporter_name = rec.get("reporterDesc") or raw.title()
                    value_usd = rec.get("primaryValue")
                    if not partner_name or partner_name == "World":
                        continue

                    # Emit country entities (dedupe via id)
                    reporter_id = f"uncomtrade_country_{reporter_name.replace(' ', '_')}"
                    partner_id = f"uncomtrade_country_{partner_name.replace(' ', '_')}"

                    results.append(Entity(
                        id=reporter_id,
                        name=reporter_name,
                        entity_type="Country",
                        properties={"source": "UN Comtrade", "iso3": country_code},
                        sources=["https://comtrade.un.org/"],
                    ).model_dump())

                    results.append(Entity(
                        id=partner_id,
                        name=partner_name,
                        entity_type="Country",
                        properties={"source": "UN Comtrade"},
                        sources=["https://comtrade.un.org/"],
                    ).model_dump())

                    # Trade relationship
                    results.append({
                        "from_entity": reporter_id,
                        "to_entity": partner_id,
                        "relationship_type": flow_label,
                        "properties": {
                            "year": year,
                            "value_usd": value_usd,
                            "flow": flow_code,
                            "source": "UN Comtrade",
                        },
                        "confidence": 1.0,
                        "sources": ["https://comtrade.un.org/"],
                    })

        return results

    def get_schema(self) -> Dict[str, Any]:
        return {
            "entities": ["Country"],
            "relationships": ["exports_to", "imports_from"],
            "update_frequency": "annual",
            "coverage": "Bilateral trade flows USD (UN member states)",
        }


register_plugin(UNComtradePlugin({}))
