"""Factory functions for creating CharacterManager instances."""

from typing import Dict, Any, Optional

from loguru import logger

from .character_manager import CharacterManager
from .manager_registry import get_all_manager_specs
from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader
from services.gamedata.game_rules_service import GameRulesService


def create_character_manager(
    character_data: Dict[str, Any],
    game_data_loader: Optional[DynamicGameDataLoader] = None,
    rules_service: Optional[GameRulesService] = None,
    lazy: bool = True,
    save_path: Optional[str] = None,
) -> CharacterManager:
    """Create a fully-configured CharacterManager with all managers registered."""
    # Create the base CharacterManager
    manager = CharacterManager(
        character_data,
        game_data_loader=game_data_loader,
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
    game_data_loader: Optional[DynamicGameDataLoader] = None,
    rules_service: Optional[GameRulesService] = None,
    lazy: bool = True,
) -> CharacterManager:
    """Get a cached CharacterManager or create a new one using the factory."""
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
        game_data_loader=game_data_loader,
        rules_service=rules_service,
        lazy=lazy
    )
    
    # Cache it in thread-local cache only
    set_character_manager(character_id, manager)  # Thread-local cache
    
    return manager


def invalidate_character_cache(character_id: int, clear_thread_local: bool = True) -> None:
    """Invalidate cached CharacterManager instances for a character."""
    from gamedata.middleware import clear_character_manager
    
    if clear_thread_local:
        clear_character_manager(character_id)
        logger.info(f"Cleared thread-local cache for character {character_id}")