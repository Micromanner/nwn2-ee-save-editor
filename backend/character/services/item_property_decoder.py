"""
Item Property Decoder Service

Decodes NWN2 item properties using the full itempropdef.2da table data.
Maps PropertyName IDs to human-readable descriptions and bonus calculations.
"""


from typing import Dict, List, Any, Optional, Tuple
from loguru import logger
from gamedata.dynamic_loader.field_mapping_utility import field_mapper


class ItemPropertyDecoder:
    """Service for decoding NWN2 item properties using game data tables"""
    
    def __init__(self, rules_service):
        """
        Initialize decoder with rules service access
        
        Args:
            rules_service: GameRulesService instance for accessing tables
        """
        self.rules_service = rules_service
        self._property_cache = {}
        self._subtype_caches = {}
        self._ability_map = {0: 'Str', 1: 'Dex', 2: 'Con', 3: 'Int', 4: 'Wis', 5: 'Cha'}
        self._save_map = {0: 'fortitude', 1: 'reflex', 2: 'will'}
        
        # Unified context lists for data-driven subtype resolution
        # Keys correspond to the mapped values in subtype_map
        self._context_lists = {
            'abilities': self._ability_map,
            'saving_throws': {0: 'Fortitude', 1: 'Reflex', 2: 'Will'},
            'save_elements': self._get_all_save_elements(),
            'damage_types': self._get_all_damage_types(),
            'immunity_types': self._get_all_immunity_types(),
            'spells': self._get_iprp_table_options('iprp_spells'),
            'feats': self._get_iprp_table_options('iprp_feats'),
            'alignments': self._get_iprp_table_options('iprp_alignment'),
            'alignment_groups': {0: 'Good', 1: 'Evil', 2: 'Lawful', 3: 'Chaotic'},
            'racial_groups': self._get_iprp_table_options('racialtypes'),
            'visual_effects': self._get_iprp_table_options('iprp_visualfx'),
            'light': {
                0: 'Dim (5m)', 1: 'Bright (5m)', 2: 'Dim (10m)', 3: 'Bright (10m)', 
                4: 'Dim (15m)', 5: 'Bright (15m)', 6: 'Dim (20m)', 7: 'Bright (20m)'
            }
        }
        
        self._init_property_mappings()
        
        # Property-specific overrides to fix known 2DA discrepancies or suppress noise
        # format: prop_id: { 'subtype_table': str, 'cost_table': str, 'param1_table': str, 'suppress_p1': bool, 'suppress_cost': bool }
        self.property_overrides = {
            # Base Game Overrides - Correcting Table Mappings
            0: {'cost_table': 'iprp_bonuscost', 'force_cost_idx': 1, 'suppress_p1': True},      # Ability Bonus: Value is in Cost
            1: {'suppress_p1': True},      # AC Bonus: P1 (Abilities) is junk
            6: {'suppress_p1': True},      # Enhancement Bonus: P1 (Abilities) is junk
            10: {'suppress_p1': True},    # Attack/Damage Penalty: Value is in P1->Cost
            11: {'cost_table': 'iprp_weightcost', 'force_cost_idx': 10, 'suppress_p1': True}, # Weight Reduction: Fix table index (10=iprp_weightcost)
            # 16: Damage Bonus - Now handled by logic (Param1->CostTable, suppress Param1)
            81: {'cost_table': 'iprp_weightinc', 'force_cost_idx': 30, 'suppress_p1': True}, # Weight Increase: Fix table (30=iprp_weightinc)
            
            # Properties with misaligned Value tables (using specific tables instead of junk 2DA indices)
            45: {'param1_table': 'iprp_bonuscost'}, # Mighty: use bonuscost (+1 to +20)
            51: {'suppress_p1': True},      # Regeneration: P1 (Abilities) is junk, uses Cost for value
            55: {'param1_table': 'iprp_skillcost'}, # Thieves' Tools: use skillcost (+1 to +50)
            56: {'param1_table': 'iprp_bonuscost'}, # Attack Bonus: use bonuscost
            63: {'suppress_cost': True, 'suppress_p1': True}, # Class Limitation: metadata handled by Subtype
            66: {'param1_table': 'iprp_bonushp'},   # Bonus Hitpoints: use bonushp table
            67: {'param1_table': 'iprp_meleecost'}, # Regeneration Vampiric: use meleecost (+1 to +20)
            84: {'suppress_p1': True},    # Arcane Spell Failure: use P1 (-50% to +50%), mapped to Cost
            
            # Pure Binary Toggles - Suppressing all options to keep UI flat
            14: {'suppress_p1': True, 'suppress_cost': True}, # Boomerang
            25: {'suppress_p1': True, 'suppress_cost': True}, # Dancing
            26: {'suppress_p1': True, 'suppress_cost': True}, # Darkvision
            30: {'suppress_p1': True, 'suppress_cost': True}, # Double Stack
            31: {'suppress_p1': True, 'suppress_cost': True}, # Half-Elves
            32: {'suppress_p1': True, 'suppress_cost': True}, # Enhanced Container
            35: {'suppress_p1': True, 'suppress_cost': True}, # Haste
            36: {'suppress_p1': True, 'suppress_cost': True}, # Holy Avenger
            38: {'suppress_p1': True, 'suppress_cost': True}, # Improved Evasion
            43: {'suppress_p1': True, 'suppress_cost': True}, # Keen
            46: {'suppress_p1': True, 'suppress_cost': True}, # Mind Blank
            47: {'suppress_p1': True, 'suppress_cost': True}, # No Combat Damage
            61: {'suppress_p1': True, 'suppress_cost': True}, # Unlimited Ammunition
            68: {'suppress_p1': True, 'suppress_cost': True}, # Vorpal
            69: {'suppress_p1': True, 'suppress_cost': True}, # Wounding
            71: {'suppress_p1': True, 'suppress_cost': True}, # True Seeing
            75: {'suppress_p1': True, 'suppress_cost': True}, # Freedom of Movement
            
            # Complex Properties with specific UI needs
            12: {'suppress_cost': True, 'suppress_p1': True}, # Bonus Feat: only needs Subtype
            44: {'suppress_cost': True, 'suppress_p1': True}, # Light: only needs Subtype
            63: {'suppress_cost': True, 'suppress_p1': True}, # Use Limitation Class: only needs Subtype
            76: {'suppress_cost': True, 'suppress_p1': True}, # Poison: only needs Subtype
            77: {'suppress_p1': True},                         # Monster Damage: only needs Subtype
            22: {'suppress_cost': True},                       # Damage Reduction: Subtype is type, P1 is amount
            23: {'suppress_p1': True},                         # Damage Resistance: Subtype is type, P1->Cost is amount
            87: {'suppress_p1': True, 'suppress_cost': True},
            88: {'suppress_p1': True, 'suppress_cost': True},
            39: {'suppress_subtype': True},
        }
    
    def _init_property_mappings(self):
        """Initialize property type mappings and caches"""
        try:
            itempropdef_table = self.rules_service.get_table('itempropdef')
            if itempropdef_table:
                for prop_id, prop_data in enumerate(itempropdef_table):
                    if prop_data:
                        self._property_cache[prop_id] = {
                            'id': prop_id,
                            'label': field_mapper.get_field_value(prop_data, 'Label', f'Property_{prop_id}'),
                            'subtype_ref': field_mapper.get_field_value(prop_data, 'SubTypeResRef', ''),
                            'cost_table_ref': field_mapper.get_field_value(prop_data, 'CostTableResRef', ''),
                            'param1_ref': field_mapper.get_field_value(prop_data, 'Param1ResRef', ''),
                            'description': field_mapper.get_field_value(prop_data, 'Description', ''),
                            'game_str_ref': field_mapper.get_field_value(prop_data, 'GameStrRef', ''),
                        }
            logger.info(f"Cached {len(self._property_cache)} item property definitions")
        except Exception as e:
            logger.error(f"Failed to load itempropdef table: {e}")
    
    def decode_property(self, property_data: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """
        Decode a single item property into human-readable information
        
        Args:
            property_data: Raw property data from GFF
            
        Returns:
            Decoded property information or None if property is invalid
        """
        if property_data is None:
            return None
        property_name = property_data.get('PropertyName', 0)
        subtype = property_data.get('Subtype', 0)
        cost_table = property_data.get('CostTable', 0)
        cost_value = property_data.get('CostValue', 0)
        param1 = property_data.get('Param1', 0)
        param1_value = property_data.get('Param1Value', 0)
        
        prop_def = self._property_cache.get(property_name)
        if not prop_def:
            return {
                'property_id': property_name,
                'label': f'Unknown Property {property_name}',
                'description': 'Unknown property type',
                'subtype': subtype,
                'cost_value': cost_value,
                'param1': param1,
                'param1_value': param1_value,
                'raw_data': property_data,
                'decoded': False
            }
        
        # Decode based on property types
        decoded_info = self._decode_specific_property(
            property_name, prop_def, subtype, cost_value, param1, param1_value
        )

        if decoded_info is None:
            return None

        decoded_info.update({
            'property_id': property_name,
            'raw_data': property_data,
            'decoded': True
        })

        return decoded_info
    
    def _decode_specific_property(self, prop_id: int, prop_def: Dict[str, Any],
                                 subtype: int, cost_value: int, param1: int,
                                 param1_value: int) -> Dict[str, Any]:
        """Decode specific property types based on ID and definition"""

        label = prop_def['label'].lower()
        base_description = prop_def['description']

        if prop_id == 0:
            ability_name = self._ability_map.get(subtype, f'Unknown Ability {subtype}')
            return {
                'label': f'{ability_name} +{cost_value}',
                'description': f'{base_description} +{cost_value} enhancement bonus to {ability_name}',
                'bonus_type': 'ability',
                'ability': ability_name.lower(),
                'bonus_value': cost_value
            }
        
        elif prop_id == 1:
            return {
                'label': f'AC +{cost_value}',
                'description': f'{base_description} +{cost_value} armor bonus to AC',
                'bonus_type': 'ac_armor', 
                'bonus_value': cost_value
            }
        
        elif prop_id == 6:
            return {
                'label': f'Enhancement +{cost_value}',
                'description': f'{base_description} +{cost_value} enhancement bonus',
                'bonus_type': 'enhancement',
                'bonus_value': cost_value
            }
        
        elif prop_id == 15:
            spell_data = self._get_spell_data(subtype)
            uses_data = self._get_charge_uses(cost_value)

            spell_name = spell_data.get('name', f'Spell_{subtype}')
            caster_level = spell_data.get('caster_level', 1)
            uses_label = uses_data.get('label', 'Unknown')

            return {
                'label': f'Cast {spell_name} ({caster_level})',
                'description': f'Use: {spell_name} (Caster Level {caster_level}) - {uses_label}',
                'bonus_type': 'spell',
                'spell_id': subtype,
                'spell_name': spell_name,
                'caster_level': caster_level,
                'uses_per_day': uses_label
            }
        
        elif prop_id in [40, 41]:
            if prop_id == 40:
                save_element_name = self._get_save_element_name(subtype)
                if subtype == 0:
                    return {
                        'label': f'Saves +{cost_value}',
                        'description': f'{base_description} +{cost_value} to all saving throws',
                        'bonus_type': 'saves_all',
                        'bonus_value': cost_value
                    }
                else:
                    return {
                        'label': f'Saves +{cost_value} vs {save_element_name}',
                        'description': f'{base_description} +{cost_value} vs {save_element_name}',
                        'bonus_type': 'save_element',
                        'element_type': save_element_name,
                        'bonus_value': cost_value
                    }
            else:
                save_name = self._save_map.get(subtype)
                if save_name:
                    return {
                        'label': f'{save_name.title()} Save +{cost_value}',
                        'description': f'{base_description} +{cost_value} to {save_name} saves',
                        'bonus_type': 'save_specific',
                        'save_type': save_name,
                        'bonus_value': cost_value
                    }
                else:
                    return {
                        'label': f'Saving Throw +{cost_value}',
                        'description': f'{base_description} +{cost_value} bonus to saves',
                        'bonus_type': 'save_specific',
                        'bonus_value': cost_value
                    }
        
        elif prop_id == 52:
            skill_name = self._get_skill_name(subtype)
            return {
                'label': f'{skill_name} +{cost_value}',
                'description': f'{base_description} +{cost_value} competence bonus to {skill_name}',
                'bonus_type': 'skill',
                'skill_id': subtype,
                'skill_name': skill_name,
                'bonus_value': cost_value
            }
        
        elif prop_id in [56, 57, 58, 59]:
            if prop_id == 56:
                return {
                    'label': f'Attack +{cost_value}',
                    'description': f'{base_description} +{cost_value} enhancement bonus to attack',
                    'bonus_type': 'attack',
                    'bonus_value': cost_value
                }
            else:
                target_type = self._decode_target_type(prop_id, subtype)
                return {
                    'label': f'Attack +{cost_value} vs {target_type}',
                    'description': f'{base_description} +{cost_value} vs {target_type}',
                    'bonus_type': 'attack_conditional',
                    'target_type': target_type,
                    'bonus_value': cost_value
                }
        
        elif prop_id == 16:
            damage_type = self._get_damage_type_name(subtype)
            val = cost_value if cost_value > 0 else param1_value
            amount_label = self._get_iprp_table_options('iprp_damagecost').get(val, f'+{val}')
            return {
                'label': f'{damage_type.title()} Damage {amount_label}',
                'description': f'{base_description} {amount_label} {damage_type} damage',
                'bonus_type': 'damage',
                'damage_type': damage_type,
                'bonus_value': amount_label
            }
        
        elif prop_id == 23:
            damage_type = self._get_damage_type_name(subtype)
            resist_value = self._get_resistance_value(cost_value)
            return {
                'label': f'Resist {damage_type.title()} {resist_value}/-',
                'description': f'Damage Resistance: {resist_value} points of {damage_type} damage reduction',
                'bonus_type': 'resistance',
                'damage_type': damage_type,
                'resistance_value': resist_value
            }
        
        elif prop_id == 37:
            immunity_type = self._get_immunity_type_name(subtype)
            return {
                'label': f'Immunity: {immunity_type}',
                'description': f'Complete immunity to {immunity_type}',
                'bonus_type': 'immunity',
                'immunity_type': immunity_type.lower(),
                'immunity_id': subtype
            }

        elif prop_id == 39:
            sr_options = self._get_iprp_table_options('iprp_srcost')
            actual_sr = sr_options.get(cost_value, str(10 + (cost_value * 2)))
            return {
                'label': f'Spell Resistance {actual_sr}',
                'description': f'{base_description} {actual_sr} spell resistance',
                'bonus_type': 'spell_resistance',
                'resistance_value': actual_sr
            }
        
        elif prop_id == 44:
            light_data = self._get_light_data(cost_value, param1)
            brightness = light_data.get('brightness', 'Normal')
            color = light_data.get('color', '')

            label = f'Light {brightness}'
            if color:
                label += f' {color}'

            return {
                'label': label,
                'description': f'Provides {brightness.lower()} illumination ({color})',
                'bonus_type': 'utility',
                'light_brightness': brightness,
                'light_color': color
            }

        elif prop_id == 70:
            trap_strength = {0: 'Minor', 1: 'Average', 2: 'Strong', 3: 'Deadly', 4: 'Epic'}
            trap_types = {
                0: 'Random', 1: 'Spike', 2: 'Holy', 3: 'Tangle', 4: 'Acid',
                5: 'Fire', 6: 'Electrical', 7: 'Gas', 8: 'Frost',
                9: 'Acid Splash', 10: 'Sonic', 11: 'Negative'
            }
            strength = trap_strength.get(subtype, f'Level {subtype}')
            trap_type = trap_types.get(cost_value, f'Type {cost_value}')

            return {
                'label': f'{strength} {trap_type} Trap',
                'description': f'Trap: {strength} {trap_type}',
                'bonus_type': 'trap',
                'trap_type': trap_type,
                'trap_strength': strength
            }

        elif prop_id == 75:
            return {
                'label': 'Freedom of Movement',
                'description': 'Immunity to paralysis and movement-impairing effects',
                'bonus_type': 'immunity',
                'immunity_type': 'movement_effects'
            }
        
        elif prop_id == 92:
            damage_type = self._get_damage_type_name(subtype)
            vuln_value = self._get_vulnerability_value(cost_value)
            return {
                'label': f'Vulnerability: {damage_type.title()} {vuln_value}%',
                'description': f'Damage Vulnerability: {vuln_value}% extra {damage_type} damage taken',
                'bonus_type': 'vulnerability',
                'damage_type': damage_type,
                'vulnerability_value': vuln_value
            }

        elif prop_id == 90:
            bypass_type = self._decode_dr_bypass(param1)
            return {
                'label': f'Damage Reduction {cost_value}/{bypass_type}',
                'description': f'Damage Reduction: {cost_value} points reduced, bypassed by {bypass_type}',
                'bonus_type': 'damage_reduction',
                'dr_amount': cost_value,
                'dr_bypass': bypass_type
            }

        elif prop_id == 63:
            class_name = self._get_class_name(subtype)
            return {
                'label': f'Use Limitation: {class_name}',
                'description': f'Only useable by {class_name}',
                'bonus_type': 'use_limitation',
                'limitation_type': 'class',
                'class_id': subtype,
                'class_name': class_name
            }

        elif prop_id == 13:
            class_name = self._get_class_name(subtype)
            spell_level_ordinal = self._get_ordinal(cost_value)
            return {
                'label': f'Bonus {spell_level_ordinal} Level Spell Slot',
                'description': f'Bonus {spell_level_ordinal} level spell slot for {class_name}',
                'bonus_type': 'spell_slot',
                'class_id': subtype,
                'class_name': class_name,
                'spell_level': cost_value
            }

        elif prop_id == 27:
            ability_name = self._ability_map.get(subtype, f'Unknown Ability {subtype}')
            return {
                'label': f'{ability_name} -{cost_value}',
                'description': f'{base_description} -{cost_value} penalty to {ability_name}',
                'bonus_type': 'ability_penalty',
                'ability': ability_name.lower(),
                'penalty_value': -cost_value
            }

        else:
            return self._generic_decode(prop_def, subtype, cost_value, param1)
    
    def _get_spell_name(self, spell_id: int) -> str:
        """Get spell name from spell ID"""
        try:
            spell_data = self.rules_service.get_by_id('spells', spell_id)
            if spell_data:
                return field_mapper.get_field_value(spell_data, 'Label', f'Spell {spell_id}')
        except Exception:
            pass
        return f'Spell {spell_id}'
    
    def _get_skill_name(self, skill_id: int) -> str:
        """Get skill name from skill ID"""
        try:
            skill_data = self.rules_service.get_by_id('skills', skill_id)
            if skill_data:
                return field_mapper.get_field_value(skill_data, 'Label', f'Skill {skill_id}')
        except Exception:
            pass
        return f'Skill {skill_id}'

    def _get_class_name(self, class_id: int) -> str:
        """Get class name from class ID"""
        try:
            class_data = self.rules_service.get_by_id('classes', class_id)
            if class_data and hasattr(class_data, 'label'):
                return class_data.label
        except Exception as e:
            logger.error(f"Failed to get class name for ID {class_id}: {e}")
        return f'Class {class_id}'

    def _get_ordinal(self, num: int) -> str:
        """Convert number to ordinal string (0 -> 1st, 1 -> 2nd, etc.)"""
        num = num + 1
        if 10 <= num % 100 <= 20:
            suffix = 'th'
        else:
            suffix = {1: 'st', 2: 'nd', 3: 'rd'}.get(num % 10, 'th')
        return f'{num}{suffix}'

    def _get_immunity_type_name(self, immunity_id: int) -> str:
        """Get immunity type name from iprp_immunity.2da"""
        options = self._get_iprp_table_options('iprp_immunity')
        if options and immunity_id in options:
            return options[immunity_id]
        return f'Immunity {immunity_id}'

    def _get_save_element_name(self, element_id: int) -> str:
        """Get save element name from iprp_saveelement ID"""
        options = self._get_iprp_table_options('iprp_saveelement')
        if options and element_id in options:
            label = options[element_id]
            # Clean up the label which often includes "Saving Throw: " prefix in 2DA
            return label.replace('Saving Throw: ', '').replace('Save:', '').strip()
        return f'Element {element_id}'
    
    def _decode_target_type(self, prop_id: int, subtype: int) -> str:
        """Decode target type for conditional bonuses"""
        if prop_id == 57:
            alignment_groups = {0: 'Good', 1: 'Evil', 2: 'Lawful', 3: 'Chaotic'}
            return alignment_groups.get(subtype, f'Alignment {subtype}')
        elif prop_id == 58:
            try:
                race_data = self.rules_service.get_by_id('racialtypes', subtype)
                if race_data:
                    return field_mapper.get_field_value(race_data, 'Label', f'Race {subtype}')
            except Exception:
                pass
            return f'Race {subtype}'
        return f'Target {subtype}'
    
    def _get_damage_type_name(self, damage_type_id: int) -> str:
        """Get damage type name from ID using iprp_damagetype.2da"""
        options = self._get_iprp_table_options('iprp_damagetype')
        if options and damage_type_id in options:
            return options[damage_type_id].lower()
        return f'type_{damage_type_id}'
        
    def _get_all_save_elements(self) -> Dict[int, str]:
        """Return all save element types mapped by ID"""
        return self._get_iprp_table_options('iprp_saveelement') or {}

    def _get_all_damage_types(self) -> Dict[int, str]:
        """Return all damage types mapped by ID"""
        return self._get_iprp_table_options('iprp_damagetype') or {}

    def _get_all_immunity_types(self) -> Dict[int, str]:
        """Return all immunity types mapped by ID"""
        return self._get_iprp_table_options('iprp_immunity') or {}

    def _get_spell_data(self, spell_row_id: int) -> Dict[str, Any]:
        """Look up spell data from iprp_spells.2da"""
        try:
            spell_table = self.rules_service.get_table('iprp_spells')
            if spell_table and spell_row_id < len(spell_table):
                spell_row = spell_table[spell_row_id]
                if spell_row:
                    from gamedata.dynamic_loader.field_mapping_utility import field_mapper
                    return {
                        'name': field_mapper.get_field_value(spell_row, 'Label', f'Spell_{spell_row_id}'),
                        'caster_level': int(field_mapper.get_field_value(spell_row, 'CasterLvl', 1))
                    }
        except Exception as e:
            logger.debug(f"Failed to look up spell {spell_row_id}: {e}")
        return {'name': f'Spell_{spell_row_id}', 'caster_level': 1}

    def _get_charge_uses(self, cost_value: int) -> Dict[str, str]:
        """Look up charge/uses data from iprp_chargecost.2da"""
        try:
            charge_table = self.rules_service.get_table('iprp_chargecost')
            if charge_table and cost_value < len(charge_table):
                charge_row = charge_table[cost_value]
                if charge_row:
                    from gamedata.dynamic_loader.field_mapping_utility import field_mapper
                    label = field_mapper.get_field_value(charge_row, 'Label', 'Unknown')
                    return {'label': label.replace('_', ' ')}
        except Exception as e:
            logger.debug(f"Failed to look up charges {cost_value}: {e}")
        return {'label': 'Unknown'}

    def _get_resistance_value(self, cost_value: int) -> int:
        """Look up resistance amount from iprp_resistcost.2da"""
        try:
            resist_table = self.rules_service.get_table('iprp_resistcost')
            if resist_table and cost_value < len(resist_table):
                resist_row = resist_table[cost_value]
                if resist_row:
                    from gamedata.dynamic_loader.field_mapping_utility import field_mapper
                    amount = field_mapper.get_field_value(resist_row, 'Amount', cost_value)
                    return int(amount)
        except Exception as e:
            logger.debug(f"Failed to look up resistance {cost_value}: {e}")
        return cost_value

    def _get_vulnerability_value(self, cost_value: int) -> int:
        """Look up vulnerability percentage from iprp_damvulcost.2da"""
        try:
            vuln_table = self.rules_service.get_table('iprp_damvulcost')
            if vuln_table and cost_value < len(vuln_table):
                vuln_row = vuln_table[cost_value]
                if vuln_row:
                    from gamedata.dynamic_loader.field_mapping_utility import field_mapper
                    value = field_mapper.get_field_value(vuln_row, 'Value', cost_value)
                    return int(value)
        except Exception as e:
            logger.debug(f"Failed to look up vulnerability {cost_value}: {e}")
        return cost_value

    def _get_light_data(self, cost_value: int, param1: int) -> Dict[str, str]:
        """Look up light brightness and color from iprp_lightcost.2da and lightcolor.2da"""
        try:
            light_table = self.rules_service.get_table('iprp_lightcost')
            color_table = self.rules_service.get_table('lightcolor')
            from gamedata.dynamic_loader.field_mapping_utility import field_mapper

            brightness = 'Normal'
            if light_table and cost_value < len(light_table):
                light_row = light_table[cost_value]
                if light_row:
                    label = field_mapper.get_field_value(light_row, 'Label', 'Normal')
                    brightness = label.replace('Type:_', '').replace('_', ' ').strip('()')

            color = ''
            if color_table and param1 < len(color_table):
                color_row = color_table[param1]
                if color_row:
                    color = field_mapper.get_field_value(color_row, 'LABEL', '')

            return {'brightness': brightness, 'color': color}
        except Exception as e:
            logger.debug(f"Failed to look up light data: {e}")
        return {'brightness': 'Normal', 'color': ''}

    def _decode_dr_bypass(self, param1: int) -> str:
        """Decode damage reduction bypass type"""
        bypass_types = {
            0: 'None',
            1: 'Magic', 
            2: 'Silver',
            3: 'Cold Iron',
            4: 'Adamantine',
            5: 'Good',
            6: 'Evil',
            7: 'Lawful', 
            8: 'Chaotic'
        }
        return bypass_types.get(param1, f'Type {param1}')
    
    def _generic_decode(self, prop_def: Dict[str, Any], subtype: int, 
                       cost_value: int, param1: int) -> Dict[str, Any]:
        """Generic decode for unknown property types"""
        label = prop_def['label']
        description = prop_def['description']
        
        # Try to extract meaningful info from label/description
        if cost_value > 0:
            if any(word in label.lower() for word in ['bonus', '+']):
                label = f'{label} +{cost_value}'
                description = f'{description} +{cost_value}'
        
        return {
            'label': label,
            'description': description or f'Property effect (ID {prop_def["id"]})',
            'bonus_type': 'unknown',
            'subtype': subtype,
            'cost_value': cost_value,
            'param1': param1
        }
    
    def decode_all_properties(self, properties_list: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Decode all properties in a list"""
        decoded = [self.decode_property(prop) for prop in properties_list]
        return [p for p in decoded if p is not None]

    def _resolve_indexed_column(self, prop_def: Dict[str, Any], column_name: str, preferred_table: str) -> Optional[int]:
        """Extract an integer index from a 2DA column, resolving names back to indices if needed"""
        val = field_mapper.get_field_value(prop_def, column_name)
        if val is None or str(val) == '****':
            return None
            
        try:
            f_val = float(val)
            if not f_val.is_integer():
                return None
            return int(f_val)
        except (ValueError, TypeError):
            pass
            
        if isinstance(val, str):
            mapping_tables = [preferred_table]
            other = 'iprp_paramtable' if preferred_table == 'iprp_costtable' else 'iprp_costtable'
            mapping_tables.append(other)
            
            for m_table in mapping_tables:
                table = self.rules_service.get_table(m_table)
                if table:
                    target = val.lower()
                    for i, row in enumerate(table):
                        if not row: continue
                        name = str(field_mapper.get_field_value(row, 'Name') or '').lower()
                        resref = str(field_mapper.get_field_value(row, 'TableResRef') or '').lower()
                        if name == target or resref == target:
                            return i
        return None

    def get_editor_property_metadata(self, context: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Generate metadata for the item editor UI based on itempropdef.2da"""
        metadata = []
        prop_defs = self.rules_service.get_table('itempropdef')
        
        self.subtype_map = {
            'ability': 'abilities',
            'decreaseabilityscore': 'abilities',
            'abilitybonus': 'abilities',
            'skill': 'skills',
            'decreasedskill': 'skills',
            'castspell': 'spells',
            'spellimmunity_specific': 'spells',
            'onhitcastspell': 'spells',
            'bonusfeats': 'feats',
            'damagetype': 'damage_types',
            'armordamagetype': 'damage_types',
            'damageresist': 'damage_types',
            'damageimmunity_fixed': 'damage_types',
            'damageimmunity': 'damage_types',
            'damagepenalty': 'damage_types',
            'damage_vulnerability': 'damage_types',
            'damage_vulnerability_fixed': 'damage_types',
            'damagemelee': 'damage_types',
            'damageranged': 'damage_types',
            'damage': 'damage_types',
            'damagereduced': 'damage_types',
            'damagenone': 'damage_types',
            'damage_reduction': 'damage_types',
            'damagereduction': 'damage_types',
            'massive_criticals': 'damage_types',
            'saveselement': 'save_elements',
            'improvedsavingthrows': 'save_elements',
            'reducedsavingthrows': 'save_elements',
            'savingthrow': 'saving_throws',
            'improvedsavingthrowsspecific': 'saving_throws',
            'reducedspecificsavingthrow': 'saving_throws',
            'reducedspecificsaving_throw': 'saving_throws',
            'immunity': 'immunity_types',
            'armorracinggroup': 'racial_groups',
            'armorracialgroup': 'racial_groups',
            'enhancementracialgroup': 'racial_groups',
            'damageracialgroup': 'racial_groups',
            'attackbonusracialgroup': 'racial_groups',
            'uselimitationracial': 'racial_groups',
            'damageracialtype': 'racial_groups',
            'racialtype': 'racial_groups',
            'racialtypes': 'racial_groups',
            'armoralignmentgroup': 'alignment_groups',
            'damagealignmentgroup': 'alignment_groups',
            'enhancementalignmentgroup': 'alignment_groups',
            'attackbonusalignmentgroup': 'alignment_groups',
            'uselimitationalignmentgroup': 'alignment_groups',
            'armorspecificalignment': 'alignments',
            'damagespecificalignment': 'alignments',
            'enhancementspecificalignment': 'alignments',
            'attackbonusspecificalignment': 'alignments',
            'uselimitationspecificalignment': 'alignments',
            'specificalignment': 'alignments',
            'uselimitationclass': 'classes',
            'classes': 'classes',
            'light': 'light',
            'improvedmagicresist': 'iprp_srcost',
            'singlebonusspellofle': 'classes'
        }

        label_overrides = {
            12: "Bonus Feat",
            15: "Cast Spell",
            39: "Spell Resistance",
            52: "Skill Bonus",
            68: "Vorpal",
            69: "Wounding",
            72: "On Monster Hit",
            73: "Turn Resistance",
            74: "Massive Criticals",
            75: "Freedom of Movement",
            76: "Poison",
            77: "Monster Damage",
            78: "Immunity: Spells by Level",
            79: "Special Walk",
            80: "Healer's Kit",
            81: "Weight Increase",
            83: "Visual Effect",
            84: "Arcane Spell Failure",
            85: "Arrow Catching",
            86: "Bashing (Shield Bash)",
            87: "Animated (Shield)",
            88: "Wild (Armor/Shield)",
            89: "Etherealness",
            90: "Damage Reduction",
            91: "Immunity: Damage Type",
            92: "Damage Vulnerability"
        }

        for prop_id, prop_def in enumerate(prop_defs):
            if not prop_def: continue
            
            overrides = self.property_overrides.get(prop_id, {})
            original_label = field_mapper.get_field_value(prop_def, 'Label')
            if not original_label or original_label == '****' or original_label == 'None':
                name_val = field_mapper.get_field_value(prop_def, 'Name')
                if isinstance(name_val, int):
                    original_label = self.rules_service.rm.get_string(name_val)
                else:
                    original_label = name_val if name_val != '****' else f'Property {prop_id}'
            
            if not original_label or original_label.lower().startswith('del_') or original_label.upper() == 'DELETED' or original_label.lower() == 'padding':
                continue
            
            # Additional filter for SubTypeResRef == 'padding'
            subtype_ref = field_mapper.get_field_value(prop_def, 'SubTypeResRef', '').lower()
            if subtype_ref == 'padding':
                continue

            clean_label = original_label.replace('Property_', '').replace('_', ' ')
            if prop_id in label_overrides:
                clean_label = label_overrides[prop_id]

            # Subtype logic
            subtype_ref = field_mapper.get_field_value(prop_def, 'SubTypeResRef', '').lower()
            mapping_val = self.subtype_map.get(subtype_ref, subtype_ref)
            
            if prop_id in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 16, 17, 18, 19, 20, 21, 23, 24, 27, 28, 29, 39, 40, 41, 44, 45, 48, 49, 50, 52, 53, 56, 57, 58, 59, 60, 66, 67, 70, 73, 74, 76, 77, 78, 80, 81, 82, 84, 85, 91, 92]:
                cost_table_idx = self._resolve_indexed_column(prop_def, 'Param1ResRef', 'iprp_costtable')
                param1_idx = None
            else:
                cost_table_idx = self._resolve_indexed_column(prop_def, 'CostTableResRef', 'iprp_costtable')
                param1_idx = self._resolve_indexed_column(prop_def, 'Param1ResRef', 'iprp_paramtable')

            # 2. Secondary table index (Param1)

            # FINAL OVERRIDE APPLICATION
            if 'force_cost_idx' in overrides:
                cost_table_idx = overrides['force_cost_idx']
            if 'force_p1_idx' in overrides:
                param1_idx = overrides['force_p1_idx']

            flat_ids = {
                14, 25, 26, 30, 31, 32, 35, 36, 
                38, 43, 46, 47, 51, 61, 66, 
                67, 68, 69, 71, 75, 87, 88, 89
            }
            
            is_flat_bonus = prop_id in flat_ids
            
            has_subtype = False
            subtype_options = {}
            if subtype_ref and not overrides.get('suppress_subtype'):
                try:
                    if mapping_val in self._context_lists:
                        subtype_options = self._context_lists[mapping_val]
                        has_subtype = True
                    elif context and mapping_val in context:
                        subtype_options = context[mapping_val]
                        has_subtype = True
                    else:
                        subtype_table = self.rules_service.get_table(subtype_ref)
                        if subtype_table:
                            subtype_options = self._get_iprp_table_options(subtype_ref)
                            if subtype_options:
                                has_subtype = True
                except (ValueError, TypeError): pass

            # Resolve cost table options
            has_cost_table = False
            cost_table_options = {}
            if cost_table_idx is not None and not overrides.get('suppress_cost'):
                try:
                    table_name = overrides.get('cost_table')
                    if not table_name:
                        table_name = self._get_mapping_table_resref('iprp_costtable', cost_table_idx, prop_id=prop_id)
                    
                    if table_name:
                        cost_table_options = self._get_iprp_table_options(table_name)
                        has_cost_table = len(cost_table_options) > 0
                except (ValueError, TypeError): pass

            # Resolve param1 options
            has_param1 = False
            param1_options = {}
            if param1_idx is not None and not overrides.get('suppress_p1'):
                try:
                    table_name = overrides.get('param1_table')
                    if not table_name:
                        table_name = self._get_mapping_table_resref('iprp_paramtable', param1_idx, prop_id=prop_id)
                    
                    if table_name:
                        if table_name.lower() == 'racialtypes':
                            param1_options = context.get('racial_groups') if context else None
                        elif table_name.lower() == 'classes':
                            param1_options = context.get('classes') if context else None
                        else:
                            param1_options = self._get_iprp_table_options(table_name)
                        
                        has_param1 = len(param1_options) > 0
                except (ValueError, TypeError): pass

            # Result
            metadata.append({
                'id': prop_id,
                'label': clean_label,
                'original_label': original_label,
                'description': field_mapper.get_field_value(prop_def, 'description', ""),
                'has_subtype': has_subtype,
                'subtype_label': 'Subtype',
                'subtype_options': subtype_options,
                'has_cost_table': has_cost_table,
                'cost_table_label': 'Value / Bonus',
                'cost_table_options': cost_table_options,
                'has_param1': has_param1,
                'param1_label': 'Modifier',
                'param1_options': param1_options,
                'is_flat': not has_subtype and not has_cost_table and not has_param1
            })
            
        return sorted(metadata, key=lambda x: x['label'])

    def _get_mapping_table_resref(self, mapping_table_name: str, index: int, prop_id: Optional[int] = None) -> Optional[str]:
        """Get the ResRef of a lookup table from a mapping table"""
        if not hasattr(self, '_mapping_cache'):
            self._mapping_cache = {}
            
        cache_key = f"{mapping_table_name}_{index}_{prop_id}"
        if cache_key in self._mapping_cache:
            return self._mapping_cache[cache_key]
            
        target_mapping_table = mapping_table_name
        if mapping_table_name == 'iprp_paramtable':
            if index > 0:
                target_mapping_table = 'iprp_costtable'
            if prop_id in [44, 70, 81, 82]:
                target_mapping_table = 'iprp_paramtable'
        
        table = self.rules_service.get_table(target_mapping_table)
        if not table or index < 0 or index >= len(table):
            return None
            
        row = table[index]
        if not row: return None
        
        if target_mapping_table == 'iprp_paramtable':
            resref = field_mapper.get_field_value(row, 'TableResRef') or field_mapper.get_field_value(row, 'Name')
        else:
            resref = field_mapper.get_field_value(row, 'Name') or field_mapper.get_field_value(row, 'TableResRef')
            
        if resref == '****' or not resref:
            resref = None
        else:
            resref = str(resref).lower()
        
        self._mapping_cache[cache_key] = resref
        return resref

    def _get_iprp_table_options(self, table_name: str) -> Optional[Dict[int, str]]:
        """Resolve a 2DA table into a Dict[int, str] options map"""
        if not table_name: return None
        
        if not hasattr(self, '_table_options_cache'):
            self._table_options_cache = {}
            
        if table_name in self._table_options_cache:
            return self._table_options_cache[table_name]
            
        table = self.rules_service.get_table(table_name)
        if not table: return None
        
        options = {}
        is_iprp = table_name.lower().startswith('iprp_')
        
        for i, row in enumerate(table):
            if not row: continue
            
            name_val = field_mapper.get_field_value(row, 'Name')
            if is_iprp and name_val and name_val != '****':
                s_name = str(name_val).strip()
                if s_name.startswith('+') or s_name.startswith('-') or (s_name and s_name[0].isdigit()):
                    options[i] = s_name
                    continue
                if s_name.lower() == 'none': continue

            game_string_ref = field_mapper.get_field_value(row, 'GameString')
            if game_string_ref and str(game_string_ref).isdigit():
                try:
                    label = self.rules_service.rm.get_string(int(game_string_ref))
                    if label and label != '****':
                        options[i] = label
                        continue
                except (ValueError, TypeError): pass

            label = field_mapper.get_field_value(row, 'Label')
            
            if isinstance(label, (int, str)) and str(label).isdigit():
                val = int(label)
                if val > 100:
                    try:
                        translated = self.rules_service.rm.get_string(val)
                        if translated and translated != '****':
                            label = translated
                    except (ValueError, TypeError): pass

            if (not label or label == '****') and name_val and name_val != '****':
                label = name_val
            
            if label and label != '****':
                s_label = str(label)
                if s_label not in options.values():
                    options[i] = s_label
        
        self._table_options_cache[table_name] = options
        return options

    def _get_all_save_elements(self) -> Dict[int, str]:
        """Return all save element types mapped by ID"""
        return {
            0: 'Universal', 1: 'Acid', 2: 'Backstab', 3: 'Cold',
            4: 'Death', 5: 'Disease', 6: 'Divine', 7: 'Electrical',
            8: 'Fear', 9: 'Fire', 10: 'Illusion', 11: 'Mind-Affecting',
            12: 'Negative Energy', 13: 'Poison', 14: 'Positive Energy',
            15: 'Sonic', 16: 'Traps', 17: 'Spells', 18: 'Law',
            19: 'Chaos', 20: 'Good', 21: 'Evil'
        }

    def _get_all_damage_types(self) -> Dict[int, str]:
        """Return all damage types mapped by ID"""
        return {
            0: 'Bludgeoning', 1: 'Piercing', 2: 'Slashing', 3: 'Subdual',
            4: 'Physical', 5: 'Magical', 6: 'Acid', 7: 'Cold',
            8: 'Divine', 9: 'Electrical', 10: 'Fire', 11: 'Negative',
            12: 'Positive', 13: 'Sonic'
        }

    def _get_all_immunity_types(self) -> Dict[int, str]:
        """Return all immunity types mapped by ID"""
        return {
            0: 'Backstab', 1: 'Level/Ability Drain', 2: 'Mind-Affecting Spells',
            3: 'Poison', 4: 'Disease', 5: 'Fear', 6: 'Knockdown',
            7: 'Paralysis', 8: 'Critical Hits', 9: 'Death Magic'
        }
    
    def get_item_bonuses(self, properties_list: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Extract quantified bonuses from raw property data for combat calculations"""
        bonuses = {
            'abilities': {},
            'saves': {},
            'skills': {},
            'combat': {},
            'ac': {},
            'immunities': [],
            'special': {}
        }
        
        for prop in properties_list:
            if prop is None:
                continue
                
            property_name = prop.get('PropertyName', 0)
            subtype = prop.get('Subtype', 0)
            cost_value = prop.get('CostValue', 0)
            param1_value = prop.get('Param1Value', 0)
            
            if property_name == 0:
                ability = self._ability_map.get(subtype)
                if ability and cost_value > 0:
                    bonuses['abilities'][ability] = cost_value
                    
            elif property_name == 1:
                if cost_value > 0:
                    bonuses['ac']['deflection'] = cost_value
                    
            elif property_name == 6:
                if cost_value > 0:
                    bonuses['special']['enhancement'] = cost_value
                    
            elif property_name in [40, 41]:
                if cost_value > 0:
                    if property_name == 40:
                        if subtype == 0:
                            for save in ['fortitude', 'reflex', 'will']:
                                bonuses['saves'][save] = bonuses['saves'].get(save, 0) + cost_value
                    else:
                        save_type = self._save_map.get(subtype)
                        if save_type:
                            bonuses['saves'][save_type] = bonuses['saves'].get(save_type, 0) + cost_value
                            
            elif property_name == 10:
                val = cost_value if cost_value > 0 else param1_value
                if val > 0:
                    bonuses['combat']['attack'] = bonuses['combat'].get('attack', 0) - val
                    bonuses['combat']['damage_penalty'] = bonuses['combat'].get('damage_penalty', 0) + val
                    
            elif property_name == 60:
                val = cost_value if cost_value > 0 else param1_value
                if val > 0:
                    bonuses['combat']['attack'] = bonuses['combat'].get('attack', 0) - val

            elif property_name == 16:
                val = cost_value if cost_value > 0 else param1_value
                if val > 0:
                    bonuses['combat']['damage'] = bonuses['combat'].get('damage', [])
                    bonuses['combat']['damage'].append({
                        'type': self._get_damage_type_name(subtype),
                        'amount_idx': val
                    })
                    
            elif property_name == 52:
                if cost_value > 0:
                    skill_name = self._get_skill_name(subtype)
                    bonuses['skills'][skill_name] = cost_value
                    
            elif property_name in [56, 57, 58, 59]:
                if cost_value > 0:
                    bonuses['combat']['attack'] = bonuses['combat'].get('attack', 0) + cost_value
                    
            elif property_name == 75:
                bonuses['immunities'].append('movement_effects')
                    
            elif property_name == 23:
                if cost_value > 0:
                    damage_type = self._get_damage_type_name(subtype)
                    bonuses['special'][f'resistance_{damage_type}'] = cost_value
                    
            elif property_name == 39:
                if cost_value > 0:
                    bonuses['special']['spell_resistance'] = cost_value
                    
            elif property_name == 44:
                bonuses['special']['light'] = 1
                
            elif property_name == 90:
                if cost_value > 0:
                    bonuses['special']['damage_reduction'] = cost_value
                    
            elif property_name == 92:
                if cost_value > 0:
                    damage_type = self._get_damage_type_name(subtype)
                    bonuses['special'][f'vulnerability_{damage_type}'] = cost_value
                
            
        return bonuses