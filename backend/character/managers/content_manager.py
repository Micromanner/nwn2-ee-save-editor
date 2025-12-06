"""
Content Manager - handles custom content detection logic and campaign/module info
Manages detection and tracking of custom content (feats, spells, classes) vs vanilla content
Also extracts and manages campaign, module, and quest information from save files
"""

from typing import Dict, List, Any, Optional, Union, TYPE_CHECKING
from loguru import logger
import os

from ..events import EventEmitter
from gamedata.dynamic_loader.field_mapping_utility import field_mapper  # type: ignore

if TYPE_CHECKING:
    pass

# Using global loguru logger


class ContentManager(EventEmitter):
    def __init__(self, character_manager):
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.rules_service = character_manager.rules_service
        
        # Custom content tracking - initialized from character manager
        self.custom_content: Dict[str, Dict[str, Any]] = {}
        
        # Campaign/module/quest data
        self.campaign_data: Dict[str, Any] = {}
        self.module_info: Dict[str, Any] = {}
        self.module_variables: Dict[str, Any] = {'integers': {}, 'floats': {}, 'strings': {}}
        self.all_modules: Dict[str, Dict[str, Any]] = {}  # All available modules from .z files
        self.current_module_name: Optional[str] = None

        # Quest definition lookups
        self.quest_definitions: Dict[str, Dict[str, Any]] = {}

        # Extract campaign data if this is from a savegame
        self._extract_campaign_data()
        
        # Initialize custom content detection
        self._detect_custom_content_dynamic()
        
        logger.info("ContentManager initialized")
    
    def is_custom_content(self, content_type: str, content_id: int) -> bool:
        """Check if a specific content ID is custom content"""
        key = f"{content_type}_{content_id}"
        return key in self.custom_content
    
    def _detect_custom_content_dynamic(self):
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
        """Dynamically determine the maximum spell level based on character data"""
        max_level = 9  # Default NWN2 maximum
        
        # Check for highest spell level in known lists
        for level in range(20):  # Check up to level 20 for custom content
            if self.gff.get(f'KnownList{level}'):
                max_level = max(max_level, level)
            else:
                break  # Stop at first missing level
        
        return max_level
    
    def _is_vanilla_content(self, table_name: str, content_id: int) -> bool:
        """Check if content ID exists in vanilla game data using rules service"""
        try:
            content_data = self.rules_service.get_by_id(table_name, content_id)
            return content_data is not None
        except Exception:
            return False
    
    def _get_content_name(self, table_name: str, content_id: int) -> str:
        """Get content name from game data or fallback to generic name"""
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
        """Detect content source using dynamic validation against loaded data"""
        if not self._is_vanilla_content(table_name, content_id):
            return "custom-mod"
        return "vanilla"
    
    def get_custom_content_summary(self) -> Dict[str, Any]:
        """Get a summary of all detected custom content"""
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
        """Get all custom content items of a specific type"""
        return [
            item for item in self.custom_content.values()
            if item['type'] == content_type
        ]
    
    def _strip_nwn2_formatting(self, text: str) -> str:
        """
        Strip NWN2 text formatting codes from a string

        NWN2 uses codes like {0}, {1100}, {1200} for colors/fonts.
        These should be stripped for display purposes.

        Args:
            text: Text that may contain formatting codes

        Returns:
            Text with formatting codes removed
        """
        import re
        # Pattern matches {digits} at start of string or anywhere
        # Examples: {0}, {1100}, {1200}
        return re.sub(r'\{\d+\}', '', text).strip()

    def _resolve_localized_string(self, loc_string: Any, resource_manager=None) -> str:
        """Resolve a CExoLocString to actual text"""
        if isinstance(loc_string, str):
            return self._strip_nwn2_formatting(loc_string)

        if not isinstance(loc_string, dict):
            return ''

        # Try inline substrings first (custom content often uses these)
        substrings = loc_string.get('substrings', [])
        if substrings and len(substrings) > 0:
            text = substrings[0].get('string', '')
            if text:
                return self._strip_nwn2_formatting(text)

        # Fall back to TLK string reference
        string_ref = loc_string.get('string_ref', 0)
        if string_ref and string_ref > 0 and resource_manager:
            try:
                resolved = resource_manager.get_string(string_ref)
                if resolved and not resolved.startswith('{StrRef:'):
                    return self._strip_nwn2_formatting(resolved)
            except Exception as e:
                logger.debug(f"Failed to resolve string_ref {string_ref}: {e}")

        return ''

    def _extract_quest_definitions(self) -> None:
        """
        FEATURE ON HOLD: Quest mapping is disabled in frontend due to duplicate/incorrect mappings.
        See dialogue_mapping_service.py docstring and QUEST_MAPPING_UX_PROBLEM.md for details.

        Extract quest definitions from module.jrl
        Checks campaign folder first, then falls back to .mod file
        """
        if not hasattr(self, 'module_info') or not self.module_info:
            logger.info("ContentManager: No module info available for quest extraction")
            return

        campaign_id = self.module_info.get('campaign_id', '')
        if not campaign_id:
            logger.info("ContentManager: No campaign_id available for quest extraction")
            return

        logger.info("ContentManager: Extracting quest definitions from module.jrl")

        try:
            from config.nwn2_settings import nwn2_paths
            from parsers.gff import GFFParser
            from parsers.resource_manager import ResourceManager
            import tempfile
            import lzma

            module_jrl_data = None
            source = None

            campaign_file = self.find_campaign_file()
            if campaign_file:
                campaign_folder = os.path.dirname(campaign_file)
                jrl_path = os.path.join(campaign_folder, 'module.jrl')

                if os.path.exists(jrl_path):
                    logger.info(f"ContentManager: Found module.jrl in campaign folder: {jrl_path}")
                    with open(jrl_path, 'rb') as f:
                        module_jrl_data = f.read()
                    source = 'campaign'

            if not module_jrl_data and self.current_module_name:
                logger.info(f"ContentManager: Checking for module.jrl in current module .z file: {self.current_module_name}")

                if hasattr(self.character_manager, 'save_path'):
                    save_dir = self.character_manager.save_path
                    z_file_path = os.path.join(save_dir, f'{self.current_module_name}.z')

                    if os.path.exists(z_file_path):
                        try:
                            with lzma.open(z_file_path, 'rb') as f:
                                decompressed = f.read()

                            with tempfile.NamedTemporaryFile(delete=False, suffix='.erf') as tmp:
                                tmp.write(decompressed)
                                tmp_path = tmp.name

                            try:
                                from parsers import ERFParser
                                parser = ERFParser()
                                parser.read(tmp_path)

                                module_jrl_data = parser.extract_resource('module.jrl')
                                if module_jrl_data:
                                    logger.info(f"ContentManager: Found module.jrl in .z file")
                                    source = 'module'
                            finally:
                                try:
                                    os.unlink(tmp_path)
                                except:
                                    pass

                        except Exception as e:
                            logger.warning(f"ContentManager: Failed to extract module.jrl from .z file: {e}")

            if not module_jrl_data:
                logger.info("ContentManager: No module.jrl found")
                return

            gff_parser = GFFParser()
            from io import BytesIO
            jrl_gff = gff_parser.load(BytesIO(module_jrl_data))
            jrl_data = jrl_gff.to_dict()

            # Initialize ResourceManager for TLK string lookups
            resource_manager = None
            try:
                resource_manager = ResourceManager()
            except Exception as e:
                logger.warning(f"ContentManager: Could not initialize ResourceManager for TLK lookups: {e}")

            categories = jrl_data.get('Categories', [])
            quest_count = 0
            resolved_count = 0

            for category in categories:
                if not isinstance(category, dict):
                    continue

                category_tag = category.get('Tag', '')
                category_name_data = category.get('Name', {})
                category_name_str = self._resolve_localized_string(category_name_data, resource_manager)

                # Fallback to tag if name couldn't be resolved
                if not category_name_str:
                    category_name_str = category_tag

                entry_list = category.get('EntryList', [])

                for entry in entry_list:
                    if not isinstance(entry, dict):
                        continue

                    entry_id = entry.get('ID', 0)
                    text_data = entry.get('Text', {})
                    text = self._resolve_localized_string(text_data, resource_manager)

                    if text:
                        resolved_count += 1

                    quest_key = f"{category_tag}_{entry_id}"

                    self.quest_definitions[quest_key] = {
                        'category_tag': category_tag,
                        'category_name': category_name_str,
                        'entry_id': entry_id,
                        'text': text,
                        'xp': entry.get('XP', 0),
                        'end': entry.get('End', 0),
                        'source': source
                    }
                    quest_count += 1

            logger.info(f"ContentManager: Extracted {quest_count} quest definitions from {source} ({resolved_count} with text)")

            if quest_count > 0:
                sample_keys = list(self.quest_definitions.keys())[:5]
                logger.debug(f"ContentManager: Sample quest definition keys: {sample_keys}")

        except Exception as e:
            logger.error(f"ContentManager: Failed to extract quest definitions: {e}", exc_info=True)

    def parse_variable_name(self, var_name: str) -> Dict[str, Any]:
        """Parse a plot variable name to extract human-readable information"""
        import re

        parsed = {
            'original': var_name,
            'display_name': var_name,
            'description': '',
            'category': 'General',
            'variable_type_hint': 'state'
        }

        # Check for companion influence FIRST (before act pattern)
        if 'Influence' in var_name:
            parsed['category'] = 'Companion'
            parsed['variable_type_hint'] = 'influence'

        # Check for reputation FIRST (before other patterns)
        elif var_name.startswith('Rep'):
            faction_name = var_name[3:]  # Remove "Rep" prefix
            readable = re.sub(r'([A-Z])', r' \1', faction_name).strip()
            parsed['display_name'] = f"{readable} Reputation"
            parsed['description'] = f"Reputation with faction: {readable}"
            parsed['category'] = 'Reputation'
            parsed['variable_type_hint'] = 'reputation'

        # Check for act/chapter prefix (e.g., "10_", "11_", "00_") - check BEFORE State/Quest patterns
        elif re.match(r'^(\d+)_(.+)$', var_name):
            act_match = re.match(r'^(\d+)_(.+)$', var_name)
            act_num = act_match.group(1)
            remainder = act_match.group(2)
            parsed['category'] = f'Act {act_num}'

            # Check for Hungarian notation prefix (only strip if followed by uppercase)
            # e.g., bGotHealQuest -> strip 'b', but brelaina_state -> keep 'b'
            if len(remainder) > 1 and remainder[0] == 'b' and remainder[1].isupper():
                parsed['variable_type_hint'] = 'boolean'
                remainder = remainder[1:]  # Remove 'b' prefix
            elif len(remainder) > 1 and remainder[0] == 'n' and remainder[1].isupper():
                parsed['variable_type_hint'] = 'number'
                remainder = remainder[1:]

            # Convert CamelCase to readable text
            readable = re.sub(r'([A-Z])', r' \1', remainder).strip()
            parsed['display_name'] = f"Act {act_num}: {readable}"
            parsed['description'] = f"{'Boolean flag' if parsed['variable_type_hint'] == 'boolean' else 'Numeric variable'} for {readable}"

        # Check for "State" suffix (quest progression variables) - only for non-act variables
        elif var_name.endswith('State'):
            base_name = var_name[:-5]  # Remove "State"
            readable = re.sub(r'([A-Z])', r' \1', base_name).strip()
            parsed['display_name'] = f"{readable} (Quest State)"
            parsed['description'] = f"Quest progression tracking for {readable}"
            parsed['variable_type_hint'] = 'progression'
            parsed['category'] = 'Quest Progress'

        # Check for "Quest" in name - only for non-act variables
        elif 'Quest' in var_name:
            readable = re.sub(r'([A-Z])', r' \1', var_name).strip()
            parsed['display_name'] = readable
            parsed['description'] = f"Quest variable: {readable}"
            parsed['category'] = 'Quest'

        # Generic CamelCase parsing
        else:
            readable = re.sub(r'([A-Z])', r' \1', var_name).strip()
            if readable != var_name:
                parsed['display_name'] = readable
                parsed['description'] = f"Game variable: {readable}"

        return parsed

    def get_quest_info(self, quest_key: str) -> Optional[Dict[str, Any]]:
        """Get quest information by quest key"""
        # Direct lookup by exact key (e.g., "q_attack_10")
        if quest_key in self.quest_definitions:
            return self.quest_definitions[quest_key]

        # Try with underscore variations (in case the key format differs slightly)
        # Only match if the entire category_tag matches exactly
        for key, quest in self.quest_definitions.items():
            category_tag = quest.get('category_tag', '')
            # Only match if quest_key IS exactly the category_tag
            # or starts with category_tag followed by underscore and digits
            if quest_key == category_tag:
                logger.debug(f"ContentManager: Matched quest key '{quest_key}' to category '{key}'")
                return quest

        return None

    def get_all_quests(self) -> Dict[str, Dict[str, Any]]:
        """FEATURE ON HOLD: Quest mapping disabled. Get all quest definitions."""
        return self.quest_definitions.copy()

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

            # Extract quest definitions
            self._extract_quest_definitions()

            logger.info(f"ContentManager: Campaign data extraction complete - module: {self.module_info.get('module_name', 'None')}, campaign: {self.module_info.get('campaign', 'None')}")

        except Exception as e:
            logger.error(f"ContentManager: Failed to extract campaign data: {e}", exc_info=True)
    
    def _parse_module_from_z_file(self, z_file_path: str) -> Optional[Dict[str, Any]]:
        """Parse module.ifo from a single .z file"""
        import lzma
        import tempfile
        from parsers import ERFParser
        from parsers.gff import GFFParser
        from io import BytesIO

        try:
            # Decompress LZMA
            with lzma.open(z_file_path, 'rb') as f:
                decompressed = f.read()

            # Write to temp file for ERF parser
            with tempfile.NamedTemporaryFile(delete=False, suffix='.erf') as tmp:
                tmp.write(decompressed)
                tmp_path = tmp.name

            try:
                # Parse ERF archive
                parser = ERFParser()
                parser.read(tmp_path)

                # Extract module.ifo
                module_ifo_bytes = parser.extract_resource('module.ifo')

                if not module_ifo_bytes:
                    logger.warning(f"ContentManager: module.ifo not found in {os.path.basename(z_file_path)}")
                    return None
            finally:
                # Clean up temp file
                try:
                    os.unlink(tmp_path)
                except:
                    pass

            # Parse the module.ifo GFF file
            gff_parser = GFFParser()
            module_gff = gff_parser.load(BytesIO(module_ifo_bytes))
            module_ifo = module_gff.to_dict()

            return module_ifo

        except Exception as e:
            logger.error(f"ContentManager: Failed to parse {os.path.basename(z_file_path)}: {e}")
            return None

    def _extract_module_info(self, handler) -> None:
        """Extract module information from all .z files in save directory"""
        logger.info("ContentManager: Extracting module info from area .z files")
        try:
            # Get current module name
            current_module = handler.extract_current_module()
            self.current_module_name = current_module

            if current_module:
                logger.info(f"ContentManager: Current module is '{current_module}'")
            else:
                logger.warning("ContentManager: No current module found in save")

            # Find all .z files in save directory
            import glob

            save_dir = handler.save_dir
            z_files = glob.glob(os.path.join(save_dir, '*.z'))

            logger.info(f"ContentManager: Found {len(z_files)} area files (.z)")

            if not z_files:
                logger.warning("ContentManager: No .z files found in save directory")
                return

            # Parse ALL .z files
            for z_file in z_files:
                module_id = os.path.splitext(os.path.basename(z_file))[0]
                logger.info(f"ContentManager: Parsing {module_id}.z")

                module_ifo = self._parse_module_from_z_file(z_file)
                if not module_ifo:
                    continue

                # Process this module's data
                module_data = self._process_module_ifo(module_ifo, module_id)

                # Store in all_modules
                self.all_modules[module_id] = module_data

                # If this is the current module, set it as the active one
                if module_id == current_module:
                    self.module_info = module_data['info']
                    self.module_variables = module_data['variables']
                    logger.info(f"ContentManager: Set '{module_id}' as current module")

            # If no current module was found but we have modules, use the first one
            if not self.module_info and self.all_modules:
                first_module_id = list(self.all_modules.keys())[0]
                self.module_info = self.all_modules[first_module_id]['info']
                self.module_variables = self.all_modules[first_module_id]['variables']
                logger.info(f"ContentManager: Using first module '{first_module_id}' as fallback")

            logger.info(f"ContentManager: Loaded {len(self.all_modules)} modules total")

        except Exception as e:
            logger.error(f"ContentManager: Failed to extract module info: {e}", exc_info=True)

    def _process_module_ifo(self, module_ifo: Dict[str, Any], module_id: str) -> Dict[str, Any]:
        """Process module.ifo data and return structured info + variables"""

        # Get module name (localized string)
        mod_name_data = module_ifo.get('Mod_Name', {})

        if isinstance(mod_name_data, dict) and 'substrings' in mod_name_data:
            substrings = mod_name_data.get('substrings', [])
            module_name = substrings[0].get('string', '') if substrings else ''
        elif isinstance(mod_name_data, str):
            module_name = mod_name_data
        else:
            module_name = ''

        # Fallback to module_id if no name found
        if not module_name:
            module_name = module_id.replace('_', ' ').title()

        # Get entry area
        area_name = module_ifo.get('Mod_Entry_Area', '')

        # Get Campaign_ID
        campaign_id = module_ifo.get('Campaign_ID', '')

        # Parse module description (localized string)
        mod_desc_data = module_ifo.get('Mod_Description', {})
        module_description = ''
        if isinstance(mod_desc_data, dict) and 'substrings' in mod_desc_data:
            substrings = mod_desc_data.get('substrings', [])
            module_description = substrings[0].get('string', '') if substrings else ''
        elif isinstance(mod_desc_data, str):
            module_description = mod_desc_data

        # Determine campaign from module name or campaign_id
        campaign = ''
        if campaign_id:
            # Look up campaign name from campaign.cam
            # Temporarily set campaign_id in a temp dict
            temp_info = {'campaign_id': campaign_id}
            old_module_info = self.module_info
            self.module_info = temp_info
            campaign_file = self.find_campaign_file()
            self.module_info = old_module_info

            if campaign_file:
                try:
                    from parsers.gff import GFFParser
                    gff_parser = GFFParser()
                    campaign_gff = gff_parser.read(campaign_file)
                    campaign_data = campaign_gff.to_dict()

                    display_name_data = campaign_data.get('DisplayName', {})
                    if isinstance(display_name_data, dict) and 'substrings' in display_name_data:
                        substrings = display_name_data.get('substrings', [])
                        campaign = substrings[0].get('string', '') if substrings else ''
                except Exception as e:
                    logger.warning(f"ContentManager: Could not load campaign name: {e}")

        # Fallback to module name detection if campaign not found
        if not campaign and module_name:
            if 'Neverwinter' in module_name or 'West Harbor' in module_name or 'Old Owl Well' in module_name:
                campaign = 'Neverwinter Nights 2 Campaign'
            elif 'Rashemen' in module_name or 'Mulsantir' in module_name:
                campaign = 'Mask of the Betrayer'
            elif 'Samarach' in module_name or 'Samargol' in module_name:
                campaign = 'Storm of Zehir'

        # Extract VarTable (module variables)
        var_table = module_ifo.get('VarTable', [])

        # Parse VarTable into separate dictionaries by type
        module_variables = {
            'integers': {},
            'floats': {},
            'strings': {}
        }

        for var in var_table:
            if not isinstance(var, dict):
                continue

            var_name = var.get('Name', '')
            var_type = var.get('Type', 0)
            var_value = var.get('Value', None)

            if not var_name or var_value is None:
                continue

            # Type: 1=int, 2=float, 3=string (NWN2 GFF types)
            if var_type == 1:
                module_variables['integers'][var_name] = int(var_value)
            elif var_type == 2:
                module_variables['floats'][var_name] = float(var_value)
            elif var_type == 3:
                module_variables['strings'][var_name] = str(var_value)

        total_vars = sum(len(v) for v in module_variables.values())
        logger.info(f"ContentManager: {module_id}: '{module_name}' has {total_vars} variables")

        return {
            'info': {
                'module_name': module_name,
                'area_name': area_name,
                'campaign': campaign,
                'entry_area': area_name,
                'module_description': module_description,
                'campaign_id': campaign_id,
                'current_module': module_id
            },
            'variables': module_variables
        }
    
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

    def find_campaign_file(self) -> Optional[str]:
        """Find the campaign.cam file for the current save's campaign"""
        campaign_id = self.module_info.get('campaign_id', '')
        if not campaign_id:
            logger.warning("ContentManager: No Campaign_ID available to find campaign file")
            return None

        logger.info(f"ContentManager: Searching for campaign with GUID: {campaign_id}")

        try:
            from config.nwn2_settings import nwn2_paths
            campaigns_dir = nwn2_paths.campaigns

            if not os.path.exists(campaigns_dir):
                logger.warning(f"ContentManager: Campaigns directory not found: {campaigns_dir}")
                return None

            # Search all campaign directories for matching campaign.cam
            for campaign_name in os.listdir(campaigns_dir):
                campaign_path = os.path.join(campaigns_dir, campaign_name)

                if not os.path.isdir(campaign_path):
                    continue

                campaign_file = os.path.join(campaign_path, 'campaign.cam')

                if not os.path.exists(campaign_file):
                    continue

                # Parse the campaign.cam to check GUID
                try:
                    from parsers.gff import GFFParser
                    gff_parser = GFFParser()
                    campaign_gff = gff_parser.read(campaign_file)
                    campaign_data = campaign_gff.to_dict()

                    file_guid = campaign_data.get('GUID', '')

                    if file_guid == campaign_id:
                        logger.info(f"ContentManager: Found matching campaign: {campaign_name}")
                        return campaign_file

                except Exception as e:
                    logger.warning(f"ContentManager: Failed to parse {campaign_file}: {e}")
                    continue

            logger.warning(f"ContentManager: No campaign.cam found matching GUID {campaign_id}")
            return None

        except Exception as e:
            logger.error(f"ContentManager: Error finding campaign file: {e}", exc_info=True)
            return None

    def get_campaign_settings(self) -> Optional[Dict[str, Any]]:
        """Get campaign settings from campaign.cam file"""
        campaign_file = self.find_campaign_file()

        if not campaign_file:
            return None

        try:
            from parsers.gff import GFFParser
            gff_parser = GFFParser()
            campaign_gff = gff_parser.read(campaign_file)
            campaign_data = campaign_gff.to_dict()

            # Extract key settings
            mod_names_raw = campaign_data.get('ModNames', [])
            module_names = []
            if isinstance(mod_names_raw, list):
                for mod in mod_names_raw:
                    if isinstance(mod, dict) and 'ModuleName' in mod:
                        module_names.append(mod['ModuleName'])
                    elif isinstance(mod, str):
                        module_names.append(mod)

            settings = {
                'campaign_file_path': campaign_file,
                'guid': campaign_data.get('GUID', ''),
                'level_cap': campaign_data.get('LvlCap', 20),
                'xp_cap': campaign_data.get('XPCap', 0),
                'companion_xp_weight': campaign_data.get('CompXPWt', 0.0),
                'henchman_xp_weight': campaign_data.get('HenchXPWt', 0.0),
                'attack_neutrals': campaign_data.get('AttackNeut', 0),
                'auto_xp_award': campaign_data.get('AutoXPAwd', 1),
                'journal_sync': campaign_data.get('JournalSynch', 1),
                'no_char_changing': campaign_data.get('NoCharChanging', 0),
                'use_personal_reputation': campaign_data.get('UsePersonalRep', 0),
                'start_module': campaign_data.get('StartModule', ''),
                'module_names': module_names,
            }

            # Get display name and description
            display_name_data = campaign_data.get('DisplayName', {})
            if isinstance(display_name_data, dict) and 'substrings' in display_name_data:
                substrings = display_name_data.get('substrings', [])
                settings['display_name'] = substrings[0].get('string', '') if substrings else ''
            else:
                settings['display_name'] = str(display_name_data) if display_name_data else ''

            description_data = campaign_data.get('Description', {})
            if isinstance(description_data, dict) and 'substrings' in description_data:
                substrings = description_data.get('substrings', [])
                settings['description'] = substrings[0].get('string', '') if substrings else ''
            else:
                settings['description'] = str(description_data) if description_data else ''

            logger.info(f"ContentManager: Successfully loaded campaign settings from {campaign_file}")
            return settings

        except Exception as e:
            logger.error(f"ContentManager: Failed to get campaign settings: {e}", exc_info=True)
            return None

    def update_campaign_settings(self, settings: Dict[str, Any]) -> bool:
        """Update campaign settings in campaign.cam file"""
        campaign_file = self.find_campaign_file()

        if not campaign_file:
            logger.error("ContentManager: Cannot update campaign settings - campaign file not found")
            return False

        try:
            from parsers.gff import GFFParser, GFFWriter

            # Read current campaign.cam
            gff_parser = GFFParser()
            campaign_gff = gff_parser.read(campaign_file)

            # Update fields
            if 'level_cap' in settings:
                campaign_gff.set_field('LvlCap', settings['level_cap'])

            if 'xp_cap' in settings:
                campaign_gff.set_field('XPCap', settings['xp_cap'])

            if 'companion_xp_weight' in settings:
                campaign_gff.set_field('CompXPWt', float(settings['companion_xp_weight']))

            if 'henchman_xp_weight' in settings:
                campaign_gff.set_field('HenchXPWt', float(settings['henchman_xp_weight']))

            if 'attack_neutrals' in settings:
                campaign_gff.set_field('AttackNeut', int(settings['attack_neutrals']))

            if 'auto_xp_award' in settings:
                campaign_gff.set_field('AutoXPAwd', int(settings['auto_xp_award']))

            if 'journal_sync' in settings:
                campaign_gff.set_field('JournalSynch', int(settings['journal_sync']))

            if 'no_char_changing' in settings:
                campaign_gff.set_field('NoCharChanging', int(settings['no_char_changing']))

            if 'use_personal_reputation' in settings:
                campaign_gff.set_field('UsePersonalRep', int(settings['use_personal_reputation']))

            # Write back to file
            writer = GFFWriter.from_parser(gff_parser)
            writer.write(campaign_file, campaign_gff)

            logger.info(f"ContentManager: Successfully updated campaign settings in {campaign_file}")
            return True

        except Exception as e:
            logger.error(f"ContentManager: Failed to update campaign settings: {e}", exc_info=True)
            return False

    def get_all_available_modules(self) -> List[Dict[str, Any]]:
        """Get list of all available modules from save"""
        modules = []
        for module_id, module_data in self.all_modules.items():
            info = module_data['info']
            variables = module_data['variables']
            var_count = sum(len(v) for v in variables.values())

            modules.append({
                'id': module_id,
                'name': info['module_name'],
                'campaign': info['campaign'],
                'variable_count': var_count,
                'is_current': module_id == self.current_module_name
            })

        return modules

    def get_module_by_id(self, module_id: str) -> Optional[Dict[str, Any]]:
        """Get specific module data by ID"""
        if module_id not in self.all_modules:
            return None

        module_data = self.all_modules[module_id]
        return {
            **module_data['info'],
            'variables': {
                'integers': module_data['variables']['integers'],
                'strings': module_data['variables']['strings'],
                'floats': module_data['variables']['floats'],
                'total_count': sum(len(v) for v in module_data['variables'].values())
            }
        }

    def get_module_variables(self, module_id: Optional[str] = None) -> Dict[str, Any]:
        """Get module variables from VarTable"""
        if module_id and module_id in self.all_modules:
            variables = self.all_modules[module_id]['variables']
        else:
            variables = self.module_variables

        return {
            'integers': variables.get('integers', {}),
            'strings': variables.get('strings', {}),
            'floats': variables.get('floats', {}),
            'total_count': sum(len(v) for v in variables.values())
        }

    def update_module_variable(
        self,
        var_name: str,
        value: Union[int, str, float],
        var_type: str = 'int',
        module_id: Optional[str] = None
    ) -> bool:
        """
        Update a module variable in VarTable.

        Args:
            var_name: Name of the variable to update
            value: New value for the variable
            var_type: Type of variable ('int', 'float', 'string')
            module_id: Optional module ID. If None, updates current module.
                       If specified, updates the variable in that module's .z file.

        Returns:
            True if update succeeded, False otherwise
        """
        if not hasattr(self.character_manager, 'save_path'):
            logger.error("ContentManager: No save_path available")
            return False

        save_path = self.character_manager.save_path
        if not save_path or not os.path.exists(save_path):
            logger.error(f"ContentManager: Save path doesn't exist: {save_path}")
            return False

        target_module = module_id or self.current_module_name
        is_current_module = (target_module == self.current_module_name)

        try:
            if is_current_module:
                return self._update_current_module_variable(
                    save_path, var_name, value, var_type
                )
            else:
                return self._update_z_file_module_variable(
                    save_path, target_module, var_name, value, var_type
                )

        except Exception as e:
            logger.error(f"ContentManager: Failed to update module variable: {e}", exc_info=True)
            return False

    def _update_current_module_variable(
        self,
        save_path: str,
        var_name: str,
        value: Union[int, str, float],
        var_type: str
    ) -> bool:
        """Update variable in standalone module.ifo for current module"""
        from parsers.savegame_handler import SaveGameHandler
        from parsers.gff import GFFParser, GFFWriter
        from io import BytesIO

        handler = SaveGameHandler(save_path, create_load_backup=False)

        module_ifo_bytes = handler.extract_module_ifo()
        if not module_ifo_bytes:
            logger.error("ContentManager: No module.ifo found in save")
            return False

        gff_parser = GFFParser()
        module_gff = gff_parser.load(BytesIO(module_ifo_bytes))

        updated_gff = self._update_var_table(module_gff, var_name, value, var_type)

        writer = GFFWriter.from_parser(gff_parser)
        module_ifo_data = writer.to_bytes(updated_gff)

        handler.update_module_ifo(module_ifo_data)

        self._update_variable_cache(self.current_module_name, var_name, value, var_type)

        logger.info(f"ContentManager: Updated current module variable {var_name} = {value} ({var_type})")
        self.emit('module_variable_updated', {
            'variable_name': var_name,
            'value': value,
            'variable_type': var_type,
            'module_id': self.current_module_name
        })

        return True

    def _update_z_file_module_variable(
        self,
        save_path: str,
        module_id: str,
        var_name: str,
        value: Union[int, str, float],
        var_type: str
    ) -> bool:
        """Update variable in .z file using ERF writer"""
        import lzma
        from parsers import ERFParser
        from parsers.gff import GFFParser, GFFWriter
        from io import BytesIO

        z_file_path = os.path.join(save_path, f'{module_id}.z')
        if not os.path.exists(z_file_path):
            logger.error(f"ContentManager: Module .z file not found: {z_file_path}")
            return False

        logger.info(f"ContentManager: Updating variable in {module_id}.z")

        with lzma.open(z_file_path, 'rb') as f:
            decompressed_data = f.read()

        parser = ERFParser()
        parser.parse_from_bytes(decompressed_data)
        parser.load_all_resources()

        module_ifo_bytes = parser.extract_resource('module.ifo')
        if not module_ifo_bytes:
            logger.error(f"ContentManager: module.ifo not found in {module_id}.z")
            return False

        gff_parser = GFFParser()
        module_gff = gff_parser.load(BytesIO(module_ifo_bytes))

        updated_gff = self._update_var_table(module_gff, var_name, value, var_type)

        writer = GFFWriter.from_parser(gff_parser)
        updated_module_ifo = writer.to_bytes(updated_gff)

        parser.update_resource('module.ifo', updated_module_ifo)

        updated_erf_data = parser.to_bytes()

        with lzma.open(z_file_path, 'wb') as f:
            f.write(updated_erf_data)

        self._update_variable_cache(module_id, var_name, value, var_type)

        logger.info(f"ContentManager: Updated {module_id} variable {var_name} = {value} ({var_type})")
        self.emit('module_variable_updated', {
            'variable_name': var_name,
            'value': value,
            'variable_type': var_type,
            'module_id': module_id
        })

        return True

    def _update_var_table(self, module_gff, var_name: str, value: Union[int, str, float], var_type: str):
        """Update or add a variable in the VarTable of a module GFF"""
        module_ifo = module_gff.to_dict()

        var_table = module_ifo.get('VarTable', [])
        if not isinstance(var_table, list):
            var_table = []

        var_found = False
        gff_type = 1 if var_type == 'int' else (2 if var_type == 'float' else 3)

        for var in var_table:
            if isinstance(var, dict) and var.get('Name') == var_name:
                var['Value'] = value
                var['Type'] = gff_type
                var_found = True
                break

        if not var_found:
            var_table.append({
                'Name': var_name,
                'Type': gff_type,
                'Value': value
            })

        module_gff.set_field('VarTable', var_table)
        return module_gff

    def _update_variable_cache(self, module_id: str, var_name: str, value: Union[int, str, float], var_type: str):
        """Update the local variable cache after a successful update"""
        if module_id in self.all_modules:
            variables = self.all_modules[module_id]['variables']
        elif module_id == self.current_module_name:
            variables = self.module_variables
        else:
            return

        if var_type == 'int':
            variables['integers'][var_name] = int(value)
        elif var_type == 'float':
            variables['floats'][var_name] = float(value)
        elif var_type == 'string':
            variables['strings'][var_name] = str(value)