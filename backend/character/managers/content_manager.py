"""
Content Manager - handles custom content detection logic and campaign/module info
Manages detection and tracking of custom content (feats, spells, classes) vs vanilla content
Also extracts and manages campaign, module, and quest information from save files
"""

from typing import Dict, List, Any, Optional, TYPE_CHECKING
import logging
import os

from ..events import EventEmitter
from gamedata.dynamic_loader.field_mapping_utility import field_mapper  # type: ignore

if TYPE_CHECKING:
    pass

logger = logging.getLogger(__name__)


class ContentManager(EventEmitter):
    """Manages custom content detection, campaign info, and quest tracking"""
    
    def __init__(self, character_manager):
        """
        Initialize the ContentManager
        
        Args:
            character_manager: Reference to parent CharacterManager
        """
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.rules_service = character_manager.rules_service
        
        # Custom content tracking - initialized from character manager
        self.custom_content: Dict[str, Dict[str, Any]] = {}
        
        # Campaign/module/quest data
        self.campaign_data: Dict[str, Any] = {}
        self.module_info: Dict[str, Any] = {}
        
        # Extract campaign data if this is from a savegame
        self._extract_campaign_data()
        
        # Initialize custom content detection
        self._detect_custom_content_dynamic()
        
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
        
        # Check feats using improved validation
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
        
        # Check spells with dynamic spell level range
        max_spell_level = self._get_max_spell_level()
        for spell_level in range(max_spell_level + 1):
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
        
        # Check classes using improved validation
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
    
    def _get_max_spell_level(self) -> int:
        """
        Dynamically determine the maximum spell level based on character data
        
        Returns:
            Maximum spell level found in character data (defaults to 9)
        """
        max_level = 9  # Default NWN2 maximum
        
        # Check for highest spell level in known lists
        for level in range(20):  # Check up to level 20 for custom content
            if self.gff.get(f'KnownList{level}'):
                max_level = max(max_level, level)
            else:
                break  # Stop at first missing level
        
        return max_level
    
    def _is_vanilla_content(self, table_name: str, content_id: int) -> bool:
        """
        Check if content ID exists in vanilla game data using rules service
        
        Args:
            table_name: 2DA table name (feat, spells, classes, etc.)
            content_id: ID to check
            
        Returns:
            True if content exists in loaded game data
        """
        try:
            content_data = self.rules_service.get_by_id(table_name, content_id)
            return content_data is not None
        except Exception:
            return False
    
    def _get_content_name(self, table_name: str, content_id: int) -> str:
        """
        Get content name from game data using FieldMappingUtility or fallback to generic name
        
        Args:
            table_name: 2DA table name
            content_id: Content ID
            
        Returns:
            Content name or generic fallback
        """
        try:
            content_data = self.rules_service.get_by_id(table_name, content_id)
            if content_data:
                # Use FieldMappingUtility to handle different name field variations
                # cspell:ignore spellname strref
                name_fields = ['name', 'label', 'feat', 'spellname', 'strref']
                for name_field in name_fields:
                    name = field_mapper.get_field_value(content_data, name_field, '')
                    if name and str(name).strip() and str(name) != '****':
                        return str(name)
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
    
    def _extract_campaign_data(self) -> None:
        """Extract campaign, module, and quest data from save files"""
        # Check if we have a save path (only for savegames)
        if not hasattr(self.character_manager, 'save_path'):
            logger.info("ContentManager: No save_path on character_manager - skipping campaign data extraction")
            return
            
        save_path = self.character_manager.save_path
        logger.info(f"ContentManager: Checking save_path: {save_path}")
        
        if not save_path or not os.path.exists(save_path):
            logger.warning(f"ContentManager: Save path doesn't exist or is None: {save_path}")
            return
            
        logger.info(f"ContentManager: Starting campaign data extraction from {save_path}")
        
        try:
            from parsers.savegame_handler import SaveGameHandler
            handler = SaveGameHandler(save_path)
            logger.info("ContentManager: Created SaveGameHandler")
            
            # Extract module info from module.ifo
            self._extract_module_info(handler)
            
            # Extract quest data from globals.xml
            self._extract_quest_data(handler)
            
            # Detect current module from currentmodule.txt
            self._detect_current_module(save_path)
            
            logger.info(f"ContentManager: Campaign data extraction complete - module: {self.module_info.get('module_name', 'None')}, campaign: {self.module_info.get('campaign', 'None')}")
            
        except Exception as e:
            logger.error(f"ContentManager: Failed to extract campaign data: {e}", exc_info=True)
    
    def _extract_module_info(self, handler) -> None:
        """Extract module information from module.ifo"""
        logger.info("ContentManager: Attempting to extract module.ifo")
        try:
            # First get the current module name
            current_module = handler.extract_current_module()
            if not current_module:
                logger.warning("ContentManager: No current module found in save")
                return
            
            logger.info(f"ContentManager: Current module is '{current_module}'")
            
            # Use shared ResourceManager to find the module file
            from parsers.erf import ERFParser
            
            # Use the shared ResourceManager from rules_service
            rm = self.rules_service.rm
            module_path = rm.find_module(f"{current_module}.mod")
            if not module_path:
                logger.warning(f"ContentManager: Could not find {current_module}.mod")
                return
            
            logger.info(f"ContentManager: Found module at {module_path}")
            
            # Parse .mod file as ERF archive and extract module.ifo
            parser = ERFParser()
            parser.read(module_path)
            module_ifo_bytes = parser.extract_resource('module.ifo')
            
            if not module_ifo_bytes:
                logger.warning(f"ContentManager: Could not find module.ifo in {current_module}.mod")
                return
            
            # Parse the module.ifo GFF file
            from parsers.gff import GFFParser
            from io import BytesIO
            gff_parser = GFFParser()
            module_gff = gff_parser.load(BytesIO(module_ifo_bytes))
            module_ifo = module_gff.to_dict()
            
            if not module_ifo:
                logger.warning("ContentManager: Failed to parse module.ifo")
                return
            
            logger.info(f"ContentManager: module.ifo has {len(module_ifo)} fields")
            
            # Get module name (localized string)
            mod_name_data = module_ifo.get('Mod_Name', {})
            logger.info(f"ContentManager: Raw Mod_Name data: {mod_name_data}")
            
            if isinstance(mod_name_data, dict) and 'substrings' in mod_name_data:
                module_name = mod_name_data.get('substrings', [{}])[0].get('string', '')
                logger.info(f"ContentManager: Extracted module name from substrings: '{module_name}'")
            elif isinstance(mod_name_data, str):
                module_name = mod_name_data
                logger.info(f"ContentManager: Module name is direct string: '{module_name}'")
            else:
                module_name = ''
                logger.warning(f"ContentManager: Could not parse module name, type: {type(mod_name_data)}")
                
            # Get entry area
            area_name = module_ifo.get('Mod_Entry_Area', '')
            logger.info(f"ContentManager: Entry area: '{area_name}'")
            
            # Get some other interesting fields for debugging
            logger.info(f"ContentManager: Mod_ID: {module_ifo.get('Mod_ID', 'Not found')}")
            logger.info(f"ContentManager: Mod_Version: {module_ifo.get('Mod_Version', 'Not found')}")
            logger.info(f"ContentManager: Mod_Creator_ID: {module_ifo.get('Mod_Creator_ID', 'Not found')}")
            
            # Determine campaign from module name
            campaign = ''
            if module_name:
                if 'Neverwinter' in module_name or 'West Harbor' in module_name:
                    campaign = 'Original Campaign'
                    logger.info(f"ContentManager: Detected Original Campaign from module name")
                elif 'Rashemen' in module_name or 'Mulsantir' in module_name:
                    campaign = 'Mask of the Betrayer'
                    logger.info(f"ContentManager: Detected Mask of the Betrayer from module name")
                elif 'Samarach' in module_name or 'Samargol' in module_name:
                    campaign = 'Storm of Zehir'
                    logger.info(f"ContentManager: Detected Storm of Zehir from module name")
                else:
                    logger.info(f"ContentManager: Could not determine campaign from module name: '{module_name}'")
                    
            self.module_info = {
                'module_name': module_name,
                'area_name': area_name,
                'campaign': campaign,
                'entry_area': area_name,
                'module_description': str(module_ifo.get('Mod_Description', ''))[:100]  # Truncate for logging
            }
            
            logger.info(f"ContentManager: Successfully extracted module info: {module_name} ({campaign})")
            
        except Exception as e:
            logger.error(f"ContentManager: Failed to extract module info: {e}", exc_info=True)
    
    def _extract_quest_data(self, handler) -> None:
        """Extract quest data from globals.xml"""
        logger.info("ContentManager: Attempting to extract quest data from globals.xml")
        try:
            # Extract globals.xml as string
            globals_xml = handler.extract_globals_xml()
            if not globals_xml:
                logger.warning("ContentManager: No globals.xml data returned from handler")
                return
            
            # Parse the XML data
            import xml.etree.ElementTree as ET
            root = ET.fromstring(globals_xml)
            
            # Extract global variables into a dictionary
            globals_data = {}
            for var in root.findall('.//Variable'):
                name = var.get('name', '')
                value = var.get('value', '')
                if name:
                    globals_data[name] = value
            
            logger.info(f"ContentManager: globals.xml has {len(globals_data)} variables")
            
            # Parse quest variables from globals.xml
            # This is simplified - actual quest parsing would be more complex
            quest_count = 0
            completed_count = 0
            quest_examples = []  # Track some examples for logging
            
            # Count quest-related variables (simplified heuristic)
            for key, value in globals_data.items():
                if 'quest' in key.lower() or 'q_' in key.lower():
                    quest_count += 1
                    if str(value).lower() in ['true', '1', 'complete', 'done']:
                        completed_count += 1
                    
                    # Log first 5 quest variables as examples
                    if len(quest_examples) < 5:
                        quest_examples.append(f"{key}={value}")
            
            logger.info(f"ContentManager: Found {quest_count} quest-related variables")
            if quest_examples:
                logger.info(f"ContentManager: Quest variable examples: {quest_examples}")
            
            # Also look for some known quest patterns
            story_vars = [k for k in globals_data.keys() if k.startswith('STORY_') or k.startswith('MAIN_')]
            if story_vars:
                logger.info(f"ContentManager: Found {len(story_vars)} story variables: {story_vars[:5]}")
            
            self.campaign_data = {
                'total_quests': quest_count,
                'completed_quests': completed_count,
                'active_quests': quest_count - completed_count,
                'quest_completion_rate': round((completed_count / max(quest_count, 1)) * 100, 1),
                'quest_details': {
                    'summary': {
                        'completed_quests': completed_count,
                        'active_quests': quest_count - completed_count,
                        'total_quest_variables': quest_count
                    },
                    'progress_stats': {
                        'total_completion_rate': round((completed_count / max(quest_count, 1)) * 100, 1)
                    }
                }
            }
            
            logger.info(f"ContentManager: Extracted quest data: {completed_count}/{quest_count} quests completed ({self.campaign_data['quest_completion_rate']}%)")
            
        except Exception as e:
            logger.error(f"ContentManager: Failed to extract quest data: {e}", exc_info=True)
            # Set default values
            self.campaign_data = {
                'total_quests': 0,
                'completed_quests': 0,
                'active_quests': 0,
                'quest_completion_rate': 0,
                'quest_details': {
                    'summary': {
                        'completed_quests': 0,
                        'active_quests': 0,
                        'total_quest_variables': 0
                    },
                    'progress_stats': {
                        'total_completion_rate': 0
                    }
                }
            }
    
    def _detect_current_module(self, save_path: str) -> None:
        """Detect current module from currentmodule.txt or extract from save"""
        logger.info("ContentManager: Looking for current module info")
        try:
            # Try to extract from SaveGameHandler first
            from parsers.savegame_handler import SaveGameHandler
            handler = SaveGameHandler(save_path)
            current_module = handler.extract_current_module()
            
            if current_module:
                self.module_info['current_module'] = current_module
                logger.info(f"ContentManager: Current module from save: '{current_module}'")
            else:
                # Fallback to checking currentmodule.txt file
                from pathlib import Path
                save_dir = Path(save_path)
                module_txt = save_dir / "currentmodule.txt"
                
                logger.info(f"ContentManager: Checking for {module_txt}")
                
                if module_txt.exists():
                    current_module = module_txt.read_text().strip()
                    self.module_info['current_module'] = current_module
                    logger.info(f"ContentManager: Current module from currentmodule.txt: '{current_module}'")
                else:
                    logger.info(f"ContentManager: currentmodule.txt not found at {module_txt}")
        except Exception as e:
            logger.error(f"ContentManager: Failed to detect current module: {e}", exc_info=True)
    
    def get_campaign_info(self) -> Dict[str, Any]:
        """Get campaign and module information"""
        return {
            **self.module_info,
            **self.campaign_data
        }
    
    def get_module_name(self) -> str:
        """Get the module name"""
        return self.module_info.get('module_name', '')
    
    def get_campaign_name(self) -> str:
        """Get the campaign name"""
        return self.module_info.get('campaign', '')
    
    def get_area_name(self) -> str:
        """Get the current area name"""
        return self.module_info.get('area_name', '')
    
    def get_quest_summary(self) -> Dict[str, Any]:
        """Get quest summary data"""
        return self.campaign_data.get('quest_details', {})