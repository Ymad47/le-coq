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
