"""
Singleton pattern for DynamicGameDataLoader to prevent redundant initialization.

This ensures only one instance is created per process, dramatically improving performance.
"""
import threading
import logging
from typing import Optional
from .dynamic_game_data_loader import DynamicGameDataLoader
from utils.performance_profiler import get_profiler

logger = logging.getLogger(__name__)

# Thread-safe singleton storage
_loader_instance: Optional[DynamicGameDataLoader] = None
_loader_lock = threading.Lock()


def get_dynamic_game_data_loader(force_reload: bool = False, resource_manager=None, progress_callback=None) -> DynamicGameDataLoader:
    """
    Get the singleton DynamicGameDataLoader instance.
    
    This function ensures only one DynamicGameDataLoader is created per process,
    preventing the ~11 second initialization on every request.
    
    Args:
        force_reload: If True, creates a new instance (useful for testing/reloading)
        resource_manager: ResourceManager instance to use (only used on first creation)
        progress_callback: Optional callback for progress updates (only used on first creation)
        
    Returns:
        The singleton DynamicGameDataLoader instance
    """
    global _loader_instance
    
    # Fast path - if already initialized
    if _loader_instance is not None and not force_reload:
        return _loader_instance
    
    # Thread-safe initialization
    with _loader_lock:
        # Double-check pattern
        if _loader_instance is None or force_reload:
            profiler = get_profiler()
            
            with profiler.profile("Create DynamicGameDataLoader Singleton"):
                logger.info("Creating singleton DynamicGameDataLoader instance...")
                
                # Create with priority_only=False for full data loading
                # This is better than lazy loading since we'll need most data anyway
                _loader_instance = DynamicGameDataLoader(
                    resource_manager=resource_manager,  # Use provided ResourceManager
                    use_async=False,  # Avoid async issues in Django
                    priority_only=False,  # Load all data upfront
                    validate_relationships=True,
                    progress_callback=progress_callback  # Forward progress callback
                )
                
                profiler.add_metadata("table_count", len(_loader_instance.table_data))
                logger.info(f"DynamicGameDataLoader singleton created with {len(_loader_instance.table_data)} tables")
            
    return _loader_instance


def clear_loader_cache():
    """
    Clear the cached loader instance.
    
    Useful for testing or when game data files change.
    """
    global _loader_instance
    
    with _loader_lock:
        if _loader_instance:
            logger.info("Clearing DynamicGameDataLoader singleton cache")
            _loader_instance = None


def is_loader_ready() -> bool:
    """
    Check if the singleton loader is ready without creating it.
    
    Returns:
        True if loader exists and is ready, False otherwise
    """
    if _loader_instance is None:
        return False
    
    return _loader_instance.is_ready()


def wait_for_loader_ready(timeout: float = 30.0, check_interval: float = 0.1) -> bool:
    """
    Wait for the singleton loader to be ready, creating it if necessary.
    
    Args:
        timeout: Maximum time to wait in seconds (default 30s)
        check_interval: How often to check ready status in seconds (default 0.1s)
        
    Returns:
        True if ready within timeout, False if timeout exceeded
        
    Raises:
        RuntimeError: If initialization failed with an error
    """
    # Get or create the loader
    loader = get_dynamic_game_data_loader()
    
    # Wait for it to be ready
    return loader.wait_for_ready(timeout, check_interval)