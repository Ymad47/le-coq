#!/usr/bin/env python3

import sys
import json
import asyncio
import os

# Add the plugins directory to the path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'plugins'))

async def test_wikipedia_plugin():
    try:
        # Import the specific plugin to register it
        import plugins.sources.wikipedia
        
        # Import the plugin system
        from plugins import get_plugin
        plugin = get_plugin('wikipedia')
        
        if plugin is None:
            raise Exception('Unknown plugin: wikipedia')
        
        result = await plugin.fetch({'query': 'test'})
        
        # Return the result as JSON
        print(json.dumps({
            'success': True,
            'data': result
        }))
    except Exception as e:
        print(json.dumps({
            'success': False,
            'error': str(e)
        }))

if __name__ == '__main__':
    asyncio.run(test_wikipedia_plugin())