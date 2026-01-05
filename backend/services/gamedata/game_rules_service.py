"""Combines data access with dynamic rule detection for NWN2 game rules."""

from typing import Dict, List, Optional, Any
from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader
from .rule_detector import RuleDetector


class GameRulesService:
    """Wraps DynamicGameDataLoader with RuleDetector for prerequisite validation."""

    def __init__(self, resource_manager: Optional[Any] = None, load_mode: str = 'full'):
        """Initialize with optional ResourceManager for the singleton loader."""
        self._loader = get_dynamic_game_data_loader(resource_manager=resource_manager)
        self.rule_detector = RuleDetector(self._loader.rm)
        self.rm = self._loader.rm

    def get_table(self, table_name: str) -> List[Any]:
        """Get all instances for a table."""
        return self._loader.get_table(table_name)

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
