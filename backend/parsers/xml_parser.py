"""
Parser for NWN2 XML files containing quest states and global variables.
This version uses a data-driven, hybrid approach for maximum flexibility
and can heuristically discover companions in custom modules.
"""
import xml.etree.ElementTree as ET
import re
from typing import Dict, List, Set, Tuple, Any
import logging
from datetime import datetime, timezone
import pprint

# --- Basic Logging Setup ---
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

# --- Configuration for Companion Detection (Explicit) ---
# This remains the most reliable way to get full details for known companions.
COMPANION_DEFINITIONS: Dict[str, Dict[str, str]] = {
    # Companion IDs should be lowercase and simple for consistent keying
    'neeshka': {
        'name': 'Neeshka',
        'influence_var': r'00_nInfluenceneeshka',
        'joined_var': r'00_bNeeshka_Joined',
    },
    'khelgar': {
        'name': 'Khelgar',
        'influence_var': r'00_nInfluencekhelgar',
        'joined_var': r'00_bKhelgar_Joined',
    },
    'elanee': {
        'name': 'Elanee',
        'influence_var': r'00_nInfluenceelanee',
        'joined_var': r'00_bElanee_Joined',
    },
    'qara': {
        'name': 'Qara',
        'influence_var': r'00_nInfluenceqara',
        'joined_var': r'00_bQaraJoined',
    },
    'casavir': {
        'name': 'Casavir',
        'influence_var': r'00_nInfluencecasavir',
        'joined_var': r'00_bCasavir_Joined',
    },
    'grobnar': {
        'name': 'Grobnar',
        'influence_var': r'00_nInfluencegrobnar',
        'joined_var': r'00_bGrobnar_Joined',
    },
    'sand': {
        'name': 'Sand',
        'influence_var': r'00_nInfluencesand',
        'joined_var': r'00_bSand_Joined',
        'met_var': r'SandIntroDone',
    },
    'bishop': {
        'name': 'Bishop',
        'influence_var': r'00_nInfluencebishop',
        'joined_var': r'00_bBishop_Joined',
    },
    'shandra': {
        'name': 'Shandra',
        'influence_var': r'00_nInfluenceshandra',
        'joined_var': r'00_bShandra_Joined',
        'met_var': r'bShandraMet',
    },
    'ammon_jerro': {
        'name': 'Ammon Jerro',
        'influence_var': r'00_nInfluenceammon',
        'joined_var': r'00_bAmmon_Joined',
        'met_var': r'bAmmonMet',
    },
    'zhjaeve': {
        'name': 'Zhjaeve',
        'influence_var': r'00_nInfluencezhjaeve',
        'joined_var': r'00_bZhjaeve_Joined',
        'met_var': r'bZhjaeveMet',
    },
    'construct': {
        'name': 'Construct',
        'influence_var': r'00_nInfluenceconstruct',
        'joined_var': r'00_bConstruct_Joined',
        'met_var': r'bConstructMet',
    },
    'safiya': {
        'name': 'Safiya',
        'influence_var': r'00_nInfluencesafiya',
        'joined_var': r'00_bSafiya_Joined',
        'met_var': r'bSafiyaMet',
    },
    'gann': {
        'name': 'Gann',
        'influence_var': r'00_nInfluencegann',
        'joined_var': r'00_bGann_Joined',
        'met_var': r'bGannMet',
    },
    'kaelyn': {
        'name': 'Kaelyn the Dove',
        'influence_var': r'00_nInfluencekaelyn',
        'joined_var': r'00_bKaelyn_Joined',
        'met_var': r'bKaelynMet',
    },
    'okku': {
        'name': 'Okku',
        'influence_var': r'00_nInfluenceokku',
        'joined_var': r'00_bOkku_Joined',
        'met_var': r'bOkkuMet',
    },
    'one_of_many': {
        'name': 'One of Many',
        'influence_var': r'00_nInfluenceoneofmany',
        'joined_var': r'00_bOneOfMany_Joined',
        'met_var': r'bOneOfManyMet',
    },
}

