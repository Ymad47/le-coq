"""
Tests unitaires pour le système de plugins Le Coq.
"""

import pytest
import asyncio
from plugins import list_plugins, get_plugin, call_plugin_method_sync
from plugins.sources.wikipedia import WikipediaPlugin


class TestPluginRegistry:
    """Tests pour le registre de plugins"""

    def test_list_plugins_not_empty(self):
        """La liste des plugins ne doit pas être vide"""
        plugins = list_plugins()
        assert len(plugins) > 0, "Devrait avoir au moins un plugin"

    def test_list_plugins_contains_wikipedia(self):
        """Wikipedia devrait être dans la liste"""
        plugins = list_plugins()
        assert "wikipedia" in plugins, "Wikipedia devrait être enregistré"

    def test_list_plugins_contains_gdelt(self):
        """GDELT devrait être dans la liste"""
        plugins = list_plugins()
        assert "gdelt" in plugins, "GDELT devrait être enregistré"

    def test_list_plugins_contains_opencorporates(self):
        """OpenCorporates devrait être dans la liste"""
        plugins = list_plugins()
        assert "opencorporates" in plugins, "OpenCorporates devrait être enregistré"

    def test_get_plugin_wikipedia(self):
        """Récupérer le plugin Wikipedia"""
        plugin = get_plugin("wikipedia")
        assert plugin is not None, "Wikipedia plugin devrait exister"
        assert isinstance(plugin, WikipediaPlugin), "Devrait être une instance de WikipediaPlugin"

    def test_get_plugin_nonexistent(self):
        """Plugin inexistant devrait retourner None"""
        plugin = get_plugin("nonexistent")
        assert plugin is None, "Plugin inexistant devrait retourner None"


class TestWikipediaPlugin:
    """Tests pour le plugin Wikipedia"""

    @pytest.mark.asyncio
    async def test_wikipedia_validate(self):
        """Wikipedia.validate() devrait retourner True"""
        plugin = WikipediaPlugin({})
        result = await plugin.validate()
        assert result is True, "Wikipedia devrait être valide (pas de clé API requise)"

    @pytest.mark.asyncio
    async def test_wikipedia_fetch_basic(self):
        """Wikipedia.fetch() devrait retourner des résultats"""
        plugin = WikipediaPlugin({})
        args = {"query": "Rust programming language", "limit": 1}
        result = await plugin.fetch(args)
        
        assert isinstance(result, list), "fetch() devrait retourner une liste"
        # Peut être vide si API rate-limited, mais devrait être une liste

    @pytest.mark.asyncio
    async def test_wikipedia_fetch_empty_query(self):
        """Wikipedia.fetch() avec query vide"""
        plugin = WikipediaPlugin({})
        args = {"query": "", "limit": 1}
        result = await plugin.fetch(args)
        
        assert isinstance(result, list), "fetch() devrait retourner une liste même avec query vide"


class TestCallPluginMethodSync:
    """Tests pour le wrapper synchrone"""

    def test_call_validate_sync(self):
        """Appeler validate() via wrapper synchrone"""
        result = call_plugin_method_sync("wikipedia", "validate", {})
        assert result is True, "validate() devrait retourner True"

    def test_call_fetch_sync(self):
        """Appeler fetch() via wrapper synchrone"""
        args = {"query": "Python programming", "limit": 1}
        result = call_plugin_method_sync("wikipedia", "fetch", args)
        assert isinstance(result, list), "fetch() devrait retourner une liste"

    def test_call_nonexistent_plugin(self):
        """Plugin inexistant devrait lever une erreur"""
        with pytest.raises(ValueError, match="not found"):
            call_plugin_method_sync("nonexistent", "validate", {})

    def test_call_nonexistent_method(self):
        """Méthode inexistante devrait lever une erreur"""
        with pytest.raises(ValueError, match="not found"):
            call_plugin_method_sync("wikipedia", "nonexistent_method", {})


if __name__ == "__main__":
    pytest.main([__file__, "-v"])


class TestCredentialedSources:
    """Sources that need credentials must fail loudly, not return []."""

    async def test_acled_without_credentials_raises(self):
        from plugins.sources.acled import ACLEDPlugin

        plugin = ACLEDPlugin({})
        plugin.email = plugin.password = ""
        plugin._access_token = plugin._refresh_token = None
        with pytest.raises(RuntimeError, match="no credentials"):
            await plugin.fetch({"country": "Niger"})

    async def test_opencorporates_without_token_raises(self):
        from plugins.sources.opencorporates import OpenCorporatesPlugin

        plugin = OpenCorporatesPlugin({"api_key": ""})
        with pytest.raises(RuntimeError, match="no API token"):
            await plugin.fetch({"q": "TotalEnergies"})

    def test_opencorporates_previous_names_are_strings(self):
        from plugins.sources.opencorporates import _previous_names

        company = {"previous_names": [{"company_name": "TOTAL SA"}, {"company_name": None}, "ELF"]}
        assert _previous_names(company) == ["TOTAL SA", "ELF"]


class TestAcledMapping:
    """ACLED events must become items the backend can store."""

    EVENT = {
        "event_id_cnty": "NIG1234",
        "event_date": "2026-09-05",
        "event_type": "Battles",
        "sub_event_type": "Armed clash",
        "location": "Niamey",
        "country": "Niger",
        "latitude": "13.51",
        "longitude": "2.11",
        "fatalities": "12",
        "source": "Le Sahel",
        "notes": "Clash reported.",
    }

    def test_event_becomes_location_event_and_relation(self):
        from plugins.sources.acled import ACLEDPlugin

        loc, ev, rel = ACLEDPlugin({})._event_to_items(self.EVENT)

        for entity in (loc, ev):
            assert {"id", "name", "entity_type", "aliases", "properties", "sources"} <= entity.keys()
        assert loc["entity_type"] == "Location"
        assert ev["entity_type"] == "Event"
        assert ev["published_at"] == "2026-09-05T00:00:00Z"
        assert ev["properties"]["severity"] == "high"
        assert rel["from_entity"] == ev["id"]
        assert rel["to_entity"] == loc["id"]
        assert rel["relationship_type"] == "occurred_in"

    def test_missing_source_gives_empty_sources(self):
        from plugins.sources.acled import ACLEDPlugin

        event = dict(self.EVENT, source=None)
        loc, ev, rel = ACLEDPlugin({})._event_to_items(event)
        assert loc["sources"] == ev["sources"] == rel["sources"] == []
