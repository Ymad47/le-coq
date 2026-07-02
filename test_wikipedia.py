#!/usr/bin/env python3
"""
Debug test script for the Wikipedia plugin
"""

import sys
import os
import asyncio
import aiohttp

# Add the plugins directory to the path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'plugins'))

# Import the plugin directly
from plugins.sources.wikipedia import WikipediaPlugin


async def debug_wikipedia_search():
    """Debug the Wikipedia search directly"""
    print("Debugging Wikipedia search...")
    
    HEADERS = {
        "User-Agent": "Le-Coq-OSINT/1.0 (https://github.com/Ymad47/le-coq) python-aiohttp/3.14"
    }
    
    search_query = "Emmanuel Macron"
    
    try:
        async with aiohttp.ClientSession(headers=HEADERS) as session:
            search_url = f"https://en.wikipedia.org/api/rest_v1/page/search/{search_query}"
            print(f"Searching URL: {search_url}")
            
            async with session.get(
                search_url,
                params={"limit": 10},
                timeout=aiohttp.ClientTimeout(total=30)
            ) as resp:
                print(f"Search status: {resp.status}")
                if resp.status == 200:
                    data = await resp.json()
                    print(f"Search response keys: {list(data.keys())}")
                    if "pages" in data:
                        print(f"Found {len(data['pages'])} pages")
                        for i, page in enumerate(data["pages"][:3]):  # Show first 3
                            print(f"  Page {i+1}: {page.get('title', 'No title')}")
                    else:
                        print("No 'pages' key in response")
                        print(f"Full response: {data}")
                else:
                    text = await resp.text()
                    print(f"Error response: {text[:200]}...")
    except Exception as e:
        print(f"Exception: {e}")


async def test_wikipedia_plugin():
    """Test the Wikipedia plugin"""
    print("Testing Wikipedia plugin...")
    
    # Create plugin instance
    plugin = WikipediaPlugin({})
    
    # Validate connection
    print("Validating API connection...")
    is_valid = await plugin.validate()
    print(f"API connection: {'OK' if is_valid else 'FAILED'}")
    
    if is_valid:
        # Test search
        print("\nTesting search for 'Emmanuel Macron'...")
        results = await plugin.fetch({"q": "Emmanuel Macron"})
        print(f"Found {len(results)} results")
        
        if results:
            print("\nFirst result:")
            result = results[0]
            print(f"  ID: {result['id']}")
            print(f"  Name: {result['name']}")
            print(f"  Type: {result['entity_type']}")
            print(f"  Description: {result['properties'].get('description', '')[:100]}...")
            print(f"  URL: {result['properties'].get('url', '')}")


if __name__ == "__main__":
    asyncio.run(debug_wikipedia_search())
    print("\n" + "="*50 + "\n")
    asyncio.run(test_wikipedia_plugin())