# --- Configuration for Quest Variable Identification ---
# Prioritized list of patterns to categorize integer variables. Lower priority number is checked first.
QUEST_PATTERNS: List[Tuple[str, str, int]] = [
    # 1. Exclusions (highest priority to be skipped)
    (r'^_OG.*', 'exclude', 1),
    (r'^__conv.*', 'exclude', 1),
    (r'^WM_.*', 'exclude', 1),
    (r'.*(Num|Indx|Count|Fmn|Spc|Col|StN|FcN|LcN)$', 'exclude', 1),
    (r'.*NumKilled$', 'exclude', 1),
    (r'.*Influence.*', 'exclude', 1), # Exclude from quests, handled separately
    (r'.*(rep|reputation)$', 'exclude', 1), # Exclude from quests
    (r'^(MinimalDifficultyLevel|LastWriteTime|CAMPAIGN_SETUP_FLAG|N2_.*)$', 'exclude', 1),
    
    # 2. Completion Patterns
    (r'.*(Done|Over|Dead)$', 'completed', 5),
    (r'.*Complete(d)?$', 'completed', 5),
    
    # 3. Active/State Patterns
    (r'.*(State|Plot)$', 'state', 10), # Special handling for progression
    (r'.*(Quest|Mission|Intro|Go|Enabled|Visited|Met)$', 'active', 11),
]


