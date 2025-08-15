"""
Factory functions for creating properly configured CharacterManager instances.
"""

from typing import Dict, Any, Optional
from .character_manager import CharacterManager
from .manager_registry import get_all_manager_specs
from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader
from gamedata.services.game_rules_service import GameRulesService
import logging

logger = logging.getLogger(__name__)


def create_character_manager(
    character_data: Dict[str, Any],
    gff_element=None,
    game_data_loader: Optional[DynamicGameDataLoader] = None,
    rules_service: Optional[GameRulesService] = None,
    lazy: bool = True,
    save_path: Optional[str] = None
) -> CharacterManager:
    """
    Factory function that creates a fully-configured CharacterManager with all managers registered.
    
    This ensures consistent manager configuration across all views, enabling proper
    caching and event communication between managers.
    
    Args:
        character_data: Raw GFF character data
        gff_element: Optional GFFElement for direct updates
        game_data_loader: Optional DynamicGameDataLoader instance
        rules_service: Optional GameRulesService instance
        lazy: If True (default), use lazy loading for managers
        save_path: Optional path to save directory (for campaign data extraction)
        
    Returns:
        CharacterManager instance with all managers registered
    """
    # Create the base CharacterManager
    manager = CharacterManager(
        character_data,
        game_data_loader=game_data_loader,
        gff_element=gff_element,
        rules_service=rules_service
    )
    
    # Set save_path if provided (for ContentManager to extract campaign data)
    if save_path:
        manager.save_path = save_path
    
    # Register ALL managers to ensure proper event communication
    # This is critical for caching - all instances must have the same managers
    for name, manager_class in get_all_manager_specs():
        try:
            manager.register_manager(name, manager_class, lazy=lazy)
            logger.debug(f"Registered {name} manager (lazy={lazy})")
        except Exception as e:
            logger.error(f"Failed to register {name} manager: {e}")
            # Continue with other managers rather than failing completely
            # This allows partial functionality if some managers have issues
    
    # Initialize custom content detection through ContentManager after it's registered
    content_manager = manager.get_manager('content')
    if content_manager:
        content_manager._detect_custom_content_dynamic()
        manager.custom_content = content_manager.custom_content
        logger.info(f"Detected {len(manager.custom_content)} custom content items")
    
    logger.info(f"Created CharacterManager with {len(manager._managers)} managers registered")
    
    return manager


def get_or_create_character_manager(
    character_id: int,
    character_data: Dict[str, Any],
    gff_element=None,
    game_data_loader: Optional[DynamicGameDataLoader] = None,
    rules_service: Optional[GameRulesService] = None,
    lazy: bool = True
) -> CharacterManager:
    """
    Get a cached CharacterManager or create a new one using the factory.
    
    This function checks the thread-local cache (request-only) and creates
    a new instance if needed. Persistent caching is now handled at the data
    level, not the CharacterManager instance level.
    
    Args:
        character_id: Character database ID for cache lookup
        character_data: Raw GFF character data (used if creating new)
        gff_element: Optional GFFElement for direct updates
        game_data_loader: Optional DynamicGameDataLoader instance
        rules_service: Optional GameRulesService instance
        lazy: If True (default), use lazy loading for managers
        
    Returns:
        CharacterManager instance with all managers registered
    """
    from gamedata.middleware import get_character_manager, set_character_manager
    
    # Check thread-local cache (request-only)
    cached_manager = get_character_manager(character_id)
    if cached_manager:
        # Verify it has all required managers
        required_managers = [name for name, _ in get_all_manager_specs()]
        has_all = all(name in cached_manager._managers for name in required_managers)
        
        if has_all:
            logger.debug(f"Using thread-local cached CharacterManager for character {character_id}")
            return cached_manager
        else:
            logger.warning(f"Thread-local cached manager for character {character_id} missing managers, creating new")
    
    # Create new manager with factory
    manager = create_character_manager(
        character_data,
        gff_element=gff_element,
        game_data_loader=game_data_loader,
        rules_service=rules_service,
        lazy=lazy
    )
    
    # Cache it in thread-local cache only
    set_character_manager(character_id, manager)  # Thread-local cache
    
    return manager


def invalidate_character_cache(character_id: int, clear_thread_local: bool = True) -> None:
    """
    Invalidate cached CharacterManager instances.
    
    This should be called when:
    - A character is saved to disk
    - Major changes are made that require a full reload
    - Testing requires a fresh instance
    
    Args:
        character_id: Character database ID to invalidate
        clear_thread_local: If True, clear from thread-local cache
        
    Note: With the in-memory save system, persistent caching is no longer needed.
    """
    from gamedata.middleware import clear_character_manager
    
    if clear_thread_local:
        clear_character_manager(character_id)
        logger.info(f"Cleared thread-local cache for character {character_id}")