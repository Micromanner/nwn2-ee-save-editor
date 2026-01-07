"""Manage character saving throw calculations, including base saves, modifiers, feats, and effects."""

from typing import Dict, List, Tuple, Any
from loguru import logger

from ..events import EventEmitter, EventType, EventData
from gamedata.dynamic_loader.field_mapping_utility import field_mapper


class SaveManager(EventEmitter):
    """Manages saving throw calculations including all modifiers."""
    
    def __init__(self, character_manager):
        """Initialize the SaveManager with parent CharacterManager."""
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.rules_service = character_manager.rules_service
        
        self.temporary_modifiers = {
            'fortitude': 0,
            'reflex': 0,
            'will': 0
        }
        
        self._racial_cache = {}

        self._initialize_data_lookups()
        self._register_event_handlers()
    
    def _initialize_data_lookups(self):
        """Initialize data-driven lookups for save calculations."""
        # Raise error if data is missing - we cannot function without rules
        try:
            self._build_racial_save_cache()
        except Exception as e:
            logger.error(f"Failed to initialize save data lookups: {e}")
            raise RuntimeError(f"SaveManager initialization failed: {e}") from e
    
    def _build_racial_save_cache(self):
        """Build cache of racial save bonuses from racialtypes.2da."""
        self._racial_cache = {}
        
        races = self.rules_service.get_table('racialtypes')
        if not races:
            raise RuntimeError("Could not load 'racialtypes' table")

        for race in races:
            race_id = getattr(race, 'id', None) if hasattr(race, 'id') else None
            # Some rows might be padding/invalid
            if race_id is None:
                continue
            
            self._racial_cache[race_id] = self.get_racial_saves(race_id)
    
    def _register_event_handlers(self):
        """Register handlers for events that affect saves."""
        self.character_manager.on(EventType.ATTRIBUTE_CHANGED, self._on_attribute_changed)
        self.character_manager.on(EventType.CLASS_CHANGED, self._on_class_changed)
        self.character_manager.on(EventType.FEAT_ADDED, self._on_feat_changed)
        self.character_manager.on(EventType.FEAT_REMOVED, self._on_feat_changed)
    
    def calculate_saving_throws(self) -> Dict[str, Any]:
        """Calculate all saving throws with complete breakdown."""

        # 1. Base Saves (Classes + Epic)
        base_saves = self._calculate_base_saves()
        
        # 2. Ability Modifiers
        ability_manager = self.character_manager.get_manager('ability')
        if not ability_manager:
            raise RuntimeError("AbilityManager is required for save calculation but is missing.")
            
        total_modifiers = ability_manager.get_total_modifiers()
        con_mod = total_modifiers.get('Con', 0)
        dex_mod = total_modifiers.get('Dex', 0)
        wis_mod = total_modifiers.get('Wis', 0)

        # 3. Equipment Bonuses
        inventory_manager = self.character_manager.get_manager('inventory')
        if not inventory_manager:
            raise RuntimeError("InventoryManager is required for save calculation but is missing.")

        equipment_bonuses = inventory_manager.get_equipment_bonuses()
        
        # 4. Feat Bonuses (includes Class-specific like Divine Grace)
        feat_bonuses = self._calculate_feat_bonuses()

        # 5. Racial Bonuses
        racial_bonuses = self._calculate_racial_bonuses()

        # 6. Resistance/Misc Bonuses
        resistance_bonuses = self._calculate_resistance_bonuses()
        
        # Add equipment saves to resistance/universal bonuses
        save_bonuses = equipment_bonuses.get('saves', {})
        if save_bonuses:
            resistance_bonuses['fortitude'] += save_bonuses.get('fortitude', 0)
            resistance_bonuses['reflex'] += save_bonuses.get('reflex', 0)
            resistance_bonuses['will'] += save_bonuses.get('will', 0)

        misc_fort = self.gff.get('fortbonus', 0)
        misc_ref = self.gff.get('refbonus', 0)
        misc_will = self.gff.get('willbonus', 0)
        
        # Calculate Totals
        fort_total = (base_saves['base_fortitude'] + con_mod +
                     feat_bonuses['fortitude'] + racial_bonuses['fortitude'] +
                     resistance_bonuses['fortitude'] + self.temporary_modifiers['fortitude'] +
                     misc_fort)

        ref_total = (base_saves['base_reflex'] + dex_mod +
                     feat_bonuses['reflex'] + racial_bonuses['reflex'] +
                     resistance_bonuses['reflex'] + self.temporary_modifiers['reflex'] +
                     misc_ref)

        will_total = (base_saves['base_will'] + wis_mod +
                      feat_bonuses['will'] + racial_bonuses['will'] +
                      resistance_bonuses['will'] + self.temporary_modifiers['will'] +
                      misc_will)

        result = {
            'fortitude': {
                'total': fort_total,
                'base': base_saves['base_fortitude'],
                'ability': con_mod,
                'feat': feat_bonuses['fortitude'],
                'racial': racial_bonuses['fortitude'],
                'resistance': resistance_bonuses['fortitude'],
                'temporary': self.temporary_modifiers['fortitude'],
                'misc': misc_fort,
                'breakdown': self._format_breakdown('Fortitude', fort_total, 
                    base_saves['base_fortitude'], con_mod, 'CON',
                    feat_bonuses['fortitude'], racial_bonuses['fortitude'],
                    resistance_bonuses['fortitude'], self.temporary_modifiers['fortitude'], misc_fort)
            },
            'reflex': {
                'total': ref_total,
                'base': base_saves['base_reflex'],
                'ability': dex_mod,
                'feat': feat_bonuses['reflex'],
                'racial': racial_bonuses['reflex'],
                'resistance': resistance_bonuses['reflex'],
                'temporary': self.temporary_modifiers['reflex'],
                'misc': misc_ref,
                'breakdown': self._format_breakdown('Reflex', ref_total,
                    base_saves['base_reflex'], dex_mod, 'DEX',
                    feat_bonuses['reflex'], racial_bonuses['reflex'],
                    resistance_bonuses['reflex'], self.temporary_modifiers['reflex'], misc_ref)
            },
            'will': {
                'total': will_total,
                'base': base_saves['base_will'],
                'ability': wis_mod,
                'feat': feat_bonuses['will'],
                'racial': racial_bonuses['will'],
                'resistance': resistance_bonuses['will'],
                'temporary': self.temporary_modifiers['will'],
                'misc': misc_will,
                'breakdown': self._format_breakdown('Will', will_total,
                    base_saves['base_will'], wis_mod, 'WIS',
                    feat_bonuses['will'], racial_bonuses['will'],
                    resistance_bonuses['will'], self.temporary_modifiers['will'], misc_will)
            }
        }

        return result

    def _calculate_base_saves(self) -> Dict[str, int]:
        """Calculate total base saving throws from all classes (Heroic + Epic)."""
        total_fort = 0
        total_ref = 0
        total_will = 0
        
        lvl_stat_list = self.gff.get('LvlStatList', [])

        
        if lvl_stat_list and isinstance(lvl_stat_list, list):
            # Calculate from level history (use available data)

            current_class_levels = {}
            for i, level_entry in enumerate(lvl_stat_list):
                char_level = i + 1
                class_id = level_entry.get('LvlStatClass', -1)

                if class_id == -1:
                    continue

                current_class_levels[class_id] = current_class_levels.get(class_id, 0) + 1
                class_lvl = current_class_levels[class_id]

                if char_level <= 20:
                    class_data = self.rules_service.get_by_id('classes', class_id)
                    if class_data:
                        current_saves = self._calculate_base_save_delta(class_data, class_lvl)
                        prev_saves = self._calculate_base_save_delta(class_data, class_lvl - 1)

                        total_fort += (current_saves['fortitude'] - prev_saves['fortitude'])
                        total_ref += (current_saves['reflex'] - prev_saves['reflex'])
                        total_will += (current_saves['will'] - prev_saves['will'])
                else:
                    # Epic Levels (21+): +1 to All Saves at every EVEN character level
                    if char_level % 2 == 0:
                        total_fort += 1
                        total_ref += 1
                        total_will += 1
        else:
            logger.warning("No LvlStatList found - returning zero base saves")


        return {
            'fortitude': total_fort,
            'reflex': total_ref,
            'will': total_will,
            'base_fortitude': total_fort,
            'base_reflex': total_ref,
            'base_will': total_will
        }

    def _calculate_base_save_delta(self, class_data, level: int) -> Dict[str, int]:
        """Calculate base saves for a single class at a specific level (clamped at 20)."""
        save_table_name = field_mapper.get_field_value(class_data, 'saving_throw_table', '')
        if not save_table_name or level <= 0:
            return {'fortitude': 0, 'reflex': 0, 'will': 0}
            
        save_table = self.rules_service.get_table(save_table_name.lower())
        if not save_table:
            return {'fortitude': 0, 'reflex': 0, 'will': 0}
        
        # Tables are 0-indexed, so level 1 is index 0. Max index is 19 (for level 20).
        level_idx = min(level - 1, 19)
        if level_idx < len(save_table):
            save_row = save_table[level_idx]
            return {
                'fortitude': field_mapper._safe_int(field_mapper.get_field_value(save_row, 'fort_save_table', '0'), 0),
                'reflex': field_mapper._safe_int(field_mapper.get_field_value(save_row, 'ref_save_table', '0'), 0),
                'will': field_mapper._safe_int(field_mapper.get_field_value(save_row, 'will_save_table', '0'), 0)
            }
        
        return {'fortitude': 0, 'reflex': 0, 'will': 0}

    def _invalidate_saves_cache(self):
        """No-op - caching removed due to thread safety issues with concurrent requests."""
        pass

    def _calculate_feat_bonuses(self) -> Dict[str, int]:
        """Calculate save bonuses from feats by delegating to FeatManager."""
        feat_manager = self.character_manager.get_manager('feat')
        if not feat_manager:
            raise RuntimeError("FeatManager is required for save calculation.")

        bonuses = feat_manager.get_save_bonuses()

        # Add special class-based bonuses (e.g., Paladin Divine Grace)
        class_bonuses = self._calculate_class_save_bonuses()
        for save_type in bonuses:
            bonuses[save_type] += class_bonuses.get(save_type, 0)

        return bonuses
    
    def _calculate_class_save_bonuses(self) -> Dict[str, int]:
        """Calculate class save bonuses from feats (Divine Grace, Dark One's Luck), delegating to FeatManager."""
        bonuses = {'fortitude': 0, 'reflex': 0, 'will': 0}
        
        # We need FeatManager to check for the feats
        feat_manager = self.character_manager.get_manager('feat')
        if not feat_manager:
            raise RuntimeError("FeatManager is required for class bonus calculation.")

        # We need AbilityManager to get the modifiers
        attr_manager = self.character_manager.get_manager('ability')
        if not attr_manager:
            raise RuntimeError("AbilityManager is required for class bonus calculation.")
            
        all_modifiers = attr_manager.get_all_modifiers()
        cha_mod = all_modifiers.get('CHA', 0)
        
        # Divine Grace (Feat 214) - Paladin/Blackguard
        # Adds CHA bonus to all saves
        if feat_manager.has_feat(214) and cha_mod > 0:
            bonuses['fortitude'] += cha_mod
            bonuses['reflex'] += cha_mod
            bonuses['will'] += cha_mod
            
        # Dark One's Luck (Feat 400) - Warlock
        # Adds CHA bonus to all saves
        if feat_manager.has_feat(400) and cha_mod > 0:
            bonuses['fortitude'] += cha_mod
            bonuses['reflex'] += cha_mod
            bonuses['will'] += cha_mod
            
        return bonuses
    
    def _calculate_racial_bonuses(self) -> Dict[str, int]:
        """Calculate save bonuses from race using cached data."""
        bonuses = {'fortitude': 0, 'reflex': 0, 'will': 0}
        
        race_id = self.gff.get('Race', 0)
        
        try:
            # Use data-driven lookup
            if race_id in self._racial_cache:
                racial_bonuses = self._racial_cache[race_id]
                bonuses['fortitude'] += racial_bonuses['fortitude']
                bonuses['reflex'] += racial_bonuses['reflex']
                bonuses['will'] += racial_bonuses['will']
            else:
                 # If not in cache, try direct lookup or it might be invalid race
                 race_data = self.rules_service.get_by_id('racialtypes', race_id)
                 if race_data:
                     # Update cache?
                     r_bonuses = field_mapper.get_racial_saves(race_data)
                     bonuses['fortitude'] += r_bonuses['fortitude']
                     bonuses['reflex'] += r_bonuses['reflex']
                     bonuses['will'] += r_bonuses['will']
        except Exception as e:
            logger.error(f"Error calculating racial bonuses for race {race_id}: {e}")
            raise
        
        return bonuses
    
    def _calculate_resistance_bonuses(self) -> Dict[str, int]:
        """Calculate resistance bonuses (reserved for spells/effects)."""
        return {'fortitude': 0, 'reflex': 0, 'will': 0}
    
    def add_temporary_modifier(self, save_type: str, modifier: int, duration: float = 0):
        """Add a temporary save modifier."""
        if save_type in self.temporary_modifiers:
            self.temporary_modifiers[save_type] += modifier
            logger.debug(f"Added {modifier:+d} temporary {save_type} save modifier")
            self._invalidate_saves_cache()
    
    def remove_temporary_modifier(self, save_type: str, modifier: int):
        """Remove a temporary save modifier."""
        if save_type in self.temporary_modifiers:
            self.temporary_modifiers[save_type] -= modifier
            logger.debug(f"Removed {modifier:+d} temporary {save_type} save modifier")
            self._invalidate_saves_cache()
    
    def clear_temporary_modifiers(self):
        """Clear all temporary save modifiers."""
        self.temporary_modifiers = {'fortitude': 0, 'reflex': 0, 'will': 0}
        self._invalidate_saves_cache()
    
    def _format_breakdown(self, save_name: str, total: int, base: int, 
                         ability: int, ability_name: str, feat: int, 
                         racial: int, resistance: int, temporary: int, misc: int = 0) -> str:
        """Format a save breakdown string for display."""
        parts = [f"{save_name} +{total} ="]
        parts.append(f"base {base:+d}")
        parts.append(f"{ability_name} {ability:+d}")
        
        if feat != 0: parts.append(f"feats {feat:+d}")
        if racial != 0: parts.append(f"racial {racial:+d}")
        if resistance != 0: parts.append(f"resistance {resistance:+d}")
        if temporary != 0: parts.append(f"temporary {temporary:+d}")
        if misc != 0: parts.append(f"misc {misc:+d}")
        
        return " + ".join(parts)
    
    def check_save(self, save_type: str, dc: int, modifier: int = 0, take_20: bool = False) -> Dict[str, Any]:
        """Check if a save would succeed against a DC."""
        saves = self.calculate_saving_throws()
        
        if save_type not in saves:
            raise ValueError(f"Invalid save type: {save_type}")
        
        total_bonus = saves[save_type]['total'] + modifier
        
        if take_20:
            return {
                'success': (total_bonus + 20) >= dc,
                'total_bonus': total_bonus,
                'dc': dc,
                'roll_needed': max(1, dc - total_bonus),
                'auto_success': total_bonus + 20 >= dc,
                'auto_fail': total_bonus + 1 < dc
            }
        
        roll_needed = max(1, dc - total_bonus)
        success_chance = max(0, min(95, (21 - roll_needed) * 5))
        
        return {
            'success': None,
            'total_bonus': total_bonus,
            'dc': dc,
            'roll_needed': roll_needed,
            'success_chance': success_chance,
            'auto_success': roll_needed <= 1,
            'auto_fail': roll_needed > 20
        }
    
    def _get_character_class_level(self, class_id: int) -> int:
        """Get level in a specific class by ID."""
        class_list = self.gff.get('ClassList', [])
        for class_info in class_list:
            if class_info.get('Class', -1) == class_id:
                return class_info.get('ClassLevel', 0)
        return 0
    
    def _on_attribute_changed(self, event: EventData):
        """Handle attribute changes that affect saves."""
        self._invalidate_saves_cache()
    
    def _on_class_changed(self, event: EventData):
        """Handle class changes that affect saves."""
        self._invalidate_saves_cache()

    def _on_feat_changed(self, event: EventData):
        """Handle feat changes that affect saves."""
        self._invalidate_saves_cache()
    
    def get_save_summary(self) -> Dict[str, Any]:
        """Get comprehensive save summary."""
        saves = self.calculate_saving_throws()
        return {
            'fortitude': saves['fortitude'],
            'reflex': saves['reflex'],
            'will': saves['will'],
            'conditions': self._get_save_conditions(),
            'immunities': self._get_immunities()
        }
    
    def _get_save_conditions(self) -> List[str]:
        """Get special save conditions (e.g., evasion) from feats."""
        conditions = []
        feat_manager = self.character_manager.get_manager('feat')
        if not feat_manager:
             return conditions 
             pass

        if feat_manager:
            if feat_manager.has_feat_by_name('Evasion'):
                conditions.append("Evasion (no damage on successful Reflex save)")
            if feat_manager.has_feat_by_name('ImprovedEvasion'):
                conditions.append("Improved Evasion (half damage on failed Reflex save)")
            if feat_manager.has_feat_by_name('SlipperyMind'):
                conditions.append("Slippery Mind (reroll failed Will saves vs mind-affecting)")
            if feat_manager.has_feat_by_name('DivineSpark'):
                conditions.append("Divine Spark (immune to energy drain)")
            
        return conditions
    
    def _get_immunities(self) -> List[str]:
        """Get save-related immunities from feats."""
        immunities = []
        feat_manager = self.character_manager.get_manager('feat')
        
        if feat_manager:
            if feat_manager.has_feat_by_name('DivineHealth'):
                immunities.append("Disease immunity")
            if feat_manager.has_feat_by_name('AuraOfCourage'):
                immunities.append("Fear immunity")
            if feat_manager.has_feat_by_name('PurityOfBody'):
                immunities.append("Disease immunity")
            if feat_manager.has_feat_by_name('DiamondBody'):
                immunities.append("Poison immunity")
            if feat_manager.has_feat_by_name('StillMind'):
                immunities.append("+2 vs Enchantment")
        
        return immunities
    
    def validate(self) -> Tuple[bool, List[str]]:
        """Validate save configuration."""
        errors = []
        saves = self.calculate_saving_throws()
        
        for save_type in ['fortitude', 'reflex', 'will']:
            if saves[save_type]['total'] < -10:
                errors.append(f"{save_type.capitalize()} save is unusually low")
        
        return len(errors) == 0, errors
    
    def calculate_fortitude_save(self) -> int:
        """Calculate fortitude save total."""
        return self.calculate_saving_throws()['fortitude']['total']
    
    def calculate_reflex_save(self) -> int:
        """Calculate reflex save total."""
        return self.calculate_saving_throws()['reflex']['total']
    
    def calculate_will_save(self) -> int:
        """Calculate will save total."""
        return self.calculate_saving_throws()['will']['total']
    
    def get_racial_saves(self, race_id: int) -> Dict[str, int]:
        """Get racial save bonuses from race data."""
        try:
            race_data = self.rules_service.get_by_id('racialtypes', race_id)
            if race_data:
                return field_mapper.get_racial_saves(race_data)
        except Exception as e:
            logger.error(f"Could not get racial saves for race {race_id}: {e}")
            raise # Strict error handling
        
        return {'fortitude': 0, 'reflex': 0, 'will': 0}

    def set_misc_save_bonus(self, save_type: str, value: int) -> Dict[str, Any]:
        """Set miscellaneous saving throw bonus."""
        save_field_map = {
            'fortitude': 'fortbonus',
            'reflex': 'refbonus', 
            'will': 'willbonus'
        }
        
        if save_type not in save_field_map:
            raise ValueError(f"Invalid save type: {save_type}")
        
        gff_field = save_field_map[save_type]
        old_value = self.gff.get(gff_field, 0)
        value = max(-35, min(255, int(value)))

        self.gff.set(gff_field, value)
        self._invalidate_saves_cache()

        new_saves = {
            'fortitude': self.calculate_fortitude_save(),
            'reflex': self.calculate_reflex_save(),
            'will': self.calculate_will_save()
        }
        
        self.emit(EventData(
            event_type=EventType.ATTRIBUTE_CHANGED,
            source_manager='save_manager',
            timestamp=0
        ))
        
        return {
            'save_type': save_type,
            'gff_field': gff_field,
            'old_value': old_value,
            'new_value': value,
            'new_saves': new_saves
        }