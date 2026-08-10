#!/usr/bin/env python3
"""
Le Coq Plugin Bridge - Connects Zig CLI to Python plugins
"""

import sys
import os
import asyncio
import json

# Add the plugins directory to the path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'plugins'))

# Import plugins directly to ensure they're registered
from plugins.sources.wikipedia import WikipediaPlugin
from plugins import get_plugin, list_plugins

# Manually register the plugin since auto-registration might not work in this context
from plugins import register_plugin
register_plugin(WikipediaPlugin({}))


async def search_wikipedia(query: str):
    """Search Wikipedia using our plugin"""
    # Get the Wikipedia plugin
    plugin = get_plugin("wikipedia")
    if not plugin:
        print("Error: Wikipedia plugin not found")
        return
    
    # Validate connection
    is_valid = await plugin.validate()
    if not is_valid:
        print("Error: Cannot connect to Wikipedia API")
        return
    
    # Perform search
    results = await plugin.fetch({"q": query})
    return results


async def list_plugins_command():
    """List all available plugins"""
    plugins = []
    for plugin_name in list_plugins():
        plugin = get_plugin(plugin_name)
        if plugin:
            plugins.append({
                "name": plugin_name,
                "description": plugin.description
            })
    return plugins


async def main():
    if len(sys.argv) < 2:
        print("Usage: plugin_bridge.py <command> [args...]")
        return
    
    command = sys.argv[1]
    
    if command == "search":
        if len(sys.argv) < 3:
            print("Usage: plugin_bridge.py search <query>")
            return
        
        query = sys.argv[2]
        results = await search_wikipedia(query)
        
        if results:
            # Print results in a simple format
            print(f"Found {len(results)} results:")
            for i, result in enumerate(results[:5], 1):  # Show top 5 results
                print(f"{i}. {result['name']}")
                print(f"   Type: {result['entity_type']}")
                description = result['properties'].get('description', '')
                if description:
                    # Truncate description to 100 characters
                    truncated = description[:100] + "..." if len(description) > 100 else description
                    print(f"   Description: {truncated}")
                print(f"   URL: {result['properties'].get('url', '')}")
                print()
        else:
            print("No results found")
    
    elif command == "list":
        plugins = await list_plugins_command()
        print("Available plugins:")
        for plugin in plugins:
            print(f"  {plugin['name']}: {plugin['description']}")
    
    else:
        print(f"Unknown command: {command}")
        print("Available commands: search, list")


if __name__ == "__main__":
    asyncio.run(main())