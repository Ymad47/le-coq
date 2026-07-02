#!/usr/bin/env python3
"""
Test script for Le Coq plugins
"""

import sys
import os
import asyncio

# Add the plugins directory to the path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from plugins import get_plugin, list_plugins


async def test_plugins():
    """Test all registered plugins"""
    print("Available plugins:", list_plugins())
    
    # Test Wikipedia plugin
    wiki_plugin = get_plugin("wikipedia")
    if wiki_plugin:
        print(f"\nTesting {wiki_plugin.name} plugin...")
        
        # Validate connection
        is_valid = await wiki_plugin.validate()
        print(f"API connection: {'OK' if is_valid else 'FAILED'}")
        
        if is_valid:
            # Test search
            results = await wiki_plugin.fetch({"q": "Emmanuel Macron"})
            print(f"Found {len(results)} results")
            if results:
                print("First result:")
                print(f"  Name: {results[0]['name']}")
                print(f"  Type: {results[0]['entity_type']}")
                print(f"  Description: {results[0]['properties'].get('description', '')[:100]}...")
    
    # Test OpenCorporates plugin
    oc_plugin = get_plugin("opencorporates")
    if oc_plugin:
        print(f"\nTesting {oc_plugin.name} plugin...")
        print(f"Plugin description: {oc_plugin.description}")


if __name__ == "__main__":
    asyncio.run(test_plugins())