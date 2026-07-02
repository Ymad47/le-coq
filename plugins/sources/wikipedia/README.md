# Wikipedia Plugin for Le Coq

This plugin provides access to Wikipedia data for the Le Coq OSINT framework.

## Features

- Search Wikipedia for topics, people, organizations, and locations
- Retrieve detailed information including descriptions, URLs, and metadata
- Real-time data access with no rate limiting for public use
- Integration with the Le Coq plugin system

## Usage

The plugin can be used through the Le Coq CLI or directly in Python code:

```python
from plugins.sources.wikipedia import WikipediaPlugin

# Create plugin instance
plugin = WikipediaPlugin({})

# Search for entities
results = await plugin.fetch({"q": "Emmanuel Macron"})

# Access entity data
for entity in results:
    print(f"Name: {entity['name']}")
    print(f"Description: {entity['properties']['description']}")
    print(f"URL: {entity['properties']['url']}")
```

## Data Schema

The plugin returns entities with the following structure:

- **ID**: Unique identifier (wiki_{title})
- **Name**: Entity name
- **Type**: Topic, Person, Organization, or Location
- **Properties**:
  - description: Brief description from Wikipedia
  - url: Wikipedia page URL
  - thumbnail: Thumbnail image URL (if available)
  - page_id: Wikipedia page ID
  - wordcount: Number of words in the article
  - size: Size of the article in bytes
  - snippet: Search result snippet

## API Information

- **Base URL**: https://en.wikipedia.org/api/rest_v1/
- **Search Endpoint**: https://en.wikipedia.org/w/api.php
- **Rate Limit**: Unlimited for public use
- **Update Frequency**: Real-time

## Requirements

- aiohttp
- pydantic (from Le Coq core)

## License

This plugin is part of the Le Coq project and is licensed under AGPL-3.0.