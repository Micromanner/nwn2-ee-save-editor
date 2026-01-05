"""Singleton pattern for DynamicGameDataLoader to prevent redundant initialization."""
import threading
from typing import Optional
from loguru import logger
from .dynamic_game_data_loader import DynamicGameDataLoader
from utils.performance_profiler import get_profiler


_loader_instance: Optional[DynamicGameDataLoader] = None
_loader_lock = threading.Lock()


def get_dynamic_game_data_loader(force_reload: bool = False, resource_manager=None, progress_callback=None) -> DynamicGameDataLoader:
    """Get the singleton DynamicGameDataLoader instance."""
    global _loader_instance
    
    if _loader_instance is not None and not force_reload:
        return _loader_instance
    
    with _loader_lock:
        if _loader_instance is None or force_reload:
            profiler = get_profiler()
            
            with profiler.profile("Create DynamicGameDataLoader Singleton"):
                logger.info("Creating singleton DynamicGameDataLoader instance...")
                
                _loader_instance = DynamicGameDataLoader(
                    resource_manager=resource_manager,
                    use_async=False,
                    priority_only=False,
                    validate_relationships=True,
                    progress_callback=progress_callback
                )
                
                profiler.add_metadata("table_count", len(_loader_instance.table_data))
                logger.info(f"DynamicGameDataLoader singleton created with {len(_loader_instance.table_data)} tables")
            
    return _loader_instance


def clear_loader_cache():
    """Clear the cached loader instance."""
    global _loader_instance
    
    with _loader_lock:
        if _loader_instance:
            logger.info("Clearing DynamicGameDataLoader singleton cache")
            _loader_instance = None


def is_loader_ready() -> bool:
    """Check if the singleton loader is ready without creating it."""
    if _loader_instance is None:
        return False
    return _loader_instance.is_ready()


def wait_for_loader_ready(timeout: float = 30.0, check_interval: float = 0.1) -> bool:
    """Wait for the singleton loader to be ready."""
    loader = get_dynamic_game_data_loader()
    return loader.wait_for_ready(timeout, check_interval)