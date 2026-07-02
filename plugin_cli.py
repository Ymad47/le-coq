#!/usr/bin/env python3
"""
Le Coq Plugin CLI - Demonstration script
"""

import sys
import os
import asyncio
import argparse

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
    print(f"Searching Wikipedia for: {query}")
    
    # Get the Wikipedia plugin
    plugin = get_plugin("wikipedia")
    if not plugin:
        print("Error: Wikipedia plugin not found")
        return
    
    # Validate connection
    print("Validating API connection...")
    is_valid = await plugin.validate()
    if not is_valid:
        print("Error: Cannot connect to Wikipedia API")
        return
    
    # Perform search
    print("Performing search...")
    results = await plugin.fetch({"q": query})
    
    if not results:
        print("No results found")
        return
    
    print(f"\nFound {len(results)} results:\n")
    
    # Display results
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


def main():
    parser = argparse.ArgumentParser(description="Le Coq Plugin CLI Demo")
    parser.add_argument("command", choices=["search", "list"], help="Command to execute")
    parser.add_argument("query", nargs="?", help="Search query (for search command)")
    
    args = parser.parse_args()
    
    if args.command == "list":
        print("Available plugins:")
        for plugin_name in list_plugins():
            plugin = get_plugin(plugin_name)
            if plugin:
                print(f"  - {plugin_name}: {plugin.description}")
        return
    
    elif args.command == "search":
        if not args.query:
            print("Error: Search query is required")
            return
        
        # Run the async search function
        asyncio.run(search_wikipedia(args.query))


if __name__ == "__main__":
    main()