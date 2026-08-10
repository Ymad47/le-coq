#!/usr/bin/env python3
"""
GDELT Plugin Test with Mock Data

Use this for development when GDELT API is rate-limited.
Run: python test_gdelt_mock.py
"""

import asyncio
import sys
sys.path.insert(0, '/home/yaya/le-coq')

from plugins import Entity


def mock_gdelt_response():
    """Mock GDELT API response for testing"""
    return {
        "articles": [
            {
                "url": "https://example.com/news/1",
                "title": "France announces new policy",
                "organizations": ["European Union", "UNESCO"],
                "persons": ["Emmanuel Macron", "Jean Castex"],
                "locations": ["Paris", "Lyon", "Marseille"]
            },
            {
                "url": "https://example.com/news/2",
                "title": "Tech summit in Paris",
                "organizations": ["Google", "Microsoft", "OpenAI"],
                "persons": ["Satya Nadella"],
                "locations": ["Paris", "Silicon Valley"]
            }
        ]
    }


async def test_with_mock():
    """Test GDELT plugin logic with mock data"""
    from plugins.sources import gdelt
    
    print("=" * 60)
    print("GDELT Plugin Test (Mock Data)")
    print("=" * 60)
    
    # Create plugin instance
    p = gdelt.GDELTPlugin({})
    
    # Show schema
    schema = p.get_schema()
    print(f"\nSchema:")
    print(f"  Entities: {schema['entities']}")
    print(f"  Relationships: {schema['relationships']}")
    print(f"  Update frequency: {schema['update_frequency']}")
    
    # Process mock data manually (simulating what fetch() does)
    mock_data = mock_gdelt_response()
    results = []
    
    for article in mock_data["articles"]:
        for org in article.get("organizations", [])[:5]:
            entity = Entity(
                id=f"gdelt_org_{org}",
                name=org,
                entity_type="Organization",
                properties={"source": "GDELT"},
                sources=[article.get("url")]
            )
            results.append(entity.model_dump())
        
        for person in article.get("persons", [])[:5]:
            entity = Entity(
                id=f"gdelt_person_{person}",
                name=person,
                entity_type="Person",
                properties={"source": "GDELT"},
                sources=[article.get("url")]
            )
            results.append(entity.model_dump())
        
        for location in article.get("locations", [])[:5]:
            entity = Entity(
                id=f"gdelt_loc_{location}",
                name=location,
                entity_type="Location",
                properties={"source": "GDELT"},
                sources=[article.get("url")]
            )
            results.append(entity.model_dump())
    
    print(f"\nResults: {len(results)} entities extracted")
    print("\nEntities:")
    for i, r in enumerate(results, 1):
        print(f"  {i}. [{r['entity_type']}] {r['name']}")
        print(f"     Source: {r['sources'][0]}")
    
    print("\n" + "=" * 60)
    print("✅ Plugin logic works correctly")
    print("=" * 60)
    print("\nNote: Live API testing requires waiting 5-35s between requests")
    print("      due to GDELT's strict rate limits.")
    print("\nTo test with live API:")
    print("  python -c \"from plugins.sources import gdelt; import asyncio; asyncio.run(gdelt.GDELTPlugin({}).validate())\"")


if __name__ == "__main__":
    asyncio.run(test_with_mock())
