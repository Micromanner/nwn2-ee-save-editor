"""
Item Property Decoder Service

Decodes NWN2 item properties using the full itempropdef.2da table data.
Maps PropertyName IDs to human-readable descriptions and bonus calculations.
"""

from typing import Dict, List, Any, Optional, Tuple
import logging
from gamedata.dynamic_loader.field_mapping_utility import field_mapper

logger = logging.getLogger(__name__)


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
        
        # Initialize decoder mappings
        self._init_property_mappings()
    
    def _init_property_mappings(self):
        """Initialize property type mappings and caches"""
        # Cache commonly used mappings
        self._ability_map = {0: 'Str', 1: 'Dex', 2: 'Con', 3: 'Int', 4: 'Wis', 5: 'Cha'}
        self._save_map = {0: 'fortitude', 1: 'reflex', 2: 'will'}
        
        # Load itempropdef data
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
    
    def decode_property(self, property_data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Decode a single item property into human-readable information
        
        Args:
            property_data: Raw property data from GFF
            
        Returns:
            Decoded property information
        """
        property_name = property_data.get('PropertyName', 0)
        subtype = property_data.get('Subtype', 0)
        cost_table = property_data.get('CostTable', 0)
        cost_value = property_data.get('CostValue', 0)
        param1 = property_data.get('Param1', 0)
        param1_value = property_data.get('Param1Value', 0)
        
        # Get property definition
        prop_def = self._property_cache.get(property_name)
        if not prop_def:
            return {
                'property_id': property_name,
                'label': f'Unknown Property {property_name}',
                'description': 'Unknown property type',
                'subtype': subtype,
                'cost_value': cost_value,
                'raw_data': property_data,
                'decoded': False
            }
        
        # Decode based on known property types
        decoded_info = self._decode_specific_property(
            property_name, prop_def, subtype, cost_value, param1, param1_value
        )
        
        # Add common fields
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
        
        # Property ID 0: Ability Bonus
        if prop_id == 0:
            ability_name = self._ability_map.get(subtype, f'Unknown Ability {subtype}')
            return {
                'label': f'{ability_name} +{cost_value}',
                'description': f'{base_description} +{cost_value} enhancement bonus to {ability_name}',
                'bonus_type': 'ability',
                'ability': ability_name.lower(),
                'bonus_value': cost_value
            }
        
        # Property ID 1: AC Bonus  
        elif prop_id == 1:
            return {
                'label': f'AC +{cost_value}',
                'description': f'{base_description} +{cost_value} armor bonus to AC',
                'bonus_type': 'ac_armor', 
                'bonus_value': cost_value
            }
        
        # Property ID 6: Enhancement Bonus
        elif prop_id == 6:
            return {
                'label': f'Enhancement +{cost_value}',
                'description': f'{base_description} +{cost_value} enhancement bonus',
                'bonus_type': 'enhancement',
                'bonus_value': cost_value
            }
        
        # Property ID 15: Cast Spell
        elif prop_id == 15:
            spell_name = self._get_spell_name(subtype)
            uses = param1 if param1 != 255 else 'Unlimited'
            return {
                'label': f'Cast {spell_name}',
                'description': f'Use: {spell_name} ({uses} uses per day)',
                'bonus_type': 'spell',
                'spell_id': subtype,
                'spell_name': spell_name,
                'uses_per_day': uses
            }
        
        # Property ID 40/41: Saving Throw Bonuses
        elif prop_id in [40, 41]:
            if prop_id == 40:
                # Property 40 uses iprp_saveelement subtypes
                # Subtype 0 = Universal (applies to Fort/Ref/Will)
                # Subtype 5 = Disease resistance (specific save type, NOT universal)
                # Other subtypes = Fear, Poison, etc (specific save types)
                save_element_name = self._get_save_element_name(subtype)

                if subtype == 0:  # Universal - applies to all three main saves
                    return {
                        'label': f'Saves +{cost_value}',
                        'description': f'{base_description} +{cost_value} to all saving throws',
                        'bonus_type': 'saves_all',
                        'bonus_value': cost_value
                    }
                else:  # Specific element (disease, fear, poison, etc) - does NOT apply to main saves
                    return {
                        'label': f'Saves +{cost_value} vs {save_element_name}',
                        'description': f'{base_description} +{cost_value} vs {save_element_name}',
                        'bonus_type': 'save_element',
                        'element_type': save_element_name,
                        'bonus_value': cost_value
                    }
            else:
                # Property 41 uses iprp_savingthrow subtypes (Fort/Ref/Will)
                save_name = self._save_map.get(subtype, f'Save {subtype}')
                return {
                    'label': f'{save_name.title()} Save +{cost_value}',
                    'description': f'{base_description} +{cost_value} to {save_name} saves',
                    'bonus_type': 'save_specific',
                    'save_type': save_name,
                    'bonus_value': cost_value
                }
        
        # Property ID 52: Skill Bonus
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
        
        # Property ID 56-59: Attack Bonuses
        elif prop_id in [56, 57, 58, 59]:
            if prop_id == 56:
                return {
                    'label': f'Attack +{cost_value}',
                    'description': f'{base_description} +{cost_value} enhancement bonus to attack',
                    'bonus_type': 'attack',
                    'bonus_value': cost_value
                }
            else:
                # Conditional attack bonuses vs alignment/race
                target_type = self._decode_target_type(prop_id, subtype)
                return {
                    'label': f'Attack +{cost_value} vs {target_type}',
                    'description': f'{base_description} +{cost_value} vs {target_type}',
                    'bonus_type': 'attack_conditional',
                    'target_type': target_type,
                    'bonus_value': cost_value
                }
        
        # Property ID 16: Damage Bonus
        elif prop_id == 16:
            damage_type = self._get_damage_type_name(subtype)
            return {
                'label': f'{damage_type.title()} Damage +{cost_value}',
                'description': f'{base_description} +{cost_value} {damage_type} damage',
                'bonus_type': 'damage',
                'damage_type': damage_type,
                'bonus_value': cost_value
            }
        
        # Property ID 23: Damage Resistance
        elif prop_id == 23:
            damage_type = self._get_damage_type_name(subtype)
            return {
                'label': f'Resist {damage_type.title()} {cost_value}/-',
                'description': f'Damage Resistance: {cost_value} points of {damage_type} damage reduction',
                'bonus_type': 'resistance',
                'damage_type': damage_type,
                'resistance_value': cost_value
            }
        
        # Property ID 39: Spell Resistance
        elif prop_id == 39:
            return {
                'label': f'Spell Resistance {cost_value}',
                'description': f'{base_description} {cost_value} spell resistance',
                'bonus_type': 'spell_resistance',
                'resistance_value': cost_value
            }
        
        # Property ID 44: Light
        elif prop_id == 44:
            return {
                'label': 'Light',
                'description': 'Provides illumination',
                'bonus_type': 'utility',
                'light_radius': cost_value if cost_value > 0 else 1
            }
        
        # Property ID 75: Freedom of Movement
        elif prop_id == 75:
            return {
                'label': 'Freedom of Movement',
                'description': 'Immunity to paralysis and movement-impairing effects',
                'bonus_type': 'immunity',
                'immunity_type': 'movement_effects'
            }
        
        # Property ID 92: Damage Vulnerability
        elif prop_id == 92:
            damage_type = self._get_damage_type_name(subtype)
            return {
                'label': f'Vulnerability: {damage_type.title()} +{cost_value}',
                'description': f'Damage Vulnerability: +{cost_value} extra {damage_type} damage taken',
                'bonus_type': 'vulnerability',
                'damage_type': damage_type,
                'vulnerability_value': cost_value
            }
        
        # Property ID 90: Damage Reduction (Modern NWN2 system)
        elif prop_id == 90:
            bypass_type = self._decode_dr_bypass(param1)
            return {
                'label': f'Damage Reduction {cost_value}/{bypass_type}',
                'description': f'Damage Reduction: {cost_value} points reduced, bypassed by {bypass_type}',
                'bonus_type': 'damage_reduction',
                'dr_amount': cost_value,
                'dr_bypass': bypass_type
            }
        
        # Generic decode for unknown properties
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

    def _get_save_element_name(self, element_id: int) -> str:
        """Get save element name from iprp_saveelement ID"""
        element_names = {
            0: 'Universal',
            1: 'Acid',
            2: 'Backstab',
            3: 'Cold',
            4: 'Death',
            5: 'Disease',
            6: 'Divine',
            7: 'Electrical',
            8: 'Fear',
            9: 'Fire',
            10: 'Illusion',
            11: 'Mind-Affecting',
            12: 'Negative Energy',
            13: 'Poison',
            14: 'Positive Energy',
            15: 'Sonic',
            16: 'Traps',
            17: 'Spells',
            18: 'Law',
            19: 'Chaos',
            20: 'Good',
            21: 'Evil'
        }
        return element_names.get(element_id, f'Element {element_id}')
    
    def _decode_target_type(self, prop_id: int, subtype: int) -> str:
        """Decode target type for conditional bonuses"""
        if prop_id == 57:  # vs Alignment Group
            alignment_groups = {0: 'Good', 1: 'Evil', 2: 'Lawful', 3: 'Chaotic'}
            return alignment_groups.get(subtype, f'Alignment {subtype}')
        elif prop_id == 58:  # vs Racial Group  
            try:
                race_data = self.rules_service.get_by_id('racialtypes', subtype)
                if race_data:
                    return field_mapper.get_field_value(race_data, 'Label', f'Race {subtype}')
            except Exception:
                pass
            return f'Race {subtype}'
        return f'Target {subtype}'
    
    def _get_damage_type_name(self, damage_type_id: int) -> str:
        """Get damage type name from ID"""
        damage_types = {
            0: 'bludgeoning',
            1: 'piercing', 
            2: 'slashing',
            3: 'subdual',
            4: 'physical',
            5: 'magical',
            6: 'acid',
            7: 'cold',
            8: 'divine',
            9: 'electrical',
            10: 'fire',
            11: 'negative',
            12: 'positive',
            13: 'sonic'
        }
        return damage_types.get(damage_type_id, f'type_{damage_type_id}')
    
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
        return [self.decode_property(prop) for prop in properties_list]
    
    def get_item_bonuses(self, properties_list: List[Dict[str, Any]]) -> Dict[str, Any]:
        """
        Extract quantified bonuses from raw property data for combat calculations
        
        This method handles ALL the business logic for converting raw property data
        into usable bonuses that other managers can consume directly.
        
        Returns:
            Dict with bonus categories and values ready for other managers
        """
        bonuses = {
            'abilities': {},  # ability_name -> bonus_value  
            'saves': {},      # save_type -> bonus_value  
            'skills': {},     # skill_name -> bonus_value
            'combat': {},     # attack/damage -> bonus_value
            'ac': {},         # ac_type -> bonus_value
            'immunities': [], # list of immunity types
            'special': {}     # other special properties
        }
        
        for prop in properties_list:
            # Extract raw values for business logic
            property_name = prop.get('PropertyName', 0)
            subtype = prop.get('Subtype', 0)
            cost_value = prop.get('CostValue', 0)
            
            # Handle each property type with complete business logic
            if property_name == 0:  # Ability Bonus
                ability = self._ability_map.get(subtype)
                if ability and cost_value > 0:
                    bonuses['abilities'][ability] = cost_value  # Store as 'Dex', not 'dex'
                    
            elif property_name == 1:  # AC Bonus (enchantment/deflection bonus)
                if cost_value > 0:
                    bonuses['ac']['deflection'] = cost_value
                    
            elif property_name == 6:  # Enhancement Bonus  
                if cost_value > 0:
                    bonuses['special']['enhancement'] = cost_value
                    
            elif property_name in [40, 41]:  # Saving Throw Bonuses
                if cost_value > 0:
                    if property_name == 40:
                        # Property 40 uses iprp_saveelement
                        # ONLY subtype 0 (Universal) applies to Fort/Ref/Will
                        # Other subtypes (Disease=5, Fear=8, etc) are specific save elements
                        if subtype == 0:  # Universal - applies to all three main saves
                            for save in ['fortitude', 'reflex', 'will']:
                                bonuses['saves'][save] = bonuses['saves'].get(save, 0) + cost_value
                        # Disease/Fear/Poison/etc saves are NOT added to main saves
                    else:  # Property 41 - Specific save (Fort/Ref/Will)
                        save_type = self._save_map.get(subtype)
                        if save_type:
                            bonuses['saves'][save_type] = bonuses['saves'].get(save_type, 0) + cost_value
                            
            elif property_name == 52:  # Skill Bonus
                if cost_value > 0:
                    skill_name = self._get_skill_name(subtype)
                    bonuses['skills'][skill_name] = cost_value
                    
            elif property_name in [56, 57, 58, 59]:  # Attack Bonuses
                if cost_value > 0:
                    bonuses['combat']['attack'] = bonuses['combat'].get('attack', 0) + cost_value
                    
            elif property_name == 75:  # Freedom of Movement
                bonuses['immunities'].append('movement_effects')
                
            elif property_name == 16:  # Damage Bonus
                if cost_value > 0:
                    bonuses['combat']['damage'] = bonuses['combat'].get('damage', 0) + cost_value
                    
            elif property_name == 23:  # Damage Resistance
                if cost_value > 0:
                    damage_type = self._get_damage_type_name(subtype)
                    bonuses['special'][f'resistance_{damage_type}'] = cost_value
                    
            elif property_name == 39:  # Spell Resistance  
                if cost_value > 0:
                    bonuses['special']['spell_resistance'] = cost_value
                    
            elif property_name == 44:  # Light
                bonuses['special']['light'] = 1
                
            elif property_name == 90:  # Damage Reduction (Modern NWN2 system only)
                if cost_value > 0:
                    bonuses['special']['damage_reduction'] = cost_value
                    # Note: PropertyName 22 (legacy DR system) is intentionally not supported
                    
            elif property_name == 92:  # Damage Vulnerability
                if cost_value > 0:
                    damage_type = self._get_damage_type_name(subtype)
                    bonuses['special'][f'vulnerability_{damage_type}'] = cost_value
                
            # Add more property types as needed based on actual game data
            
        return bonuses