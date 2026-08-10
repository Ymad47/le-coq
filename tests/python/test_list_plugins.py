#!/usr/bin/env python3

import sys
import json
import os

# Add the plugins directory to the path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'plugins'))

try:
    # Import plugins to register them
    import plugins.sources.wikipedia
    
    from plugins import list_plugins
    plugins = list_plugins()
    print(json.dumps(list(plugins)))
except Exception as e:
    print(json.dumps({'error': str(e)}))