"""Field Mapping Utility - Centralized 2DA field name mapping for NWN2 compatibility."""
from loguru import logger
from typing import Dict, List, Optional, Any, Union


class FieldMappingUtility:
    """Centralized field name mapping utility for NWN2 2DA files."""

    # Cache for reverse field mapping (built once on first use)
    _reverse_field_map = None

    # Comprehensive field mapping patterns
    FIELD_PATTERNS = {
        'str_adjust': ['str_adjust', 'StrAdjust', 'strength_adjust', 'STRAdjust', 'StrMod'],
        'dex_adjust': ['dex_adjust', 'DexAdjust', 'dexterity_adjust', 'DEXAdjust', 'DexMod'],
        'con_adjust': ['con_adjust', 'ConAdjust', 'constitution_adjust', 'CONAdjust', 'ConMod'],
        'int_adjust': ['int_adjust', 'IntAdjust', 'intelligence_adjust', 'INTAdjust', 'IntMod'],
        'wis_adjust': ['wis_adjust', 'WisAdjust', 'wisdom_adjust', 'WISAdjust', 'WisMod'],
        'cha_adjust': ['cha_adjust', 'ChaAdjust', 'charisma_adjust', 'CHAAdjust', 'ChaMod'],
        'fort_save': ['fort_save', 'fortitude_save', 'FortSave', 'Fort', 'FortitudeBonus', 'FortBonus'],
        'ref_save': ['ref_save', 'reflex_save', 'RefSave', 'Ref', 'ReflexBonus', 'RefBonus'],
        'will_save': ['will_save', 'WillSave', 'Will', 'WillBonus'],
        'creature_size': ['creature_size', 'size', 'CreatureSize', 'Size', 'RaceSize'],
        'movement_rate': ['movement_rate', 'base_speed', 'speed', 'MovementRate', 'BaseSpeed', 'Speed', 'Endurance'],
        'ac_attack_mod': ['ACATTACKMOD', 'ac_attack_mod', 'AcAttackMod', 'ACAttackMod', 'ACMod'],
        'label': ['LABEL', 'label', 'Label'],
        'name': ['NAME', 'name', 'Name', 'label', 'Label', 'NameRef'],
        'feat_name_strref': ['FEAT', 'Feat'],
        'key_ability': ['KeyAbility', 'key_ability', 'keyability', 'KEYABILITY'],
        'skill_index': ['SkillIndex', 'skill_index', 'skillindex', 'SKILLINDEX', 'Skill'],
        'class_skill': ['ClassSkill', 'class_skill', 'classskill', 'CLASSSKILL', 'IsClassSkill'],
        'prereq_str': ['prereq_str', 'PreReqStr', 'MinStr', 'min_str', 'ReqStr'],
        'prereq_dex': ['prereq_dex', 'PreReqDex', 'MinDex', 'min_dex', 'ReqDex'],
        'prereq_con': ['prereq_con', 'PreReqCon', 'MinCon', 'min_con', 'ReqCon'],
        'prereq_int': ['prereq_int', 'PreReqInt', 'MinInt', 'min_int', 'ReqInt'],
        'prereq_wis': ['prereq_wis', 'PreReqWis', 'MinWis', 'min_wis', 'ReqWis'],
        'prereq_cha': ['prereq_cha', 'PreReqCha', 'MinCha', 'min_cha', 'ReqCha'],
        'prereq_feat1': ['prereq_feat1', 'PreReqFeat1', 'ReqFeat1', 'prereqfeat1', 'PREREQFEAT1'],
        'prereq_feat2': ['prereq_feat2', 'PreReqFeat2', 'ReqFeat2', 'prereqfeat2', 'PREREQFEAT2'],
        'prereq_bab': ['prereq_bab', 'PreReqBAB', 'MinAttackBonus', 'MinBAB', 'ReqBAB'],
        'required_class': ['required_class', 'reqclass', 'ReqClass', 'ClassReq', 'MinLevelClass'],
        'min_level': ['min_level', 'minlevel', 'MinLevel', 'LevelReq', 'ReqLevel'],
        'prereq_spell_level': ['prereq_spell_level', 'MinSpell', 'SpellLevel', 'ReqSpellLevel'],
        'favored_class': ['favored_class', 'FavoredClass', 'favoured_class', 'FavouredClass'],
        'feat_index': ['FeatIndex', 'feat_index', 'featindex', 'feat_id'],
        'granted_on_level': ['GrantedOnLevel', 'granted_on_level', 'grantedlevel', 'level'],
        'racial_feats': ['racial_feats', 'feats', 'special_abilities', 'RacialFeats', 'Feats'],
        'feat0': ['Feat0', 'feat0', 'Feat', 'feat'],
        'feat1': ['Feat1', 'feat1'],
        'feat2': ['Feat2', 'feat2'],
        'feat3': ['Feat3', 'feat3'],
        'feat4': ['Feat4', 'feat4'],
        'feat5': ['Feat5', 'feat5'],
        'attack_bonus_table': ['AttackBonusTable', 'attack_bonus_table', 'AttackTable', 'BABTable'],
        'saving_throw_table': ['SavingThrowTable', 'saving_throw_table', 'SaveTable', 'SavTable'],
        'skills_table': ['SkillsTable', 'skills_table', 'SkillTable'],
        'feats_table': ['FeatsTable', 'feats_table', 'FeatTable', 'featstable', 'FEATSTABLE'],
        'bonus_feats_table': ['BonusFeatsTable', 'bonus_feats_table', 'BonusFeatTable', 'bonus_feat_table'],
        'hit_die': ['HitDie', 'hit_die', 'HD', 'HitDice'],
        'skill_point_base': ['SkillPointBase', 'skill_point_base', 'SkillPoints', 'SP'],
        'max_level': ['MaxLevel', 'max_level', 'max_lvl', 'MaxLvl'],
        'has_arcane': ['HasArcane', 'has_arcane', 'arcane', 'Arcane'],
        'has_divine': ['HasDivine', 'has_divine', 'divine', 'Divine'],
        'primary_ability': ['PrimaryAbil', 'primary_ability', 'primary_abil', 'PrimAbil'],
        'bab': ['BAB', 'bab', 'AttackBonus', 'BaseAttack'],
        'fort_save_table': ['FortSave', 'fort_save', 'fort', 'Fort', 'FortitudeBonus'],
        'ref_save_table': ['RefSave', 'ref_save', 'ref', 'Ref', 'ReflexBonus'],
        'will_save_table': ['WillSave', 'will_save', 'will', 'Will', 'WillBonus'],
        'spell_caster': ['SpellCaster', 'spell_caster', 'IsCaster', 'Caster'],
        'prereq_table': ['PreReqTable', 'prereq_table', 'prereqtable', 'PrerequisiteTable'],
        'spell_gain_table': ['spell_gain_table', 'SpellGainTable', 'SpellTable'],
        'spell_known_table': ['spell_known_table', 'SpellKnownTable', 'KnownTable'],
        'align_restrict': ['align_restrict', 'AlignRestrict', 'AlignmentRestrict'],
        'align_restrict_type': ['align_restrict_type', 'AlignRstrctType', 'AlignmentType'],
        'player_race': ['player_race', 'PlayerRace', 'PCRace', 'Playable'],
        'player_class': ['player_class', 'PlayerClass', 'PCClass', 'Playable'],
        'icon': ['ICON', 'icon', 'Icon', 'IconResRef', 'IconRef'],
        'bordered_icon': ['bordered_icon', 'BorderedIcon', 'IconBordered'],
        'damage_type': ['damage_type', 'DamageType', 'DmgType'],
        'damage_die': ['damage_die', 'DamageDie', 'DmgDie'],
        'crit_threat': ['crit_threat', 'CritThreat', 'ThreatRange'],
        'crit_mult': ['crit_mult', 'CritMult', 'CritMultiplier'],
        'base_item': ['base_item', 'BaseItem', 'ItemType', 'Type'],
        'item_class': ['item_class', 'ItemClass', 'Class'],
        'weapon_type': ['weapon_type', 'WeaponType', 'WpnType'],
        'cost': ['cost', 'Cost', 'Price', 'Value'],
        'weight': ['weight', 'Weight', 'Wt'],
        'base_race': ['base_race', 'BaseRace', 'baserace', 'BASERACE'],
        'subrace_name': ['Name', 'name', 'Label', 'label'],
        'subrace_label': ['Label', 'label', 'Name', 'name'],
        'effective_character_level': ['ecl', 'ECL', 'effective_character_level', 'EffectiveCharacterLevel'],
        'has_favored_class': ['has_favored_class', 'HasFavoredClass', 'hasfavoredclass', 'HASFAVOREDCLASS'],
        'description': ['DESCRIPTION', 'description', 'Description', 'Desc', 'DescRef'],
        'category': ['category', 'Category', 'Cat', 'Type'],
        'type': ['CATEGORY', 'FeatCategory', 'Category', 'Type', 'category'],
        'constant': ['constant', 'Constant', 'Const', 'ConstantValue'],
        'bonus': ['Bonus', 'bonus', 'BONUS']
    }
    
    def get_field_value(self, data_object: Any, field_pattern: str, default: Any = None) -> Any:
        """Get field value from data object using field pattern mapping."""
        if data_object is None:
            return default
            
        field_names = self.FIELD_PATTERNS.get(field_pattern, [field_pattern])
        
        for field_name in field_names:
            try:
                if hasattr(data_object, field_name):
                    value = getattr(data_object, field_name)
                    if (value is not None and 
                        str(value).strip() and 
                        str(value) != '****' and
                        not str(value).startswith('<Mock')):
                        return value
                
                for case_variant in [field_name.lower(), field_name.upper()]:
                    if case_variant != field_name and hasattr(data_object, case_variant):
                        value = getattr(data_object, case_variant)
                        if (value is not None and 
                            str(value).strip() and 
                            str(value) != '****' and
                            not str(value).startswith('<Mock')):
                            return value
            except (AttributeError, TypeError):
                continue
                
        return default

    @classmethod
    def _get_reverse_field_map(cls) -> Dict[str, str]:
        """Get or build the reverse field mapping cache."""
        if cls._reverse_field_map is None:
            cls._reverse_field_map = {}
            for pattern, aliases in cls.FIELD_PATTERNS.items():
                for alias in aliases:
                    if alias not in cls._reverse_field_map:
                        cls._reverse_field_map[alias] = pattern
                    if alias.lower() not in cls._reverse_field_map:
                        cls._reverse_field_map[alias.lower()] = pattern
                    if alias.upper() not in cls._reverse_field_map:
                        cls._reverse_field_map[alias.upper()] = pattern
        return cls._reverse_field_map

    def get_feat_fields_batch(self, feat_data: Any) -> Dict[str, Any]:
        """Batch extract all fields needed for feat display in a single pass."""
        if feat_data is None:
            return {}

        field_map = self._get_reverse_field_map()

        result = {}
        if hasattr(feat_data, 'to_dict'):
            attrs = feat_data.to_dict()
        elif hasattr(feat_data, '__dict__'):
            attrs = feat_data.__dict__
        else:
            attrs = {attr: getattr(feat_data, attr) for attr in dir(feat_data) if not attr.startswith('_')}

        for attr_name, attr_value in attrs.items():
            if attr_name in field_map:
                pattern = field_map[attr_name]
                if pattern not in result:
                    if (attr_value is not None and
                        str(attr_value).strip() and
                        str(attr_value) != '****' and
                        not str(attr_value).startswith('<Mock')):
                        result[pattern] = attr_value

        return result

    def get_ability_modifiers(self, race_data: Any) -> Dict[str, int]:
        """Get racial ability modifiers with comprehensive field mapping."""
        modifiers = {
            'Str': 0, 'Dex': 0, 'Con': 0,
            'Int': 0, 'Wis': 0, 'Cha': 0
        }
        
        for attr in modifiers.keys():
            field_pattern = f'{attr.lower()}_adjust'
            modifiers[attr] = self._safe_int(
                self.get_field_value(race_data, field_pattern, 0)
            )
            
        return modifiers
    
    def get_racial_saves(self, race_data: Any) -> Dict[str, int]:
        """Get racial saving throw bonuses with comprehensive field mapping."""
        saves = {'fortitude': 0, 'reflex': 0, 'will': 0}
        
        save_patterns = {
            'fortitude': 'fort_save',
            'reflex': 'ref_save', 
            'will': 'will_save'
        }
        
        for save_type, field_pattern in save_patterns.items():
            saves[save_type] = self._safe_int(
                self.get_field_value(race_data, field_pattern, 0)
            )
            
        return saves
    
    def get_feat_prerequisites(self, feat_data: Any) -> Dict[str, Any]:
        """Get feat prerequisites with comprehensive field mapping."""
        prereqs = {
            'abilities': {},
            'feats': [],
            'class': None,
            'level': 0,
            'bab': 0,
            'spell_level': 0
        }
        
        for ability in ['str', 'dex', 'con', 'int', 'wis', 'cha']:
            field_pattern = f'prereq_{ability}'
            prereqs['abilities'][ability.capitalize()] = self._safe_int(
                self.get_field_value(feat_data, field_pattern, 0)
            )
        
        for i in [1, 2]:
            field_pattern = f'prereq_feat{i}'
            feat_req = self._safe_int(
                self.get_field_value(feat_data, field_pattern, 0)
            )
            if feat_req > 0:
                prereqs['feats'].append(feat_req)
        
        prereqs['class'] = self._safe_int(
            self.get_field_value(feat_data, 'required_class', -1)
        )
        prereqs['level'] = self._safe_int(
            self.get_field_value(feat_data, 'min_level', 0)
        )
        prereqs['bab'] = self._safe_int(
            self.get_field_value(feat_data, 'prereq_bab', 0)
        )
        prereqs['spell_level'] = self._safe_int(
            self.get_field_value(feat_data, 'prereq_spell_level', 0)
        )
        
        return prereqs
    
    def get_racial_feats(self, race_data: Any) -> List[int]:
        """Get racial feats with comprehensive field mapping."""
        feats = []
        
        racial_feats = self.get_field_value(race_data, 'racial_feats')
        if racial_feats:
            if isinstance(racial_feats, (list, tuple)):
                for feat in racial_feats:
                    feat_id = self._safe_int(feat)
                    if feat_id > 0 and feat_id not in feats:
                        feats.append(feat_id)
            elif isinstance(racial_feats, str):
                for feat_str in racial_feats.split(','):
                    feat_id = self._safe_int(feat_str.strip())
                    if feat_id > 0 and feat_id not in feats:
                        feats.append(feat_id)
        
        for i in range(10):
            field_pattern = f'feat{i}' if i > 0 else 'feat0'
            feat_id = self._safe_int(
                self.get_field_value(race_data, field_pattern, 0)
            )
            if feat_id > 0 and feat_id not in feats:
                feats.append(feat_id)
        
        return feats
        
    def get_class_properties(self, class_data: Any) -> Dict[str, Any]:
        """Get class properties with comprehensive field mapping and proper type conversion."""
        properties = {}
        
        properties['label'] = self.get_field_value(class_data, 'label', '')
        properties['name'] = self.get_field_value(class_data, 'name', '')
        properties['hit_die'] = self._safe_int(
            self.get_field_value(class_data, 'hit_die', 8)
        )
        properties['skill_points'] = self._safe_int(
            self.get_field_value(class_data, 'skill_point_base', 2)
        )
        properties['max_level'] = self._safe_int(
            self.get_field_value(class_data, 'max_level', 20)
        )
        
        properties['attack_bonus_table'] = self.get_field_value(class_data, 'attack_bonus_table', '')
        properties['saving_throw_table'] = self.get_field_value(class_data, 'saving_throw_table', '')
        properties['skills_table'] = self.get_field_value(class_data, 'skills_table', '')
        properties['feats_table'] = self.get_field_value(class_data, 'feats_table', '')
        
        properties['spell_caster'] = self._safe_bool(
            self.get_field_value(class_data, 'spell_caster', 0)
        )
        properties['has_arcane'] = self._safe_bool(
            self.get_field_value(class_data, 'has_arcane', 0)
        )
        properties['has_divine'] = self._safe_bool(
            self.get_field_value(class_data, 'has_divine', 0)
        )
        properties['spell_gain_table'] = self.get_field_value(class_data, 'spell_gain_table', '')
        properties['spell_known_table'] = self.get_field_value(class_data, 'spell_known_table', '')
        
        properties['primary_ability'] = self.get_field_value(class_data, 'primary_ability', '')
        properties['align_restrict'] = self.get_alignment_restriction(class_data)
        
        prereq_table = self.get_field_value(class_data, 'prereq_table', '')
        properties['prereq_table'] = prereq_table
        properties['prereq_parsed'] = self.parse_prerequisite_table(prereq_table)
        
        properties['player_class'] = self._safe_bool(
            self.get_field_value(class_data, 'player_class', 1)
        )
        
        properties['description'] = self._safe_int(
            self.get_field_value(class_data, 'description', 0)
        )
        
        return properties
    
    def _safe_int(self, value: Any, default: int = 0) -> int:
        """Safely convert value to int, handling hex strings and NWN2 2DA string values."""
        if value is None:
            return default
            
        # Handle string values
        if isinstance(value, str):
            value = value.strip()
            if not value or value == '****':
                return default
                
            # Handle hex strings (e.g., "0x02" for alignment restrictions)
            if value.startswith('0x') or value.startswith('0X'):
                try:
                    return int(value, 16)
                except ValueError:
                    return default
        
        try:
            return int(value)
        except (ValueError, TypeError):
            return default
    
    def _safe_bool(self, value: Any, default: bool = False) -> bool:
        """Safely convert value to bool, handling NWN2 2DA string values."""
        if value is None:
            return default
            
        if isinstance(value, str):
            value = value.strip()
            if not value or value == '****':
                return default
                
            if value.lower() in ('true', 'yes', '1'):
                return True
            elif value.lower() in ('false', 'no', '0'):
                return False
                
        try:
            return bool(int(value))
        except (ValueError, TypeError):
            return default
    
    def _safe_hex_int(self, value: Any, default: int = 0) -> int:
        """Safely convert hex string to int (for alignment restrictions, etc.)."""
        if value is None:
            return default
            
        if isinstance(value, str):
            value = value.strip()
            if not value or value == '****':
                return default
                
            if value.startswith('0x') or value.startswith('0X'):
                try:
                    return int(value, 16)
                except ValueError:
                    return default
        
        return self._safe_int(value, default)
    
    def parse_prerequisite_table(self, prereq_table_name: str) -> Dict[str, Any]:
        """Parse prerequisite table name into structured requirements."""
        if not prereq_table_name or prereq_table_name == '****':
            return {}
            
        table_name_str = str(prereq_table_name)
        
        parts = table_name_str.split('_')
        if len(parts) < 3 or parts[0] != 'CLS' or parts[1] != 'PRES':
            return {'raw_table': table_name_str}
            
        identifier = '_'.join(parts[2:])  # Join remaining parts
        
        return {
            'table_type': 'class_prerequisites',
            'identifier': identifier,
            'raw_table': table_name_str
        }
    
    def get_table_value_safe(self, table_data: Any, column: str, row: int = 0, default: Any = None) -> Any:
        """Safely get value from 2DA table data with proper column name mapping."""
        if not table_data:
            return default
            
        column_variants = [
            column,
            column.lower(),
            column.upper(),
            ''.join(word.capitalize() for word in column.split('_'))
        ]
        
        for col_name in column_variants:
            try:
                if hasattr(table_data, 'get_cell'):
                    value = table_data.get_cell(row, col_name)
                    if value is not None and str(value) != '****':
                        return value
                elif hasattr(table_data, col_name):
                    column_data = getattr(table_data, col_name)
                    if hasattr(column_data, '__getitem__') and len(column_data) > row:
                        value = column_data[row]
                        if value is not None and str(value) != '****':
                            return value
            except (AttributeError, IndexError, KeyError, TypeError):
                continue
                
        return default
    
    def get_class_table_values(self, class_data: Any, level: int = 1) -> Dict[str, Any]:
        """Get all table-based values for a class at a specific level."""
        values = {
            'bab': 0,
            'fort_save': 0,
            'ref_save': 0,
            'will_save': 0,
            'hit_die': 8,
            'skill_points': 2
        }
        
        attack_table = self.get_field_value(class_data, 'attack_bonus_table', '')
        save_table = self.get_field_value(class_data, 'saving_throw_table', '')
        
        values['attack_bonus_table'] = attack_table
        values['saving_throw_table'] = save_table
        
        values['hit_die'] = self._safe_int(
            self.get_field_value(class_data, 'hit_die', 8)
        )
        values['skill_points'] = self._safe_int(
            self.get_field_value(class_data, 'skill_point_base', 2)
        )
        
        return values
    
    def get_alignment_restriction(self, class_data: Any) -> int:
        """Get alignment restriction value with proper hex parsing."""
        align_restrict = self.get_field_value(class_data, 'align_restrict', '0x00')
        return self._safe_hex_int(align_restrict, 0)
    
    def get_robust_field_value(self, data_object: Any, field_patterns: List[str], 
                               convert_type: str = 'auto', default: Any = None) -> Any:
        """Get field value with multiple fallback patterns and automatic type conversion."""
        value = None
        
        for pattern in field_patterns:
            value = self.get_field_value(data_object, pattern)
            if value is not None:
                break
        
        if value is None:
            return default
            
        if convert_type == 'int':
            return self._safe_int(value, default if isinstance(default, int) else 0)
        elif convert_type == 'bool':
            return self._safe_bool(value, default if isinstance(default, bool) else False)
        elif convert_type == 'hex':
            return self._safe_hex_int(value, default if isinstance(default, int) else 0)
        elif convert_type == 'str':
            return str(value) if value is not None else (default or '')
        else:  # 'auto'
            if isinstance(value, str):
                value_stripped = value.strip()
                if value_stripped.startswith('0x') or value_stripped.startswith('0X'):
                    return self._safe_hex_int(value, 0)
                elif value_stripped.isdigit() or (value_stripped.startswith('-') and value_stripped[1:].isdigit()):
                    return self._safe_int(value, 0)
                elif value_stripped.lower() in ('true', 'false', '0', '1'):
                    return self._safe_bool(value, False)
            return value
    
    def bulk_field_extraction(self, data_object: Any, field_mapping: Dict[str, Dict]) -> Dict[str, Any]:
        """Extract multiple fields at once using comprehensive mapping."""
        results = {}
        
        for result_key, config in field_mapping.items():
            patterns = config.get('patterns', [result_key])
            convert_type = config.get('type', 'auto')
            default = config.get('default', None)
            
            results[result_key] = self.get_robust_field_value(
                data_object, patterns, convert_type, default
            )
        
        return results
    
    def validate_field_access(self, data_object: Any, expected_fields: List[str]) -> Dict[str, bool]:
        """Validate which expected fields are accessible on a data object."""
        results = {}
        
        for field_pattern in expected_fields:
            value = self.get_field_value(data_object, field_pattern)
            results[field_pattern] = value is not None
            
        return results
    
    def get_subrace_properties(self, subrace_data: Any) -> Dict[str, Any]:
        """Get comprehensive subrace properties from racialsubtypes.2da data."""
        properties = {}
        
        properties['label'] = self.get_field_value(subrace_data, 'subrace_label', '')
        properties['name'] = self.get_field_value(subrace_data, 'subrace_name', '')
        properties['base_race'] = self._safe_int(
            self.get_field_value(subrace_data, 'base_race', 0)
        )
        properties['ecl'] = self._safe_int(
            self.get_field_value(subrace_data, 'effective_character_level', 0)
        )
        
        properties['ability_modifiers'] = self.get_ability_modifiers(subrace_data)
        
        properties['feats_table'] = self.get_field_value(subrace_data, 'feats_table', '')
        
        properties['favored_class'] = self._safe_int(
            self.get_field_value(subrace_data, 'favored_class', -1)
        )
        properties['has_favored_class'] = self._safe_bool(
            self.get_field_value(subrace_data, 'has_favored_class', 0)
        )
        
        properties['player_race'] = self._safe_bool(
            self.get_field_value(subrace_data, 'player_race', 1)
        )
        
        return properties

field_mapper = FieldMappingUtility()