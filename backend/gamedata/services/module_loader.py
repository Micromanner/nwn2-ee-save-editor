"""
Module Game Rules Loader
Loads game rules with module-specific overrides from hakpaks
"""

from typing import Dict, List, Optional, Any
from pathlib import Path
import logging
import copy

from .game_rules_service import GameRulesService
from parsers.resource_manager import ResourceManager
from parsers.gff import GFFParser
from config.nwn2_settings import nwn2_paths, NWN2_SETTINGS

logger = logging.getLogger(__name__)


class ModuleGameRulesLoader:
    """
    Loads game rules with module-specific content overlays
    """
    
    @classmethod
    def load_for_character(cls, character_data: Dict[str, Any]) -> GameRulesService:
        """
        Load appropriate game rules for a character based on its module
        
        Args:
            character_data: Parsed character data (from GFF)
            
        Returns:
            GameRulesService instance with module overrides if applicable
        """
        # Extract module information
        module_name = character_data.get('Mod_Name', '')
        module_entry = character_data.get('Mod_Entry', {})
        
        # Check if this is base game
        if not module_name or module_name == 'OfficialCampaign':
            return game_rules  # Use global base game instance
        
        # Get hakpak list from module entry
        hakpak_list = []
        mod_hak_list = module_entry.get('Mod_HakList', [])
        for hak_entry in mod_hak_list:
            hak_name = hak_entry.get('Mod_Hak', '')
            if hak_name:
                hakpak_list.append(hak_name)
        
        if not hakpak_list:
            # Module but no custom content
            return game_rules
        
        # Load module-specific rules
        return cls.load_for_module(module_name, hakpak_list)
    
    @classmethod
    def load_for_module(cls, module_name: str, hakpak_list: List[str]) -> GameRulesService:
        """
        Load game rules with specific module overrides
        
        Args:
            module_name: Name of the module
            hakpak_list: List of hakpak names
            
        Returns:
            GameRulesService with module content merged
        """
        logger.info(f"Loading game rules for module '{module_name}' with {len(hakpak_list)} hakpaks")
        
        # Use nwn2_paths for NWN2 installation path
        nwn2_path = nwn2_paths.game_folder
        if not nwn2_path.exists():
            logger.warning("NWN2 installation not found - using base game rules only")
            return game_rules
        
        # Create a new GameRulesService instance with custom data
        custom_data = cls._load_module_data(nwn2_path, module_name, hakpak_list)
        
        if not custom_data:
            return game_rules
        
        # Create new instance with overrides
        return GameRulesService(custom_data=custom_data)
    
    @classmethod
    def _load_module_data(cls, nwn2_path: Path, module_name: str, 
                         hakpak_list: List[str]) -> Optional[Dict[str, Any]]:
        """
        Load and merge all module data from hakpaks
        
        Returns:
            Dict with merged custom content or None
        """
        custom_data = {
            'classes': {},
            'feats': {},
            'skills': {},
            'spells': {},
            'races': {},
            'base_items': {},
        }
        
        # Use ResourceManager to load hakpak content
        with ResourceManager() as rm:
            # Find the module by name first
            module_path = rm.find_module(module_name)
            if not module_path:
                logger.warning(f"Module '{module_name}' not found")
                return None
            
            # Load the module and its HAKs
            if not rm.set_module(module_path):
                logger.warning(f"Failed to load module '{module_name}'")
                return None
            
            # Get overridden 2DA files
            override_tables = [
                ('classes', 'classes.2da'),
                ('feats', 'feat.2da'),
                ('skills', 'skills.2da'),
                ('spells', 'spells.2da'),
                ('races', 'racialtypes.2da'),
                ('base_items', 'baseitems.2da'),
            ]
            
            for data_key, tda_name in override_tables:
                tda = rm.get_2da_with_overrides(tda_name)
                if tda and tda != rm.get_2da(tda_name):
                    # This is an override - parse it into our format
                    custom_data[data_key] = cls._parse_2da_to_dict(tda, data_key)
                    logger.info(f"Loaded {len(custom_data[data_key])} custom {data_key}")
        
        # Return None if no custom content found
        has_content = any(custom_data[key] for key in custom_data)
        return custom_data if has_content else None
    
    @classmethod
    def _parse_2da_to_dict(cls, tda_parser, data_type: str) -> Dict[int, Dict[str, Any]]:
        """
        Convert TDA parser data to dict format expected by GameRulesService
        
        Args:
            tda_parser: Parsed 2DA file
            data_type: Type of data (classes, feats, etc.)
            
        Returns:
            Dict mapping ID to data dict
        """
        result = {}
        
        for row_id in range(tda_parser.get_resource_count()):
            row_data = {}
            
            # Get all columns for this row
            for col_name in tda_parser.columns:
                value = tda_parser.get_string(row_id, col_name)
                if value and value != '****':
                    # Try to convert to appropriate type
                    if value.isdigit():
                        row_data[col_name.lower()] = int(value)
                    elif value.replace('.', '').isdigit():
                        row_data[col_name.lower()] = float(value)
                    else:
                        row_data[col_name.lower()] = value
            
            # Add row if it has data
            if row_data:
                # Use label if available, otherwise use row ID
                label = row_data.get('label', f'{data_type}_{row_id}')
                row_data['id'] = row_id
                row_data['label'] = label
                result[row_id] = row_data
        
        return result
    
    @classmethod
    def detect_custom_content(cls, character_data: Dict[str, Any]) -> Dict[str, List[int]]:
        """
        Detect custom content IDs in a character
        
        Args:
            character_data: Parsed character data
            
        Returns:
            Dict mapping content type to list of custom IDs
        """
        custom_ids = {
            'classes': [],
            'feats': [],
            'items': [],
            'spells': [],
        }
        
        # Check classes (custom usually >= 100)
        for class_entry in character_data.get('ClassList', []):
            class_id = class_entry.get('Class', 0)
            if class_id >= 100:
                custom_ids['classes'].append(class_id)
        
        # Check feats (custom usually >= 3000)
        for feat in character_data.get('FeatList', []):
            feat_id = feat.get('Feat', 0)
            if feat_id >= 3000:
                custom_ids['feats'].append(feat_id)
        
        # Check spells (custom usually >= 3000)
        for class_entry in character_data.get('ClassList', []):
            for spell_level in range(10):
                spell_list = class_entry.get(f'KnownList{spell_level}', [])
                for spell in spell_list:
                    spell_id = spell.get('Spell', 0)
                    if spell_id >= 3000:
                        custom_ids['spells'].append(spell_id)
        
        # Check items (would need to check equipment and inventory)
        # Custom items typically have IDs >= 10000
        
        return custom_ids