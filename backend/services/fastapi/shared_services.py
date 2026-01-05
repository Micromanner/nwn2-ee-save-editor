"""Independent registry for shared services to avoid circular imports."""

from typing import Optional, Any, Dict
from loguru import logger

# Independent shared services registry - populated by fastapi_server at startup
_shared_registry: Dict[str, Any] = {}

def register_shared_service(name: str, service: Any) -> None:
    """Register a shared service (called by fastapi_server during startup)."""
    global _shared_registry
    _shared_registry[name] = service
    logger.debug(f"Registered shared service: {name}")

def get_shared_resource_manager() -> Optional[Any]:
    """Get the shared ResourceManager instance without triggering fastapi_server imports."""
    return _shared_registry.get('resource_manager')

def get_shared_game_data_loader() -> Optional[Any]:
    """Get the shared game data loader instance."""
    return _shared_registry.get('game_data_loader')

def get_shared_service(name: str) -> Optional[Any]:
    """Get any shared service by name."""
    return _shared_registry.get(name)

def clear_shared_services() -> None:
    """Clear all shared services (useful for testing)."""
    global _shared_registry
    _shared_registry.clear()
    logger.debug("Cleared all shared services")

def list_shared_services() -> Dict[str, str]:
    """List all registered shared services."""
    return {name: type(service).__name__ for name, service in _shared_registry.items()}

def is_service_available(name: str) -> bool:
    """Check if a specific service is available."""
    return name in _shared_registry and _shared_registry[name] is not None