"""
Game Data Loader - utility functions for getting game data loader instances
"""

from typing import Optional
import logging

logger = logging.getLogger(__name__)

def get_game_data_loader():
    """
    Get a singleton instance of the dynamic game data loader
    
    Returns:
        DynamicGameDataLoader instance or None if creation fails
    """
    try:
        from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader
        return get_dynamic_game_data_loader()
    except Exception as e:
        logger.error(f"Failed to get dynamic game data loader: {e}")
        return None