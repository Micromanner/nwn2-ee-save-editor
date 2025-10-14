"""
Rule Detector - Auto-detects game rules and requirements from 2DA files
Based on pattern analysis of 442 2DA files, enables dynamic rule detection
without hardcoding for full mod compatibility
"""
from loguru import logger
import re
from typing import Dict, List, Optional, Any, Set, Tuple, Protocol


# Medium-Severity Fix: Use Protocols to define explicit interfaces for dependencies.
class TwoDA(Protocol):
    """Defines the expected interface for a 2DA file object."""
    def get_resource_count(self) -> int: ...
    def get_row_dict(self, row_id: int) -> Dict[str, Any]: ...
    def get_cell(self, row_id: int, column_name: str) -> Any: ...
    def get_rows_as_dicts(self) -> List[Dict[str, Any]]: ...
    def get_column_headers(self) -> List[str]: ...


class ResourceManager(Protocol):
    """Defines the expected interface for the resource manager dependency."""
    def get_2da_with_overrides(self, name: str) -> Optional[TwoDA]: ...


class RuleDetector:
    """
    Automatically detects column purposes and relationships in 2DA files without
    hardcoding, enabling compatibility with mods that follow standard NWN2
    naming conventions. This version is optimized for performance by caching
    column analysis results and relies on explicit data definitions where possible.
    """

    NULL_VALUE = '****'
    MAX_LEVEL = 40

    REQUIREMENT_PATTERNS: Dict[str, re.Pattern] = {
        'min_class_level': re.compile(r'MINLEVELCLASS', re.I),
        'granted_level': re.compile(r'GRANTEDONLEVEL', re.I),
        'min_level': re.compile(r'^(MIN)?LEVEL$', re.I),
        'min_str': re.compile(r'MINSTR', re.I),
        'min_dex': re.compile(r'MINDEX', re.I),
        'min_con': re.compile(r'MINCON', re.I),
        'min_int': re.compile(r'MININT', re.I),
        'min_wis': re.compile(r'MINWIS', re.I),
        'min_cha': re.compile(r'MINCHA', re.I),
        'or_prereq_feat': re.compile(r'ORFEAT\d+$', re.I),
        'prereq_feat': re.compile(r'(PREREQ|REQ)?FEAT\d+$', re.I),
        'alignment_restrict': re.compile(r'ALIGN(MENT)?(RESTRICT|RSTRCT)', re.I),
        'spell_level': re.compile(r'SPELLLEVEL\d+', re.I),
    }

    REFERENCE_PATTERNS: Dict[str, re.Pattern] = {
        'spell_id': re.compile(r'SPELL(ID)?$', re.I),
        'feat_index': re.compile(r'FEAT(INDEX|ID)$', re.I),
        'class_id': re.compile(r'CLASS(ID)?$', re.I),
        'skill_index': re.compile(r'SKILL(INDEX)$', re.I),
        'str_ref': re.compile(r'STR(ING)?REF$', re.I),
        'description': re.compile(r'DESCRIPTION$', re.I),
        'feat_label': re.compile(r'FEATLABEL$', re.I),
        'skill_label': re.compile(r'SKILLLABEL$', re.I),
        'label': re.compile(r'^LABEL$', re.I),
        'name': re.compile(r'NAME$', re.I),
        'icon_ref': re.compile(r'ICON(RESREF)?$', re.I),
        'feats_table': re.compile(r'FEATSTABLE', re.I),
        'skills_table': re.compile(r'SKILLSTABLE', re.I),
        'saving_throw_table': re.compile(r'SAVINGTHROWTABLE', re.I),
        'spell_gain_table': re.compile(r'SPELLGAINTABLE', re.I),
        'spell_known_table': re.compile(r'SPELLKNOWNTABLE', re.I),
        'favored_class': re.compile(r'^FAVOREDCLASS$', re.I),
        'weapon_type': re.compile(r'^WEAPONTYPE$', re.I),
        'base_item': re.compile(r'^BASEITEM$', re.I),
        'domain_id': re.compile(r'^DOMAIN\d*$', re.I),
        'school_id': re.compile(r'^SCHOOL$', re.I),
    }

    _ALL_PATTERNS = {**REQUIREMENT_PATTERNS, **REFERENCE_PATTERNS}
    TABLE_TYPE_PATTERNS: Dict[str, re.Pattern] = {
        'class_feat_progression': re.compile(r'^cls_feat_'),
        'class_skill_list': re.compile(r'^cls_skill_'),
        'class_saves': re.compile(r'^cls_savthr_'),
        'class_prerequisites': re.compile(r'^cls_pres_'),
    }

    def __init__(self, resource_manager: ResourceManager):
        self.rm = resource_manager
        self._cache: Dict[str, Dict[str, Any]] = {}
        self._spells_2da_cache: Optional[TwoDA] = None
        self._classes_2da_cache: Optional[TwoDA] = None
        self._class_spell_columns_cache: Optional[List[str]] = None

    def _get_classes_2da(self) -> Optional[TwoDA]:
        """Loads and caches the classes.2da file."""
        if self._classes_2da_cache is None:
            self._classes_2da_cache = self.rm.get_2da_with_overrides('classes')
        return self._classes_2da_cache

    def get_requirements(self, table_name: str, row_data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Auto-detects and extracts requirements from a 2DA row, using context-aware
        parsers for different requirement formats (e.g., feat.2da vs cls_pres_*.2da).
        """
        self._analyze_and_cache_columns(table_name)
        column_map = self._cache[table_name]['map']
        requirements: Dict[str, Any] = {}
        feats: Dict[str, List[int]] = {}

        # Context-aware parsing for cls_pres_*.2da requirement format
        if 'ReqType' in row_data and str(row_data['ReqType']) != self.NULL_VALUE:
            req_type = str(row_data['ReqType']).upper()
            try:
                if req_type == 'SKILL':
                    skill_id = int(row_data['ReqParam1'])
                    ranks = int(row_data['ReqParam2'])
                    requirements.setdefault('required_skills', []).append({'id': skill_id, 'ranks': ranks})
                elif req_type == 'FEAT':
                    feat_id = int(row_data['ReqParam1'])
                    feats.setdefault('all_of', []).append(feat_id)
            except (ValueError, TypeError, KeyError):
                 logger.warning(f"Could not parse complex requirement in table '{table_name}'. Row: {row_data}")

        # General pattern-based parsing for simple requirements
        for col, value in row_data.items():
            if not col or value is None or str(value) == self.NULL_VALUE: continue
            purpose = column_map.get(col)
            if purpose and purpose in self.REQUIREMENT_PATTERNS:
                try:
                    if purpose == 'prereq_feat': feats.setdefault('all_of', []).append(int(value))
                    elif purpose == 'or_prereq_feat': feats.setdefault('one_of', []).append(int(value))
                    elif purpose not in requirements: requirements[purpose] = int(value)
                except (ValueError, TypeError):
                    logger.warning(f"Could not parse value '{value}' for column '{col}' in table '{table_name}'.")

        # High-Severity Fix: Context-aware parsing for feat.2da's paired skill requirement columns.
        if table_name == 'feat':
            skill_pairs = [('REQSKILL', 'ReqSkillMinRanks'), ('REQSKILL2', 'ReqSkillMinRanks2')]
            for skill_col, rank_col in skill_pairs:
                if skill_col in row_data and rank_col in row_data:
                    skill_val, rank_val = row_data[skill_col], row_data[rank_col]
                    if str(skill_val) != self.NULL_VALUE and str(rank_val) != self.NULL_VALUE:
                        try:
                            skill_id = int(skill_val)
                            ranks = int(rank_val)
                            requirements.setdefault('required_skills', []).append({'id': skill_id, 'ranks': ranks})
                        except (ValueError, TypeError):
                            logger.warning(f"Could not parse skill req pair ('{skill_col}', '{rank_col}') in feat.2da. Row: {row_data}")

        if feats:
            requirements['prereq_feats'] = feats
        return requirements

    def _analyze_and_cache_columns(self, table_name: str) -> None:
        """Analyzes and caches the purpose of each column in a 2DA table."""
        if table_name in self._cache: return
        table_2da = self.rm.get_2da_with_overrides(table_name)
        if not table_2da:
            self._cache[table_name] = {'map': {}, 'type': None}
            return
        column_map: Dict[str, str] = {}
        for header in table_2da.get_column_headers():
            if not header: continue
            for purpose, pattern in self._ALL_PATTERNS.items():
                if pattern.match(header):
                    column_map[header] = purpose
                    break
        table_type = self._get_table_type_no_cache(table_name)
        self._cache[table_name] = {'map': column_map, 'type': table_type}

    def _get_table_type_no_cache(self, table_name: str) -> Optional[str]:
        """Internal method to identify table type without checking cache."""
        for table_type, pattern in self.TABLE_TYPE_PATTERNS.items():
            if pattern.match(table_name.lower()): return table_type
        return None
    
    def find_class_progression_table(self, class_label: str, table_type: str) -> Optional[str]:
        """
        Find the progression table for a class based on its label and type.
        
        Args:
            class_label: The class label (e.g., 'Fighter', 'Wizard')
            table_type: Type of table to find ('feat', 'skill', 'pres', 'savthr', etc.)
            
        Returns:
            Table name if found, None otherwise
        """
        # First check if class has explicit table reference
        classes_2da = self._get_classes_2da()
        if classes_2da:
            # Find the class row
            for i in range(classes_2da.get_resource_count()):
                label = classes_2da.get_string(i, "Label")
                if label and label.lower() == class_label.lower():
                    # Check for explicit table references
                    if table_type == 'feat':
                        table_ref = classes_2da.get_string(i, "FeatsTable")
                        if table_ref and str(table_ref) != self.NULL_VALUE:
                            return table_ref
                    elif table_type == 'skill':
                        table_ref = classes_2da.get_string(i, "SkillsTable")
                        if table_ref and str(table_ref) != self.NULL_VALUE:
                            return table_ref
                    elif table_type == 'savthr':
                        table_ref = classes_2da.get_string(i, "SavingThrowTable")
                        if table_ref and str(table_ref) != self.NULL_VALUE:
                            return table_ref
                    elif table_type == 'pres':
                        table_ref = classes_2da.get_string(i, "PreReqTable")
                        if table_ref and str(table_ref) != self.NULL_VALUE:
                            return table_ref
                    break
        
        # Fallback to pattern-based search
        # Try common naming patterns
        patterns = [
            f"cls_{table_type}_{class_label.lower()}",
            f"cls_{table_type}_{class_label.lower()[:4]}",  # Shortened version
            f"cls_{table_type}_{class_label.lower()[:3]}",  # Even shorter
        ]
        
        for pattern in patterns:
            table = self.rm.get_2da_with_overrides(pattern)
            if table:
                return pattern
        
        return None
    
    def get_available_feats(self, character_data: Dict[str, Any]) -> List[int]:
        """
        Get all feats available to a character based on their current state.
        
        Args:
            character_data: Character information including level, attributes, feats, etc.
            
        Returns:
            List of available feat IDs
        """
        available_feats = []
        feat_2da = self.rm.get_2da_with_overrides('feat')
        
        if not feat_2da:
            return available_feats
        
        char_level = character_data.get('level', 1)
        char_feats = set(character_data.get('feats', []))
        char_skills = character_data.get('skills', {})
        char_classes = character_data.get('classes', {})
        
        # Get character attributes
        abilities = {
            'STR': character_data.get('strength', 10),
            'DEX': character_data.get('dexterity', 10),
            'CON': character_data.get('constitution', 10),
            'INT': character_data.get('intelligence', 10),
            'WIS': character_data.get('wisdom', 10),
            'CHA': character_data.get('charisma', 10)
        }
        
        for i in range(feat_2da.get_resource_count()):
            row = feat_2da.get_row_dict(i)
            if not row or row.get('LABEL') == self.NULL_VALUE:
                continue
            
            # Check if character already has this feat
            if i in char_feats:
                continue
            
            # Get requirements for this feat
            reqs = self.get_requirements('feat', row)
            
            # Check all requirements
            meets_requirements = True
            
            # Check level requirements
            if 'min_level' in reqs and char_level < reqs['min_level']:
                meets_requirements = False
            
            # Check attribute requirements
            for attr in ['min_str', 'min_dex', 'min_con', 'min_int', 'min_wis', 'min_cha']:
                if attr in reqs:
                    attr_key = attr[-3:].upper()
                    if abilities.get(attr_key, 0) < reqs[attr]:
                        meets_requirements = False
                        break
            
            # Check prerequisite feats
            if 'prereq_feats' in reqs:
                # Check "all of" requirements
                for feat_id in reqs['prereq_feats'].get('all_of', []):
                    if feat_id not in char_feats:
                        meets_requirements = False
                        break
                
                # Check "one of" requirements
                one_of = reqs['prereq_feats'].get('one_of', [])
                if one_of and not any(f in char_feats for f in one_of):
                    meets_requirements = False
            
            # Check skill requirements
            if 'required_skills' in reqs:
                for skill_req in reqs['required_skills']:
                    skill_id = skill_req['id']
                    required_ranks = skill_req['ranks']
                    if char_skills.get(skill_id, 0) < required_ranks:
                        meets_requirements = False
                        break
            
            # Check class-specific requirements
            if 'min_level_class' in reqs:
                # This would need the specific class ID from somewhere
                # For now, just check if they have any levels in any class
                if not char_classes:
                    meets_requirements = False
            
            if meets_requirements:
                available_feats.append(i)
        
        return available_feats
    
    def get_spell_classes(self, spell_id: int) -> Dict[str, int]:
        """
        Get which classes can cast a spell and at what level.
        
        Args:
            spell_id: The spell ID to check
            
        Returns:
            Dict mapping class column names to spell levels
        """
        if self._spells_2da_cache is None:
            self._spells_2da_cache = self.rm.get_2da_with_overrides('spells')
        
        if not self._spells_2da_cache or spell_id >= self._spells_2da_cache.get_resource_count():
            return {}
        
        # Get spell row
        spell_row = self._spells_2da_cache.get_row_dict(spell_id)
        if not spell_row:
            return {}
        
        # Cache class spell columns if not already done
        if self._class_spell_columns_cache is None:
            self._class_spell_columns_cache = []
            # Common class spell columns in spells.2da
            potential_columns = [
                'Bard', 'Cleric', 'Druid', 'Paladin', 'Ranger',
                'Wiz_Sorc', 'Wizard', 'Sorcerer', 'Warlock',
                'Spirit_Shaman', 'Favored_Soul'
            ]
            
            # Check which columns actually exist
            headers = self._spells_2da_cache.get_column_headers()
            for col in potential_columns:
                if col in headers:
                    self._class_spell_columns_cache.append(col)
        
        # Extract spell levels for each class
        result = {}
        for col in self._class_spell_columns_cache:
            level = spell_row.get(col)
            if level is not None and str(level) != self.NULL_VALUE:
                try:
                    level_int = int(level)
                    if level_int >= 0 and level_int <= 9:  # Valid spell levels
                        result[col] = level_int
                except (ValueError, TypeError):
                    pass
        
        return result
    
    def detect_relationships(self, table_name: str, columns: List[str]) -> Dict[str, Tuple[str, str]]:
        """
        Detect foreign key relationships from column names.
        
        Args:
            table_name: Name of the table being analyzed
            columns: List of column names
            
        Returns:
            Dict mapping column names to (target_table, relationship_type) tuples
        """
        relationships = {}
        
        for column in columns:
            if not column:
                continue
            
            # Check against reference patterns
            for ref_type, pattern in self.REFERENCE_PATTERNS.items():
                if pattern.match(column):
                    # Map reference type to target table
                    if ref_type == 'spell_id':
                        relationships[column] = ('spells', 'lookup')
                    elif ref_type == 'feat_index':
                        relationships[column] = ('feat', 'lookup')
                    elif ref_type == 'class_id':
                        relationships[column] = ('classes', 'lookup')
                    elif ref_type == 'skill_index':
                        relationships[column] = ('skills', 'lookup')
                    elif ref_type == 'favored_class':
                        relationships[column] = ('classes', 'lookup')
                    elif ref_type == 'weapon_type':
                        relationships[column] = ('weapontypes', 'lookup')
                    elif ref_type == 'base_item':
                        relationships[column] = ('baseitems', 'lookup')
                    elif ref_type == 'domain_id':
                        relationships[column] = ('domains', 'lookup')
                    elif ref_type == 'school_id':
                        relationships[column] = ('spellschools', 'lookup')
                    elif ref_type in ('feats_table', 'skills_table', 'saving_throw_table',
                                     'spell_gain_table', 'spell_known_table'):
                        # These contain table names
                        relationships[column] = ('dynamic', 'table_reference')
                    break
            
            # Additional pattern checks for columns not caught by REFERENCE_PATTERNS
            column_lower = column.lower()
            
            # Check for ID suffixes
            if column_lower.endswith('_id') or column_lower.endswith('id'):
                if column not in relationships:
                    # Try to infer table name
                    potential_table = column_lower.replace('_id', '').replace('id', '')
                    # Common mappings
                    table_mappings = {
                        'spell': 'spells',
                        'feat': 'feat',
                        'class': 'classes',
                        'skill': 'skills',
                        'race': 'racialtypes',
                        'item': 'baseitems',
                        'baseitem': 'baseitems',
                    }
                    if potential_table in table_mappings:
                        relationships[column] = (table_mappings[potential_table], 'lookup')
            
            # Check for table suffixes
            elif column_lower.endswith('table'):
                if column not in relationships:
                    relationships[column] = ('dynamic', 'table_reference')
            
            # Check for PREREQFEAT patterns
            elif self.REQUIREMENT_PATTERNS['prereq_feat'].match(column):
                relationships[column] = ('feat', 'lookup')
            elif self.REQUIREMENT_PATTERNS['or_prereq_feat'].match(column):
                relationships[column] = ('feat', 'lookup')
        
        return relationships
    
    def get_column_purpose(self, table_name: str, column_name: str) -> Optional[str]:
        """
        Get the detected purpose of a column.
        
        Args:
            table_name: Table containing the column
            column_name: Column to analyze
            
        Returns:
            Purpose string if detected, None otherwise
        """
        self._analyze_and_cache_columns(table_name)
        column_map = self._cache.get(table_name, {}).get('map', {})
        return column_map.get(column_name)