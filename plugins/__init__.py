"""
Le Coq Plugin System

Base class and registry for OSINT data source plugins.
"""

from abc import ABC, abstractmethod
from typing import Dict, List, Optional, Any
from datetime import datetime
from pydantic import BaseModel


class DataSourcePlugin(ABC):
    """Base class for Le Coq data source plugins"""
    
    name: str = "base"
    version: str = "1.0.0"
    description: str = "Base data source plugin"
    
    def __init__(self, config: Dict[str, Any]):
        self.config = config
        self.api_key = config.get("api_key")
        self.rate_limit = config.get("rate_limit", 100)  # requests per minute
    
    @abstractmethod
    async def fetch(self, query: Dict[str, Any]) -> List[Dict[str, Any]]:
        """
        Fetch data from source
        
        Args:
            query: Dict with search parameters
            
        Returns:
            List of entity/relationship dicts
        """
        pass
    
    @abstractmethod
    async def validate(self) -> bool:
        """Validate API connection / credentials"""
        pass
    
    def get_schema(self) -> Dict[str, Any]:
        """Return expected data schema"""
        return {
            "entities": ["Person", "Organization"],
            "relationships": ["works_for", "owns"],
            "update_frequency": "daily"
        }


class Entity(BaseModel):
    """Standard entity model"""
    id: str
    name: str
    entity_type: str
    aliases: List[str] = []
    properties: Dict[str, Any] = {}
    sources: List[str] = []
    created_at: datetime = None
    
    class Config:
        arbitrary_types_allowed = True
        json_encoders = {
            datetime: lambda v: v.isoformat() if v else None
        }
    
    def __init__(self, **data):
        if 'created_at' not in data:
            data['created_at'] = datetime.utcnow()
        super().__init__(**data)


class Relationship(BaseModel):
    """Standard relationship model"""
    id: str
    from_entity: str
    to_entity: str
    relationship_type: str
    properties: Dict[str, Any] = {}
    confidence: float = 1.0
    sources: List[str] = []


class Event(BaseModel):
    """Standard event model"""
    id: str
    title: str
    date: datetime
    location: str
    participants: List[str] = []
    event_type: str = "general"
    description: str = ""
    sources: List[str] = []
    severity: str = "low"
    properties: Dict[str, Any] = {}
    
    class Config:
        arbitrary_types_allowed = True
        json_encoders = {
            datetime: lambda v: v.isoformat() if v else None
        }


# Plugin registry
_plugins: Dict[str, DataSourcePlugin] = {}


def register_plugin(plugin: DataSourcePlugin):
    """Register a plugin instance"""
    _plugins[plugin.name] = plugin


def get_plugin(name: str) -> Optional[DataSourcePlugin]:
    """Get a plugin by name"""
    return _plugins.get(name)


def list_plugins() -> List[str]:
    """List all registered plugin names"""
    return list(_plugins.keys())


def call_plugin_method_sync(plugin_name: str, method: str, args: Dict[str, Any]) -> Any:
    """
    Call a plugin method synchronously (wraps async methods)
    
    Args:
        plugin_name: Name of the plugin
        method: Method name (fetch, validate, etc.)
        args: Method arguments
        
    Returns:
        Method result
    """
    import asyncio
    
    plugin = get_plugin(plugin_name)
    if plugin is None:
        raise ValueError(f"Plugin '{plugin_name}' not found")
    
    method_fn = getattr(plugin, method, None)
    if method_fn is None:
        raise ValueError(f"Method '{method}' not found on plugin '{plugin_name}'")
    
    # Call async method and await result
    if method == "validate":
        coro = method_fn()
    elif method == "fetch":
        coro = method_fn(args)
    else:
        coro = method_fn(**args) if args else method_fn()
    
    # Check if it's a coroutine before awaiting
    import inspect
    if inspect.iscoroutine(coro):
        return asyncio.run(coro)
    else:
        # Synchronous method (already returned a value)
        return coro


# Auto-discover plugins
def _auto_discover():
    """Auto-discover and register plugins from plugins.sources package"""
    import importlib
    from pathlib import Path
    
    sources_dir = Path(__file__).parent / "sources"
    if not sources_dir.exists():
        return
    
    for file_path in sources_dir.glob("*.py"):
        if file_path.name.startswith("_"):
            continue
        
        module_name = f"plugins.sources.{file_path.stem}"
        try:
            importlib.import_module(module_name)
        except Exception as e:
            pass  # Silent fail for auto-discovery


# Auto-discover on import
_auto_discover()
