"""Ability Manager - handles character abilities (Strength, Dexterity, etc.)."""

from typing import Dict, List, Optional, Any, Tuple
from loguru import logger
import time

from ..events import EventEmitter, EventType, EventData, ClassChangedEvent, LevelGainedEvent


class AbilityManager(EventEmitter):
    """Manages character ability scores."""

    ATTRIBUTES = ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']

    ABILITY_MAPPING = {
        'strength': 'Str',
        'dexterity': 'Dex',
        'constitution': 'Con',
        'intelligence': 'Int',
        'wisdom': 'Wis',
        'charisma': 'Cha'
    }

    def __init__(self, character_manager):
        """Initialize AbilityManager with parent CharacterManager."""
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.game_rules_service = character_manager.rules_service
        self._attributes_cache: Dict[str, Optional[Dict[str, int]]] = {}
        self.character_manager.on(EventType.CLASS_CHANGED, self._on_class_changed)
        self.character_manager.on(EventType.LEVEL_GAINED, self._on_level_gained)
    
    def get_attributes(self, include_equipment: bool = True, include_racial: bool = True) -> Dict[str, int]:
        """Get all attribute scores with optional equipment and racial bonuses."""
        cache_key = f"{'eq' if include_equipment else 'no_eq'}_{'race' if include_racial else 'no_race'}"

        if self._attributes_cache.get(cache_key) is not None:
            return self._attributes_cache[cache_key].copy()

        attributes = {}
        for attr in self.ATTRIBUTES:
            base_value = self.gff.get(attr)
            if base_value is None:
                raise ValueError(f"Attribute {attr} missing from GFF")
            attributes[attr] = base_value

        if include_racial:
            race_manager = self.character_manager.get_manager('race')
            if race_manager:
                deltas = race_manager.get_racial_modifier_deltas()
                for attr, delta in deltas.items():
                    if delta != 0:
                        attributes[attr] += delta

        if include_equipment:
            inventory_manager = self.character_manager.get_manager('inventory')
            if inventory_manager:
                equipment_bonuses = inventory_manager.get_equipment_bonuses()
                attributes_bonuses = equipment_bonuses.get('attributes', {}) or {}
                for attr in self.ATTRIBUTES:
                    equipment_bonus = attributes_bonuses.get(attr, 0)
                    if equipment_bonus != 0:
                        attributes[attr] += equipment_bonus

        self._attributes_cache[cache_key] = attributes.copy()
        return attributes

    def _invalidate_attributes_cache(self):
        """Invalidate the attributes cache."""
        self._attributes_cache = {}

    def get_attribute_modifiers(self) -> Dict[str, int]:
        """Calculate attribute modifiers using D&D rules: (score - 10) / 2."""
        attributes = self.get_attributes(include_equipment=False)
        return {attr: (value - 10) // 2 for attr, value in attributes.items()}

    def validate_attribute_value(self, attribute: str, value: int, context: str = "") -> Dict[str, Any]:
        """Validate attribute value against engine limits (3-50)."""
        result = {
            'valid': True,
            'attribute': attribute,
            'value': value,
            'context': context,
            'errors': [],
            'warnings': [],
            'corrected_value': value
        }
        
        if attribute not in self.ATTRIBUTES:
            result['valid'] = False
            result['errors'].append(f"Unknown attribute '{attribute}'. Valid attributes: {', '.join(self.ATTRIBUTES)}")
            return result

        if value < 3:
            result['valid'] = False
            result['errors'].append(f"{attribute} cannot be less than 3 (engine limit)")
            result['corrected_value'] = 3
        elif value > 50:
            result['valid'] = False
            result['errors'].append(f"{attribute} cannot be greater than 50 (engine limit)")
            result['corrected_value'] = 50
        elif value > 100:
            result['valid'] = False
            result['errors'].append(f"{attribute} value {value} indicates corrupted data")
            result['corrected_value'] = 50

        return result
    
    def validate_all_attributes(self, attributes: Dict[str, int], context: str = "") -> Dict[str, Any]:
        """Validate all attributes against engine limits."""
        results = {
            'valid': True,
            'context': context,
            'attributes': {},
            'total_errors': 0,
            'total_warnings': 0,
            'summary': []
        }
        
        for attr, value in attributes.items():
            attr_result = self.validate_attribute_value(attr, value, context)
            results['attributes'][attr] = attr_result
            
            if not attr_result['valid']:
                results['valid'] = False
                results['total_errors'] += len(attr_result['errors'])
            
            results['total_warnings'] += len(attr_result['warnings'])

        if results['total_errors'] > 0:
            results['summary'].append(f"Found {results['total_errors']} validation errors")
        if results['total_warnings'] > 0:
            results['summary'].append(f"Found {results['total_warnings']} warnings")
        if results['valid']:
            results['summary'].append("All attributes pass validation")
        
        return results

    def set_attribute(self, attribute: str, value: int, validate: bool = True) -> Dict[str, Any]:
        """Set a character attribute and handle cascading effects."""
        if attribute not in self.ATTRIBUTES:
            raise ValueError(f"Invalid attribute: {attribute}")

        old_value = self.gff.get(attribute)
        if old_value is None:
            raise ValueError(f"Attribute {attribute} missing from GFF")
        
        if validate:
            validation_result = self.validate_attribute_value(attribute, value, "set_attribute")
            if not validation_result['valid']:
                raise ValueError(f"Invalid {attribute} value: {'; '.join(validation_result['errors'])}")
            for warning in validation_result['warnings']:
                logger.warning(f"Attribute warning: {warning}")

        self.gff.set(attribute, value)

        if value > old_value:
            attr_to_index = {'Str': 0, 'Dex': 1, 'Con': 2, 'Int': 3, 'Wis': 4, 'Cha': 5}
            idx = attr_to_index.get(attribute)
            if idx is not None:
                try:
                    class_manager = self.character_manager.get_manager('class')
                    if class_manager:
                        class_manager.record_ability_change(idx)
                except Exception as e:
                    logger.warning(f"Failed to record ability change history: {e}")

        self._invalidate_attributes_cache()
        old_modifier = (old_value - 10) // 2
        new_modifier = (value - 10) // 2
        
        change = {
            'attribute': attribute,
            'old_value': old_value,
            'new_value': value,
            'old_modifier': old_modifier,
            'new_modifier': new_modifier
        }

        cascading_changes = []
        if attribute == 'Str':
            combat_change = self._update_str_combat_modifiers(old_modifier, new_modifier)
            if combat_change:
                cascading_changes.append(combat_change)
        elif attribute == 'Dex':
            ac_change = self._update_ac_components(old_modifier, new_modifier)
            if ac_change:
                cascading_changes.append(ac_change)
            combat_change = self._update_dex_combat_modifiers(old_modifier, new_modifier)
            if combat_change:
                cascading_changes.append(combat_change)
            save_change = self._update_saving_throw('reflex', old_modifier, new_modifier)
            if save_change:
                cascading_changes.append(save_change)
        elif attribute == 'Con':
            hp_change = self._recalculate_hit_points(old_modifier, new_modifier)
            if hp_change:
                cascading_changes.append(hp_change)
            save_change = self._update_saving_throw('fortitude', old_modifier, new_modifier)
            if save_change:
                cascading_changes.append(save_change)
        elif attribute in ['Int', 'Wis', 'Cha']:
            spell_change = self._update_spell_components(attribute, old_modifier, new_modifier)
            if spell_change:
                cascading_changes.append(spell_change)
            if attribute == 'Wis':
                save_change = self._update_saving_throw('will', old_modifier, new_modifier)
                if save_change:
                    cascading_changes.append(save_change)

        event = EventData(
            event_type=EventType.ATTRIBUTE_CHANGED,
            source_manager='attribute',
            timestamp=time.time()
        )
        event.changes = [change]
        event.cascading_changes = cascading_changes
        self.character_manager.emit(event)
        
        logger.info(f"Set {attribute} from {old_value} to {value}")
        return change
    
    def set_all_attributes(self, attributes: Dict[str, int], validate: bool = True) -> List[Dict[str, Any]]:
        """Set multiple attributes at once with cascading effects."""
        changes = []
        all_cascading_changes = []

        converted_attributes = {}
        for attr, value in attributes.items():
            gff_field = self.ABILITY_MAPPING.get(attr.lower())
            if gff_field:
                converted_attributes[gff_field] = value
            elif attr in self.ATTRIBUTES:
                converted_attributes[attr] = value
            else:
                logger.warning(f"Unknown attribute name: {attr}")
                continue

        for attr, value in converted_attributes.items():
            try:
                change = self.set_attribute(attr, value, validate)
                changes.append(change)
                if hasattr(change, 'cascading_effects') and change.cascading_effects:
                    all_cascading_changes.extend(change.cascading_effects)
            except Exception as e:
                logger.error(f"Failed to set attribute {attr} to {value}: {e}")
                changes.append({
                    'attribute': attr,
                    'old_value': self.gff.get(attr),
                    'new_value': value,
                    'error': str(e),
                    'success': False
                })

        if all_cascading_changes and changes:
            for change in reversed(changes):
                if change.get('success', True):
                    change['all_cascading_effects'] = all_cascading_changes
                    break

        return changes
    
    def apply_racial_modifiers(self, race_id: int) -> List[Dict[str, Any]]:
        """Apply racial attribute modifiers for a given race ID."""
        race = self.game_rules_service.get_by_id('racialtypes', race_id)
        if not race:
            raise ValueError(f"Unknown race ID: {race_id}")

        from gamedata.dynamic_loader.field_mapping_utility import field_mapper
        racial_mods = field_mapper.get_ability_modifiers(race)

        changes = []
        for attr, mod in racial_mods.items():
            if mod != 0:
                current = self.gff.get(attr)
                if current is None:
                    raise ValueError(f"Attribute {attr} missing from GFF")
                change = self.set_attribute(attr, current + mod, validate=False)
                change['racial_modifier'] = mod
                changes.append(change)

        return changes

    def get_total_attribute_points(self) -> int:
        """Get sum of all base attribute values."""
        attributes = self.get_attributes(include_equipment=False)
        return sum(attributes.values())

    def get_ability_points_summary(self) -> Dict[str, int]:
        """Summarize ability points granted at levels 4, 8, 12, etc."""
        lvl_stat_list = self.gff.get('LvlStatList', [])
        if not lvl_stat_list or not isinstance(lvl_stat_list, list):
            return {'total_available': 0, 'total_spent': 0, 'available': 0}

        total_char_level = len(lvl_stat_list)
        total_available = total_char_level // 4

        total_spent = 0
        for level_entry in lvl_stat_list:
            if isinstance(level_entry, dict) and level_entry.get('LvlStatAbility') is not None:
                total_spent += 1

        return {
            'total_available': total_available,
            'total_spent': total_spent,
            'available': max(0, total_available - total_spent)
        }
    
    def apply_ability_increase(self, attribute: str) -> Dict[str, Any]:
        """Apply a +1 ability score increase to an attribute."""
        if attribute not in self.ATTRIBUTES:
            raise ValueError(f"Invalid attribute: {attribute}")

        current = self.gff.get(attribute)
        if current is None:
            raise ValueError(f"Attribute {attribute} missing from GFF")

        change = self.set_attribute(attribute, current + 1, validate=False)
        change['reason'] = 'ability_increase'
        return change

    def _on_class_changed(self, event: ClassChangedEvent):
        """Handle class change events."""
        self._invalidate_attributes_cache()
        if event.is_level_adjustment:
            return
        self._adjust_level_up_bonuses_for_level(event.level)

    def _on_level_gained(self, event: LevelGainedEvent):
        """Handle level gained events."""
        self._invalidate_attributes_cache()

    def get_hit_points(self) -> Dict[str, int]:
        """Get character current and max hit points."""
        current_hp = self.gff.get('CurrentHitPoints')
        max_hp = self.gff.get('MaxHitPoints')
        if current_hp is None or max_hp is None:
            raise ValueError("Hit point fields missing from GFF")
        return {'current': current_hp, 'max': max_hp}

    def get_saving_throw_modifiers(self) -> Dict[str, int]:
        """Get ability modifiers for Fortitude, Reflex, and Will saves."""
        modifiers = self.get_attribute_modifiers()
        return {
            'fortitude': modifiers['Con'],
            'reflex': modifiers['Dex'],
            'will': modifiers['Wis']
        }

    def get_skill_modifiers(self) -> Dict[int, int]:
        """Get ability modifiers for each skill based on its governing attribute."""
        modifiers = self.get_attribute_modifiers()
        skill_mods = {}

        from gamedata.dynamic_loader.field_mapping_utility import field_mapper
        skills_table = self.game_rules_service.get_table('skills')
        if skills_table:
            for skill in skills_table:
                skill_id = field_mapper.get_field_value(skill, 'id', -1)
                if skill_id != -1:
                    key_ability = field_mapper.get_field_value(skill, 'key_ability', '')
                    if key_ability:
                        attr_name = key_ability.capitalize()
                        if attr_name in modifiers:
                            skill_mods[skill_id] = modifiers[attr_name]

        return skill_mods
    
    def _recalculate_hit_points(self, old_con_mod: int, new_con_mod: int) -> Optional[Dict[str, Any]]:
        """Recalculate hit points when Constitution modifier changes."""
        class_list = self.gff.get('ClassList')
        if not class_list:
            return None

        total_level = sum(
            int(cls.get('ClassLevel', 0)) if cls.get('ClassLevel') else 0
            for cls in class_list
            if isinstance(cls, dict)
        )
        if total_level == 0:
            return None

        con_mod_diff = new_con_mod - old_con_mod
        hp_change = total_level * con_mod_diff
        if hp_change == 0:
            return None

        current_hp = self.gff.get('CurrentHitPoints')
        max_hp = self.gff.get('MaxHitPoints')
        if current_hp is None or max_hp is None:
            raise ValueError("Hit point fields missing from GFF")

        new_max_hp = max_hp + hp_change
        new_current_hp = max(1, min(current_hp + hp_change, new_max_hp))

        self.gff.set('MaxHitPoints', new_max_hp)
        self.gff.set('CurrentHitPoints', new_current_hp)
        self.gff.set('HitPoints', new_max_hp)

        return {
            'type': 'hp_recalculation',
            'reason': 'constitution_change',
            'old_con_modifier': old_con_mod,
            'new_con_modifier': new_con_mod,
            'level': total_level,
            'hp_change_per_level': con_mod_diff,
            'total_hp_change': hp_change,
            'old_max_hp': max_hp,
            'new_max_hp': new_max_hp,
            'old_current_hp': current_hp,
            'new_current_hp': new_current_hp
        }

    def get_encumbrance_limits(self) -> Dict[str, Any]:
        """Get encumbrance thresholds based on Strength."""
        strength = self.gff.get('Str')
        if strength is None:
            raise ValueError("Str attribute missing from GFF")
        strength = int(strength)

        from gamedata.dynamic_loader.field_mapping_utility import field_mapper
        encumbrance_data = self.game_rules_service.get_by_id('encumbrance', strength)
        if not encumbrance_data:
            raise ValueError(f"No encumbrance data for Strength {strength}")

        normal_capacity = int(field_mapper.get_field_value(encumbrance_data, 'normal'))
        heavy_threshold = int(field_mapper.get_field_value(encumbrance_data, 'heavy'))
        if normal_capacity is None or heavy_threshold is None:
            raise ValueError(f"Invalid encumbrance data for Strength {strength}")

        medium_threshold = int(heavy_threshold * 0.67)
        
        return {
            'strength': strength,
            'normal_capacity': normal_capacity,
            'medium_load': medium_threshold,
            'heavy_load': heavy_threshold,
            'current_weight': 0  # Would need to calculate from inventory
        }
    
    def _update_ac_components(self, old_dex_mod: int, new_dex_mod: int) -> Optional[Dict[str, Any]]:
        """Track AC component changes when DEX modifier changes."""
        if old_dex_mod == new_dex_mod:
            return None
        return {
            'type': 'ac_component_update',
            'reason': 'dexterity_change',
            'old_dex_modifier': old_dex_mod,
            'new_dex_modifier': new_dex_mod,
            'dex_ac_change': new_dex_mod - old_dex_mod,
            'note': 'Total AC change depends on armor max dex bonus'
        }
    
    def _update_spell_components(self, attribute: str, old_mod: int, new_mod: int) -> Optional[Dict[str, Any]]:
        """Track spell component changes when INT/WIS/CHA modifier changes."""
        if old_mod == new_mod:
            return None

        affected_spells = self._get_spells_using_attribute(attribute)
        if not affected_spells:
            return None

        dc_change = new_mod - old_mod
        bonus_spells = {}
        for spell_level in range(1, 10):
            if new_mod >= spell_level:
                old_bonus = max(0, (old_mod - spell_level + 1) // 4 + 1) if old_mod >= spell_level else 0
                new_bonus = max(0, (new_mod - spell_level + 1) // 4 + 1)
                if old_bonus != new_bonus:
                    bonus_spells[spell_level] = {'old': old_bonus, 'new': new_bonus, 'change': new_bonus - old_bonus}

        affected_classes = list(set(spell_info['class_name'] for spell_info in affected_spells.values()))
        
        result = {
            'type': 'spell_component_update',
            'reason': f'{attribute.lower()}_change',
            'attribute': attribute,
            'old_modifier': old_mod,
            'new_modifier': new_mod,
            'affected_classes': affected_classes,
            'affected_spells': affected_spells,
            'spell_dc_change': dc_change,
            'bonus_spells': bonus_spells
        }
        
        if attribute == 'Cha':
            turn_undead_classes = [cls for cls in affected_classes if cls.lower() in ['cleric', 'paladin']]
            if turn_undead_classes:
                old_turns = 3 + old_mod
                new_turns = 3 + new_mod
                result['turn_undead'] = {
                    'old_uses': max(0, old_turns),
                    'new_uses': max(0, new_turns),
                    'change': new_turns - old_turns,
                    'classes': turn_undead_classes
                }

        return result

    def _get_spells_using_attribute(self, attribute: str) -> Dict[int, Dict[str, Any]]:
        """Get all spells known by the character that use the specified attribute."""
        affected_spells = {}
        attribute_to_classes = {
            'Int': ['Wiz_Sorc'],
            'Wis': ['Cleric', 'Druid', 'Ranger'],
            'Cha': ['Wiz_Sorc', 'Bard', 'Paladin', 'Warlock']
        }

        target_class_types = attribute_to_classes.get(attribute, [])
        if not target_class_types:
            return affected_spells

        for spell_level in range(10):
            known_spells = self.gff.get(f'KnownList{spell_level}', [])
            for spell_entry in known_spells:
                if not isinstance(spell_entry, dict):
                    continue
                spell_id = spell_entry.get('Spell', -1)
                if spell_id == -1:
                    continue
                spell_info = self._get_spell_casting_info(spell_id, attribute, target_class_types)
                if spell_info:
                    affected_spells[spell_id] = spell_info

        for spell_level in range(1, 10):
            mem_data = self.gff.get(f'SpellLvlMem{spell_level}', [])
            
            for mem_entry in mem_data:
                if not isinstance(mem_entry, dict):
                    continue
                memorized_list = mem_entry.get('MemorizedList', [])
                for mem_spell in memorized_list:
                    if not isinstance(mem_spell, dict):
                        continue
                    spell_id = mem_spell.get('Spell', -1)
                    if spell_id == -1:
                        continue
                    spell_info = self._get_spell_casting_info(spell_id, attribute, target_class_types)
                    if spell_info:
                        affected_spells[spell_id] = spell_info

        return affected_spells

    def _get_spell_casting_info(self, spell_id: int, attribute: str, target_class_types: List[str]) -> Optional[Dict[str, Any]]:
        """Get casting info if spell uses the specified attribute, else None."""
        from gamedata.dynamic_loader.field_mapping_utility import field_mapper
        try:
            spell_data = self.game_rules_service.get_by_id('spells', spell_id)
            if not spell_data:
                return None

            for class_type in target_class_types:
                spell_level_str = field_mapper.get_field_value(spell_data, class_type.lower(), '')
                if spell_level_str and str(spell_level_str).strip() and str(spell_level_str) != '****':
                    try:
                        spell_level = int(spell_level_str)
                        return {
                            'spell_id': spell_id,
                            'spell_label': field_mapper.get_field_value(spell_data, 'label', f'Spell_{spell_id}'),
                            'class_type': class_type,
                            'class_name': self._class_type_to_name(class_type),
                            'spell_level': spell_level,
                            'casting_attribute': attribute
                        }
                    except (ValueError, TypeError):
                        continue
            return None
        except Exception as e:
            logger.warning(f"Error checking spell {spell_id}: {e}")
            return None
    
    def _class_type_to_name(self, class_type: str) -> str:
        """Convert class type from spells.2da to readable name."""
        try:
            if class_type == 'Wiz_Sorc':
                return 'Wizard/Sorcerer'

            from gamedata.dynamic_loader.field_mapping_utility import field_mapper
            classes = self.game_rules_service.get_table('classes')
            if classes:
                for class_data in classes:
                    label = field_mapper.get_field_value(class_data, 'label', '')
                    if label.lower() == class_type.lower():
                        name = field_mapper.get_field_value(class_data, 'name', label)
                        return name if name else label
        except Exception as e:
            logger.warning(f"Could not find class name for type {class_type}: {e}")
        return class_type

    def _update_str_combat_modifiers(self, old_mod: int, new_mod: int) -> Optional[Dict[str, Any]]:
        """Track combat modifier changes when STR modifier changes."""
        if old_mod == new_mod:
            return None

        modifier_change = new_mod - old_mod
        has_weapon_finesse = self._has_feat_by_name('WeaponFinesse')
        
        result = {
            'type': 'combat_modifiers_update',
            'reason': 'strength_change',
            'old_str_modifier': old_mod,
            'new_str_modifier': new_mod,
            'melee_damage_bonus_change': modifier_change,  # Always uses STR
            'note': 'STR modifier applies to melee damage'
        }
        
        if has_weapon_finesse:
            result['melee_attack_bonus_change'] = 0  # No change for finesse weapons
            result['note'] += ' (Weapon Finesse: DEX used for finesse weapon attacks)'
        else:
            result['melee_attack_bonus_change'] = modifier_change
            result['note'] += ' and melee attack rolls'
        
        return result
    
    def _update_dex_combat_modifiers(self, old_mod: int, new_mod: int) -> Optional[Dict[str, Any]]:
        """Track combat modifier changes when DEX modifier changes."""
        if old_mod == new_mod:
            return None
        
        modifier_change = new_mod - old_mod
        
        # Check if character has Weapon Finesse
        has_weapon_finesse = self._has_feat_by_name('WeaponFinesse')
        
        result = {
            'type': 'combat_modifiers_update',
            'reason': 'dexterity_change',
            'old_dex_modifier': old_mod,
            'new_dex_modifier': new_mod,
            'ranged_attack_bonus_change': modifier_change,
            'initiative_bonus_change': modifier_change,
            'note': 'DEX modifier applies to ranged attack rolls and initiative'
        }
        
        if has_weapon_finesse:
            result['finesse_melee_attack_bonus_change'] = modifier_change
            result['note'] += ' (Weapon Finesse: also applies to finesse weapon melee attacks)'
        
        return result
    
    def _has_feat_by_name(self, feat_label: str) -> bool:
        """Check if character has a feat by its label name."""
        feat_manager = self.character_manager.get_manager('feat')
        return feat_manager.has_feat_by_name(feat_label) if feat_manager else False

    def _update_saving_throw(self, save_type: str, old_mod: int, new_mod: int) -> Optional[Dict[str, Any]]:
        """Track saving throw changes when its governing attribute changes."""
        if old_mod == new_mod:
            return None
        modifier_change = new_mod - old_mod
        return {
            'type': 'saving_throw_update',
            'save_type': save_type,
            'old_modifier': old_mod,
            'new_modifier': new_mod,
            'save_bonus_change': modifier_change,
            'note': f'{save_type.capitalize()} save {"improved" if modifier_change > 0 else "reduced"} by {abs(modifier_change)}'
        }

    def get_attribute(self, attribute: str) -> int:
        """Get a single attribute value by GFF field name."""
        if attribute not in self.ATTRIBUTES:
            raise ValueError(f"Invalid attribute: {attribute}")
        value = self.gff.get(attribute)
        if value is None:
            raise ValueError(f"Attribute {attribute} missing from GFF")
        return value

    def get_ability_score(self, ability_name: str) -> int:
        """Get ability score using standard name mapping (strength, dexterity, etc.)."""
        gff_field = self.ABILITY_MAPPING.get(ability_name.lower())
        if not gff_field:
            raise ValueError(f"Unknown ability name: {ability_name}")
        value = self.gff.get(gff_field)
        if value is None:
            raise ValueError(f"Ability {ability_name} missing from GFF")
        return value

    def get_ability_scores(self) -> Dict[str, int]:
        """Get all base ability scores using standard names (without equipment)."""
        gff_attributes = self.get_attributes(include_equipment=False)
        return {
            ability: gff_attributes[gff_field]
            for ability, gff_field in self.ABILITY_MAPPING.items()
        }

    def set_ability_score(self, ability_name: str, value: int):
        """Set ability score using standard name mapping."""
        gff_field = self.ABILITY_MAPPING.get(ability_name.lower())
        if gff_field:
            self.set_attribute(gff_field, value)
        else:
            raise ValueError(f"Unknown ability name: {ability_name}")
    
    def get_attribute_modifier(self, attribute: str) -> int:
        """Get D&D modifier for a single attribute."""
        value = self.get_attribute(attribute)
        return (value - 10) // 2

    def get_attribute_dependencies(self) -> Dict[str, List[str]]:
        """Get static list of game mechanics that depend on each attribute."""
        return {
            'Str': [
                'Melee attack rolls',
                'Melee damage rolls',
                'Carrying capacity',
                'Some skill checks (Climb, Jump, Swim)',
                'Combat maneuvers (Trip, Bull Rush, etc.)'
            ],
            'Dex': [
                'Ranged attack rolls',
                'AC bonus (limited by armor)',
                'Reflex saves',
                'Initiative',
                'Some skill checks (Hide, Move Silently, etc.)',
                'Weapon Finesse melee attacks'
            ],
            'Con': [
                'Hit points per level',
                'Fortitude saves',
                'Concentration checks'
            ],
            'Int': [
                'Skill points per level',
                'Wizard spell slots',
                'Some skill checks (Appraise, Craft, etc.)',
                'Combat Expertise AC bonus'
            ],
            'Wis': [
                'Will saves',
                'Cleric/Druid/Ranger spell slots',
                'Some skill checks (Listen, Spot, etc.)',
                'Monk AC bonus'
            ],
            'Cha': [
                'Sorcerer/Bard/Paladin spell slots',
                'Turn Undead uses',
                'Some skill checks (Bluff, Diplomacy, etc.)',
                'Paladin saving throw bonus'
            ]
        }

    def get_all_modifiers(self) -> Dict[str, int]:
        """Get all ability modifiers using uppercase GFF field names."""
        return {attr.upper(): self.get_attribute_modifier(attr) for attr in self.ATTRIBUTES}

    def get_racial_modifiers(self) -> Dict[str, int]:
        """Get combined racial + subrace attribute modifiers (delta)."""
        race_manager = self.character_manager.get_manager('race')
        if race_manager:
            return race_manager.get_racial_modifier_deltas()
        return {attr: 0 for attr in self.ATTRIBUTES}

    def get_item_modifiers(self) -> Dict[str, int]:
        """Get attribute modifiers from equipped items."""
        inventory_manager = self.character_manager.get_manager('inventory')
        if not inventory_manager:
            return {attr: 0 for attr in self.ATTRIBUTES}

        equipment_bonuses = inventory_manager.get_equipment_bonuses()
        attribute_bonuses = equipment_bonuses.get('attributes', {})
        result = {attr: 0 for attr in self.ATTRIBUTES}
        result.update(attribute_bonuses)
        return result

    def get_enhancement_modifiers(self) -> Dict[str, int]:
        """Get enhancement modifiers (placeholder - not yet implemented)."""
        return {attr: 0 for attr in self.ATTRIBUTES}
    
    def get_temporary_modifiers(self) -> Dict[str, int]:
        """Get temporary modifiers (placeholder - not yet implemented)."""
        return {attr: 0 for attr in self.ATTRIBUTES}

    def get_level_up_modifiers(self) -> Dict[str, int]:
        """Get level-up attribute bonuses from LvlStatList (every 4 levels grants +1)."""
        class_list = self.gff.get('ClassList', [])
        total_level = sum(
            cls.get('ClassLevel', 0)
            for cls in class_list
            if isinstance(cls, dict)
        )

        levelup_bonuses = {attr: 0 for attr in self.ATTRIBUTES}
        lvl_stat_list = self.gff.get('LvlStatList', [])

        ABILITY_INDEX_MAP = {0: 'Str', 1: 'Dex', 2: 'Con', 3: 'Int', 4: 'Wis', 5: 'Cha'}

        for level_idx, level_entry in enumerate(lvl_stat_list):
            if not isinstance(level_entry, dict):
                continue
            if 'LvlStatAbility' in level_entry:
                ability_index = level_entry['LvlStatAbility']
                if ability_index in ABILITY_INDEX_MAP:
                    attr_name = ABILITY_INDEX_MAP[ability_index]
                    levelup_bonuses[attr_name] += 1

        return levelup_bonuses

    def _adjust_level_up_bonuses_for_level(self, new_total_level: int):
        """Adjust level-up bonuses when character level decreases."""
        levelup_list = self.gff.get('LevelUpList', [])
        bonuses_to_remove = {attr: 0 for attr in self.ATTRIBUTES}
        valid_levelup_entries = []

        for levelup_entry in levelup_list:
            if not isinstance(levelup_entry, dict):
                continue
            entry_level = levelup_entry.get('Level', 0)
            if entry_level <= new_total_level:
                valid_levelup_entries.append(levelup_entry)
            else:
                for attr in self.ATTRIBUTES:
                    for field_name in [f'{attr}Gain', f'Ability{attr}', attr]:
                        if field_name in levelup_entry:
                            bonus_amount = levelup_entry.get(field_name, 0)
                            if bonus_amount > 0:
                                bonuses_to_remove[attr] += bonus_amount

        if any(bonus > 0 for bonus in bonuses_to_remove.values()):
            current_attrs = self.get_attributes(include_equipment=False)
            for attr in self.ATTRIBUTES:
                if bonuses_to_remove[attr] > 0:
                    old_value = current_attrs[attr]
                    new_value = max(3, old_value - bonuses_to_remove[attr])
                    self.gff.set(attr, new_value)

            self.gff.set('LevelUpList', valid_levelup_entries)

            event = EventData(
                event_type=EventType.ATTRIBUTE_CHANGED,
                source_manager='ability',
                timestamp=time.time()
            )
            event.level_reduction = True
            event.new_level = new_total_level
            event.removed_bonuses = bonuses_to_remove
            event.removed_entries = len(levelup_list) - len(valid_levelup_entries)
            self.character_manager.emit(event)
        elif len(valid_levelup_entries) != len(levelup_list):
            self.gff.set('LevelUpList', valid_levelup_entries)

    def get_total_modifiers(self) -> Dict[str, int]:
        """Get total effective D&D modifiers from all sources."""
        base_attrs = self.get_attributes(include_equipment=False)
        racial_mods = self.get_racial_modifiers()
        item_mods = self.get_item_modifiers()
        enhancement_mods = self.get_enhancement_modifiers()
        temp_mods = self.get_temporary_modifiers()

        total_modifiers = {}
        for attr in self.ATTRIBUTES:
            effective_score = (
                base_attrs[attr] +
                racial_mods[attr] +
                item_mods[attr] +
                enhancement_mods[attr] +
                temp_mods[attr]
            )
            total_modifiers[attr] = (effective_score - 10) // 2
        return total_modifiers

    def get_effective_attributes(self) -> Dict[str, int]:
        """Get effective attribute scores including equipment and racial bonuses."""
        return self.get_attributes(include_equipment=True, include_racial=True)

    def calculate_point_buy_total(self) -> int:
        """Calculate point buy cost for current attributes (informational only)."""
        POINT_BUY_COSTS = {
            8: 0, 9: 1, 10: 2, 11: 3, 12: 4, 13: 5, 14: 6,
            15: 8, 16: 10, 17: 13, 18: 16
        }
        total_cost = 0
        base_attributes = self.get_attributes(include_equipment=False)
        for attr in self.ATTRIBUTES:
            value = base_attributes.get(attr, 10)
            if value <= 8:
                cost = 0
            elif value >= 18:
                cost = 16
            else:
                cost = POINT_BUY_COSTS.get(value, 0)
            total_cost += cost
        return total_cost

    def update_base_attributes(self, base_values: Dict[str, int]) -> List[Dict[str, Any]]:
        """
        Update base attributes (excluding level-up bonuses).
        This is used when editing 'starting' stats, preserving level-up history.
        """
        level_up_mods = self.get_level_up_modifiers()
        gff_values = {
            attr: val + level_up_mods.get(attr, 0)
            for attr, val in base_values.items()
        }
        return self.set_all_attributes(gff_values)

    def set_attribute_by_name(self, name: str, value: int) -> Dict[str, Any]:
        """Set attribute by case-insensitive name (e.g., 'strength', 'str')."""
        gff_field = self.ABILITY_MAPPING.get(name.lower())
        if not gff_field:
            if name in self.ATTRIBUTES:
                gff_field = name
            else:
                raise ValueError(f"Invalid attribute name: {name}")
                
        return self.set_attribute(gff_field, value)

    def validate(self) -> Tuple[bool, List[str]]:
        """Validate all ability scores against engine limits (3-50)."""
        errors = []
        for ability_name in self.ABILITY_MAPPING.keys():
            value = self.get_ability_score(ability_name)
            if value < 3 or value > 50:
                errors.append(f"{ability_name.title()} must be between 3 and 50 (got {value})")
        return len(errors) == 0, errors
