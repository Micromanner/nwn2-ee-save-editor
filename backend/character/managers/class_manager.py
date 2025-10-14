"""
Class Manager - handles class changes, multiclassing, and level progression
Refactored from class_change_service.py to work with event system
"""

from typing import Dict, List, Tuple, Optional, Any
from loguru import logger
import time

from ..events import EventEmitter, EventType, ClassChangedEvent, LevelGainedEvent
from gamedata.dynamic_loader.field_mapping_utility import FieldMappingUtility

# Using global loguru logger


class ClassManager(EventEmitter):
    """Manages character class changes and progression"""
    
    def __init__(self, character_manager):
        """
        Initialize the ClassManager
        
        Args:
            character_manager: Reference to the parent CharacterManager
        """
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.rules_service = character_manager.rules_service
        self.field_mapper = FieldMappingUtility()
        
        # Cache for performance
        self._class_cache = {}
        self._bab_table_cache = {}
        self._save_table_cache = {}
    
    def change_class(self, new_class_id: int, preserve_level: bool = True, 
                    cheat_mode: bool = False) -> Dict[str, Any]:
        """
        Change character's primary class
        
        Args:
            new_class_id: The new class ID
            preserve_level: Keep the same total level
            cheat_mode: Bypass validation if True
            
        Returns:
            Dict with all changes made
        """
        logger.info(f"Changing class to {new_class_id}")
        
        # Start tracking changes
        changes = {
            'class_change': {
                'old_class': None,
                'new_class': new_class_id,
                'level': 0
            },
            'stats_updated': {},
            'feats_changed': {'removed': [], 'added': []},
            'spells_changed': {},
            'skills_reset': False,
            'custom_content_preserved': []
        }
        
        # Basic validation for save file integrity only
        if not cheat_mode:
            new_class = self.rules_service.get_by_id('classes', new_class_id)
            if not new_class:
                raise ValueError(f"Invalid class ID: {new_class_id}")
        
        # Get class info
        new_class = self.rules_service.get_by_id('classes', new_class_id)
        if not new_class:
            raise ValueError(f"Invalid class ID: {new_class_id}")
        
        # Get current state
        class_list = self.gff.get('ClassList', [])
        old_class_id = class_list[0].get('Class', 0) if class_list else None
        total_level = sum(c.get('ClassLevel', 0) for c in class_list) or 1
        
        changes['class_change']['old_class'] = old_class_id
        changes['class_change']['level'] = total_level
        
        # Begin transaction if not already in one
        transaction_started = False
        if not self.character_manager._current_transaction:
            self.character_manager.begin_transaction()
            transaction_started = True
        
        try:
            # 1. Update class list
            self._update_class_list(new_class_id, total_level)
            
            # 2. Update derived stats
            stat_changes = self._update_class_stats(new_class, total_level)
            changes['stats_updated'] = stat_changes
            
            # 3. Emit class changed event
            event = ClassChangedEvent(
                event_type=EventType.CLASS_CHANGED,  # Will be overridden by __post_init__
                source_manager='class',
                timestamp=time.time(),
                old_class_id=old_class_id,
                new_class_id=new_class_id,
                level=total_level,
                preserve_feats=self._get_preserved_feats()
            )
            self.character_manager.emit(event)
            
            # 4. Handle skills (will be updated by SkillManager via event)
            changes['skills_reset'] = True
            
            # Commit transaction if we started it
            if transaction_started:
                self.character_manager.commit_transaction()
            
        except Exception as e:
            # Rollback on error if we started the transaction
            if transaction_started:
                self.character_manager.rollback_transaction()
            logger.error(f"Error during class change: {e}")
            raise
        
        return changes
    
    def add_class_level(self, class_id: int, cheat_mode: bool = False) -> Dict[str, Any]:
        """
        Add a level in a specific class (multiclassing or leveling up)
        
        Args:
            class_id: Class to add level in
            cheat_mode: Bypass validation
            
        Returns:
            Dict with changes made
        """
        logger.info(f"Adding level in class {class_id}")
        
        # Basic validation for save file integrity only
        if not cheat_mode:
            new_class = self.rules_service.get_by_id('classes', class_id)
            if not new_class:
                raise ValueError(f"Invalid class ID: {class_id}")
        
        # Get class info
        new_class = self.rules_service.get_by_id('classes', class_id)
        if not new_class:
            raise ValueError(f"Invalid class ID: {class_id}")
        
        # Check for prestige class level limits
        if not cheat_mode:
            self._validate_class_level_limits(class_id, new_class)
        
        # Update class list
        class_list = self.gff.get('ClassList', [])
        total_level = sum(c.get('ClassLevel', 0) for c in class_list) + 1
        
        # Find existing class or add new
        class_found = False
        for class_entry in class_list:
            if class_entry.get('Class') == class_id:
                class_entry['ClassLevel'] += 1
                class_found = True
                break
        
        if not class_found:
            class_list.append({
                'Class': class_id,
                'ClassLevel': 1
            })
        
        self.gff.set('ClassList', class_list)
        
        # Emit level gained event
        event = LevelGainedEvent(
            event_type=EventType.LEVEL_GAINED,  # Will be overridden by __post_init__
            source_manager='class',
            timestamp=time.time(),
            class_id=class_id,
            new_level=total_level,
            total_level=total_level
        )
        self.emit(event)
        
        return {
            'class_id': class_id,
            'new_total_level': total_level,
            'multiclass': not class_found
        }
    
    def adjust_class_level(self, class_id: int, level_change: int, cheat_mode: bool = False) -> Dict[str, Any]:
        """
        Adjust levels in a specific class (add or remove levels)
        
        Args:
            class_id: Class to adjust level in
            level_change: Number of levels to add (+) or remove (-)
            cheat_mode: Bypass validation
            
        Returns:
            Dict with changes made
        """
        if level_change == 0:
            return {}
        
        if level_change > 0:
            # Add levels one by one
            changes = {}
            for _ in range(level_change):
                changes = self.add_class_level(class_id, cheat_mode)
            return changes
        else:
            # Remove levels - find the class and reduce level
            class_list = self.gff.get('ClassList', [])
            
            # Begin transaction if not already in one
            transaction_started = False
            if not self.character_manager._current_transaction:
                self.character_manager.begin_transaction()
                transaction_started = True
            
            try:
                for class_entry in class_list:
                    if class_entry.get('Class') == class_id:
                        current_level = class_entry.get('ClassLevel', 0)
                        new_level = max(0, current_level + level_change)  # level_change is negative
                        
                        if new_level == 0:
                            # Remove class entirely
                            if transaction_started:
                                self.character_manager.rollback_transaction()
                            return self.remove_class(class_id)
                        else:
                            # Just reduce level
                            class_entry['ClassLevel'] = new_level
                            self.gff.set('ClassList', class_list)
                            
                            # Recalculate stats
                            new_class = self.rules_service.get_by_id('classes', class_id)
                            total_level = sum(c.get('ClassLevel', 0) for c in class_list)
                            changes = self._update_class_stats(new_class, total_level)
                            
                            # Emit class changed event to notify other managers
                            event = ClassChangedEvent(
                                event_type=EventType.CLASS_CHANGED,
                                source_manager='class',
                                timestamp=time.time(),
                                old_class_id=class_id,
                                new_class_id=class_id,  # Same class, different level
                                level=total_level,
                                preserve_feats=self._get_preserved_feats()
                            )
                            self.character_manager.emit(event)
                            
                            # Commit transaction if we started it
                            if transaction_started:
                                self.character_manager.commit_transaction()
                            
                            return {
                                'class_id': class_id,
                                'level_change': level_change,
                                'new_class_level': new_level,
                                'new_total_level': total_level,
                                **changes
                            }
                
                # Rollback if we started transaction but didn't find class
                if transaction_started:
                    self.character_manager.rollback_transaction()
                    
            except Exception as e:
                # Rollback on error if we started the transaction
                if transaction_started:
                    self.character_manager.rollback_transaction()
                logger.error(f"Error during level adjustment: {e}")
                raise
            
            # Class not found
            raise ValueError(f"Character does not have class {class_id}")
    
    def _update_class_list(self, new_class_id: int, total_level: int):
        """
        Update the character's class list for class change
        WARNING: This method is only for single-class characters!
        For multiclass characters, use change_specific_class() instead.
        """
        class_list = self.gff.get('ClassList', [])
        
        # If multiclass, this method should not be used
        if len(class_list) > 1:
            raise ValueError("Cannot use _update_class_list for multiclass characters. Use change_specific_class() instead.")
        
        # Check if new class has level limits and cap the level if needed
        new_class = self.rules_service.get_by_id('classes', new_class_id)
        if new_class:
            max_level_raw = self.field_mapper.get_field_value(new_class, 'max_level', '0')
            try:
                max_level = int(max_level_raw) if max_level_raw not in ['****', ''] else 0
            except (ValueError, TypeError):
                max_level = 0
            
            # If it's a prestige class (has max level), cap the level
            if max_level > 0 and total_level > max_level:
                logger.info(f"Capping level from {total_level} to {max_level} for prestige class {new_class_id}")
                total_level = max_level
        
        # For single class characters, replace the class entirely
        self.gff.set('ClassList', [{
            'Class': new_class_id,
            'ClassLevel': total_level
        }])
        
        # Update primary class field
        self.gff.set('Class', new_class_id)
    
    def change_specific_class(self, old_class_id: int, new_class_id: int, preserve_level: bool = True) -> Dict[str, Any]:
        """
        Change a specific class in a multiclass character without affecting other classes
        
        Args:
            old_class_id: The class ID to replace
            new_class_id: The new class ID
            preserve_level: Keep the same level in that class
            
        Returns:
            Dict with change details
        """
        logger.info(f"Changing specific class from {old_class_id} to {new_class_id}")
        
        # Validate new class exists
        new_class = self.rules_service.get_by_id('classes', new_class_id)
        if not new_class:
            raise ValueError(f"Invalid class ID: {new_class_id}")
        
        class_list = self.gff.get('ClassList', [])
        total_level = sum(c.get('ClassLevel', 0) for c in class_list)
        
        # Find the class to change
        class_found = False
        class_level = 0
        for class_entry in class_list:
            if class_entry.get('Class') == old_class_id:
                class_level = class_entry.get('ClassLevel', 0)
                
                # Check if new class has level limits and cap the level if needed
                max_level_raw = self.field_mapper.get_field_value(new_class, 'max_level', '0')
                try:
                    max_level = int(max_level_raw) if max_level_raw not in ['****', ''] else 0
                except (ValueError, TypeError):
                    max_level = 0
                
                # If it's a prestige class (has max level), cap the level
                if max_level > 0 and class_level > max_level:
                    logger.info(f"Capping level from {class_level} to {max_level} for prestige class {new_class_id}")
                    class_level = max_level
                    class_entry['ClassLevel'] = class_level
                
                class_entry['Class'] = new_class_id
                class_found = True
                break
        
        if not class_found:
            raise ValueError(f"Character does not have class {old_class_id}")
        
        # Begin transaction if not already in one
        transaction_started = False
        if not self.character_manager._current_transaction:
            self.character_manager.begin_transaction()
            transaction_started = True
        
        try:
            # Update the class list
            self.gff.set('ClassList', class_list)
            
            # Update primary class if it was changed
            current_primary = self.gff.get('Class', 0)
            if current_primary == old_class_id:
                self.gff.set('Class', new_class_id)
            
            # Recalculate all stats since classes changed
            self._recalculate_all_stats()
            
            # Emit class changed event
            event = ClassChangedEvent(
                event_type=EventType.CLASS_CHANGED,
                source_manager='class',
                timestamp=time.time(),
                old_class_id=old_class_id,
                new_class_id=new_class_id,
                level=class_level,
                preserve_feats=self._get_preserved_feats()
            )
            self.character_manager.emit(event)
            
            # Commit transaction if we started it
            if transaction_started:
                self.character_manager.commit_transaction()
                
        except Exception as e:
            # Rollback on error if we started the transaction
            if transaction_started:
                self.character_manager.rollback_transaction()
            logger.error(f"Error during specific class change: {e}")
            raise
        
        return {
            'class_change': {
                'old_class': old_class_id,
                'new_class': new_class_id,
                'level': class_level
            },
            'multiclass_preserved': True,
            'total_level': total_level,
            'stats_updated': True
        }
    
    def _update_class_stats(self, new_class, total_level: int) -> Dict[str, Any]:
        """Update HP, BAB, saves based on new class"""
        changes = {}
        
        # Get ability modifiers
        modifiers = self._calculate_ability_modifiers()
        
        # 1. Hit Points
        old_hp = self.gff.get('HitPoints', 0)
        new_hp = self._calculate_hit_points(new_class, total_level, modifiers['CON'])
        self.gff.set('HitPoints', new_hp)
        self.gff.set('MaxHitPoints', new_hp)
        self.gff.set('CurrentHitPoints', new_hp)
        changes['hit_points'] = {'old': old_hp, 'new': new_hp}
        
        # 2. Base Attack Bonus - get from CombatManager (proper separation of concerns)
        old_bab = self.gff.get('BaseAttackBonus', 0)
        combat_manager = self.character_manager.get_manager('combat')
        if combat_manager:
            combat_manager.invalidate_bab_cache()  # Ensure fresh calculation after class change
            new_bab = combat_manager.calculate_base_attack_bonus()
        else:
            new_bab = self.calculate_total_bab()  # Fallback if no combat manager
        self.gff.set('BaseAttackBonus', new_bab)
        changes['bab'] = {'old': old_bab, 'new': new_bab}
        
        # 3. Saves - use total from all classes for multiclass
        saves = self.calculate_total_saves()
        old_saves = {
            'fortitude': self.gff.get('FortSave', 0),
            'reflex': self.gff.get('RefSave', 0),
            'will': self.gff.get('WillSave', 0)
        }
        
        self.gff.set('FortSave', saves['fortitude'])
        self.gff.set('RefSave', saves['reflex'])
        self.gff.set('WillSave', saves['will'])
        
        changes['saves'] = {
            'fortitude': {'old': old_saves['fortitude'], 'new': saves['fortitude']},
            'reflex': {'old': old_saves['reflex'], 'new': saves['reflex']},
            'will': {'old': old_saves['will'], 'new': saves['will']}
        }
        
        return changes
    
    def _calculate_ability_modifiers(self) -> Dict[str, int]:
        """Calculate ability modifiers using AbilityManager"""
        attr_manager = self.character_manager.get_manager('ability')
        if attr_manager:
            return attr_manager.get_all_modifiers()
        
        # Fallback: Use attribute manager
        if hasattr(self.character_manager, 'get_manager'):
            attribute_manager = self.character_manager.get_manager('ability')
            if attribute_manager:
                scores = attribute_manager.get_ability_scores()
                return {
                    'STR': (scores.get('strength', 10) - 10) // 2,
                    'DEX': (scores.get('dexterity', 10) - 10) // 2,
                    'CON': (scores.get('constitution', 10) - 10) // 2,
                    'INT': (scores.get('intelligence', 10) - 10) // 2,
                    'WIS': (scores.get('wisdom', 10) - 10) // 2,
                    'CHA': (scores.get('charisma', 10) - 10) // 2
                }
        
        # Final fallback to direct GFF access
        abilities = {
            'STR': self.gff.get('Str', 10),
            'DEX': self.gff.get('Dex', 10),
            'CON': self.gff.get('Con', 10),
            'INT': self.gff.get('Int', 10),
            'WIS': self.gff.get('Wis', 10),
            'CHA': self.gff.get('Cha', 10)
        }
        
        return {
            ability: (value - 10) // 2
            for ability, value in abilities.items()
        }
    
    def _calculate_hit_points(self, class_data, level: int, con_modifier: int) -> int:
        """Calculate total hit points for class and level"""
        # Max HP at level 1, average for other levels
        # Use FieldMappingUtility for proper field access
        hit_die = self.field_mapper._safe_int(
            self.field_mapper.get_field_value(class_data, 'hit_die', 8), 8
        )
        base_hp = hit_die
        if level > 1:
            avg_roll = (hit_die + 1) // 2
            base_hp += avg_roll * (level - 1)
        
        total_hp = base_hp + (con_modifier * level)
        return max(1, total_hp)  # Minimum 1 HP
    
    def _calculate_bab(self, class_data, level: int) -> int:
        """Calculate BAB for a single class and level"""
        # Use FieldMappingUtility for proper field access
        bab_table_name = self.field_mapper.get_field_value(class_data, 'attack_bonus_table', '')
        if not bab_table_name:
            class_label = self.field_mapper.get_field_value(class_data, 'label', 'Unknown')
            logger.warning(f"No BAB table found for class {class_label}")
            return 0
            
        # Cache BAB table data (convert to lowercase for lookup)
        bab_table_name_lower = bab_table_name.lower()
        if bab_table_name_lower not in self._bab_table_cache:
            bab_table = self.rules_service.get_table(bab_table_name_lower)
            if bab_table:
                self._bab_table_cache[bab_table_name_lower] = bab_table
            else:
                logger.warning(f"BAB table '{bab_table_name}' not found")
                return 0
        
        bab_table = self._bab_table_cache[bab_table_name_lower]
        
        # Get BAB for level (level - 1 because tables are 0-indexed)
        level_idx = min(level - 1, 19)  # Cap at 20
        if level_idx < len(bab_table):
            bab_row = bab_table[level_idx]
            # Use FieldMappingUtility to get BAB value with proper field mapping
            bab_value = self.field_mapper.get_field_value(bab_row, 'bab', '0')
            return self.field_mapper._safe_int(bab_value, 0)
        
        return 0
    
    def calculate_total_bab(self) -> int:
        """
        DEPRECATED: BAB calculation moved to CombatManager for proper separation of concerns.
        Use combat_manager.calculate_base_attack_bonus() instead.
        
        This method is kept for backward compatibility only.
        
        Returns:
            Total base attack bonus
        """
        logger.warning("calculate_total_bab() is deprecated. Use CombatManager.calculate_base_attack_bonus() instead.")
        class_list = self.gff.get('ClassList', [])
        total_bab = 0
        
        for class_info in class_list:
            class_id = class_info.get('Class', -1)
            class_level = class_info.get('ClassLevel', 0)
            
            if class_level > 0:
                class_data = self.rules_service.get_by_id('classes', class_id)
                if class_data:
                    class_bab = self._calculate_bab(class_data, class_level)
                    total_bab += class_bab
                    class_label = self.field_mapper.get_field_value(class_data, 'label', f'Class {class_id}')
                    logger.debug(f"Class {class_label} (lvl {class_level}): BAB +{class_bab}")
        
        # Removed excessive logging - now handled by CombatManager
        return total_bab
    
    def _calculate_saves(self, class_data, level: int, modifiers: Dict[str, int]) -> Dict[str, int]:
        """Calculate saving throws for a single class"""
        # Use FieldMappingUtility for proper field access
        save_table_name = self.field_mapper.get_field_value(class_data, 'saving_throw_table', '')
        if not save_table_name:
            class_label = self.field_mapper.get_field_value(class_data, 'label', 'Unknown')
            logger.warning(f"No save table found for class {class_label}")
            return {
                'fortitude': modifiers['CON'],
                'reflex': modifiers['DEX'],
                'will': modifiers['WIS']
            }
            
        # Cache save table data (convert to lowercase for lookup)
        save_table_name_lower = save_table_name.lower()
        if save_table_name_lower not in self._save_table_cache:
            save_table = self.rules_service.get_table(save_table_name_lower)
            if save_table:
                self._save_table_cache[save_table_name_lower] = save_table
            else:
                logger.warning(f"Save table '{save_table_name}' not found")
                return {
                    'fortitude': modifiers['CON'],
                    'reflex': modifiers['DEX'],
                    'will': modifiers['WIS']
                }
        
        save_table = self._save_table_cache[save_table_name_lower]
        
        # Get saves for level (level - 1 because tables are 0-indexed)
        level_idx = min(level - 1, 19)  # Cap at 20
        if level_idx < len(save_table):
            save_row = save_table[level_idx]
            # Use FieldMappingUtility to get save values with proper field mapping
            fort_base = self.field_mapper._safe_int(
                self.field_mapper.get_field_value(save_row, 'fort_save_table', '0'), 0
            )
            ref_base = self.field_mapper._safe_int(
                self.field_mapper.get_field_value(save_row, 'ref_save_table', '0'), 0
            )
            will_base = self.field_mapper._safe_int(
                self.field_mapper.get_field_value(save_row, 'will_save_table', '0'), 0
            )
        else:
            fort_base = ref_base = will_base = 0
        
        return {
            'fortitude': fort_base + modifiers['CON'],
            'reflex': ref_base + modifiers['DEX'],
            'will': will_base + modifiers['WIS']
        }
    
    def calculate_total_saves(self) -> Dict[str, int]:
        """
        Calculate total saving throws from all classes (for multiclass)
        
        Returns:
            Dict with fortitude, reflex, and will saves
        """
        class_list = self.gff.get('ClassList', [])
        
        # Get ability modifiers
        modifiers = self._calculate_ability_modifiers()
        con_mod = modifiers['CON']
        dex_mod = modifiers['DEX']
        wis_mod = modifiers['WIS']
        
        # For multiclass, each class contributes its own save progression (they STACK)
        total_fort = 0
        total_ref = 0
        total_will = 0

        for class_info in class_list:
            class_id = class_info.get('Class', -1)
            class_level = class_info.get('ClassLevel', 0)

            if class_level > 0:
                class_data = self.rules_service.get_by_id('classes', class_id)
                if class_data:
                    # Get base saves without modifiers
                    saves = self._calculate_saves(class_data, class_level, {'CON': 0, 'DEX': 0, 'WIS': 0})
                    total_fort += saves['fortitude']
                    total_ref += saves['reflex']
                    total_will += saves['will']

        return {
            'fortitude': total_fort + con_mod,
            'reflex': total_ref + dex_mod,
            'will': total_will + wis_mod,
            'base_fortitude': total_fort,
            'base_reflex': total_ref,
            'base_will': total_will
        }
    
    def _get_preserved_feats(self) -> List[int]:
        """Get list of feat IDs that should be preserved during class change"""
        preserved = set()
        
        # 1. Get epithet/story feats
        # Get feat manager to detect epithet feats
        feat_manager = self.character_manager.get_manager('feat')
        epithet_feats = feat_manager.detect_epithet_feats() if feat_manager else set()
        preserved.update(epithet_feats)
        
        # 2. Get custom content that should be preserved
        for content_id, info in self.character_manager.custom_content.items():
            if info['type'] == 'feat' and not info.get('removable', True):
                preserved.add(info['id'])
        
        # 3. Get racial feats
        race_id = self.gff.get('Race', 0)
        racial_feats = self._get_racial_feats(race_id)
        preserved.update(racial_feats)
        
        # 4. Get background/history feats (typically have specific IDs or patterns)
        feat_list = self.gff.get('FeatList', [])
        for feat in feat_list:
            feat_id = feat.get('Feat', -1)
            
            # Preserve domain feats (usually have IDs in specific ranges)
            if 4000 <= feat_id <= 4999:  # Common domain feat range
                preserved.add(feat_id)
            
            # Preserve background feats
            if self._is_background_feat(feat_id):
                preserved.add(feat_id)
        
        logger.info(f"Preserving {len(preserved)} feats during class change")
        return list(preserved)
    
    def get_class_summary(self) -> Dict[str, Any]:
        """Get summary of character's classes"""
        class_list = self.gff.get('ClassList', [])
        
        classes = []
        for c in class_list:
            class_id = c.get('Class', 0)
            class_data = self.rules_service.get_by_id('classes', class_id)
            
            # Safely get class name
            if class_data:
                class_name = self.field_mapper.get_field_value(class_data, 'label', 
                    self.field_mapper.get_field_value(class_data, 'name', f"Unknown Class {class_id}"))
            else:
                class_name = f"Unknown Class {class_id}"
            
            classes.append({
                'id': class_id,
                'level': c.get('ClassLevel', 0),
                'name': class_name
            })
        
        return {
            'classes': classes,
            'total_level': sum(c.get('ClassLevel', 0) for c in class_list),
            'multiclass': len(class_list) > 1,
            'can_multiclass': len(class_list) < 3
        }
    
    def get_attack_bonuses(self) -> Dict[str, Any]:
        """
        Get all attack bonuses including BAB and ability modifiers
        
        Returns:
            Dict with melee, ranged, and touch attack bonuses
        """
        # Get BAB from CombatManager (proper separation of concerns)
        combat_manager = self.character_manager.get_manager('combat')
        bab = combat_manager.calculate_base_attack_bonus() if combat_manager else self.calculate_total_bab()
        
        # Get ability modifiers
        modifiers = self._calculate_ability_modifiers()
        str_mod = modifiers['STR']
        dex_mod = modifiers['DEX']
        
        # Check for Weapon Finesse
        has_weapon_finesse = self._has_feat_by_name('WeaponFinesse')
        
        # Calculate attack bonuses
        melee_bonus = bab + str_mod
        finesse_bonus = bab + dex_mod if has_weapon_finesse else None
        ranged_bonus = bab + dex_mod
        touch_bonus = bab  # Touch attacks ignore armor, use BAB only
        
        # Multiple attacks at higher BAB
        attacks = []
        current_bab = bab
        while current_bab > 0:
            attacks.append(current_bab)
            current_bab -= 5
        
        return {
            'base_attack_bonus': bab,
            'melee_attack_bonus': melee_bonus,
            'finesse_attack_bonus': finesse_bonus,
            'ranged_attack_bonus': ranged_bonus,
            'touch_attack_bonus': touch_bonus,
            'multiple_attacks': attacks,
            'str_modifier': str_mod,
            'dex_modifier': dex_mod,
            'has_weapon_finesse': has_weapon_finesse
        }
    
    def _has_feat_by_name(self, feat_label: str) -> bool:
        """Check if character has a feat by its label"""
        # Use FeatManager directly
        feat_manager = self.character_manager.get_manager('feat')
        if feat_manager:
            return feat_manager.has_feat_by_name(feat_label)
        
        # Fallback implementation
        feat_list = self.gff.get('FeatList', [])
        
        for feat in feat_list:
            feat_id = feat.get('Feat', -1)
            feat_data = self.rules_service.get_by_id('feat', feat_id)
            if feat_data:
                label = getattr(feat_data, 'label', '')
                if label == feat_label:
                    return True
        
        return False
    
    def _get_racial_feats(self, race_id: int) -> List[int]:
        """Get racial feats for a specific race"""
        racial_feats = []
        
        # Get race data
        race_data = self.rules_service.get_by_id('racialtypes', race_id)
        if not race_data:
            return racial_feats
        
        # Check for racial feat table
        feat_table_name = getattr(race_data, 'feat_table', None)
        if feat_table_name:
            feat_table = self.rules_service.get_table(feat_table_name.lower())
            if feat_table:
                for feat_entry in feat_table:
                    feat_id = getattr(feat_entry, 'feat_index', -1)
                    if feat_id >= 0:
                        racial_feats.append(feat_id)
        
        return racial_feats
    
    def _is_background_feat(self, feat_id: int) -> bool:
        """Check if a feat is a background/history feat that should be preserved"""
        # Get feat data
        feat_data = self.rules_service.get_by_id('feat', feat_id)
        if not feat_data:
            return False
        
        # Check feat properties
        label = getattr(feat_data, 'label', '').lower()
        category = getattr(feat_data, 'categories', '').lower()
        
        # Background feat patterns
        background_patterns = [
            'background', 'history', 'past', 'origin',
            'blessing', 'curse', 'gift', 'legacy',
            'shard', 'silver', 'influence', 'touched'
        ]
        
        for pattern in background_patterns:
            if pattern in label or pattern in category:
                return True
        
        # Check if feat cannot be removed (indicator of special feat)
        removable = getattr(feat_data, 'removable', 1)
        if removable == 0:
            return True
        
        return False
    
    def validate(self) -> Tuple[bool, List[str]]:
        """Validate current class configuration - corruption prevention only"""
        errors = []
        
        class_list = self.gff.get('ClassList', [])
        
        # Check for valid classes (prevent crashes from invalid class references)
        for class_entry in class_list:
            class_id = class_entry.get('Class', 0)
            class_data = self.rules_service.get_by_id('classes', class_id)
            if not class_data:
                errors.append(f"Invalid class ID: {class_id}")
        
        # Check level bounds (prevent GFF corruption)
        total_level = sum(c.get('ClassLevel', 0) for c in class_list)
        if total_level > 60:  # NWN2 max with epic levels - prevent GFF corruption
            errors.append(f"Total level {total_level} exceeds maximum of 60")
        if total_level < 1:
            errors.append("Total level must be at least 1")
        
        return len(errors) == 0, errors
    
    def _validate_class_level_limits(self, class_id: int, class_data) -> None:
        """
        Validate prestige class level limits
        
        Args:
            class_id: The class ID to validate
            class_data: The class data from the game data loader
            
        Raises:
            ValueError: If adding this level would exceed the class maximum
        """
        # Get current level in this class
        current_level = 0
        class_list = self.gff.get('ClassList', [])
        for class_entry in class_list:
            if class_entry.get('Class') == class_id:
                current_level = class_entry.get('ClassLevel', 0)
                break
        
        # Get max level from class data using field mapper
        max_level_raw = self.field_mapper.get_field_value(class_data, 'max_level', '0')
        try:
            max_level = int(max_level_raw) if max_level_raw not in ['****', ''] else 0
        except (ValueError, TypeError):
            max_level = 0
        
        # Only check if it's a prestige class (has max level > 0)
        if max_level > 0:
            new_level = current_level + 1
            if new_level > max_level:
                class_name = self.field_mapper.get_field_value(class_data, 'label', 
                    self.field_mapper.get_field_value(class_data, 'name', f'Class {class_id}'))
                raise ValueError(f"Cannot add level to {class_name}: maximum level is {max_level}, character already has {current_level} levels")
    
    def get_class_level_info(self, class_id: int) -> Dict[str, Any]:
        """
        Get level information for a class including max level and remaining levels
        
        Args:
            class_id: The class ID to check
            
        Returns:
            Dict with current level, max level, and remaining levels
        """
        # Get current level in this class
        current_level = 0
        class_list = self.gff.get('ClassList', [])
        for class_entry in class_list:
            if class_entry.get('Class') == class_id:
                current_level = class_entry.get('ClassLevel', 0)
                break
        
        # Get class data and max level
        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return {
                'current_level': current_level,
                'max_level': None,
                'remaining_levels': None,
                'is_prestige': False,
                'can_level_up': True
            }
        
        max_level_raw = self.field_mapper.get_field_value(class_data, 'max_level', '0')
        try:
            max_level = int(max_level_raw) if max_level_raw not in ['****', ''] else 0
        except (ValueError, TypeError):
            max_level = 0
        
        is_prestige = max_level > 0
        remaining_levels = max_level - current_level if is_prestige else None
        can_level_up = not is_prestige or remaining_levels > 0
        
        return {
            'current_level': current_level,
            'max_level': max_level if is_prestige else None,
            'remaining_levels': remaining_levels,
            'is_prestige': is_prestige,
            'can_level_up': can_level_up
        }
    
    def get_class_by_id(self, class_id: int) -> Optional[Dict[str, Any]]:
        """
        Get class info for a specific class in ClassList
        
        Args:
            class_id: The class ID to look up
            
        Returns:
            Class info dict or None if not found
        """
        class_list = self.gff.get('ClassList', [])
        
        for class_entry in class_list:
            if class_entry.get('Class') == class_id:
                class_data = self.rules_service.get_by_id('classes', class_id)
                return {
                    'id': class_id,
                    'level': class_entry.get('ClassLevel', 0),
                    'name': self.field_mapper.get_field_value(class_data, 'label', 'Unknown') if class_data else 'Unknown',
                    'data': class_data
                }
        
        return None
    
    def remove_class(self, class_id: int) -> Dict[str, Any]:
        """
        Remove a class from multiclass (keeping others)
        
        Args:
            class_id: Class ID to remove
            
        Returns:
            Summary of changes
        """
        class_list = self.gff.get('ClassList', [])
        
        # Find the class to remove
        class_to_remove = None
        for i, class_entry in enumerate(class_list):
            if class_entry.get('Class') == class_id:
                class_to_remove = i
                break
        
        if class_to_remove is None:
            raise ValueError(f"Character does not have class {class_id}")
        
        if len(class_list) <= 1:
            raise ValueError("Cannot remove last remaining class")
        
        # Begin transaction
        txn = self.character_manager.begin_transaction()
        
        try:
            # Remove the class
            removed_class = class_list.pop(class_to_remove)
            self.gff.set('ClassList', class_list)
            
            # Recalculate stats
            self._recalculate_all_stats()
            
            # Emit event
            event = ClassChangedEvent(
                event_type=EventType.CLASS_CHANGED,
                source_manager='class',
                timestamp=time.time(),
                old_class_id=class_id,
                new_class_id=-1,  # Indicates removal
                level=removed_class.get('ClassLevel', 0)
            )
            self.character_manager.emit(event)
            
            self.character_manager.commit_transaction()
            
            return {
                'removed_class': class_id,
                'removed_levels': removed_class.get('ClassLevel', 0),
                'remaining_classes': len(class_list)
            }
            
        except Exception as e:
            self.character_manager.rollback_transaction()
            raise
    
    def get_prestige_class_options(self) -> List[Dict[str, Any]]:
        """
        Get available prestige classes based on current character
        
        Returns:
            List of prestige class options with requirements
        """
        available_prestige = []
        
        # Get all classes
        classes_table = self.rules_service.get_table('classes')
        if not classes_table:
            return available_prestige
        
        for class_data in classes_table:
            # Check if it's a prestige class
            is_prestige = getattr(class_data, 'is_prestige', 0)
            if not is_prestige:
                continue
            
            class_id = getattr(class_data, 'id', -1)
            if class_id < 0:
                continue
            
            # Check if character can take this prestige class
            can_take, reason = self.can_take_prestige_class(class_id)
            
            available_prestige.append({
                'id': class_id,
                'name': getattr(class_data, 'label', 'Unknown'),
                'can_take': can_take,
                'reason': reason,
                'requirements': self._get_prestige_requirements(class_data)
            })
        
        return available_prestige
    
    def can_take_prestige_class(self, prestige_id: int) -> Tuple[bool, str]:
        """
        Check if character meets prestige class requirements
        
        Args:
            prestige_id: Prestige class ID
            
        Returns:
            (can_take, reason) tuple
        """
        # Use character manager's prerequisite checking
        can_take, errors = self.character_manager.check_prerequisites('class', prestige_id)
        
        if can_take:
            return True, "All requirements met"
        else:
            return False, "; ".join(errors)
    
    def _get_prestige_requirements(self, class_data) -> Dict[str, Any]:
        """Extract prestige class requirements from class data"""
        requirements = {}
        
        # Base attack bonus requirement
        min_bab = getattr(class_data, 'min_attack_bonus', 0)
        if min_bab > 0:
            requirements['base_attack_bonus'] = min_bab
        
        # Skill requirements
        skill_req = getattr(class_data, 'required_skill', '')
        skill_rank = getattr(class_data, 'required_skill_rank', 0)
        if skill_req and skill_rank > 0:
            requirements['skills'] = {skill_req: skill_rank}
        
        # Feat requirements
        req_feat = getattr(class_data, 'required_feat', '')
        if req_feat:
            requirements['feats'] = [req_feat]
        
        # Alignment restrictions
        alignment_restrict = getattr(class_data, 'alignment_restrict', 0)
        if alignment_restrict > 0:
            requirements['alignment'] = self._decode_alignment_restriction(alignment_restrict)
        
        return requirements
    
    def _decode_alignment_restriction(self, restriction: int) -> str:
        """Decode alignment restriction bitmask"""
        # Common alignment restrictions
        restrictions = {
            0x01: "Lawful",
            0x02: "Chaotic", 
            0x04: "Good",
            0x08: "Evil",
            0x10: "Neutral (Law/Chaos)",
            0x20: "Neutral (Good/Evil)"
        }
        
        allowed = []
        for mask, name in restrictions.items():
            if restriction & mask:
                allowed.append(name)
        
        return " or ".join(allowed) if allowed else "Any"
    
    def get_class_features(self, class_id: int, level: int) -> Dict[str, Any]:
        """
        Get features gained at specific level for a class
        
        Args:
            class_id: The class ID
            level: The level to check
            
        Returns:
            Dict of features gained at this level
        """
        features = {
            'feats': [],
            'abilities': [],
            'spells': {},
            'bab_increase': 0,
            'save_increases': {},
            'skill_points': 0
        }
        
        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return features
        
        # Get feats granted at this level
        feats = self.get_class_feats_for_level(class_data, level)
        features['feats'] = feats
        
        # Get special abilities
        abilities = self.get_class_abilities(class_id, level)
        features['abilities'] = abilities
        
        # Calculate BAB increase
        if level > 1:
            bab_current = self._calculate_bab(class_data, level)
            bab_previous = self._calculate_bab(class_data, level - 1)
            features['bab_increase'] = bab_current - bab_previous
        else:
            features['bab_increase'] = self._calculate_bab(class_data, 1)
        
        # Calculate save increases
        for save_type in ['fortitude', 'reflex', 'will']:
            if level > 1:
                save_current = self._calculate_single_save(class_data, save_type, level)
                save_previous = self._calculate_single_save(class_data, save_type, level - 1)
                increase = save_current - save_previous
            else:
                increase = self._calculate_single_save(class_data, save_type, 1)
            
            if increase > 0:
                features['save_increases'][save_type] = increase
        
        # Get skill points
        skill_points = getattr(class_data, 'skill_point_base', 2)
        features['skill_points'] = skill_points
        
        return features
    
    def _calculate_single_save(self, class_data, save_type: str, level: int) -> int:
        """Calculate a single save value for a class at a level"""
        save_table_name = getattr(class_data, f'{save_type}_save_table', None)
        if save_table_name:
            save_table = self._get_save_table(save_table_name)
            if save_table and level <= len(save_table):
                return save_table[level - 1]
        
        # Fallback to good/poor save progression
        return self._calculate_save_progression(level, save_type in ['fortitude'])
    
    def _calculate_save_progression(self, level: int, is_good_save: bool) -> int:
        """Calculate save bonus based on good/poor progression"""
        if is_good_save:
            return 2 + (level // 2)
        else:
            return level // 3
    
    def _recalculate_all_stats(self):
        """Recalculate all class-dependent stats after class change"""
        # Recalculate BAB using CombatManager
        combat_manager = self.character_manager.get_manager('combat')
        if combat_manager:
            combat_manager.invalidate_bab_cache()  # Ensure fresh calculation after class change
            total_bab = combat_manager.calculate_base_attack_bonus()
        else:
            total_bab = self.calculate_total_bab()  # Fallback if no combat manager
        self.gff.set('BaseAttackBonus', total_bab)
        
        # Recalculate saves
        saves = self.calculate_total_saves()
        self.gff.set('FortSaveBase', saves['base_fortitude'])
        self.gff.set('RefSaveBase', saves['base_reflex'])
        self.gff.set('WillSaveBase', saves['base_will'])
    
    def _get_save_table(self, save_table_name: str) -> Optional[List[int]]:
        """Get save progression table"""
        if save_table_name in self._save_table_cache:
            return self._save_table_cache[save_table_name]
        
        # Load save table
        save_table_data = self.rules_service.get_table(save_table_name.lower())
        if not save_table_data:
            return None
        
        # Extract save values
        save_values = []
        for entry in save_table_data:
            save_value = getattr(entry, 'save_throw', 0)
            save_values.append(save_value)
        
        self._save_table_cache[save_table_name] = save_values
        return save_values
    
    def get_class_progression_summary(self, class_id: int, max_level: int = 20) -> Dict[str, Any]:
        """
        Get complete class progression summary for UI display
        
        Args:
            class_id: The class ID to analyze
            max_level: Maximum level to show progression for
            
        Returns:
            Dict with progression data formatted for frontend display
        """
        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return {}
        
        # Get basic class info using instance field_mapper
        class_name = self.field_mapper.get_field_value(class_data, 'name', 'Unknown Class')
        hit_die = self.field_mapper._safe_int(
            self.field_mapper.get_field_value(class_data, 'hit_die', '8'), 8
        )
        skill_points = self.field_mapper._safe_int(
            self.field_mapper.get_field_value(class_data, 'skill_point_base', '2'), 2
        )
        
        # Build level progression
        progression = []
        for level in range(1, min(max_level + 1, 21)):
            level_info = {
                'level': level,
                'hit_die': hit_die,
                'skill_points': skill_points,
                'bab': self._calculate_bab(class_data, level),
                'saves': self._get_saves_for_level_detailed(class_data, level),
                'features': self._get_level_features(class_data, level),
                'new_features': self._get_new_features_at_level(class_data, level)
            }
            progression.append(level_info)
        
        # Get class categories and type
        class_info = self._analyze_class_type(class_data)
        
        return {
            'class_id': class_id,
            'class_name': class_name,
            'class_info': class_info,
            'basic_stats': {
                'hit_die': hit_die,
                'skill_points_per_level': skill_points,
                'is_spellcaster': class_info['is_spellcaster'],
                'spell_type': class_info['spell_type'],
                'primary_ability': class_info['primary_ability']
            },
            'progression': progression,
            'proficiencies': self._get_class_proficiencies_detailed(class_data),
            'special_abilities': self._get_class_special_abilities(class_data)
        }
    
    def _get_saves_for_level_detailed(self, class_data, level: int) -> Dict[str, int]:
        """Get detailed save progression for a specific level"""
        save_table_name = self.field_mapper.get_field_value(class_data, 'saving_throw_table', '')
        if not save_table_name:
            # Use standard progression if no table
            return {
                'fortitude': level // 3,  # Poor save
                'reflex': level // 3,     # Poor save  
                'will': level // 3        # Poor save
            }
        
        # Convert to lowercase for table lookup
        save_table = self.rules_service.get_table(save_table_name.lower())
        
        if not save_table or level > len(save_table):
            return {'fortitude': 0, 'reflex': 0, 'will': 0}
        
        row = save_table[level - 1]
        return {
            'fortitude': self.field_mapper._safe_int(
                self.field_mapper.get_field_value(row, 'fort_save_table', '0'), 0
            ),
            'reflex': self.field_mapper._safe_int(
                self.field_mapper.get_field_value(row, 'ref_save_table', '0'), 0
            ),
            'will': self.field_mapper._safe_int(
                self.field_mapper.get_field_value(row, 'will_save_table', '0'), 0
            )
        }
    
    def _get_level_features(self, class_data, level: int) -> List[Dict[str, Any]]:
        """Get all features available at a specific level"""
        features = []
        
        # Level 1 always gets proficiencies
        if level == 1:
            features.append({
                'name': 'Weapon and Armor Proficiencies',
                'type': 'proficiency',
                'description': 'Class weapon and armor proficiencies',
                'icon': 'sword'
            })
        
        # Check for bonus feats (common pattern)
        if self._class_gets_bonus_feats(class_data) and level % 2 == 0:
            features.append({
                'name': 'Bonus Feat',
                'type': 'feat',
                'description': f'Choose a bonus feat at level {level}',
                'icon': 'star'
            })
        
        # Spellcasting progression
        if self._is_spellcaster_class_data(class_data):
            spell_info = self._get_spell_progression_at_level(class_data, level)
            if spell_info:
                features.append({
                    'name': 'Spell Progression',
                    'type': 'spell',
                    'description': f'Gain access to new spell levels',
                    'details': spell_info,
                    'icon': 'magic'
                })
        
        return features
    
    def _get_new_features_at_level(self, class_data, level: int) -> List[Dict[str, Any]]:
        """Get only new features gained specifically at this level"""
        new_features = []
        
        # This is where we'd parse class-specific feature tables
        # For now, implement common patterns
        
        if level == 1:
            new_features.append({
                'name': 'Class Skills',
                'type': 'skill',
                'description': 'Access to class skill list',
                'icon': 'book'
            })
        
        # Every 3rd level for some classes
        if level % 3 == 0 and level > 1:
            class_focus = self._get_class_focus(class_data)
            if class_focus == 'combat':
                new_features.append({
                    'name': 'Combat Improvement',
                    'type': 'combat',
                    'description': f'Enhanced combat abilities at level {level}',
                    'icon': 'sword'
                })
        
        return new_features
    
    def _analyze_class_type(self, class_data) -> Dict[str, Any]:
        """Analyze class characteristics"""
        has_arcane = self.field_mapper.get_field_value(class_data, 'has_arcane', '0') == '1'
        has_divine = self.field_mapper.get_field_value(class_data, 'has_divine', '0') == '1'
        primary_ability = self.field_mapper.get_field_value(class_data, 'primary_ability', 'STR')
        
        spell_type = 'none'
        if has_arcane:
            spell_type = 'arcane'
        elif has_divine:
            spell_type = 'divine'
        
        return {
            'is_spellcaster': has_arcane or has_divine,
            'spell_type': spell_type,
            'primary_ability': primary_ability,
            'focus': self._get_class_focus(class_data),
            'alignment_restricted': self.field_mapper.get_field_value(class_data, 'align_restrict', '0') != '0'
        }
    
    def _get_class_focus(self, class_data) -> str:
        """Determine class focus/role"""
        has_arcane = self.field_mapper.get_field_value(class_data, 'has_arcane', '0') == '1'
        has_divine = self.field_mapper.get_field_value(class_data, 'has_divine', '0') == '1'
        skill_points = self.field_mapper._safe_int(
            self.field_mapper.get_field_value(class_data, 'skill_point_base', '2'), 2
        )
        hit_die = self.field_mapper._safe_int(
            self.field_mapper.get_field_value(class_data, 'hit_die', '8'), 8
        )
        
        if has_arcane:
            return 'arcane_caster'
        elif has_divine:
            return 'divine_caster'
        elif skill_points >= 6:
            return 'skill_specialist'
        elif hit_die >= 10:
            return 'combat'
        else:
            return 'hybrid'
    
    def _class_gets_bonus_feats(self, class_data) -> bool:
        """Check if class gets bonus feats (like Fighter)"""
        class_name = self.field_mapper.get_field_value(class_data, 'name', '').lower()
        return 'fighter' in class_name or 'warrior' in class_name
    
    def _is_spellcaster_class_data(self, class_data) -> bool:
        """Check if class data indicates spellcasting"""
        has_arcane = self.field_mapper.get_field_value(class_data, 'has_arcane', '0') == '1'
        has_divine = self.field_mapper.get_field_value(class_data, 'has_divine', '0') == '1'
        return has_arcane or has_divine
    
    def _get_spell_progression_at_level(self, class_data, level: int) -> Optional[Dict[str, Any]]:
        """Get spell slot progression for a level (placeholder for now)"""
        if not self._is_spellcaster_class_data(class_data):
            return None
        
        # TODO: Implement actual spell table lookup
        # For now, return basic info
        return {
            'new_spell_level': max(0, (level + 1) // 2),
            'description': f'Spell casting progression at level {level}'
        }
    
    def _get_class_proficiencies_detailed(self, class_data) -> Dict[str, Any]:
        """Get detailed weapon and armor proficiencies"""
        # TODO: Parse actual proficiency data from class tables
        # This is a placeholder implementation
        class_focus = self._get_class_focus(class_data)
        
        proficiencies = {
            'weapons': [],
            'armor': [],
            'shields': False
        }
        
        if class_focus == 'combat':
            proficiencies['weapons'] = ['Simple', 'Martial']
            proficiencies['armor'] = ['Light', 'Medium', 'Heavy']
            proficiencies['shields'] = True
        elif class_focus in ['arcane_caster', 'divine_caster']:
            proficiencies['weapons'] = ['Simple']
            proficiencies['armor'] = ['Light'] if class_focus == 'divine_caster' else []
            proficiencies['shields'] = class_focus == 'divine_caster'
        else:
            proficiencies['weapons'] = ['Simple']
            proficiencies['armor'] = ['Light']
            proficiencies['shields'] = False
        
        return proficiencies
    
    def _get_class_special_abilities(self, class_data) -> List[Dict[str, Any]]:
        """Get class special abilities (placeholder)"""
        # TODO: Parse actual special abilities from class data
        abilities = []
        
        class_name = self.field_mapper.get_field_value(class_data, 'name', '').lower()
        
        if 'rogue' in class_name:
            abilities.append({
                'name': 'Sneak Attack',
                'description': 'Deal extra damage when flanking or attacking flat-footed enemies',
                'progression': 'Increases every 2 levels'
            })
        elif 'barbarian' in class_name:
            abilities.append({
                'name': 'Rage',
                'description': 'Enter a berserker rage for combat bonuses',
                'progression': 'Additional uses per day at higher levels'
            })
        
        return abilities
    
    def get_class_feats_for_level(self, class_data: Any, level: int) -> List[Dict[str, Any]]:
        """
        Get feats granted by a class at a specific level
        
        Args:
            class_data: Class data object from dynamic loader
            level: Character level to check
            
        Returns:
            List of feat dictionaries with 'feat_id' and 'list_type' keys
        """
        feats_for_level = []
        
        # Get the feat table name from class data
        feat_table_name = getattr(class_data, 'feats_table', None)
        if not feat_table_name:
            logger.debug(f"Class {getattr(class_data, 'label', 'Unknown')} has no feat table")
            return feats_for_level
        
        # Load the feat table
        feat_table = self.rules_service.get_table(feat_table_name.lower())
        if not feat_table:
            logger.warning(f"Feat table {feat_table_name} not found")
            return feats_for_level
        
        # Look for feats at this level
        # Class feat tables have columns like FeatIndex, GrantedOnLevel, List
        for feat_entry in feat_table:
            granted_level = getattr(feat_entry, 'granted_on_level', -1)
            if granted_level == level:
                feat_id = getattr(feat_entry, 'feat_index', -1)
                list_type = getattr(feat_entry, 'list', 3)  # Default to general list
                
                if feat_id >= 0:
                    feats_for_level.append({
                        'feat_id': feat_id,
                        'list_type': list_type,
                        'granted_on_level': granted_level
                    })
        
        return feats_for_level
    
    def get_class_abilities(self, class_id: int, level: int) -> List[Dict[str, Any]]:
        """
        Get special abilities granted by a class at a specific level
        
        Args:
            class_id: The class ID
            level: The level to check
            
        Returns:
            List of ability info dicts
        """
        abilities = []
        
        try:
            class_data = self.rules_service.get_by_id('classes', class_id)
            if not class_data:
                return abilities
            
            # Check for ability table (like cls_bfeat_* tables)
            ability_table_name = getattr(class_data, 'ability_table', None)
            if not ability_table_name:
                # Try alternate naming
                label = getattr(class_data, 'label', '').lower()
                ability_table_name = f'cls_bfeat_{label}'
            
            if ability_table_name:
                ability_table = self.rules_service.get_table(ability_table_name.lower())
                if ability_table:
                    for ability in ability_table:
                        granted_level = getattr(ability, 'granted_on_level', -1)
                        if granted_level == level:
                            ability_id = getattr(ability, 'feat_index', -1)
                            if ability_id >= 0:
                                abilities.append({
                                    'ability_id': ability_id,
                                    'type': 'feat',
                                    'level': level
                                })
        except Exception as e:
            logger.warning(f"Could not get class abilities for class {class_id} level {level}: {e}")
        
        return abilities
    
    def has_class_by_name(self, class_name: str) -> bool:
        """
        Check if character has levels in a class by name
        
        Args:
            class_name: The class name to check
            
        Returns:
            True if character has this class
        """
        class_list = self.gff.get('ClassList', [])
        
        for class_info in class_list:
            class_id = class_info.get('Class', -1)
            class_data = self.rules_service.get_by_id('classes', class_id)
            if class_data:
                label = getattr(class_data, 'label', '')
                if label.lower() == class_name.lower():
                    return True
        
        return False
    
    def get_class_level_by_name(self, class_name: str) -> int:
        """
        Get level in a specific class by name
        
        Args:
            class_name: The class name
            
        Returns:
            Class level or 0 if not found
        """
        class_list = self.gff.get('ClassList', [])
        
        for class_info in class_list:
            class_id = class_info.get('Class', -1)
            class_data = self.rules_service.get_by_id('classes', class_id)
            if class_data:
                label = getattr(class_data, 'label', '')
                if label.lower() == class_name.lower():
                    return class_info.get('ClassLevel', 0)
        
        return 0
    
    def _get_class_name(self, class_id: int) -> str:
        """Get class name from dynamic data"""
        return self._get_content_name('classes', class_id)
    
    def get_class_name(self, class_id: int) -> str:
        """Public method to get class name (for character summary)"""
        return self._get_class_name(class_id)
    
    def _get_content_name(self, table_name: str, content_id: int) -> str:
        """Get content name from dynamic data"""
        content_data = self.rules_service.get_by_id(table_name, content_id)
        if content_data:
            # Try multiple possible name fields
            for field in ['label', 'name', 'Label', 'Name']:
                name = getattr(content_data, field, '')
                if name and str(name).strip() and str(name) != '****':
                    return str(name)
        return f'{table_name.title()}_{content_id}'
    
    def _get_total_level(self) -> int:
        """Get total character level from all classes"""
        return sum(
            c.get('ClassLevel', 0) 
            for c in self.gff.get('ClassList', []) 
            if isinstance(c, dict)
        )
    
    def get_total_level(self) -> int:
        """Public method to get total character level (for character summary)"""
        return self._get_total_level()

    def get_available_classes(self) -> List[Dict[str, Any]]:
        """Get list of classes available for next level"""
        char_summary = self._create_character_summary_for_rules()
        return self.character_manager.rules_service.get_available_classes(char_summary)
    
    def get_class_progressions(self) -> Dict[str, Any]:
        """Get class progression info for all character classes"""
        progressions = {}
        for class_entry in self.gff.get('ClassList', []):
            if isinstance(class_entry, dict):
                class_id = class_entry.get('Class', 0)
                class_level = class_entry.get('ClassLevel', 0)
                
                progression = self.character_manager.rules_service.get_class_progression(
                    class_id, 
                    class_level
                )
                if progression:
                    class_name = self.character_manager._get_class_name(class_id)
                    progressions[class_name] = progression
        
        return progressions

    def _create_character_summary_for_rules(self) -> Dict[str, Any]:
        """Create character summary dict for rules service validation using dynamic data"""
        # Get ability scores from attribute manager
        attribute_manager = self.character_manager.get_manager('ability')
        if attribute_manager:
            abilities = attribute_manager.get_ability_scores()
            # Convert to expected format
            abilities_formatted = {
                'str': abilities.get('strength', 10),
                'dex': abilities.get('dexterity', 10),
                'con': abilities.get('constitution', 10),
                'int': abilities.get('intelligence', 10),
                'wis': abilities.get('wisdom', 10),
                'cha': abilities.get('charisma', 10)
            }
        else:
            # Fallback to direct access
            abilities_formatted = {
                'str': self.gff.get('Str', 10),
                'dex': self.gff.get('Dex', 10),
                'con': self.gff.get('Con', 10),
                'int': self.gff.get('Int', 10),
                'wis': self.gff.get('Wis', 10),
                'cha': self.gff.get('Cha', 10)
            }
        
        # Get skills summary from skill manager
        skill_manager = self.character_manager.get_manager('skill')
        if skill_manager:
            skills = skill_manager._extract_skills_summary()
        else:
            # Fallback to direct extraction
            skills = {}
            skill_list = self.gff.get('SkillList', [])
            for skill in skill_list:
                if isinstance(skill, dict):
                    skill_id = skill.get('Skill', -1)
                    rank = skill.get('Rank', 0)
                    if skill_id >= 0:
                        skills[skill_id] = rank
        
        return {
            'level': sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', [])),
            'classes': [
                {
                    'id': c.get('Class', 0),
                    'level': c.get('ClassLevel', 0)
                }
                for c in self.gff.get('ClassList', [])
                if isinstance(c, dict)
            ],
            'race': self.gff.get('Race', 0),
            'abilities': abilities_formatted,
            'alignment': {
                'law_chaos': self.gff.get('LawfulChaotic', 50),
                'good_evil': self.gff.get('GoodEvil', 50)
            },
            'feats': [f.get('Feat', 0) for f in self.gff.get('FeatList', []) if isinstance(f, dict)],
            'skills': skills,
            'hit_points': self.gff.get('HitPoints', 0),
            'base_attack_bonus': self.gff.get('BaseAttackBonus', 0)
        }
    
    
    def _get_class_id_by_name(self, class_name: str) -> Optional[int]:
        """
        Get class ID by name from classes.2da
        
        Args:
            class_name: Class name to lookup
            
        Returns:
            Class ID or None if not found
        """
        try:
            # Search through classes.2da for matching name
            classes_data = self.rules_service.get_table('classes')
            for row_id, class_data in enumerate(classes_data):
                if class_data:
                    # Check different name fields
                    for field_pattern in ['label', 'name']:
                        class_label = self.field_mapper.get_field_value(class_data, field_pattern, '')
                        if isinstance(class_label, str) and class_label.lower() == class_name.lower():
                            return row_id
        except Exception as e:
            logger.warning(f"Could not lookup class ID for '{class_name}': {e}")
        
        return None