class XmlParser:
    """Parser for NWN2 XML files (globals.xml, etc.)"""
    
    def __init__(self, xml_content: str):
        """
        Initialize parser with XML content
        
        Args:
            xml_content: Raw XML content from a globals.xml file.
        """
        self.xml_content = xml_content
        self.data: Dict[str, Dict[str, Any]] = {
            'integers': {},
            'strings': {},
            'floats': {},
            'vectors': {}
        }
        self._parse()

    def _parse(self):
        """Parse the XML content into structured data dictionaries."""
        try:
            root = ET.fromstring(self.xml_content)
            
            def parse_elements(parent_tag, child_tag, value_tags):
                data_dict = {}
                parent_elem = root.find(parent_tag)
                if parent_elem is None:
                    return data_dict
                
                for child_elem in parent_elem.findall(child_tag):
                    name_elem = child_elem.find('Name')
                    if name_elem is None or name_elem.text is None:
                        continue
                    
                    name = name_elem.text
                    values = {}
                    all_found = True
                    for tag_name, cast_func in value_tags.items():
                        value_elem = child_elem.find(tag_name)
                        if value_elem is not None and value_elem.text is not None:
                            try:
                                values[tag_name.lower()] = cast_func(value_elem.text)
                            except (ValueError, TypeError):
                                all_found = False
                                break
                        else:
                            all_found = False
                            break
                    
                    if all_found:
                        data_dict[name] = values if len(values) > 1 else list(values.values())[0]

                return data_dict

            self.data['integers'] = parse_elements('Integers', 'Integer', {'Value': int})
            self.data['strings'] = parse_elements('Strings', 'String', {'Value': str})
            self.data['floats'] = parse_elements('Floats', 'Float', {'Value': float})
            self.data['vectors'] = parse_elements('Vectors', 'Vector', {'X': float, 'Y': float, 'Z': float})
                        
        except ET.ParseError as e:
            logger.error(f"Failed to parse XML: {e}")
            raise ValueError(f"Invalid XML format: {e}") from e

    def _discover_potential_companions(self) -> Dict[str, Dict[str, Any]]:
        """
        Heuristically discovers companions by searching for influence variables
        with all-lowercase names, based on common scripting conventions.
        """
        discovered = {}
        # Pattern: (optional prefix)(inf/influence)(companion_name_lowercase)
        influence_pattern = re.compile(r'^(?:[a-zA-Z0-9_]*_)?(?:inf|influence)([a-z]+)$', re.IGNORECASE)
        blacklist = {'of', 'the', 'level', 'count', 'quest', 'plot', 'state'}

        for var_name, value in self.data['integers'].items():
            match = influence_pattern.match(var_name)
            if match:
                companion_name = match.group(1)
                if len(companion_name) > 2 and companion_name not in blacklist:
                    if companion_name not in discovered:
                        discovered[companion_name] = {
                            'name': companion_name.capitalize(),
                            'influence': value,
                            'recruitment': 'unknown',
                            'source': 'discovered'
                        }
        return discovered

    def _identify_quest_vars(self) -> Tuple[Set[str], Set[str]]:
        """
        Internal helper to identify and categorize quest-related variables.
        
        Returns:
            A tuple containing two sets: (completed_quests, active_quests)
        """
        completed_quests = set()
        active_quests = set()
        
        sorted_patterns = sorted(QUEST_PATTERNS, key=lambda item: item[2])

        for var_name, value in self.data['integers'].items():
            if value <= 0:
                continue

            for pattern, category, _ in sorted_patterns:
                if re.match(pattern, var_name, re.IGNORECASE):
                    if category == 'exclude':
                        break
                    
                    if category == 'completed':
                        completed_quests.add(var_name)
                        break
                        
                    if category == 'active':
                        active_quests.add(var_name)
                        break
                    
                    if category == 'state':
                        # Heuristic: High state values often mean completion.
                        if value >= 50: 
                            completed_quests.add(var_name)
                        else:
                            active_quests.add(var_name)
                        break
        
        active_quests -= completed_quests
        return completed_quests, active_quests

    def get_companion_status(self) -> Dict[str, Dict[str, Any]]:
        """
        Gets detailed companion status using both explicit definitions and heuristic discovery.
        """
        companion_status = {}
        # 1. Get high-confidence data from explicit definitions
        for comp_id, definition in COMPANION_DEFINITIONS.items():
            status = {
                'name': definition['name'],
                'influence': None,
                'recruitment': 'not_recruited',
                'source': 'explicit'
            }
            if (var := definition.get('influence_var')) in self.data['integers']:
                status['influence'] = self.data['integers'][var]
            if self.data['integers'].get(definition.get('joined_var', ''), 0) > 0:
                status['recruitment'] = 'recruited'
            elif self.data['integers'].get(definition.get('met_var', ''), 0) > 0:
                status['recruitment'] = 'met'
            if status['influence'] is not None or status['recruitment'] != 'not_recruited':
                companion_status[comp_id] = status

        # 2. Discover potential companions and merge results
        discovered_companions = self._discover_potential_companions()
        for comp_id, status in discovered_companions.items():
            if comp_id not in companion_status:
                companion_status[comp_id] = status
                
        return companion_status

    def get_quest_overview(self) -> Dict[str, Any]:
        """
        Groups quest variables by common prefixes to infer questlines.
        This method is more universal and works well for custom content.
        """
        completed, active = self._identify_quest_vars()
        all_quest_vars = completed.union(active)
        prefix_pattern = re.compile(r'^([a-zA-Z0-9]+_)+|^([a-zA-Z]+)')
        quest_groups = {}

        for var in all_quest_vars:
            match = prefix_pattern.match(var)
            if match:
                group_key = next((g for g in match.groups() if g), var).rstrip('_')
                if group_key not in quest_groups:
                    quest_groups[group_key] = {'completed': [], 'active': []}
                status = 'completed' if var in completed else 'active'
                quest_groups[group_key][status].append(var)

        return {
            'completed_count': len(completed),
            'active_count': len(active),
            'total_quest_vars': len(all_quest_vars),
            'quest_groups': {k: v for k, v in sorted(quest_groups.items())}
        }

    def get_general_info(self) -> Dict[str, Any]:
        """
        Extracts general campaign information like character name, act, and last save time.
        """
        info = {'player_name': None, 'game_act': None, 'last_saved': None}
        name_vars = ['MainCharacter', 'PlayerName']
        for var in name_vars:
            if var in self.data['strings']:
                info['player_name'] = self.data['strings'][var]
                break
        
        if '00_nAct' in self.data['integers']:
            info['game_act'] = self.data['integers']['00_nAct']

        if 'LastWriteTime' in self.data['integers']:
            timestamp = self.data['integers']['LastWriteTime']
            try:
                dt = datetime.fromtimestamp(timestamp, tz=timezone.utc)
                info['last_saved'] = dt.isoformat()
            except (ValueError, OSError):
                logger.warning(f"Invalid timestamp: {timestamp}")
                info['last_saved'] = f"Invalid timestamp: {timestamp}"
                
        return info

    def get_full_summary(self) -> Dict[str, Any]:
        """
        Generates a comprehensive summary for frontend display, including all discovered data.
        """
        companions = self.get_companion_status()

        return {
            'general_info': self.get_general_info(),
            'companion_status': companions,
            'quest_overview': self.get_quest_overview(),
            'raw_data_counts': {
                'integers': len(self.data['integers']),
                'strings': len(self.data['strings']),
                'floats': len(self.data['floats']),
                'vectors': len(self.data['vectors']),
            }
        }
    
    def update_companion_influence(self, companion_id: str, new_influence: int) -> bool:
        """
        Update the influence value for a specific companion.
        
        Args:
            companion_id: The companion identifier (e.g., 'neeshka', 'khelgar')
            new_influence: The new influence value to set
            
        Returns:
            True if update was successful, False otherwise
        """
        # Check if companion exists in definitions
        if companion_id in COMPANION_DEFINITIONS:
            influence_var = COMPANION_DEFINITIONS[companion_id].get('influence_var')
            if influence_var and influence_var in self.data['integers']:
                self.data['integers'][influence_var] = new_influence
                logger.info(f"Updated {companion_id} influence to {new_influence}")
                return True
        
        # Try discovered companions
        discovered = self._discover_potential_companions()
        if companion_id in discovered:
            # Try to find the influence variable
            influence_pattern = re.compile(rf'^(?:[a-zA-Z0-9_]*_)?(?:inf|influence){companion_id}$', re.IGNORECASE)
            for var_name in self.data['integers'].keys():
                if influence_pattern.match(var_name):
                    self.data['integers'][var_name] = new_influence
                    logger.info(f"Updated discovered companion {companion_id} influence to {new_influence}")
                    return True
        
        logger.warning(f"Could not find influence variable for companion: {companion_id}")
        return False

    def get_all_integers(self) -> Dict[str, int]:
        """Get all integer variables from globals.xml"""
        return self.data['integers'].copy()

    def get_all_strings(self) -> Dict[str, str]:
        """Get all string variables from globals.xml"""
        return self.data['strings'].copy()

    def get_all_floats(self) -> Dict[str, float]:
        """Get all float variables from globals.xml"""
        return self.data['floats'].copy()

    def get_variable(self, var_name: str, var_type: str) -> Any:
        """
        Get a specific variable value by name and type.

        Args:
            var_name: Variable name
            var_type: Variable type ('int', 'string', 'float')

        Returns:
            Variable value or None if not found
        """
        type_map = {
            'int': 'integers',
            'string': 'strings',
            'float': 'floats'
        }

        data_key = type_map.get(var_type)
        if not data_key:
            logger.warning(f"Invalid variable type: {var_type}")
            return None

        return self.data[data_key].get(var_name)

    def update_integer(self, var_name: str, value: int) -> bool:
        """
        Update or create an integer variable.

        Args:
            var_name: Variable name
            value: New integer value

        Returns:
            True if successful
        """
        try:
            self.data['integers'][var_name] = int(value)
            logger.info(f"Updated integer variable {var_name} = {value}")
            return True
        except (ValueError, TypeError) as e:
            logger.error(f"Failed to update integer {var_name}: {e}")
            return False

    def update_string(self, var_name: str, value: str) -> bool:
        """
        Update or create a string variable.

        Args:
            var_name: Variable name
            value: New string value

        Returns:
            True if successful
        """
        try:
            self.data['strings'][var_name] = str(value)
            logger.info(f"Updated string variable {var_name} = {value}")
            return True
        except Exception as e:
            logger.error(f"Failed to update string {var_name}: {e}")
            return False

    def update_float(self, var_name: str, value: float) -> bool:
        """
        Update or create a float variable.

        Args:
            var_name: Variable name
            value: New float value

        Returns:
            True if successful
        """
        try:
            self.data['floats'][var_name] = float(value)
            logger.info(f"Updated float variable {var_name} = {value}")
            return True
        except (ValueError, TypeError) as e:
            logger.error(f"Failed to update float {var_name}: {e}")
            return False

    def update_variable(self, var_name: str, value: Any, var_type: str) -> bool:
        """
        Update a variable of any type.

        Args:
            var_name: Variable name
            value: New value
            var_type: Variable type ('int', 'string', 'float')

        Returns:
            True if successful, False otherwise
        """
        if var_type == 'int':
            return self.update_integer(var_name, value)
        elif var_type == 'string':
            return self.update_string(var_name, value)
        elif var_type == 'float':
            return self.update_float(var_name, value)
        else:
            logger.warning(f"Unknown variable type: {var_type}")
            return False

    def delete_variable(self, var_name: str, var_type: str) -> bool:
        """
        Delete a variable.

        Args:
            var_name: Variable name
            var_type: Variable type ('int', 'string', 'float')

        Returns:
            True if deleted, False if not found
        """
        type_map = {
            'int': 'integers',
            'string': 'strings',
            'float': 'floats'
        }

        data_key = type_map.get(var_type)
        if not data_key:
            logger.warning(f"Invalid variable type: {var_type}")
            return False

        if var_name in self.data[data_key]:
            del self.data[data_key][var_name]
            logger.info(f"Deleted {var_type} variable: {var_name}")
            return True

        logger.warning(f"Variable not found: {var_name} ({var_type})")
        return False

    def to_xml_string(self) -> str:
        """
        Convert the current data back to XML string format.
        
        Returns:
            XML string representation of the data
        """
        root = ET.Element('Globals')
        
        # Add integers
        integers_elem = ET.SubElement(root, 'Integers')
        for name, value in self.data['integers'].items():
            int_elem = ET.SubElement(integers_elem, 'Integer')
            name_elem = ET.SubElement(int_elem, 'Name')
            name_elem.text = name
            value_elem = ET.SubElement(int_elem, 'Value')
            value_elem.text = str(value)
        
        # Add strings
        strings_elem = ET.SubElement(root, 'Strings')
        for name, value in self.data['strings'].items():
            str_elem = ET.SubElement(strings_elem, 'String')
            name_elem = ET.SubElement(str_elem, 'Name')
            name_elem.text = name
            value_elem = ET.SubElement(str_elem, 'Value')
            value_elem.text = value
        
        # Add floats
        floats_elem = ET.SubElement(root, 'Floats')
        for name, value in self.data['floats'].items():
            float_elem = ET.SubElement(floats_elem, 'Float')
            name_elem = ET.SubElement(float_elem, 'Name')
            name_elem.text = name
            value_elem = ET.SubElement(float_elem, 'Value')
            value_elem.text = str(value)
        
        # Add vectors
        vectors_elem = ET.SubElement(root, 'Vectors')
        for name, value in self.data['vectors'].items():
            vec_elem = ET.SubElement(vectors_elem, 'Vector')
            name_elem = ET.SubElement(vec_elem, 'Name')
            name_elem.text = name
            if isinstance(value, dict):
                x_elem = ET.SubElement(vec_elem, 'X')
                x_elem.text = str(value.get('x', 0))
                y_elem = ET.SubElement(vec_elem, 'Y')
                y_elem.text = str(value.get('y', 0))
                z_elem = ET.SubElement(vec_elem, 'Z')
                z_elem.text = str(value.get('z', 0))
        
        # Convert to string with proper formatting
        ET.indent(root, space='  ')
        return ET.tostring(root, encoding='unicode', xml_declaration=True)

