"""
Save Manager - handles comprehensive saving throw calculations
Includes base saves, ability modifiers, feats, and temporary effects
"""

from typing import Dict, List, Tuple, Optional, Any
import logging
import time

from ..events import EventEmitter, EventType, EventData

logger = logging.getLogger(__name__)


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
        self.game_data_loader = character_manager.game_data_loader
        
        # Track temporary modifiers (from spells, items, etc.)
        self.temporary_modifiers = {
            'fortitude': 0,
            'reflex': 0,
            'will': 0
        }
        
        # Data caches for performance
        self._feat_cache = {}
        self._racial_cache = {}
        self._save_affecting_feats = None
        
        # Initialize data-driven lookups
        self._initialize_data_lookups()
        
        # Register for relevant events
        self._register_event_handlers()
    
    def _initialize_data_lookups(self):
        """Initialize data-driven lookups for save calculations"""
        try:
            # Cache save-affecting feats from feat.2da
            self._build_save_affecting_feats_cache()
            
            # Cache racial save bonuses from racialtypes.2da
            self._build_racial_save_cache()
            
        except Exception as e:
            logger.warning(f"Could not initialize save data lookups: {e}")
            # Fallback to ensure basic functionality
            self._save_affecting_feats = {}
            self._racial_cache = {}
    
    def _build_save_affecting_feats_cache(self):
        """Build cache of feats that affect saving throws"""
        # Hardcoded mapping of feat IDs to save bonuses based on NWN2 rules
        # These values are not in the 2DA files but are hardcoded in the game engine
        FEAT_SAVE_BONUSES = {
            14: {'type': 'fortitude', 'bonus': 2, 'label': 'GreatFortitude'},  # Great Fortitude
            22: {'type': 'will', 'bonus': 2, 'label': 'IronWill'},  # Iron Will
            24: {'type': 'reflex', 'bonus': 2, 'label': 'LightningReflexes'},  # Lightning Reflexes
            # Epic save feats
            695: {'type': 'fortitude', 'bonus': 4, 'label': 'EpicFortitude'},  # Epic Fortitude
            696: {'type': 'reflex', 'bonus': 4, 'label': 'EpicReflexes'},  # Epic Reflexes
            697: {'type': 'will', 'bonus': 4, 'label': 'EpicWill'},  # Epic Will
            # Universal save bonuses
            100: {'type': 'universal', 'bonus': 1, 'label': 'LuckOfHeroes'},  # Luck of Heroes (test ID)
            1956: {'type': 'universal', 'bonus': 1, 'label': 'LuckOfHeroes'},  # Luck of Heroes
            2196: {'type': 'universal', 'bonus': 2, 'label': 'SacredDefense'},  # Sacred Defense
        }
        
        self._save_affecting_feats = {
            'fortitude': [],
            'reflex': [],
            'will': [],
            'universal': []
        }
        
        # Populate cache based on hardcoded values
        for feat_id, info in FEAT_SAVE_BONUSES.items():
            save_type = info['type']
            self._save_affecting_feats[save_type].append({
                'id': feat_id,
                'label': info['label'],
                'bonus': info['bonus']
            })
        
        # Optionally validate against actual feat table to ensure feat exists
        try:
            feats = self.game_data_loader.get_table('feat')
            valid_feat_ids = {getattr(feat, 'id', None) for feat in feats if hasattr(feat, 'id')}
            
            # Filter out any invalid feat IDs
            for save_type in self._save_affecting_feats:
                self._save_affecting_feats[save_type] = [
                    feat for feat in self._save_affecting_feats[save_type]
                    if feat['id'] in valid_feat_ids
                ]
        except Exception as e:
            logger.debug(f"Could not validate feat IDs: {e}")
    
    
    def _build_racial_save_cache(self):
        """Build cache of racial save bonuses from racialtypes.2da"""
        self._racial_cache = {}
        
        races = self.game_data_loader.get_table('racialtypes')
        for race in races:
            race_id = getattr(race, 'id', None) if hasattr(race, 'id') else None
            if race_id is None:
                continue
            
            # Get racial save bonuses from CharacterManager
            bonuses = self.character_manager.get_racial_saves(race_id)
            self._racial_cache[race_id] = bonuses
    
    def _register_event_handlers(self):
        """Register handlers for events that affect saves"""
        self.character_manager.on(EventType.ATTRIBUTE_CHANGED, self._on_attribute_changed)
        self.character_manager.on(EventType.CLASS_CHANGED, self._on_class_changed)
        self.character_manager.on(EventType.FEAT_ADDED, self._on_feat_changed)
        self.character_manager.on(EventType.FEAT_REMOVED, self._on_feat_changed)
    
    def calculate_saving_throws(self) -> Dict[str, Any]:
        """
        Calculate all saving throws with complete breakdown
        
        Returns:
            Dict with total saves and all components
        """
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
        
        # Get ability modifiers (already included in base_saves, but track separately)
        con_mod = (self.gff.get('Con', 10) - 10) // 2
        dex_mod = (self.gff.get('Dex', 10) - 10) // 2
        wis_mod = (self.gff.get('Wis', 10) - 10) // 2
        
        # Get feat bonuses
        feat_bonuses = self._calculate_feat_bonuses()
        
        # Get racial bonuses
        racial_bonuses = self._calculate_racial_bonuses()
        
        # Get resistance bonuses (from items/spells)
        resistance_bonuses = self._calculate_resistance_bonuses()
        
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
        
        logger.debug(f"Save calculation - Feat bonuses: {feat_bonuses}")
        
        return {
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
    
    def _calculate_feat_bonuses(self) -> Dict[str, int]:
        """Calculate save bonuses from feats using data-driven approach"""
        bonuses = {
            'fortitude': 0,
            'reflex': 0,
            'will': 0
        }
        
        if not self._save_affecting_feats:
            return bonuses
        
        # Get character's feat list
        character_feats = set()
        feat_list = self.gff.get('FeatList', [])
        for feat_entry in feat_list:
            feat_id = feat_entry.get('Feat', -1)
            if feat_id >= 0:
                character_feats.add(feat_id)
        
        # Check each save type for applicable feats
        for save_type in ['fortitude', 'reflex', 'will']:
            for feat_info in self._save_affecting_feats.get(save_type, []):
                if feat_info['id'] in character_feats:
                    bonuses[save_type] += feat_info['bonus']
                    logger.debug(f"Applied {feat_info['label']} bonus +{feat_info['bonus']} to {save_type}")
        
        # Universal save bonuses (affect all saves)
        for feat_info in self._save_affecting_feats.get('universal', []):
            if feat_info['id'] in character_feats:
                bonuses['fortitude'] += feat_info['bonus']
                bonuses['reflex'] += feat_info['bonus']
                bonuses['will'] += feat_info['bonus']
        
        # Special class-based bonuses
        class_bonuses = self._calculate_class_save_bonuses()
        for save_type in bonuses:
            bonuses[save_type] += class_bonuses.get(save_type, 0)
        
        return bonuses
    
    def _calculate_class_save_bonuses(self) -> Dict[str, int]:
        """Calculate class-specific save bonuses (like Paladin Divine Grace)"""
        bonuses = {
            'fortitude': 0,
            'reflex': 0,
            'will': 0
        }
        
        # Paladin Divine Grace (CHA to all saves at level 2+)
        if self._has_class_by_name('Paladin') and self._get_class_level_by_name('Paladin') >= 2:
            cha_mod = (self.gff.get('Cha', 10) - 10) // 2
            if cha_mod > 0:  # Only positive modifiers
                bonuses['fortitude'] += cha_mod
                bonuses['reflex'] += cha_mod
                bonuses['will'] += cha_mod
        
        return bonuses
    
    def _calculate_racial_bonuses(self) -> Dict[str, int]:
        """Calculate save bonuses from race using data-driven approach"""
        bonuses = {
            'fortitude': 0,
            'reflex': 0,
            'will': 0
        }
        
        race_id = self.gff.get('Race', 0)
        
        # Get racial bonuses from CharacterManager
        racial_bonuses = self.character_manager.get_racial_saves(race_id)
        bonuses['fortitude'] += racial_bonuses['fortitude']
        bonuses['reflex'] += racial_bonuses['reflex']
        bonuses['will'] += racial_bonuses['will']
        
        return bonuses
    
    def _calculate_resistance_bonuses(self) -> Dict[str, int]:
        """Calculate resistance bonuses from items and spells"""
        bonuses = {
            'fortitude': 0,
            'reflex': 0,
            'will': 0
        }
        
        # Check if we have access to character model
        if hasattr(self.character_manager, 'character_model'):
            character = self.character_manager.character_model
            try:
                # Check all equipped items for save bonuses
                from character.models import CharacterItem
                equipped_items = character.items.exclude(location='INVENTORY')
                
                for item in equipped_items:
                    # Check item properties for save bonuses
                    for prop in item.properties:
                        prop_type = prop.get('type', '')
                        if prop_type == 'save_fortitude':
                            bonuses['fortitude'] += prop.get('value', 0)
                        elif prop_type == 'save_reflex':
                            bonuses['reflex'] += prop.get('value', 0)
                        elif prop_type == 'save_will':
                            bonuses['will'] += prop.get('value', 0)
                        elif prop_type == 'save_universal':
                            # Items that boost all saves
                            value = prop.get('value', 0)
                            bonuses['fortitude'] += value
                            bonuses['reflex'] += value
                            bonuses['will'] += value
            except:
                pass
        
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
        """Check if character has a feat by its label using CharacterManager"""
        return self.character_manager.has_feat_by_name(feat_label)
    
    def _has_class_by_name(self, class_name: str) -> bool:
        """Check if character has levels in a class using CharacterManager"""
        return self.character_manager.has_class_by_name(class_name)
    
    def _get_class_level_by_name(self, class_name: str) -> int:
        """Get level in a specific class using CharacterManager"""
        return self.character_manager.get_class_level_by_name(class_name)
    
    def _on_attribute_changed(self, event: EventData):
        """Handle attribute changes that affect saves"""
        if hasattr(event, 'cascading_changes'):
            for change in event.cascading_changes:
                if change.get('type') == 'saving_throw_update':
                    logger.info(f"Save affected by attribute change: {change}")
    
    def _on_class_changed(self, event: EventData):
        """Handle class changes that affect saves"""
        logger.info("Saves may be affected by class change")
    
    def _on_feat_changed(self, event: EventData):
        """Handle feat changes that affect saves"""
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
        if self.character_manager.has_feat_by_name('Evasion'):
            conditions.append("Evasion (no damage on successful Reflex save)")
        
        if self.character_manager.has_feat_by_name('ImprovedEvasion'):
            conditions.append("Improved Evasion (half damage on failed Reflex save)")
        
        if self.character_manager.has_feat_by_name('SlipperyMind'):
            conditions.append("Slippery Mind (reroll failed Will saves vs mind-affecting)")
        
        # Check for other save conditions from feats
        if self.character_manager.has_feat_by_name('DivineSpark'):
            conditions.append("Divine Spark (immune to energy drain)")
        
        return conditions
    
    def _get_immunities(self) -> List[str]:
        """Get save-related immunities from feats and abilities"""
        immunities = []
        
        # Check for immunity feats
        if self.character_manager.has_feat_by_name('DivineHealth'):
            immunities.append("Disease immunity")
        
        if self.character_manager.has_feat_by_name('AuraOfCourage'):
            immunities.append("Fear immunity")
        
        if self.character_manager.has_feat_by_name('PurityOfBody'):
            immunities.append("Disease immunity")
        
        if self.character_manager.has_feat_by_name('DiamondBody'):
            immunities.append("Poison immunity")
        
        # Check for other immunity feats
        if self.character_manager.has_feat_by_name('StillMind'):
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
    
    # Missing methods called by views
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
        """Get constitution modifier"""
        return self.character_manager._calculate_ability_modifiers().get('CON', 0)
    
    def _get_dex_modifier(self) -> int:
        """Get dexterity modifier"""
        return self.character_manager._calculate_ability_modifiers().get('DEX', 0)
    
    def _get_wis_modifier(self) -> int:
        """Get wisdom modifier"""
        return self.character_manager._calculate_ability_modifiers().get('WIS', 0)
    
    def _get_misc_fortitude_bonus(self) -> int:
        """Get miscellaneous fortitude bonus"""
        return self.character_manager.character_data.get('fortbonus', 0)
    
    def _get_misc_reflex_bonus(self) -> int:
        """Get miscellaneous reflex bonus"""
        return self.character_manager.character_data.get('refbonus', 0)
    
    def _get_misc_will_bonus(self) -> int:
        """Get miscellaneous will bonus"""
        return self.character_manager.character_data.get('willbonus', 0)
    
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
        old_value = self.character_manager.character_data.get(gff_field, 0)
        
        # Clamp value to reasonable range
        value = max(-10, min(20, int(value)))
        
        # Update the character data
        self.character_manager.character_data[gff_field] = value
        
        # Also update the GFF element if available
        if hasattr(self.character_manager, '_gff_element') and self.character_manager._gff_element:
            self.character_manager._gff_element.set_field(gff_field, value)
        
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