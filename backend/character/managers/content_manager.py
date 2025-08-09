"""
Content Manager - handles custom content detection logic
Manages detection and tracking of custom content (feats, spells, classes) vs vanilla content
"""

from typing import Dict, List, Any, Optional, TYPE_CHECKING
import logging

from ..events import EventEmitter

if TYPE_CHECKING:
    from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader

logger = logging.getLogger(__name__)


class ContentManager(EventEmitter):
    """Manages custom content detection and tracking"""
    
    def __init__(self, character_manager):
        """
        Initialize the ContentManager
        
        Args:
            character_manager: Reference to parent CharacterManager
        """
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.game_data_loader = character_manager.game_data_loader
        
        # Custom content tracking - initialized from character manager
        self.custom_content: Dict[str, Dict[str, Any]] = {}
        
        logger.info("ContentManager initialized")
    
    def is_custom_content(self, content_type: str, content_id: int) -> bool:
        """
        Check if a specific content ID is custom content
        
        Args:
            content_type: Type of content ('feat', 'spell', 'class', etc.)
            content_id: ID of the content
            
        Returns:
            True if the content is custom, False if vanilla
        """
        key = f"{content_type}_{content_id}"
        return key in self.custom_content
    
    def _detect_custom_content_dynamic(self):
        """
        Detect custom content using dynamic game data validation
        Uses DynamicGameDataLoader to determine what's vanilla vs custom
        """
        self.custom_content = {}
        
        # Check feats using dynamic data
        feat_list = self.gff.get('FeatList', [])
        for i, feat in enumerate(feat_list):
            if isinstance(feat, dict):
                feat_id = feat.get('Feat', 0)
                if not self._is_vanilla_content('feat', feat_id):
                    feat_name = self._get_content_name('feat', feat_id)
                    self.custom_content[f'feat_{feat_id}'] = {
                        'type': 'feat',
                        'id': feat_id,
                        'name': feat_name,
                        'index': i,
                        'protected': True,
                        'source': self._detect_content_source_dynamic('feat', feat_id)
                    }
        
        # Check spells using dynamic data
        for spell_level in range(10):  # Levels 0-9
            spell_list = self.gff.get(f'KnownList{spell_level}', [])
            for i, spell in enumerate(spell_list):
                if isinstance(spell, dict):
                    spell_id = spell.get('Spell', 0)
                    if not self._is_vanilla_content('spells', spell_id):
                        spell_name = self._get_content_name('spells', spell_id)
                        self.custom_content[f'spell_{spell_id}'] = {
                            'type': 'spell',
                            'id': spell_id,
                            'name': spell_name,
                            'level': spell_level,
                            'index': i,
                            'protected': True,
                            'source': self._detect_content_source_dynamic('spells', spell_id)
                        }
        
        # Check classes using dynamic data
        for class_entry in self.gff.get('ClassList', []):
            if isinstance(class_entry, dict):
                class_id = class_entry.get('Class', 0)
                if not self._is_vanilla_content('classes', class_id):
                    class_name = self._get_content_name('classes', class_id)
                    self.custom_content[f'class_{class_id}'] = {
                        'type': 'class',
                        'id': class_id,
                        'name': class_name,
                        'level': class_entry.get('ClassLevel', 0),
                        'protected': True,
                        'source': self._detect_content_source_dynamic('classes', class_id)
                    }
    
    def _is_vanilla_content(self, table_name: str, content_id: int) -> bool:
        """
        Check if content ID exists in vanilla game data using DynamicGameDataLoader
        
        Args:
            table_name: 2DA table name (feat, spells, classes, etc.)
            content_id: ID to check
            
        Returns:
            True if content exists in loaded game data
        """
        try:
            content_data = self.game_data_loader.get_by_id(table_name, content_id)
            return content_data is not None
        except Exception:
            return False
    
    def _get_content_name(self, table_name: str, content_id: int) -> str:
        """
        Get content name from game data or fallback to generic name
        
        Args:
            table_name: 2DA table name
            content_id: Content ID
            
        Returns:
            Content name or generic fallback
        """
        try:
            content_data = self.game_data_loader.get_by_id(table_name, content_id)
            if content_data:
                # Try different name fields depending on table
                for name_field in ['name', 'label', 'feat', 'spellname']:
                    if hasattr(content_data, name_field):
                        name = getattr(content_data, name_field)
                        if name and name.strip():
                            return name
        except Exception:
            pass
        
        # Fallback to generic name
        return f"Custom {table_name.title()[:-1]} {content_id}"
    
    def _detect_content_source_dynamic(self, table_name: str, content_id: int) -> str:
        """
        Detect content source using dynamic validation against loaded data
        
        Args:
            table_name: 2DA table name
            content_id: Content ID
            
        Returns:
            Source description
        """
        # If it's not in vanilla data, it's custom
        if not self._is_vanilla_content(table_name, content_id):
            return "custom-mod"
        return "vanilla"
    
    def get_custom_content_summary(self) -> Dict[str, Any]:
        """
        Get a summary of all detected custom content
        
        Returns:
            Dictionary with custom content statistics and details
        """
        summary = {
            'total_count': len(self.custom_content),
            'by_type': {},
            'items': []
        }
        
        # Count by type
        for item in self.custom_content.values():
            content_type = item['type']
            if content_type not in summary['by_type']:
                summary['by_type'][content_type] = 0
            summary['by_type'][content_type] += 1
            
            # Add item details
            summary['items'].append({
                'type': item['type'],
                'id': item['id'],
                'name': item['name'],
                'source': item['source']
            })
        
        return summary
    
    def refresh_custom_content(self) -> None:
        """Re-detect custom content (useful after character changes)"""
        logger.info("Refreshing custom content detection")
        old_count = len(self.custom_content)
        self._detect_custom_content_dynamic()
        new_count = len(self.custom_content)
        logger.info(f"Custom content refreshed: {old_count} -> {new_count} items")
    
    def get_custom_content_by_type(self, content_type: str) -> List[Dict[str, Any]]:
        """
        Get all custom content items of a specific type
        
        Args:
            content_type: Type of content to filter by
            
        Returns:
            List of custom content items of the specified type
        """
        return [
            item for item in self.custom_content.values()
            if item['type'] == content_type
        ]
    
    def validate(self) -> tuple[bool, List[str]]:
        """
        Validate custom content state
        
        Returns:
            (is_valid, list_of_errors)
        """
        errors = []
        
        # Basic validation - ensure custom content dict is consistent
        try:
            for key, item in self.custom_content.items():
                # Check that key matches content
                expected_key = f"{item['type']}_{item['id']}"
                if key != expected_key:
                    errors.append(f"Inconsistent key {key} for {item['type']} {item['id']}")
                
                # Check required fields
                required_fields = ['type', 'id', 'name', 'source']
                for field in required_fields:
                    if field not in item:
                        errors.append(f"Missing {field} in custom content item {key}")
        except Exception as e:
            errors.append(f"Error validating custom content: {str(e)}")
        
        return len(errors) == 0, errors