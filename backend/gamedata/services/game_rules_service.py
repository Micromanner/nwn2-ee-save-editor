"""
Game Rules Service - Combines data access with dynamic rule detection
Provides business logic and validation using data from DynamicGameDataLoader and RuleDetector
"""
from typing import Dict, List, Optional, Any
from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader
from .rule_detector import RuleDetector


class GameRulesService:
    """
    Game rules service that combines data access from GameDataLoader
    with dynamic rule detection from RuleDetector.
    
    Now uses composition instead of inheritance to avoid creating multiple
    DynamicGameDataLoader instances.
    """
    
    def __init__(self, resource_manager: Optional[Any] = None, load_mode: str = 'full'):
        # Get singleton DynamicGameDataLoader instance, passing the ResourceManager
        # This ensures the singleton uses our shared ResourceManager
        self._loader = get_dynamic_game_data_loader(resource_manager=resource_manager)
        
        # Initialize RuleDetector with the loader's ResourceManager
        self.rule_detector = RuleDetector(self._loader.rm)
        
        # Store ResourceManager reference for compatibility
        self.rm = self._loader.rm
    
    # Delegate data access methods to the loader
    def get_table(self, table_name: str) -> List[Any]:
        """Get all instances for a table."""
        import logging
        logger = logging.getLogger(__name__)
        result = self._loader.get_table(table_name)
        logger.debug(f"GameRulesService.get_table('{table_name}'): result = {result}")
        logger.debug(f"GameRulesService.get_table('{table_name}'): result type = {type(result)}")
        if result:
            logger.debug(f"GameRulesService.get_table('{table_name}'): result length = {len(result)}")
        return result
    
    def get_by_id(self, table_name: str, row_id: int) -> Optional[Any]:
        """Get a specific row by ID."""
        return self._loader.get_by_id(table_name, row_id)
    
    def set_module_context(self, module_path: str) -> bool:
        """Set module context for loading module-specific overrides."""
        return self._loader.set_module_context(module_path)
    
    def get_stats(self) -> Dict[str, Any]:
        """Get statistics about loaded data."""
        return self._loader.get_stats()
    
    def get_validation_report(self) -> Optional[Any]:
        """Get the relationship validation report if available."""
        return self._loader.get_validation_report()
    
    def get_table_relationships(self, table_name: str) -> Dict[str, Any]:
        """Get relationship information for a specific table."""
        return self._loader.get_table_relationships(table_name)
    
