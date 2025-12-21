"""
GameState Manager - handles editing of game state data
Manages editing of quests, reputation, influence, and campaign variables
"""

from typing import Dict, Any, Optional, List, Union, TYPE_CHECKING
from loguru import logger
import os

from ..events import EventEmitter

if TYPE_CHECKING:
    pass


class GameStateManager(EventEmitter):
    def __init__(self, character_manager):
        super().__init__()
        self.character_manager = character_manager

        # XML parser for globals.xml (lazy loaded)
        self._xml_parser = None

        logger.info("GameStateManager initialized")

    def _get_xml_parser(self):
        """Get or create the XML parser for globals.xml"""
        if self._xml_parser is None:
            if not hasattr(self.character_manager, 'save_path'):
                logger.warning("GameStateManager: No save_path available for XML parsing")
                return None

            save_path = self.character_manager.save_path
            if not save_path or not os.path.exists(save_path):
                logger.warning(f"GameStateManager: Save path doesn't exist: {save_path}")
                return None

            try:
                from services.savegame_handler import SaveGameHandler
                from nwn2_rust import XmlParser

                handler = SaveGameHandler(save_path)
                globals_xml = handler.extract_globals_xml()

                if not globals_xml:
                    logger.warning("GameStateManager: No globals.xml found in save")
                    return None

                self._xml_parser = XmlParser(globals_xml)
                logger.info("GameStateManager: XML parser initialized")

            except Exception as e:
                logger.error(f"GameStateManager: Failed to initialize XML parser: {e}")
                return None

        return self._xml_parser

    def get_companion_influence(self) -> Dict[str, Any]:
        """Get companion influence data from globals.xml"""
        parser = self._get_xml_parser()
        if not parser:
            return {}

        try:
            return parser.get_companion_status()
        except Exception as e:
            logger.error(f"GameStateManager: Failed to get companion influence: {e}")
            return {}

    def update_companion_influence(self, companion_id: str, new_influence: int) -> bool:
        """Update companion influence value"""
        parser = self._get_xml_parser()
        if not parser:
            logger.error("GameStateManager: No XML parser available")
            return False

        try:
            success = parser.update_companion_influence(companion_id, new_influence)
            if success:
                self._save_globals_xml()
                self.emit('companion_influence_updated', {
                    'companion_id': companion_id,
                    'new_influence': new_influence
                })
            return success
        except Exception as e:
            logger.error(f"GameStateManager: Failed to update companion influence: {e}")
            return False

    def _save_globals_xml(self) -> None:
        """Save the updated globals.xml back to the save file"""
        if not self._xml_parser:
            logger.error("GameStateManager: No XML parser to save")
            return

        if not hasattr(self.character_manager, 'save_path'):
            logger.error("GameStateManager: No save_path available")
            return

        save_path = self.character_manager.save_path
        globals_path = os.path.join(save_path, 'globals.xml')

        try:
            xml_content = self._xml_parser.to_xml_string()
            with open(globals_path, 'w', encoding='utf-8') as f:
                f.write(xml_content)
            logger.info(f"GameStateManager: Successfully saved globals.xml to {globals_path}")
        except Exception as e:
            logger.error(f"GameStateManager: Failed to save globals.xml: {e}")
            raise

    def get_quest_details(self) -> Dict[str, Any]:
        """Get detailed quest information from globals.xml"""
        parser = self._get_xml_parser()
        if not parser:
            return {
                'groups': [],
                'total_quests': 0,
                'completed_quests': 0,
                'active_quests': 0,
                'unknown_quests': 0,
                'completion_rate': 0.0
            }

        try:
            quest_overview = parser.get_quest_overview()
            quest_groups = quest_overview.get('quest_groups', {})

            # Convert quest groups to the format expected by API
            groups = []
            for prefix, data in quest_groups.items():
                completed = data.get('completed', [])
                active = data.get('active', [])

                # Create QuestVariable objects
                variables = []
                for var_name in completed:
                    variables.append({
                        'name': var_name,
                        'value': parser.get_variable(var_name, 'int'),
                        'type': 'int',
                        'category': 'completed'
                    })
                for var_name in active:
                    variables.append({
                        'name': var_name,
                        'value': parser.get_variable(var_name, 'int'),
                        'type': 'int',
                        'category': 'active'
                    })

                groups.append({
                    'prefix': prefix,
                    'name': prefix.title(),
                    'variables': variables,
                    'completed_count': len(completed),
                    'active_count': len(active),
                    'total_count': len(completed) + len(active)
                })

            # Calculate statistics
            total_quests = quest_overview.get('total_quest_vars', 0)
            completed_quests = quest_overview.get('completed_count', 0)
            active_quests = quest_overview.get('active_count', 0)
            unknown_quests = total_quests - completed_quests - active_quests
            completion_rate = (completed_quests / total_quests * 100) if total_quests > 0 else 0.0

            return {
                'groups': groups,
                'total_quests': total_quests,
                'completed_quests': completed_quests,
                'active_quests': active_quests,
                'unknown_quests': unknown_quests,
                'completion_rate': completion_rate
            }

        except Exception as e:
            logger.error(f"GameStateManager: Failed to get quest details: {e}")
            return {
                'groups': [],
                'total_quests': 0,
                'completed_quests': 0,
                'active_quests': 0,
                'unknown_quests': 0,
                'completion_rate': 0.0
            }

    def update_quest_variable(self, var_name: str, value: Union[int, str, float], var_type: str = 'int') -> bool:
        """Update a single quest variable"""
        parser = self._get_xml_parser()
        if not parser:
            logger.error("GameStateManager: No XML parser available")
            return False

        try:
            success = parser.update_variable(var_name, value, var_type)
            if success:
                self._save_globals_xml()
                self.emit('quest_variable_updated', {
                    'variable_name': var_name,
                    'value': value,
                    'variable_type': var_type
                })
            return success
        except Exception as e:
            logger.error(f"GameStateManager: Failed to update quest variable: {e}")
            return False

    def batch_update_quests(self, updates: List[Dict[str, Any]]) -> bool:
        """Update multiple quest variables at once"""
        parser = self._get_xml_parser()
        if not parser:
            logger.error("GameStateManager: No XML parser available")
            return False

        try:
            success_count = 0
            for update in updates:
                var_name = update.get('variable_name')
                value = update.get('value')
                var_type = update.get('variable_type', 'int')

                if parser.update_variable(var_name, value, var_type):
                    success_count += 1
                else:
                    logger.warning(f"Failed to update {var_name}")

            if success_count > 0:
                self._save_globals_xml()
                self.emit('quests_batch_updated', {
                    'total_updates': len(updates),
                    'successful': success_count
                })

            return success_count == len(updates)

        except Exception as e:
            logger.error(f"GameStateManager: Failed to batch update quests: {e}")
            return False

    def get_all_campaign_variables(self) -> Dict[str, Any]:
        """Get all campaign variables from globals.xml"""
        parser = self._get_xml_parser()
        if not parser:
            return {
                'integers': {},
                'strings': {},
                'floats': {},
                'total_count': 0
            }

        try:
            integers = parser.get_all_integers()
            strings = parser.get_all_strings()
            floats = parser.get_all_floats()

            return {
                'integers': integers,
                'strings': strings,
                'floats': floats,
                'total_count': len(integers) + len(strings) + len(floats)
            }

        except Exception as e:
            logger.error(f"GameStateManager: Failed to get campaign variables: {e}")
            return {
                'integers': {},
                'strings': {},
                'floats': {},
                'total_count': 0
            }

    def update_campaign_variable(self, var_name: str, value: Union[int, str, float], var_type: str = 'int') -> bool:
        """Update a campaign variable"""
        return self.update_quest_variable(var_name, value, var_type)

    def get_quest_progress(self) -> List[Dict[str, Any]]:
        """Get quest progress with enriched data from quest definitions"""
        parser = self._get_xml_parser()
        if not parser:
            logger.warning("GameStateManager: No XML parser available for quest progress")
            return []

        try:
            quest_overview = parser.get_quest_overview()
            quest_groups = quest_overview.get('quest_groups', {})

            content_manager = self.character_manager.get_manager('content')
            quests = []

            for prefix, data in quest_groups.items():
                completed = data.get('completed', [])
                active = data.get('active', [])

                for var_name in completed + active:
                    quest_value = parser.get_variable(var_name, 'int')
                    is_completed = var_name in completed

                    quest_info = None
                    if content_manager:
                        quest_info = content_manager.get_quest_info(var_name)

                    if quest_info:
                        quests.append({
                            'variable': var_name,
                            'category': quest_info.get('category_name', prefix),
                            'name': quest_info.get('text', var_name),
                            'current_stage': quest_value,
                            'is_completed': is_completed,
                            'xp': quest_info.get('xp', 0),
                            'source': quest_info.get('source', 'unknown')
                        })
                    else:
                        parsed_var = content_manager.parse_variable_name(var_name) if content_manager else None
                        if parsed_var:
                            quests.append({
                                'variable': var_name,
                                'category': parsed_var.get('category', prefix.title()),
                                'name': parsed_var.get('display_name', var_name),
                                'description': parsed_var.get('description', ''),
                                'current_stage': quest_value,
                                'is_completed': is_completed,
                                'xp': 0,
                                'source': 'parsed',
                                'type_hint': parsed_var.get('variable_type_hint', 'state')
                            })
                        else:
                            quests.append({
                                'variable': var_name,
                                'category': prefix.title(),
                                'name': var_name,
                                'current_stage': quest_value,
                                'is_completed': is_completed,
                                'xp': 0,
                                'source': 'unknown'
                            })

            logger.info(f"GameStateManager: Found {len(quests)} quests with progress")
            return quests

        except Exception as e:
            logger.error(f"GameStateManager: Failed to get quest progress: {e}", exc_info=True)
            return []

    def get_all_plot_variables(self) -> Dict[str, Any]:
        """Get all plot variables from globals.xml with quest definition status"""
        parser = self._get_xml_parser()
        if not parser:
            logger.warning("GameStateManager: No XML parser available for plot variables")
            return {
                'quest_variables': [],
                'unknown_variables': [],
                'total_count': 0
            }

        try:
            all_integers = parser.get_all_integers()
            all_strings = parser.get_all_strings()
            all_floats = parser.get_all_floats()

            content_manager = self.character_manager.get_manager('content')

            quest_variables = []
            unknown_variables = []

            for var_name, value in all_integers.items():
                quest_info = content_manager.get_quest_info(var_name) if content_manager else None

                if quest_info:
                    quest_variables.append({
                        'name': var_name,
                        'value': value,
                        'type': 'int',
                        'has_definition': True,
                        'category': quest_info.get('category_name', ''),
                        'quest_text': quest_info.get('text', '')
                    })
                else:
                    parsed_var = content_manager.parse_variable_name(var_name) if content_manager else None
                    if parsed_var:
                        quest_variables.append({
                            'name': var_name,
                            'display_name': parsed_var.get('display_name', var_name),
                            'description': parsed_var.get('description', ''),
                            'value': value,
                            'type': 'int',
                            'has_definition': False,
                            'category': parsed_var.get('category', 'General'),
                            'type_hint': parsed_var.get('variable_type_hint', 'state')
                        })
                    else:
                        unknown_variables.append({
                            'name': var_name,
                            'value': value,
                            'type': 'int',
                            'has_definition': False
                        })

            for var_name, value in all_strings.items():
                parsed_var = content_manager.parse_variable_name(var_name) if content_manager else None
                if parsed_var and ('quest' in var_name.lower() or 'q_' in var_name.lower()):
                    quest_variables.append({
                        'name': var_name,
                        'display_name': parsed_var.get('display_name', var_name),
                        'description': parsed_var.get('description', ''),
                        'value': value,
                        'type': 'string',
                        'has_definition': False,
                        'category': parsed_var.get('category', 'General'),
                        'type_hint': parsed_var.get('variable_type_hint', 'state')
                    })

            for var_name, value in all_floats.items():
                parsed_var = content_manager.parse_variable_name(var_name) if content_manager else None
                if parsed_var and ('quest' in var_name.lower() or 'q_' in var_name.lower()):
                    quest_variables.append({
                        'name': var_name,
                        'display_name': parsed_var.get('display_name', var_name),
                        'description': parsed_var.get('description', ''),
                        'value': value,
                        'type': 'float',
                        'has_definition': False,
                        'category': parsed_var.get('category', 'General'),
                        'type_hint': parsed_var.get('variable_type_hint', 'state')
                    })

            logger.info(f"GameStateManager: Found {len(quest_variables)} quest variables with definitions, {len(unknown_variables)} unknown quest variables")

            return {
                'quest_variables': quest_variables,
                'unknown_variables': unknown_variables,
                'total_count': len(quest_variables) + len(unknown_variables)
            }

        except Exception as e:
            logger.error(f"GameStateManager: Failed to get plot variables: {e}", exc_info=True)
            return {
                'quest_variables': [],
                'unknown_variables': [],
                'total_count': 0
            }

    def get_enriched_quests(self) -> Dict[str, Any]:
        """Get enriched quest data using dialogue mapping service"""
        parser = self._get_xml_parser()
        if not parser:
            logger.warning("GameStateManager: No XML parser available for enriched quests")
            return {
                'quests': [],
                'unmapped_variables': [],
                'stats': {'total': 0, 'completed': 0, 'active': 0, 'unmapped': 0},
                'cache_info': {'cached': False}
            }

        try:
            content_manager = self.character_manager.get_manager('content')
            if not content_manager:
                logger.warning("GameStateManager: No content manager available")
                return self._get_fallback_quests(parser)

            from services.dialogue_mapping_service import get_dialogue_mapping_service
            mapping_service = get_dialogue_mapping_service(content_manager)

            all_integers = parser.get_all_integers()
            
            # Also get module variables (local quest state)
            module_vars = content_manager.get_module_variables()
            if module_vars:
                module_integers = module_vars.get('integers', {})
                # Merge module integers, prioritizing globals if collision (unlikely)
                for k, v in module_integers.items():
                    if k not in all_integers:
                        all_integers[k] = v

            quest_overview = parser.get_quest_overview()
            quest_groups = quest_overview.get('quest_groups', {})

            completed_vars = set()
            active_vars = set()
            for prefix, data in quest_groups.items():
                completed_vars.update(data.get('completed', []))
                active_vars.update(data.get('active', []))

            quests = []
            unmapped_variables = []
            processed_vars = set()

            for var_name, value in all_integers.items():
                if var_name in processed_vars:
                    continue

                is_completed = var_name in completed_vars
                is_active = var_name in active_vars

                quest_info = content_manager.get_quest_info(var_name)
                mapping = None
                if mapping_service:
                    mapping = mapping_service.get_mapping(var_name, value)

                known_values = []
                if mapping_service:
                    all_var_mappings = mapping_service.get_all_mappings_for_variable(var_name)
                    for m in all_var_mappings:
                        quest_def = content_manager.quest_definitions.get(
                            f"{m.journal_tag}_{m.journal_entry_id}"
                        )
                        stage_text = quest_def.get('text', '') if quest_def else ''
                        is_end = quest_def.get('end', 0) == 1 if quest_def else False
                        known_values.append({
                            'value': m.variable_value,
                            'description': stage_text[:100] if stage_text else f"Stage {m.variable_value}",
                            'is_completed': is_end
                        })

                if quest_info or mapping:
                    quest_data = {
                        'variable_name': var_name,
                        'current_value': value,
                        'variable_type': 'int',
                        'quest_info': None,
                        'known_values': known_values,
                        'confidence': 'low',
                        'source': 'campaign' if is_completed or is_active else 'module',
                        'is_completed': is_completed,
                        'is_active': is_active,
                    }

                    if quest_info:
                        quest_data['quest_info'] = {
                            'category': quest_info.get('category_tag', ''),
                            'category_name': quest_info.get('category_name', ''),
                            'entry_id': quest_info.get('entry_id', 0),
                            'quest_name': quest_info.get('category_name', var_name),
                            'current_stage_text': quest_info.get('text', ''),
                            'xp': quest_info.get('xp', 0),
                        }
                        quest_data['confidence'] = 'high'
                        quests.append(quest_data)
                        processed_vars.add(var_name)
                    elif mapping:
                        quest_def = content_manager.quest_definitions.get(
                            f"{mapping.journal_tag}_{mapping.journal_entry_id}"
                        )
                        if quest_def:
                            # Format category_name to match frontend expectations (e.g., "Act 11")
                            journal_prefix = mapping.journal_tag.split('_')[0] if '_' in mapping.journal_tag else ''
                            if journal_prefix.isdigit():
                                formatted_category = f"Act {journal_prefix}"
                            else:
                                formatted_category = quest_def.get('category_name', mapping.journal_tag)

                            quest_data['quest_info'] = {
                                'category': mapping.journal_tag,
                                'category_name': formatted_category,
                                'entry_id': mapping.journal_entry_id,
                                'quest_name': quest_def.get('category_name', mapping.journal_tag),
                                'current_stage_text': quest_def.get('text', ''),
                                'xp': quest_def.get('xp', 0),
                            }
                            if mapping.confidence >= 0.9:
                                quest_data['confidence'] = 'high'
                            elif mapping.confidence >= 0.7:
                                quest_data['confidence'] = 'medium'
                            else:
                                quest_data['confidence'] = 'low'
                            
                            # ONLY add to quests if we have actual text to show
                            if quest_def.get('text'):
                                quests.append(quest_data)
                                processed_vars.add(var_name)
                            else:
                                # No text? It's a technical variable
                                unmapped_variables.append({
                                    'variable_name': var_name,
                                    'display_name': var_name, # Use raw name for technical vars
                                    'current_value': value,
                                    'variable_type': 'int',
                                    'category': formatted_category,
                                })
                                processed_vars.add(var_name)
                        else:
                             # Mapping exists but no definition? Technical variable.
                            unmapped_variables.append({
                                'variable_name': var_name,
                                'display_name': var_name,
                                'current_value': value,
                                'variable_type': 'int',
                                'category': 'Mapped Variables',
                            })
                            processed_vars.add(var_name)

                elif is_completed or is_active:
                    # These are "Active" in globals.xml but have NO mapping to a journal entry.
                    # Therefore they are TECHNICAL VARIABLES, not Quests.
                    parsed_var = content_manager.parse_variable_name(var_name)
                    unmapped_variables.append({
                        'variable_name': var_name,
                        'display_name': parsed_var.get('display_name', var_name),
                        'current_value': value,
                        'variable_type': 'int',
                        'category': parsed_var.get('category', 'General'),
                    })
                    processed_vars.add(var_name)

                else:
                    # Filter out internal engine variables that aren't quest-related
                    skip_prefixes = (
                        '__conv',      # Conversation tracking
                        '_OG',         # Object Group spawns/tracking
                        'WM_',         # World Map pins
                        'N2_S_',       # System variables
                        'N2_',         # Other system variables
                        'Minimal',     # Engine settings
                        'LastWrite',   # Timestamps
                        'CAMPAIGN_',   # Campaign flags
                        'bGATHER_',    # Party gathering
                        'ReadyFor',    # Transition readiness
                        'bReady',      # Ready flags
                        'bDisplay',    # Display flags
                        'bUnlock',     # Unlock flags
                        'district_',   # District tracking
                        'gb',          # Global boolean tracking
                        'Watch',       # Watch/time tracking
                    )
                    if var_name.startswith(skip_prefixes):
                        processed_vars.add(var_name)
                        continue

                    # Also skip variables that are clearly internal counters
                    skip_patterns = ['_Num', '_Indx', 'Fmn', 'Col', 'NumKilled', 'VarType', 'VarValue']
                    if any(pattern in var_name for pattern in skip_patterns):
                        processed_vars.add(var_name)
                        continue

                    parsed_var = content_manager.parse_variable_name(var_name)
                    # Only include variables that look like actual quest tracking
                    if (parsed_var.get('variable_type_hint') == 'progression' or
                        'quest' in var_name.lower() or
                        (var_name.endswith('State') and not var_name.startswith('_'))):
                        unmapped_variables.append({
                            'variable_name': var_name,
                            'display_name': parsed_var.get('display_name', var_name),
                            'current_value': value,
                            'variable_type': 'int',
                            'category': parsed_var.get('category', 'General'),
                        })
                        processed_vars.add(var_name)

            stats = {
                'total': len(quests),
                'completed': len([q for q in quests if q.get('is_completed')]),
                'active': len([q for q in quests if q.get('is_active') and not q.get('is_completed')]),
                'unmapped': len(unmapped_variables),
            }

            cache_info = mapping_service.get_cache_info() if mapping_service else {'cached': False}

            logger.info(f"GameStateManager: Found {stats['total']} quests, {stats['unmapped']} unmapped variables")

            return {
                'quests': quests,
                'unmapped_variables': unmapped_variables,
                'stats': stats,
                'cache_info': cache_info,
            }

        except Exception as e:
            logger.error(f"GameStateManager: Failed to get enriched quests: {e}", exc_info=True)
            return self._get_fallback_quests(parser)

    def _get_fallback_quests(self, parser) -> Dict[str, Any]:
        """Get basic quest data without dialogue mapping as fallback"""
        try:
            quest_overview = parser.get_quest_overview()
            quest_groups = quest_overview.get('quest_groups', {})

            quests = []
            for prefix, data in quest_groups.items():
                completed = data.get('completed', [])
                active = data.get('active', [])

                for var_name in completed + active:
                    value = parser.get_variable(var_name, 'int')
                    quests.append({
                        'variable_name': var_name,
                        'current_value': value,
                        'variable_type': 'int',
                        'quest_info': {
                            'category': prefix,
                            'category_name': prefix.title(),
                            'entry_id': 0,
                            'quest_name': var_name,
                            'current_stage_text': '',
                            'xp': 0,
                        },
                        'known_values': [],
                        'confidence': 'low',
                        'source': 'campaign',
                        'is_completed': var_name in completed,
                        'is_active': var_name in active,
                    })

            return {
                'quests': quests,
                'unmapped_variables': [],
                'stats': {
                    'total': len(quests),
                    'completed': len([q for q in quests if q.get('is_completed')]),
                    'active': len([q for q in quests if q.get('is_active')]),
                    'unmapped': 0,
                },
                'cache_info': {'cached': False},
            }
        except Exception as e:
            logger.error(f"GameStateManager: Fallback quest retrieval failed: {e}")
            return {
                'quests': [],
                'unmapped_variables': [],
                'stats': {'total': 0, 'completed': 0, 'active': 0, 'unmapped': 0},
                'cache_info': {'cached': False},
            }

    def validate(self) -> tuple[bool, list[str]]:
        """Validate the game state manager state"""
        errors = []

        try:
            parser = self._get_xml_parser()
            if parser is None:
                errors.append("XML parser not available")
        except Exception as e:
            errors.append(f"Error validating XML parser: {str(e)}")

        return len(errors) == 0, errors
