"""
Save Manager - handles comprehensive saving throw calculations
Includes base saves, ability modifiers, feats, and temporary effects
"""

from typing import Dict, List, Tuple, Any
from loguru import logger

from ..events import EventEmitter, EventType, EventData
from gamedata.dynamic_loader.field_mapping_utility import field_mapper  # type: ignore

# Using global loguru logger


class SaveManager(EventEmitter):
    """Manages saving throw calculations including all modifiers"""
    
    def __init__(self, character_manager):
        """
        Initialize the SaveManager
        
        Args:
            character_manager: Reference to parent CharacterManager
        """
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.rules_service = character_manager.rules_service
        
        # Track temporary modifiers (from spells, items, etc.)
        self.temporary_modifiers = {
            'fortitude': 0,
            'reflex': 0,
            'will': 0
        }
        
        # Data caches for performance
        self._racial_cache = {}

        # Request-level save calculation cache
        self._saves_cache = None

        # Initialize data-driven lookups
        self._initialize_data_lookups()

        # Register for relevant events
        self._register_event_handlers()
    
    def _initialize_data_lookups(self):
        """Initialize data-driven lookups for save calculations"""
        try:
            # Cache racial save bonuses from racialtypes.2da
            self._build_racial_save_cache()

        except Exception as e:
            logger.warning(f"Could not initialize save data lookups: {e}")
            # Fallback to ensure basic functionality
            self._racial_cache = {}
    
    def _build_racial_save_cache(self):
        """Build cache of racial save bonuses from racialtypes.2da"""
        self._racial_cache = {}
        
        races = self.rules_service.get_table('racialtypes')
        for race in races:
            race_id = getattr(race, 'id', None) if hasattr(race, 'id') else None
            if race_id is None:
                continue
            
            # Get racial save bonuses using local method
            bonuses = self.get_racial_saves(race_id)
            self._racial_cache[race_id] = bonuses
    
    def _register_event_handlers(self):
        """Register handlers for events that affect saves"""
        self.character_manager.on(EventType.ATTRIBUTE_CHANGED, self._on_attribute_changed)
        self.character_manager.on(EventType.CLASS_CHANGED, self._on_class_changed)
        self.character_manager.on(EventType.FEAT_ADDED, self._on_feat_changed)
        self.character_manager.on(EventType.FEAT_REMOVED, self._on_feat_changed)
    
    def calculate_saving_throws(self) -> Dict[str, Any]:
        """
        Calculate all saving throws with complete breakdown and request-level caching

        Returns:
            Dict with total saves and all components
        """
        if self._saves_cache is not None:
            return self._saves_cache.copy()

        # Get base saves from ClassManager
        class_manager = self.character_manager.get_manager('class')
        if class_manager:
            base_saves = class_manager.calculate_total_saves()
        else:
            # Fallback if no class manager
            base_saves = {
                'fortitude': self.gff.get('FortSave', 0),
                'reflex': self.gff.get('RefSave', 0),
                'will': self.gff.get('WillSave', 0),
                'base_fortitude': 0,
                'base_reflex': 0,
                'base_will': 0
            }
        
        # Get ability modifiers from AbilityManager (includes base + racial + equipment + level-ups)
        ability_manager = self.character_manager.get_manager('ability')
        if ability_manager:
            total_modifiers = ability_manager.get_total_modifiers()
            con_mod = total_modifiers.get('Con', 0)
            dex_mod = total_modifiers.get('Dex', 0)
            wis_mod = total_modifiers.get('Wis', 0)
        else:
            # Fallback: calculate from base scores only
            con_mod = (self.gff.get('Con', 10) - 10) // 2
            dex_mod = (self.gff.get('Dex', 10) - 10) // 2
            wis_mod = (self.gff.get('Wis', 10) - 10) // 2

        # Get equipment save bonuses separately (these are direct save bonuses, not ability bonuses)
        inventory_manager = self.character_manager.get_manager('inventory')
        equipment_bonuses = inventory_manager.get_equipment_bonuses() if inventory_manager else {
            'ac': {}, 'attributes': {}, 'saves': {}, 'skills': {}, 'combat': {}
        }

        # Get feat bonuses
        feat_bonuses = self._calculate_feat_bonuses()

        # Get racial bonuses
        racial_bonuses = self._calculate_racial_bonuses()

        # Get resistance bonuses (from items/spells) and add equipment save bonuses
        resistance_bonuses = self._calculate_resistance_bonuses()
        logger.info(f"Equipment save bonuses: {equipment_bonuses['saves']}")
        resistance_bonuses['fortitude'] += equipment_bonuses['saves'].get('fortitude', 0)
        resistance_bonuses['reflex'] += equipment_bonuses['saves'].get('reflex', 0)
        resistance_bonuses['will'] += equipment_bonuses['saves'].get('will', 0)
        
        # Calculate totals (base does NOT include ability mods)
        fort_total = (base_saves['base_fortitude'] + con_mod +
                     feat_bonuses['fortitude'] + racial_bonuses['fortitude'] +
                     resistance_bonuses['fortitude'] + self.temporary_modifiers['fortitude'])

        ref_total = (base_saves['base_reflex'] + dex_mod +
                    feat_bonuses['reflex'] + racial_bonuses['reflex'] +
                    resistance_bonuses['reflex'] + self.temporary_modifiers['reflex'])

        will_total = (base_saves['base_will'] + wis_mod +
                     feat_bonuses['will'] + racial_bonuses['will'] +
                     resistance_bonuses['will'] + self.temporary_modifiers['will'])

        # Debug: Check GFF stored values vs calculated
        gff_fort = self.gff.get('FortSave', 0)
        gff_ref = self.gff.get('RefSave', 0)
        gff_will = self.gff.get('WillSave', 0)
        logger.info(f"GFF saves: Fort={gff_fort}, Ref={gff_ref}, Will={gff_will}")
        logger.info(f"Calculated saves: Fort={fort_total}, Ref={ref_total}, Will={will_total}")
        logger.info(f"Base saves: {base_saves}")
        logger.info(f"Ability mods: CON={con_mod}, DEX={dex_mod}, WIS={wis_mod}")
        logger.info(f"Feat bonuses: {feat_bonuses}")
        logger.info(f"Racial bonuses: {racial_bonuses}")
        logger.info(f"Resistance bonuses: {resistance_bonuses}")

        result = {
            'fortitude': {
                'total': fort_total,
                'base': base_saves['base_fortitude'],
                'ability': con_mod,
                'feat': feat_bonuses['fortitude'],
                'racial': racial_bonuses['fortitude'],
                'resistance': resistance_bonuses['fortitude'],
                'temporary': self.temporary_modifiers['fortitude'],
                'breakdown': self._format_breakdown('Fortitude', fort_total, 
                    base_saves['base_fortitude'], con_mod, 'CON',
                    feat_bonuses['fortitude'], racial_bonuses['fortitude'],
                    resistance_bonuses['fortitude'], self.temporary_modifiers['fortitude'])
            },
            'reflex': {
                'total': ref_total,
                'base': base_saves['base_reflex'],
                'ability': dex_mod,
                'feat': feat_bonuses['reflex'],
                'racial': racial_bonuses['reflex'],
                'resistance': resistance_bonuses['reflex'],
                'temporary': self.temporary_modifiers['reflex'],
                'breakdown': self._format_breakdown('Reflex', ref_total,
                    base_saves['base_reflex'], dex_mod, 'DEX',
                    feat_bonuses['reflex'], racial_bonuses['reflex'],
                    resistance_bonuses['reflex'], self.temporary_modifiers['reflex'])
            },
            'will': {
                'total': will_total,
                'base': base_saves['base_will'],
                'ability': wis_mod,
                'feat': feat_bonuses['will'],
                'racial': racial_bonuses['will'],
                'resistance': resistance_bonuses['will'],
                'temporary': self.temporary_modifiers['will'],
                'breakdown': self._format_breakdown('Will', will_total,
                    base_saves['base_will'], wis_mod, 'WIS',
                    feat_bonuses['will'], racial_bonuses['will'],
                    resistance_bonuses['will'], self.temporary_modifiers['will'])
            }
        }

        self._saves_cache = result.copy()
        return result

    def _invalidate_saves_cache(self):
        """Invalidate the saves cache when data changes"""
        self._saves_cache = None

    def _calculate_feat_bonuses(self) -> Dict[str, int]:
        """Calculate save bonuses from feats by delegating to FeatManager"""
        feat_manager = self.character_manager.get_manager('feat')
        if not feat_manager:
            logger.warning("FeatManager not available for feat bonus calculation")
            return {'fortitude': 0, 'reflex': 0, 'will': 0}

        # Get feat save bonuses from FeatManager (centralized logic)
        bonuses = feat_manager.get_save_bonuses()

        # Add special class-based bonuses (e.g., Paladin Divine Grace)
        class_bonuses = self._calculate_class_save_bonuses()
        for save_type in bonuses:
            bonuses[save_type] += class_bonuses.get(save_type, 0)

        logger.info(f"Total feat bonuses: {bonuses}")
        return bonuses
    
    def _calculate_class_save_bonuses(self) -> Dict[str, int]:
        """Calculate class-specific save bonuses using ClassManager and data-driven approach"""
        bonuses = {
            'fortitude': 0,
            'reflex': 0,
            'will': 0
        }
        
        # Use ClassManager to check for classes and levels
        class_manager = self.character_manager.get_manager('class')
        if not class_manager:
            return bonuses
        
        # Check for Paladin Divine Grace using data-driven approach
        # NOTE: Divine Grace (level 2+ Paladin gets CHA to all saves) is a hardcoded
        # game engine mechanic. The level requirement and bonus type cannot be made
        # fully data-driven as this is core D&D 3.5 Paladin class feature logic.
        try:
            # Get all classes and check for Paladin-like class with Divine Grace
            classes_table = self.rules_service.get_table('classes')
            if classes_table:
                for class_data in classes_table:
                    class_label = field_mapper.get_field_value(class_data, 'label', '')
                    class_id = field_mapper.get_field_value(class_data, 'id', -1)
                    
                    # Check if this is a Paladin class and character has it at level 2+
                    if 'paladin' in class_label.lower() and class_id >= 0:
                        # Check character's class list for this class ID
                        class_level = self._get_character_class_level(class_id)
                        if class_level >= 2:
                            # Use AttributeManager for CHA modifier
                            attr_manager = self.character_manager.get_manager('ability') or self.character_manager.get_manager('attribute')
                            if attr_manager:
                                all_modifiers = attr_manager.get_all_modifiers()
                                cha_mod = all_modifiers.get('CHA', 0)
                            else:
                                # Fallback
                                cha_mod = (self.gff.get('Cha', 10) - 10) // 2
                            
                            if cha_mod > 0:  # Only positive modifiers
                                bonuses['fortitude'] += cha_mod
                                bonuses['reflex'] += cha_mod
                                bonuses['will'] += cha_mod
                            break
                            
        except Exception as e:
            logger.debug(f"Could not calculate class save bonuses: {e}")
        
        return bonuses
    
    def _calculate_racial_bonuses(self) -> Dict[str, int]:
        """Calculate save bonuses from race using data-driven approach"""
        bonuses = {
            'fortitude': 0,
            'reflex': 0,
            'will': 0
        }
        
        race_id = self.gff.get('Race', 0)
        
        # Get racial bonuses using FieldMappingUtility and data-driven approach
        try:
            race_data = self.rules_service.get_by_id('racialtypes', race_id)
            if race_data:
                racial_bonuses = field_mapper.get_racial_saves(race_data)
                bonuses['fortitude'] += racial_bonuses['fortitude']
                bonuses['reflex'] += racial_bonuses['reflex']
                bonuses['will'] += racial_bonuses['will']
        except Exception as e:
            logger.debug(f"Could not get racial save bonuses for race {race_id}: {e}")
        
        return bonuses
    
    def _calculate_resistance_bonuses(self) -> Dict[str, int]:
        """
        Calculate resistance bonuses from spells/effects
        Note: Equipment save bonuses are handled separately via InventoryManager
        """
        bonuses = {
            'fortitude': 0,
            'reflex': 0,
            'will': 0
        }

        # Resistance bonuses from spells/effects would go here
        # Currently we don't parse spell effects from the GFF

        return bonuses
    
    def add_temporary_modifier(self, save_type: str, modifier: int, duration: float = 0):
        """
        Add a temporary save modifier (from spells, etc.)
        
        Args:
            save_type: 'fortitude', 'reflex', or 'will'
            modifier: Bonus amount
            duration: Duration in seconds (0 = until removed)
        """
        if save_type in self.temporary_modifiers:
            self.temporary_modifiers[save_type] += modifier
            logger.info(f"Added {modifier:+d} temporary {save_type} save modifier")
    
    def remove_temporary_modifier(self, save_type: str, modifier: int):
        """Remove a temporary save modifier"""
        if save_type in self.temporary_modifiers:
            self.temporary_modifiers[save_type] -= modifier
            logger.info(f"Removed {modifier:+d} temporary {save_type} save modifier")
    
    def clear_temporary_modifiers(self):
        """Clear all temporary save modifiers"""
        self.temporary_modifiers = {
            'fortitude': 0,
            'reflex': 0,
            'will': 0
        }
        logger.info("Cleared all temporary save modifiers")
    
    def _format_breakdown(self, save_name: str, total: int, base: int, 
                         ability: int, ability_name: str, feat: int, 
                         racial: int, resistance: int, temporary: int) -> str:
        """Format a save breakdown string"""
        parts = [f"{save_name} +{total} ="]
        parts.append(f"base {base:+d}")
        parts.append(f"{ability_name} {ability:+d}")
        
        if feat != 0:
            parts.append(f"feats {feat:+d}")
        if racial != 0:
            parts.append(f"racial {racial:+d}")
        if resistance != 0:
            parts.append(f"resistance {resistance:+d}")
        if temporary != 0:
            parts.append(f"temporary {temporary:+d}")
        
        return " + ".join(parts)
    
    def check_save(self, save_type: str, dc: int, 
                   modifier: int = 0, take_20: bool = False) -> Dict[str, Any]:
        """
        Check if a save would succeed against a DC
        
        Args:
            save_type: 'fortitude', 'reflex', or 'will'
            dc: Difficulty class to beat
            modifier: Additional modifier for this specific save
            take_20: Whether taking 20 (automatic success if possible)
            
        Returns:
            Dict with success status and roll needed
        """
        saves = self.calculate_saving_throws()
        
        if save_type not in saves:
            raise ValueError(f"Invalid save type: {save_type}")
        
        total_bonus = saves[save_type]['total'] + modifier
        
        if take_20:
            total_roll = 20 + total_bonus
            success = total_roll >= dc
            return {
                'success': success,
                'total_bonus': total_bonus,
                'dc': dc,
                'roll_needed': max(1, dc - total_bonus),
                'auto_success': total_bonus + 20 >= dc,
                'auto_fail': total_bonus + 1 < dc
            }
        
        # Calculate probabilities
        roll_needed = max(1, dc - total_bonus)
        success_chance = max(0, min(95, (21 - roll_needed) * 5))  # 5% per number
        
        return {
            'success': None,  # Not rolled yet
            'total_bonus': total_bonus,
            'dc': dc,
            'roll_needed': roll_needed,
            'success_chance': success_chance,
            'auto_success': roll_needed <= 1,
            'auto_fail': roll_needed > 20
        }
    
    def _has_feat_by_name(self, feat_label: str) -> bool:
        """Check if character has a feat by its label using FeatManager"""
        feat_manager = self.character_manager.get_manager('feat')
        return feat_manager.has_feat_by_name(feat_label) if feat_manager else False
    
    def _has_feat_by_id(self, feat_id: int) -> bool:
        """Check if character has a feat by its ID using FeatManager"""
        feat_manager = self.character_manager.get_manager('feat')
        return feat_manager.has_feat(feat_id) if feat_manager else False
    
    def _has_class_by_name(self, class_name: str) -> bool:
        """Check if character has levels in a class using ClassManager"""
        class_manager = self.character_manager.get_manager('class')
        return class_manager.has_class_by_name(class_name) if class_manager else False
    
    def _get_class_level_by_name(self, class_name: str) -> int:
        """Get level in a specific class using ClassManager"""
        class_manager = self.character_manager.get_manager('class')
        return class_manager.get_class_level_by_name(class_name) if class_manager else 0
    
    def _get_character_class_level(self, class_id: int) -> int:
        """Get level in a specific class by ID by checking character's ClassList"""
        class_list = self.gff.get('ClassList', [])
        
        for class_info in class_list:
            if class_info.get('Class', -1) == class_id:
                return class_info.get('ClassLevel', 0)
        
        return 0
    
    def _has_character_class(self, class_id: int) -> bool:
        """Check if character has levels in a class by ID"""
        return self._get_character_class_level(class_id) > 0
    
    def _on_attribute_changed(self, event: EventData):
        """Handle attribute changes that affect saves"""
        if hasattr(event, 'cascading_changes'):
            for change in event.cascading_changes:
                if change.get('type') == 'saving_throw_update':
                    logger.info(f"Save affected by attribute change: {change}")
    
    def _on_class_changed(self, event: EventData):
        """Handle class changes that affect saves"""
        self._invalidate_saves_cache()
        logger.info("Saves may be affected by class change")

    def _on_feat_changed(self, event: EventData):
        """Handle feat changes that affect saves"""
        self._invalidate_saves_cache()
        logger.info("Saves may be affected by feat change")
    
    def get_save_summary(self) -> Dict[str, Any]:
        """Get comprehensive save summary"""
        saves = self.calculate_saving_throws()
        
        return {
            'fortitude': saves['fortitude'],
            'reflex': saves['reflex'],
            'will': saves['will'],
            'conditions': self._get_save_conditions(),
            'immunities': self._get_immunities()
        }
    
    def _get_save_conditions(self) -> List[str]:
        """Get special save conditions (e.g., evasion, mettle) from class abilities"""
        conditions = []
        
        # Check for specific save-related feats/abilities
        feat_manager = self.character_manager.get_manager('feat')
        if feat_manager and feat_manager.has_feat_by_name('Evasion'):
            conditions.append("Evasion (no damage on successful Reflex save)")
        
        if feat_manager and feat_manager.has_feat_by_name('ImprovedEvasion'):
            conditions.append("Improved Evasion (half damage on failed Reflex save)")
        
        if feat_manager and feat_manager.has_feat_by_name('SlipperyMind'):
            conditions.append("Slippery Mind (reroll failed Will saves vs mind-affecting)")
        
        # Check for other save conditions from feats
        if feat_manager and feat_manager.has_feat_by_name('DivineSpark'):
            conditions.append("Divine Spark (immune to energy drain)")
        
        return conditions
    
    def _get_immunities(self) -> List[str]:
        """Get save-related immunities from feats and abilities"""
        immunities = []
        
        # Check for immunity feats
        feat_manager = self.character_manager.get_manager('feat')
        if feat_manager and feat_manager.has_feat_by_name('DivineHealth'):
            immunities.append("Disease immunity")
        
        if feat_manager and feat_manager.has_feat_by_name('AuraOfCourage'):
            immunities.append("Fear immunity")
        
        if feat_manager and feat_manager.has_feat_by_name('PurityOfBody'):
            immunities.append("Disease immunity")
        
        if feat_manager and feat_manager.has_feat_by_name('DiamondBody'):
            immunities.append("Poison immunity")
        
        # Check for other immunity feats
        if feat_manager and feat_manager.has_feat_by_name('StillMind'):
            immunities.append("+2 vs Enchantment")
        
        return immunities
    
    def validate(self) -> Tuple[bool, List[str]]:
        """Validate save configuration"""
        errors = []
        
        saves = self.calculate_saving_throws()
        
        # Check for unreasonably low saves
        for save_type in ['fortitude', 'reflex', 'will']:
            if saves[save_type]['total'] < -10:
                errors.append(f"{save_type.capitalize()} save is unusually low")
        
        return len(errors) == 0, errors
    
    def calculate_fortitude_save(self) -> int:
        """Calculate fortitude save total"""
        saves = self.calculate_saving_throws()
        return saves['fortitude']['total']
    
    def calculate_reflex_save(self) -> int:
        """Calculate reflex save total"""
        saves = self.calculate_saving_throws()
        return saves['reflex']['total']
    
    def calculate_will_save(self) -> int:
        """Calculate will save total"""
        saves = self.calculate_saving_throws()
        return saves['will']['total']
    
    def _get_base_fortitude_save(self) -> int:
        """Get base fortitude save"""
        saves = self.calculate_saving_throws()
        return saves['fortitude']['base']
    
    def _get_base_reflex_save(self) -> int:
        """Get base reflex save"""
        saves = self.calculate_saving_throws()
        return saves['reflex']['base']
    
    def _get_base_will_save(self) -> int:
        """Get base will save"""
        saves = self.calculate_saving_throws()
        return saves['will']['base']
    
    def _get_con_modifier(self) -> int:
        """Get constitution modifier using AttributeManager"""
        attr_manager = self.character_manager.get_manager('ability') or self.character_manager.get_manager('attribute')
        if attr_manager:
            return attr_manager.get_all_modifiers().get('CON', 0)
        return (self.gff.get('Con', 10) - 10) // 2
    
    def _get_dex_modifier(self) -> int:
        """Get dexterity modifier using AttributeManager"""
        attr_manager = self.character_manager.get_manager('ability') or self.character_manager.get_manager('attribute')
        if attr_manager:
            return attr_manager.get_all_modifiers().get('DEX', 0)
        return (self.gff.get('Dex', 10) - 10) // 2
    
    def _get_wis_modifier(self) -> int:
        """Get wisdom modifier using AttributeManager"""
        attr_manager = self.character_manager.get_manager('ability') or self.character_manager.get_manager('attribute')
        if attr_manager:
            return attr_manager.get_all_modifiers().get('WIS', 0)
        return (self.gff.get('Wis', 10) - 10) // 2
    
    def _get_misc_fortitude_bonus(self) -> int:
        """Get miscellaneous fortitude bonus"""
        return self.gff.get('fortbonus', 0)
    
    def _get_misc_reflex_bonus(self) -> int:
        """Get miscellaneous reflex bonus"""
        return self.gff.get('refbonus', 0)
    
    def _get_misc_will_bonus(self) -> int:
        """Get miscellaneous will bonus"""
        return self.gff.get('willbonus', 0)
    
    def set_misc_save_bonus(self, save_type: str, value: int) -> Dict[str, Any]:
        """
        Set miscellaneous saving throw bonus
        
        Args:
            save_type: 'fortitude', 'reflex', or 'will'
            value: The bonus value to set
            
        Returns:
            Dict with old/new values and cascading effects
        """
        # Map save types to GFF field names
        save_field_map = {
            'fortitude': 'fortbonus',
            'reflex': 'refbonus', 
            'will': 'willbonus'
        }
        
        if save_type not in save_field_map:
            raise ValueError(f"Invalid save type: {save_type}. Must be one of {list(save_field_map.keys())}")
        
        gff_field = save_field_map[save_type]
        
        # Get old value from GFF wrapper (single source of truth)
        old_value = self.gff.get(gff_field, 0)
        
        # Clamp value to engine limits
        value = max(-35, min(255, int(value)))

        # Update GFF wrapper (single source of truth)
        self.gff.set(gff_field, value)

        # Invalidate cache
        self._invalidate_saves_cache()

        # Calculate new totals
        new_saves = {
            'fortitude': self.calculate_fortitude_save(),
            'reflex': self.calculate_reflex_save(),
            'will': self.calculate_will_save()
        }
        
        # Emit change event
        from ..events import EventType, EventData
        event_data = EventData(
            event_type=EventType.ATTRIBUTE_CHANGED,
            source_manager='save_manager',
            timestamp=0
        )
        self.emit(event_data)
        
        return {
            'save_type': save_type,
            'gff_field': gff_field,
            'old_value': old_value,
            'new_value': value,
            'new_saves': new_saves
        }
    
    def get_racial_saves(self, race_id: int) -> Dict[str, int]:
        """
        Get racial save bonuses from race data using field mapping utility
        
        Args:
            race_id: The race ID
            
        Returns:
            Dict with fortitude, reflex, will save bonuses
        """
        from gamedata.dynamic_loader.field_mapping_utility import field_mapper
        
        try:
            race_data = self.rules_service.get_by_id('racialtypes', race_id)
            if race_data:
                return field_mapper.get_racial_saves(race_data)
        except Exception as e:
            logger.warning(f"Could not get racial saves for race {race_id}: {e}")
        
        # Return default values if no race data or error
        return {'fortitude': 0, 'reflex': 0, 'will': 0}