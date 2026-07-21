#!/usr/bin/env python3

import asyncio
import sys
import os

# Add the plugins directory to the path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'plugins'))

# Import the plugin system
from plugins import get_plugin, list_plugins
from plugins.sources.wikipedia import WikipediaPlugin

async def test_wikipedia_plugin():
    """Test the Wikipedia plugin directly"""
    print("Testing Wikipedia plugin...")
    
    # Create plugin instance
    plugin = WikipediaPlugin({})
    
    # Test validation
    print("Validating plugin...")
    is_valid = await plugin.validate()
    print(f"Plugin validation: {'PASS' if is_valid else 'FAIL'}")
    
    if not is_valid:
        print("Plugin validation failed!")
        return False
    
    # Test search
    print("Testing search...")
    results = await plugin.fetch({"q": "Emmanuel Macron"})
    print(f"Found {len(results)} results")
    
    if results:
        print("First result:")
        print(f"  ID: {results[0].get('id')}")
        print(f"  Name: {results[0].get('name')}")
        print(f"  Type: {results[0].get('entity_type')}")
        print(f"  Description: {results[0].get('properties', {}).get('description', '')[:100]}...")
        return True
    else:
        print("No results found!")
        return False

if __name__ == "__main__":
    success = asyncio.run(test_wikipedia_plugin())
    sys.exit(0 if success else 1)