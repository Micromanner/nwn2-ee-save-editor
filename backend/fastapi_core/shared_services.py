"""
Shared Services Registry

Independent registry for shared services that doesn't import fastapi_server.
FastAPI server registers services here, character loading accesses them here.
"""

import logging
from typing import Optional, Any, Dict

logger = logging.getLogger(__name__)

# Independent shared services registry - populated by fastapi_server at startup
_shared_registry: Dict[str, Any] = {}

def register_shared_service(name: str, service: Any) -> None:
    """
    Register a shared service (called by fastapi_server during startup).
    
    Args:
        name: Service name (e.g., 'resource_manager', 'game_data_loader')
        service: Service instance
    """
    global _shared_registry
    _shared_registry[name] = service
    logger.debug(f"Registered shared service: {name}")

def get_shared_resource_manager() -> Optional[Any]:
    """
    Get the shared ResourceManager instance without triggering fastapi_server imports.
    
    Returns:
        The shared ResourceManager instance or None if not available
    """
    return _shared_registry.get('resource_manager')

def get_shared_game_data_loader() -> Optional[Any]:
    """
    Get the shared game data loader instance.
    
    Returns:
        The shared game data loader instance or None if not available
    """
    return _shared_registry.get('game_data_loader')

def get_shared_service(name: str) -> Optional[Any]:
    """
    Get any shared service by name.
    
    Args:
        name: Service name
        
    Returns:
        Service instance or None if not available
    """
    return _shared_registry.get(name)

def clear_shared_services() -> None:
    """Clear all shared services (useful for testing)."""
    global _shared_registry
    _shared_registry.clear()
    logger.debug("Cleared all shared services")

def list_shared_services() -> Dict[str, str]:
    """
    List all registered shared services.
    
    Returns:
        Dict mapping service names to their type names
    """
    return {name: type(service).__name__ for name, service in _shared_registry.items()}

def is_service_available(name: str) -> bool:
    """
    Check if a specific service is available.
    
    Args:
        name: Service name
        
    Returns:
        True if service is available, False otherwise
    """
    return name in _shared_registry and _shared_registry[name] is not None