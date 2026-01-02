"""
Class Manager - handles class changes, multiclassing, and level progression
Refactored from class_change_service.py to work with event system
"""

from typing import Dict, List, Tuple, Optional, Any
from loguru import logger
import time

from ..events import EventEmitter, EventType, ClassChangedEvent, LevelGainedEvent
from gamedata.dynamic_loader.field_mapping_utility import FieldMappingUtility

# LvlStatList field names
LVL_STAT_LIST = "LvlStatList"
LVL_STAT_CLASS = "LvlStatClass"
LVL_STAT_HITDIE = "LvlStatHitDie"
LVL_STAT_ABILITY = "LvlStatAbility"
LVL_STAT_SKILL_LIST = "SkillList"
LVL_STAT_SKILL_POINTS = "SkillPoints"
LVL_STAT_FEAT_LIST = "FeatList"
LVL_STAT_KNOWN_LIST = "KnownList" # Prefix for known spells
LVL_STAT_KNOWN_REMOVE_LIST = "KnownRemoveList" # Prefix for removed spells

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
        
        # Register for events
        self._register_event_handlers()
    
    def _register_event_handlers(self):
        """Register handlers for relevant events"""
        self.character_manager.on(EventType.SKILL_POINTS_AWARDED, self.on_skill_points_awarded)
    
    def change_class(self, new_class_id: int, preserve_level: bool = True) -> Dict[str, Any]:
        """
        Change character's primary class
        
        Args:
            preserve_level: Keep the same total level
            
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
    
    def add_class_level(self, class_id: int) -> Dict[str, Any]:
        """
        Add a level in a specific class (multiclassing or leveling up)
        
            class_id: Class to add level in
            
        Returns:
            Dict with changes made
        """
        logger.info(f"Adding level in class {class_id}")
        
        # Get class info
        new_class = self.rules_service.get_by_id('classes', class_id)
        if not new_class:
            raise ValueError(f"Invalid class ID: {class_id}")
        
        # Update class list
        class_list = self.gff.get('ClassList', [])
        total_level = sum(c.get('ClassLevel', 0) for c in class_list) + 1
        
        # Find existing class or add new
        current_xp = self.get_experience()
        min_xp = self.level_to_xp(total_level)
        if current_xp < min_xp:
            logger.info(f"Auto-adjusting XP from {current_xp} to {min_xp} for level {total_level}")
            self.set_experience(min_xp)
        
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
        
        # Calculate current class level for event and feats
        current_class_lvl = 0
        for c in class_list:
            if c.get('Class') == class_id:
                current_class_lvl = c.get('ClassLevel', 1)
                break
        
        modifiers = self._calculate_ability_modifiers()
        hp_gained = self._calculate_hit_points(new_class, 1, modifiers['CON'])
        self._record_level_up(class_id, hp_gained)
        
        # Emit level gained event to the character manager (central bus)
        # This is moved after _record_level_up so that history entry exists when SkillManager 
        # reacts to calculate points and emits SKILL_POINTS_AWARDED back to us.
        event = LevelGainedEvent(
            event_type=EventType.LEVEL_GAINED,  # Will be overridden by __post_init__
            source_manager='class',
            timestamp=time.time(),
            class_id=class_id,
            new_level=total_level,
            total_level=total_level,
            class_level_gained=current_class_lvl
        )
        self.character_manager.emit(event)
        
        # Get awarded skill points from history (updated via event system above)
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        points_gained = 0
        if lvl_stat_list:
            last_entry = lvl_stat_list[-1]
            points_gained = last_entry.get(LVL_STAT_SKILL_POINTS, 0)

        # Calculate gains for reporting
        gains = {
            'skill_points': points_gained,
            'total_skill_points': self.gff.get('SkillPoints', 0),
            'feats': 0,
            'ability_score': False,
            'new_spells': False
        }

        # 2. Feats
        # Use shared calculation logic

        # Use shared calculation logic
        feat_slots = self._calculate_feat_slots_for_level(total_level, class_id, current_class_lvl)
        
        gains['feats'] = feat_slots['general']
        if feat_slots['bonus'] > 0:
            gains['bonus_feats'] = feat_slots['bonus']
            gains['bonus_label'] = self._get_bonus_feat_label(class_id, total_level)

            
        # 3. Ability Score (Every 4 levels)
        if total_level % 4 == 0:
            gains['ability_score'] = True
            
        # 4. Spells
        is_spellcaster = self.field_mapper.get_field_value(new_class, 'spellcaster', '0') == '1'
        # Also check for specific spell fields if generic flag missing
        if not is_spellcaster:
            # Check has_arcane, has_divine fields which might be present in some schemas
            pass 
        
        # Prestige Class Spellcasting Advancement Logic
        # Check if it's a "Prestige Caster" (Has casting, but no table - usually means advances others)
        spell_gain_table = self.field_mapper.get_field_value(new_class, 'spell_gain_table', '')
        is_prestige_caster = is_spellcaster and (not spell_gain_table or spell_gain_table == '****')
        
        if is_prestige_caster:
            # Find the best candidate Base Class to advance (Highest Level Caster)
            best_base_class_entry = None
            max_lvl = -1
            
            for c_entry in self.gff.get('ClassList', []):
                # Don't advance self if we just added it (though PrC usually isn't its own base)
                # Don't advance other PrCs (usually) - check if they have tables?
                # For simplicity/robustness, pick highest level class that HAS a spell table
                c_id = c_entry.get('Class')
                c_data = self.rules_service.get_by_id('classes', c_id)
                if c_data:
                    c_table = self.field_mapper.get_field_value(c_data, 'spell_gain_table', '')
                    if c_table and c_table != '****':
                        # Valid base caster
                        lvl = c_entry.get('ClassLevel', 0)
                        if lvl > max_lvl:
                            max_lvl = lvl
                            best_base_class_entry = c_entry
            
            if best_base_class_entry:
                # Increment effective caster level
                # Current effective level is SpellCasterLevel (if set) OR ClassLevel
                current_cl = best_base_class_entry.get('ClassLevel', 0)
                current_scl = best_base_class_entry.get('SpellCasterLevel')
                
                effective_level = current_cl
                if current_scl is not None:
                    try:
                        effective_level = int(current_scl)
                    except:
                        pass
                
                # Add 1 for the new PrC level
                new_scl = effective_level + 1
                best_base_class_entry['SpellCasterLevel'] = new_scl
                logger.info(f"Prestige Class Advancement: Increased effective caster level of Class {best_base_class_entry.get('Class')} to {new_scl}")
                
        # Simplified spellcaster check based on common fields
        gains['new_spells'] = is_spellcaster

        return {
            'class_id': class_id,
            'new_total_level': total_level,
            'multiclass': not class_found,
            'gains': gains
        }
    
    def adjust_class_level(self, class_id: int, level_change: int) -> Dict[str, Any]:
        """
        Adjust levels in a specific class (add or remove levels)
        
        Args:
            level_change: Number of levels to add (+) or remove (-)
            
        Returns:
            Dict with changes made
        """
        if level_change == 0:
            return {}
        
        if level_change > 0:
            # Add levels one by one
            changes = {}
            accumulated_gains = {
                'skill_points': 0,
                'feats': 0,
                'bonus_feats': 0,
                'bonus_label': '',
                'ability_score': False,
                'new_spells': False
            }
            
            for _ in range(level_change):
                result = self.add_class_level(class_id)
                changes = result # Keep last result for structure
                
                # Accumulate gains
                if 'gains' in result:
                    g = result['gains']
                    accumulated_gains['skill_points'] += g.get('skill_points', 0)
                    accumulated_gains['total_skill_points'] = g.get('total_skill_points', 0)
                    accumulated_gains['feats'] += g.get('feats', 0)
                    accumulated_gains['bonus_feats'] += g.get('bonus_feats', 0)
                    if g.get('bonus_label'):
                        accumulated_gains['bonus_label'] = g['bonus_label']
                    accumulated_gains['ability_score'] = accumulated_gains['ability_score'] or g.get('ability_score', False)
                    accumulated_gains['new_spells'] = accumulated_gains['new_spells'] or g.get('new_spells', False)
            
            changes['gains'] = accumulated_gains
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
                            levels_removed = current_level - new_level
                            class_entry['ClassLevel'] = new_level
                            self.gff.set('ClassList', class_list)

                            # Remove level history entries for the removed levels
                            self._remove_level_history_for_class(class_id, levels_removed)

                            # Calculate new total level from LvlStatList
                            lvl_stat_list = self.gff.get('LvlStatList', [])
                            total_level = len(lvl_stat_list)

                            # Auto-adjust XP when leveling down
                            min_xp_current = self.level_to_xp(total_level)
                            next_level_xp = self.level_to_xp(total_level + 1)
                            current_xp = self.get_experience()
                            
                            # If we have enough XP for the level we just lost (or more), reduce it
                            # This prevents the "XP Level differs from Class Level" warning
                            if current_xp >= next_level_xp:
                                logger.info(f"Auto-adjusting XP down from {current_xp} to {min_xp_current} for level {total_level}")
                                self.set_experience(min_xp_current)
                            
                            # No stat updates needed when removing levels - stats recalculated on demand
                            changes = {}

                            # Emit class changed event to notify other managers
                            event = ClassChangedEvent(
                                event_type=EventType.CLASS_CHANGED,
                                source_manager='class',
                                timestamp=time.time(),
                                old_class_id=class_id,
                                new_class_id=class_id,  # Same class, different level
                                level=total_level,
                                preserve_feats=self._get_preserved_feats(),
                                is_level_adjustment=True  # Level change only, not class swap
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
        Change a specific class in a multiclass character without affecting other classes.
        
        NOTE: This operation now strictly resets the class level to 1 and clears history for the old class,
        ignoring 'preserve_level' if it conflicts with the new behavior of cleanly swapping classes.
        
        Args:
            old_class_id: The class ID to replace
            new_class_id: The new class ID
            preserve_level: Deprecated/Ignored. Logic now resets to level 1.
            
        Returns:
            Dict with change details
        """
        logger.info(f"Changing specific class from {old_class_id} to {new_class_id} (Clean Swap)")
        
        # Validate new class exists
        new_class = self.rules_service.get_by_id('classes', new_class_id)
        if not new_class:
            raise ValueError(f"Invalid class ID: {new_class_id}")
        
        class_list = self.gff.get('ClassList', [])
        
        # Find the class index to change
        class_index = -1
        for i, class_entry in enumerate(class_list):
            if class_entry.get('Class') == old_class_id:
                class_index = i
                break
        
        if class_index == -1:
            raise ValueError(f"Character does not have class {old_class_id}")
        
        # Begin transaction if not already in one
        transaction_started = False
        if not self.character_manager._current_transaction:
            self.character_manager.begin_transaction()
            transaction_started = True
        
        try:
            # 1. Remove old class features/feats before wiping history
            self._remove_class_features_and_feats(old_class_id)
            
            # 2. Remove old class history
            self._remove_class_from_history(old_class_id)
            
            # 3. Update the class list entry to be a fresh level 0 entry
            # We recreate it to ensure no leftover data from old class (like spells, domain, etc.)
            class_list = self.gff.get('ClassList', []) # Re-get to be safe
            class_list[class_index] = {
                'Class': new_class_id,
                'ClassLevel': 0
            }
            self.gff.set('ClassList', class_list)
            
            # Update primary class if it was changed
            current_primary = self.gff.get('Class', 0)
            if current_primary == old_class_id:
                self.gff.set('Class', new_class_id)

            # 4. Advance to Level 1 to grant initial features
            # This triggers add_class_level -> _record_level_up -> _calculate_feat_slots -> grants feats/skills
            result = self.add_class_level(new_class_id)
            
            # Recalculate all stats since classes changed
            self._recalculate_all_stats()

            # Emit class changed event
            event = ClassChangedEvent(
                event_type=EventType.CLASS_CHANGED,
                source_manager='class',
                timestamp=time.time(),
                old_class_id=old_class_id,
                new_class_id=new_class_id,
                level=1,
                preserve_feats=self._get_preserved_feats()
            )
            self.character_manager.emit(event)
            
            # Commit transaction if we started it
            if transaction_started:
                self.character_manager.commit_transaction()
                
            return {
                'class_change': {
                    'old_class': old_class_id,
                    'new_class': new_class_id,
                    'level': 1
                },
                'multiclass_preserved': True,
                'total_level': sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', [])),
                'stats_updated': True,
                'details': result
            }
                
        except Exception as e:
            # Rollback on error if we started the transaction
            if transaction_started:
                self.character_manager.rollback_transaction()
            logger.error(f"Error during specific class change: {e}")
            raise
    
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
    
    def _calculate_historical_skill_points(self) -> Dict[int, int]:
        """
        Reconstruct skill points per class by extracting ranks from history entries.
        This is the most accurate method as it reflects what's actually in the save file.
        """
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not lvl_stat_list:
            return {}

        class_points = {} # class_id -> total points
        
        for entry in lvl_stat_list:
            class_id = entry.get(LVL_STAT_CLASS, -1)
            if class_id == -1: continue
            
            # Aggregate ranks from the SkillList inside this level entry
            # In LvlStatList.SkillList, each entry is {Skill: ID, Rank: PointsSpent}
            skill_list = entry.get(LVL_STAT_SKILL_LIST, [])
            points_this_level = sum(s.get("Rank", 0) for s in skill_list if isinstance(s, dict))
            
            # Fallback: if no ranks were spent, we can look at awarded points,
            # though usually we want total spent for the Classes tab breakdown.
            # Using the assigned SkillPoints from history entry if Rank sum is 0
            # can help in edge cases where history is partially populated.
            if points_this_level == 0:
                points_this_level = entry.get(LVL_STAT_SKILL_POINTS, 0)
                
            class_points[class_id] = class_points.get(class_id, 0) + points_this_level
            
        return class_points

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
        # Return 0 for invalid levels
        if level <= 0:
            return 0
            
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
        if level_idx >= 0 and level_idx < len(bab_table):
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
    
    def calculate_total_saves(self) -> Dict[str, int]:
        """Legacy shim - Delegates to SaveManager for consistent calculations"""
        save_manager = self.character_manager.get_manager('save')
        if not save_manager:
            logger.warning("SaveManager not available, returning zeroed saves")
            return {
                'fortitude': 0, 'reflex': 0, 'will': 0,
                'base_fortitude': 0, 'base_reflex': 0, 'base_will': 0
            }
            
        saves = save_manager.calculate_saving_throws()
        return {
            'fortitude': saves['fortitude']['total'],
            'reflex': saves['reflex']['total'],
            'will': saves['will']['total'],
            'base_fortitude': saves['fortitude']['base'],
            'base_reflex': saves['reflex']['base'],
            'base_will': saves['will']['base']
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
        
        # Calculate historical points first
        historical_points = self._calculate_historical_skill_points()
        
        classes = []
        for i, c in enumerate(class_list):
            class_id = c.get('Class', 0)
            class_data = self.rules_service.get_by_id('classes', class_id)
            
            # Safely get class name
            if class_data:
                class_name = self.field_mapper.get_field_value(class_data, 'label', 
                    self.field_mapper.get_field_value(class_data, 'name', f"Unknown Class {class_id}"))
            else:
                class_name = f"Unknown Class {class_id}"
            
            # Determine skill points
            # Priority 1: Historical Calculation (accurate for level-by-level stats)
            if class_id in historical_points:
                skill_points_display = historical_points[class_id]
            else:
                # Priority 2: Fallback Estimate (if history missing or class mismatch)
                skill_points_display = 0
                if class_data:
                    modifiers = self._calculate_ability_modifiers()
                    int_mod = modifiers['INT']
                    base_sp = self.field_mapper._safe_int(
                        self.field_mapper.get_field_value(class_data, 'skill_point_base', 2), 2
                    )
                    
                    skill_manager = self.character_manager.get_manager('skill')
                    racial_bonus = 0
                    if skill_manager:
                         racial_bonus = skill_manager._get_racial_skill_point_bonus_base()
                    
                    level = c.get('ClassLevel', 0)
                    sp_per_level = max(1, base_sp + int_mod + racial_bonus)
                    skill_points_display = sp_per_level * level
                    
                    if i == 0:
                         skill_points_display += (sp_per_level * 3)

            # Calculate per-class BAB
            class_level = c.get('ClassLevel', 0)
            class_bab = 0
            if class_data and class_level > 0:
                class_bab = self._calculate_bab(class_data, class_level)
            
            # Calculate per-class saves
            class_fort = 0
            class_ref = 0
            class_will = 0
            if class_data and class_level > 0:
                save_manager = self.character_manager.get_manager('save')
                if save_manager:
                    class_saves = save_manager._calculate_base_save_delta(class_data, class_level)
                    class_fort = class_saves.get('fortitude', 0)
                    class_ref = class_saves.get('reflex', 0)
                    class_will = class_saves.get('will', 0)
            
            # Get hit die
            hit_die = 8  # Default
            if class_data:
                hit_die = self.field_mapper._safe_int(
                    self.field_mapper.get_field_value(class_data, 'hit_die', 8), 8
                )
            
            classes.append({
                'id': class_id,
                'level': class_level,
                'name': class_name,
                'skill_points': skill_points_display,
                'bab': class_bab,
                'fort_save': class_fort,
                'ref_save': class_ref,
                'will_save': class_will,
                'hit_die': hit_die,
                '_raw_calc': skill_points_display
            })

        # Normalize skill points to match the actual total (Spent + Available)
        # This handles manual overrides or discrepancies in history
        skill_manager = self.character_manager.get_manager('skill')
        if skill_manager:
            available_pts = self.gff.get('SkillPoints', 0)
            spent_pts = skill_manager._calculate_spent_skill_points()
            actual_total = available_pts + spent_pts
            
            calculated_total = sum(c['_raw_calc'] for c in classes)
            
            # Only normalize if we have a discrepancy
            if calculated_total > 0 and actual_total != calculated_total:
                ratio = actual_total / calculated_total
                current_sum = 0
                max_points = -1
                max_idx = 0
                
                for idx, c in enumerate(classes):
                    scaled = int(round(c['_raw_calc'] * ratio))
                    c['skill_points'] = scaled
                    current_sum += scaled
                    
                    if c['_raw_calc'] > max_points:
                        max_points = c['_raw_calc']
                        max_idx = idx
                
                # Adjust rounding error on the largest class
                diff = actual_total - current_sum
                if diff != 0:
                     classes[max_idx]['skill_points'] += diff

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
        """Get list of racial feats for a given race"""
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

    def _get_background_feat(self) -> Optional[int]:
        """Get background feat ID if present"""
        return None

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

    # =========================================================================================
    def _record_level_up(self, class_id: int, hp_gained: int):
        """
        Create a new entry in the LvlStatList history for the new level.
        Called when a level is added.
        """
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not isinstance(lvl_stat_list, list):
            lvl_stat_list = []
        
        new_entry = {
            LVL_STAT_CLASS: class_id,
            LVL_STAT_HITDIE: hp_gained,
            LVL_STAT_SKILL_POINTS: 0,
            LVL_STAT_FEAT_LIST: [],
            LVL_STAT_SKILL_LIST: []
        }
        
        for i in range(10):
            new_entry[f"{LVL_STAT_KNOWN_LIST}{i}"] = []
            new_entry[f"{LVL_STAT_KNOWN_REMOVE_LIST}{i}"] = []

        lvl_stat_list.append(new_entry)
        self.gff.set(LVL_STAT_LIST, lvl_stat_list)
        logger.info(f"Recorded level up history: Class {class_id}, HP {hp_gained}")

    def on_skill_points_awarded(self, event: Any):
        """
        Handle SKILL_POINTS_AWARDED event by updating the latest level history entry.
        """
        if not hasattr(event, 'points'):
            return

        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not lvl_stat_list or not isinstance(lvl_stat_list, list):
            return

        # Update the last entry (which should be the one just added)
        last_entry = lvl_stat_list[-1]
        
        # Verify class matches if possible, but usually it's the last one anyway
        if last_entry.get(LVL_STAT_CLASS) == event.class_id:
            last_entry[LVL_STAT_SKILL_POINTS] = event.points
            self.gff.set(LVL_STAT_LIST, lvl_stat_list)
            logger.info(f"Updated history entry with awarded skill points: {event.points}")

    def _remove_level_history_for_class(self, class_id: int, count: int = 1):
        """
        Remove the last N entries for a specific class from LvlStatList.
        Also removes the feats, skill points, and ability increases that were
        gained at those levels, based on what's recorded in the history.

        Args:
            class_id: The class ID whose entries to remove
            count: Number of level entries to remove for this class
        """
        if count <= 0:
            return

        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not isinstance(lvl_stat_list, list) or not lvl_stat_list:
            return

        # Get managers for cleanup
        feat_manager = self.character_manager.get_manager('feat')
        skill_manager = self.character_manager.get_manager('skill')
        
        # Get protected feats
        racial_feats = set()
        race_manager = self.character_manager.get_manager('race')
        if race_manager:
            racial_feats = set(race_manager.get_all_racial_feats())

        removed_count = 0
        feats_removed = 0
        skills_refunded = 0
        ability_removed = False
        
        # Process entries in reverse order (most recent first)
        for i in range(len(lvl_stat_list) - 1, -1, -1):
            if removed_count >= count:
                break
            
            entry = lvl_stat_list[i]
            if entry.get(LVL_STAT_CLASS) != class_id:
                continue
            
            logger.info(f"Removing level history entry at index {i} for class {class_id}")
            
            # 1. Remove feats gained at this level
            feat_list = entry.get(LVL_STAT_FEAT_LIST, [])
            if isinstance(feat_list, list) and feat_manager:
                for feat_entry in feat_list:
                    feat_id = feat_entry.get('Feat')
                    if feat_id is None:
                        continue
                    
                    # Check protections
                    if feat_id in racial_feats:
                        logger.debug(f"Skipping removal of racial feat {feat_id}")
                        continue
                    if self._is_background_feat(feat_id):
                        logger.debug(f"Skipping removal of background feat {feat_id}")
                        continue
                    
                    if feat_manager.remove_feat(feat_id, force=True):
                        feats_removed += 1
                        logger.debug(f"Removed feat {feat_id} from level {i+1}")
            
            # 2. Refund skill points spent at this level
            skill_list_entry = entry.get('SkillList', [])
            if isinstance(skill_list_entry, list) and skill_manager:
                for skill_idx, skill_entry in enumerate(skill_list_entry):
                    if isinstance(skill_entry, dict):
                        ranks_spent = skill_entry.get('Rank', 0)
                        if ranks_spent > 0:
                            # Subtract ranks from the character's skill list
                            current_ranks = skill_manager.get_skill_ranks(skill_idx)
                            new_ranks = max(0, current_ranks - ranks_spent)
                            
                            # Directly update the skill list without triggering events
                            char_skill_list = self.gff.get('SkillList', [])
                            if skill_idx < len(char_skill_list):
                                if isinstance(char_skill_list[skill_idx], dict):
                                    char_skill_list[skill_idx]['Rank'] = new_ranks
                                    self.gff.set('SkillList', char_skill_list)
                                    skills_refunded += ranks_spent
                                    logger.debug(f"Removed {ranks_spent} ranks from skill {skill_idx}")
            
            # 3. Remove ability increase if recorded at this level
            ability_idx = entry.get('LvlStatAbility')
            if ability_idx is not None and ability_idx >= 0:
                # Map index to attribute name
                ability_map = {0: 'Str', 1: 'Dex', 2: 'Con', 3: 'Int', 4: 'Wis', 5: 'Cha'}
                attr_name = ability_map.get(ability_idx)
                if attr_name:
                    current_val = self.gff.get(attr_name, 10)
                    new_val = max(3, current_val - 1)  # Subtract the +1 increase
                    self.gff.set(attr_name, new_val)
                    ability_removed = True
                    logger.info(f"Removed ability increase: {attr_name} {current_val} -> {new_val}")
            
            # 4. Reduce HP based on hit die roll at this level
            hit_die_roll = entry.get('LvlStatHitDie', 0)
            if hit_die_roll > 0:
                # Calculate HP reduction: hit die roll + CON modifier
                con_score = self.gff.get('Con', 10)
                con_mod = (con_score - 10) // 2
                hp_reduction = hit_die_roll + con_mod
                hp_reduction = max(1, hp_reduction)  # Minimum 1 HP per level
                
                # Reduce max and current HP
                current_max_hp = self.gff.get('MaxHitPoints', 0)
                current_hp = self.gff.get('CurrentHitPoints', 0)
                
                new_max_hp = max(1, current_max_hp - hp_reduction)
                new_current_hp = max(1, min(current_hp - hp_reduction, new_max_hp))
                
                self.gff.set('MaxHitPoints', new_max_hp)
                self.gff.set('CurrentHitPoints', new_current_hp)
                self.gff.set('HitPoints', new_max_hp)  # Legacy field
                
                logger.info(f"Reduced HP by {hp_reduction} (die roll {hit_die_roll} + CON mod {con_mod}): "
                           f"Max {current_max_hp} -> {new_max_hp}, Current {current_hp} -> {new_current_hp}")
            
            # 5. Remove spells learned at this level
            # Spells are stored in ClassList -> class entry -> KnownList0 through KnownList9
            spells_removed = 0
            for spell_level in range(10):  # Spell levels 0-9
                known_list_key = f'KnownList{spell_level}'
                known_list_from_history = entry.get(known_list_key, [])
                if not isinstance(known_list_from_history, list) or not known_list_from_history:
                    continue
                
                # Find the class entry in ClassList and remove these spells
                class_list = self.gff.get('ClassList', [])
                for class_entry in class_list:
                    if class_entry.get('Class') != class_id:
                        continue
                    
                    # Get the known spells for this level from the class entry
                    class_known_list = class_entry.get(known_list_key, [])
                    if not isinstance(class_known_list, list):
                        continue
                    
                    # Remove each spell from history from the class known list
                    for spell_entry in known_list_from_history:
                        spell_id = spell_entry.get('Spell')
                        if spell_id is None:
                            continue
                        
                        # Find and remove the spell from the class known list
                        for j in range(len(class_known_list) - 1, -1, -1):
                            if class_known_list[j].get('Spell') == spell_id:
                                class_known_list.pop(j)
                                spells_removed += 1
                                logger.debug(f"Removed spell {spell_id} (level {spell_level})")
                                break
                    
                    # Update the class entry
                    class_entry[known_list_key] = class_known_list
                
                # Save the updated class list
                self.gff.set('ClassList', class_list)
            
            if spells_removed > 0:
                logger.info(f"Removed {spells_removed} spells from level {i+1}")
            
            # 6. Now remove the history entry
            lvl_stat_list.pop(i)
            removed_count += 1

        self.gff.set(LVL_STAT_LIST, lvl_stat_list)
        
        # Log summary
        if removed_count > 0:
            logger.info(f"Level-down cleanup for class {class_id}: Removed {removed_count} levels, "
                       f"{feats_removed} feats, refunded {skills_refunded} skill ranks, "
                       f"ability removed: {ability_removed}")

    def _remove_class_features_and_feats(self, class_id: int):
        """
        Remove feats and features associated with a class from the character's active abilities.
        Searches level history to identify feats gained from this class.
        
        Args:
            class_id: The class ID to remove features for
        """
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not isinstance(lvl_stat_list, list) or not lvl_stat_list:
            return

        feat_manager = self.character_manager.get_manager('feat')
        if not feat_manager:
            logger.warning("FeatManager not available for class removal cleanup")
            return

        # Get racial feats to protect them from deletion
        racial_feats = set()
        race_manager = self.character_manager.get_manager('race')
        if race_manager:
            racial_feats = set(race_manager.get_all_racial_feats())
            logger.info(f"_remove_class_features_and_feats: Protected racial feats set: {racial_feats}")
        else:
            logger.warning("_remove_class_features_and_feats: RaceManager not available!")
        
        count_removed = 0
        count_protected = 0
        for entry in lvl_stat_list:
            if entry.get(LVL_STAT_CLASS) == class_id:
                # Remove feats gained at this level
                feat_list = entry.get(LVL_STAT_FEAT_LIST, [])
                if isinstance(feat_list, list):
                    for feat_entry in feat_list:
                        feat_id = feat_entry.get('Feat')
                        if feat_id is not None:
                            # Check protections
                            if feat_id in racial_feats:
                                logger.debug(f"Skipping removal of racial feat {feat_id}")
                                count_protected += 1
                                continue
                                
                            if self._is_background_feat(feat_id):
                                logger.debug(f"Skipping removal of background feat {feat_id}")
                                count_protected += 1
                                continue

                            feat_manager.remove_feat(feat_id)
                            count_removed += 1
        
        if count_removed > 0 or count_protected > 0:
            logger.info(f"Cleanup for class {class_id}: Removed {count_removed} feats, Protected {count_protected} racial/background feats")
            
    def _remove_class_from_history(self, class_id: int) -> int:
        """
        Remove all LvlStatList entries for a specific class.
        Used when a class is completely removed from the character.

        Args:
            class_id: The class ID to remove from history

        Returns:
            Number of entries removed
        """
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not isinstance(lvl_stat_list, list) or not lvl_stat_list:
            return 0

        original_count = len(lvl_stat_list)
        lvl_stat_list = [entry for entry in lvl_stat_list if entry.get(LVL_STAT_CLASS) != class_id]
        removed_count = original_count - len(lvl_stat_list)

        if removed_count > 0:
            self.gff.set(LVL_STAT_LIST, lvl_stat_list)
            logger.info(f"Removed {removed_count} level history entries for class {class_id}")

        return removed_count

    def _update_class_in_history(self, old_class_id: int, new_class_id: int) -> int:
        """
        Replace all occurrences of a class ID in LvlStatList with a new class ID.
        Used when a class is swapped for another (e.g., Cleric -> Barbarian).

        Args:
            old_class_id: The class ID to replace
            new_class_id: The new class ID

        Returns:
            Number of entries updated
        """
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not isinstance(lvl_stat_list, list) or not lvl_stat_list:
            return 0

        updated_count = 0
        for entry in lvl_stat_list:
            if entry.get(LVL_STAT_CLASS) == old_class_id:
                entry[LVL_STAT_CLASS] = new_class_id
                updated_count += 1

        if updated_count > 0:
            self.gff.set(LVL_STAT_LIST, lvl_stat_list)
            logger.info(f"Updated {updated_count} level history entries from class {old_class_id} to {new_class_id}")

        return updated_count

    def record_feat_change(self, feat_id: int, added: bool):
        """
        Sync a feat change to the current level history.
        Called by FeatManager when feats are added/removed.
        
        Args:
            feat_id: The ID of the feat
            added: True if added, False if removed
        """
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not lvl_stat_list or not isinstance(lvl_stat_list, list):
            logger.warning("No level history found to sync feat change.")
            return

        current_level_idx = len(lvl_stat_list) - 1
        current_level_entry = lvl_stat_list[current_level_idx]

        feat_list = current_level_entry.get(LVL_STAT_FEAT_LIST, [])
        if not isinstance(feat_list, list):
            feat_list = []

        if added:
            if not any(f.get('Feat') == feat_id for f in feat_list):
                feat_list.append({'Feat': feat_id})
                logger.info(f"Synced feat {feat_id} addition to level history.")
        else:
            original_len = len(feat_list)
            feat_list = [f for f in feat_list if f.get('Feat') != feat_id]
            if len(feat_list) < original_len:
                logger.info(f"Synced feat {feat_id} removal from level history.")
        
        current_level_entry[LVL_STAT_FEAT_LIST] = feat_list
        self.gff.set(LVL_STAT_LIST, lvl_stat_list)

    def record_skill_change(self, skill_id: int, rank_delta: int):
        """
        Sync a skill rank change to the current level history.
        Called by SkillManager when skills are modified.
        
        Args:
            skill_id: The ID of the skill
            rank_delta: The change in rank (+1, -1, etc.)
        """
        if rank_delta == 0:
            return

        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not lvl_stat_list or not isinstance(lvl_stat_list, list):
            logger.warning("No level history found to sync skill change.")
            return

        current_level_idx = len(lvl_stat_list) - 1
        current_level_entry = lvl_stat_list[current_level_idx]

        skill_list = current_level_entry.get(LVL_STAT_SKILL_LIST, [])
        if not isinstance(skill_list, list):
            skill_list = []

        current_count = len(skill_list)
        if skill_id >= current_count:
            for _ in range(skill_id - current_count + 1):
                skill_list.append({'Rank': 0})

        current_rank_entry = skill_list[skill_id]
        if isinstance(current_rank_entry, dict):
            new_rank = current_rank_entry.get('Rank', 0) + rank_delta
            current_rank_entry['Rank'] = new_rank
            logger.debug(f"Synced skill {skill_id} rank change ({rank_delta}) to history. Level delta: {new_rank}")
        else:
            logger.warning(f"Invalid skill history entry format at index {skill_id}")
            
        current_level_entry[LVL_STAT_SKILL_LIST] = skill_list
        self.gff.set(LVL_STAT_LIST, lvl_stat_list)
    
    def record_ability_change(self, ability_index: int):
        """
        Record an ability score increase in the current level history.
        NOTE: LvlStatAbility only stores a single index (0-5) per level.
        It cannot store multiple increases or the amount increased.
        
        Args:
            ability_index: 0=STR, 1=DEX, 2=CON, 3=INT, 4=WIS, 5=CHA
        """
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not lvl_stat_list or not isinstance(lvl_stat_list, list):
            logger.warning("No level history found to sync ability change.")
            return

        current_level_idx = len(lvl_stat_list) - 1
        current_level_entry = lvl_stat_list[current_level_idx]
        
        # We can only store one ability increase per level in the standard format
        # User Rule: Only save the first change to the lvlstatlist and ignore the rest
        if current_level_entry.get(LVL_STAT_ABILITY) is not None:
             logger.info("Ability increase already recorded for this level. Ignoring new record to preserve history consistency.")
             return

        current_level_entry[LVL_STAT_ABILITY] = ability_index
        
        logger.info(f"Recorded ability increase (Index {ability_index}) to level history index {current_level_idx}")
        self.gff.set(LVL_STAT_LIST, lvl_stat_list)

    def get_spent_ability_points(self) -> int:
        """
        Calculate total ability points spent based on level history.
        Counts how many levels have an ability increase recorded.
        """
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not lvl_stat_list or not isinstance(lvl_stat_list, list):
            return 0
            
        spent = 0
        for entry in lvl_stat_list:
            # LvlStatAbility is an index (0-5) if present, None if not
            if entry.get(LVL_STAT_ABILITY) is not None:
                spent += 1
        
        return spent
    
    
    def record_spell_change(self, spell_level: int, spell_id: int, added: bool):
        """
        Sync a spell change to the current level history.
        Called by SpellManager when spells are added/removed.
        """
        if spell_level < 0 or spell_level > 9:
            logger.warning(f"Invalid spell level {spell_level} for history sync.")
            return

        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not lvl_stat_list or not isinstance(lvl_stat_list, list):
            logger.warning("No level history found to sync spell change.")
            return

        current_level_idx = len(lvl_stat_list) - 1
        current_level_entry = lvl_stat_list[current_level_idx]

        if added:
            list_key = f"{LVL_STAT_KNOWN_LIST}{spell_level}"
            spell_list = current_level_entry.get(list_key, [])
            if not isinstance(spell_list, list):
                spell_list = []

            if not any(s.get('Spell') == spell_id for s in spell_list):
                spell_list.append({'Spell': spell_id})
                logger.debug(f"Synced spell {spell_id} (level {spell_level}) addition to level history.")

            current_level_entry[list_key] = spell_list
        else:
            list_key = f"{LVL_STAT_KNOWN_REMOVE_LIST}{spell_level}"
            remove_list = current_level_entry.get(list_key, [])
            if not isinstance(remove_list, list):
                remove_list = []

            if not any(s.get('Spell') == spell_id for s in remove_list):
                remove_list.append({'Spell': spell_id})
                logger.debug(f"Synced spell {spell_id} (level {spell_level}) removal to level history.")

            current_level_entry[list_key] = remove_list

        self.gff.set(LVL_STAT_LIST, lvl_stat_list)
    
    def get_level_history(self) -> List[Dict[str, Any]]:
        """
        Get the full level up history for display.
        Parses LvlStatList and resolves IDs to human-readable names.
        """
        history_data = []
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not lvl_stat_list:
            return []

        class_level_counts: Dict[int, int] = {}

        for level_idx, entry in enumerate(lvl_stat_list):
            class_id = entry.get(LVL_STAT_CLASS, -1)
            class_level_counts[class_id] = class_level_counts.get(class_id, 0) + 1

            level_info = {
                'level': level_idx + 1,
                'class': 'Unknown',
                'class_level': class_level_counts[class_id],
                'hp_gained': entry.get(LVL_STAT_HITDIE, 0),
                'skill_points_remaining': entry.get(LVL_STAT_SKILL_POINTS, 0),
                'ability_increase': None,
                'skills_gained': [],
                'feats_gained': [],
                'spells_learned': [],
                'spells_removed': [],
            }

            level_info['class'] = self._get_rule_label('classes', class_id, f"Class {class_id}")
            
            ability_id = entry.get(LVL_STAT_ABILITY)
            if ability_id is not None:
                abilities = ['Strength', 'Dexterity', 'Constitution', 'Intelligence', 'Wisdom', 'Charisma']
                if 0 <= ability_id < len(abilities):
                    level_info['ability_increase'] = abilities[ability_id]

            skill_list = entry.get(LVL_STAT_SKILL_LIST, [])
            for skill_id, skill_entry in enumerate(skill_list):
                rank = skill_entry.get('Rank', 0)
                if rank != 0:
                    skill_name = self._get_rule_label('skills', skill_id, f"Skill {skill_id}")
                    level_info['skills_gained'].append({'name': skill_name, 'rank': rank})

            feat_list = entry.get(LVL_STAT_FEAT_LIST, [])
            feat_manager = self.character_manager.get_manager('feat')
            for feat_entry in feat_list:
                feat_id = feat_entry.get('Feat', -1)
                if feat_id >= 0 and feat_manager:
                    feat_info = feat_manager.get_feat_info_display(feat_id)
                    feat_name = feat_info.get('name', f"Feat {feat_id}") if feat_info else f"Feat {feat_id}"
                else:
                    feat_name = f"Feat {feat_id}"
                level_info['feats_gained'].append({'name': feat_name})

            spell_manager = self.character_manager.get_manager('spell')
            for i in range(10):
                known_list = entry.get(f"{LVL_STAT_KNOWN_LIST}{i}", [])
                for spell_entry in known_list:
                    spell_id = spell_entry.get('Spell', -1)
                    if spell_id >= 0 and spell_manager:
                        spell_info = spell_manager.get_spell_details(spell_id)
                        spell_name = spell_info.get('name', f"Spell {spell_id}")
                    else:
                        spell_name = f"Spell {spell_id}"
                    level_info['spells_learned'].append({'name': spell_name, 'level': i})

                remove_list = entry.get(f"{LVL_STAT_KNOWN_REMOVE_LIST}{i}", [])
                for spell_entry in remove_list:
                    spell_id = spell_entry.get('Spell', -1)
                    if spell_id >= 0 and spell_manager:
                        spell_info = spell_manager.get_spell_details(spell_id)
                        spell_name = spell_info.get('name', f"Spell {spell_id}")
                    else:
                        spell_name = f"Spell {spell_id}"
                    level_info['spells_removed'].append({'name': spell_name, 'level': i})

            history_data.append(level_info)
            
        return history_data

    def _get_rule_label(self, rule_type: str, rule_id: int, default: str) -> str:
        """Helper to get a human-readable label from game rules"""
        if rule_id is None or rule_id < 0:
            return default

        try:
            data = self.rules_service.get_by_id(rule_type, rule_id)
            if data:
                return self.field_mapper.get_field_value(data, 'label',
                       self.field_mapper.get_field_value(data, 'name', default))
        except Exception:
            pass
        return default

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

            # Remove feats and features associated with this class
            self._remove_class_features_and_feats(class_id)

            # Remove all level history entries for this class
            self._remove_class_from_history(class_id)

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
        # Use field_mapper to handle snake_case to PascalCase conversion (feats_table -> FeatsTable)
        feat_table_name = self.field_mapper.get_field_value(class_data, 'feats_table')
        if not feat_table_name:
            # logger.debug(f"Class {getattr(class_data, 'label', 'Unknown')} has no feat table")
            return feats_for_level
        
        # Load the feat table
        feat_table = self.rules_service.get_table(feat_table_name.lower())
        if not feat_table:
            # logger.warning(f"Feat table {feat_table_name} not found")
            return feats_for_level
        
        # Look for feats at this level
        # Class feat tables have columns like FeatIndex, GrantedOnLevel, List
        for feat_entry in feat_table:
            # Use field_mapper for all column access to handle case sensitivity
            granted_level_raw = self.field_mapper.get_field_value(feat_entry, 'granted_on_level', -1)
            
            # Handle potential string '****' which means None/Minus One
            try:
                if isinstance(granted_level_raw, str):
                     if granted_level_raw == '****':
                         granted_level = -1
                     else:
                         granted_level = int(granted_level_raw)
                else:
                    granted_level = int(granted_level_raw)
            except ValueError:
                granted_level = -1

            if granted_level == level:
                feat_id = self.field_mapper.get_field_value(feat_entry, 'feat_index', -1)
                # Ensure int
                try:
                    feat_id = int(feat_id)
                except (ValueError, TypeError):
                    feat_id = -1
                    
                list_type = self.field_mapper.get_field_value(feat_entry, 'list', 3)  # Default to general list
                try:
                    list_type = int(list_type)
                except (ValueError, TypeError):
                    list_type = 3
                
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
            # Use field_mapper for consistent access
            ability_table_name = self.field_mapper.get_field_value(class_data, 'ability_table')
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

    # =========================================================================================
    # Level-Up System Methods
    # =========================================================================================

    def get_level_up_requirements(self, class_id: int) -> Dict[str, Any]:
        """
        Calculate all requirements and options for leveling up in a class.
        Used by the level-up check endpoint.

        Args:
            class_id: Class ID to level up in

        Returns:
            Dict with all level-up information
        """
        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            raise ValueError(f"Invalid class ID: {class_id}")

        class_list = self.gff.get('ClassList', [])
        total_level = sum(c.get('ClassLevel', 0) for c in class_list)
        new_level = total_level + 1

        # Get class level for this specific class
        class_level = 0
        for c in class_list:
            if c.get('Class') == class_id:
                class_level = c.get('ClassLevel', 0)
                break
        new_class_level = class_level + 1

        # HP calculation - maximum HP (full hit die + CON mod)
        hit_die = self.field_mapper._safe_int(
            self.field_mapper.get_field_value(class_data, 'hit_die', 8), 8
        )
        modifiers = self._calculate_ability_modifiers()
        hp_gain = max(1, hit_die + modifiers['CON'])

        # Skill points
        skill_manager = self.character_manager.get_manager('skill')
        skill_points = 0
        if skill_manager:
            skill_points = skill_manager.calculate_skill_points_for_level(class_data, modifiers['INT'])

        # Feat slots
        feat_slots = self._calculate_feat_slots_for_level(new_level, class_id, new_class_level)
        if feat_slots['bonus'] > 0:
            feat_slots['bonus_label'] = self._get_bonus_feat_label(class_id, new_level)

        # Ability increase at levels 4, 8, 12, etc.
        has_ability_increase = (new_level % 4 == 0)

        # Current abilities for display
        ability_manager = self.character_manager.get_manager('ability')
        current_abilities = {}
        if ability_manager:
            attrs = ability_manager.get_attributes(include_equipment=False)
            current_abilities = attrs

        # Spell slots for casters
        spell_manager = self.character_manager.get_manager('spell')
        spell_slots_gained = {}
        available_spells = {}
        is_spellcaster = False
        if spell_manager and spell_manager.is_spellcaster(class_id):
            is_spellcaster = True
            spell_slots_gained = self._calculate_spell_slots_gained(class_id, new_class_level)
            available_spells = self._get_available_spells_for_level(class_id, new_class_level)

        # Available feats - only feats the character can actually take
        feat_manager = self.character_manager.get_manager('feat')
        available_feats = []
        available_bonus_feats = []
        
        if feat_manager:
            try:
                # Use get_available_feats which checks prerequisites
                available_feats = feat_manager.get_available_feats()
                
                # If we have bonus slots, calculate specific allowed list
                if feat_slots['bonus'] > 0:
                    allowed_bonus_ids = self._get_bonus_feat_ids(class_id, new_level, new_class_level)
                    if allowed_bonus_ids:
                        available_bonus_feats = feat_manager.get_available_feats(allowed_feat_ids=allowed_bonus_ids)
                    else:
                        # Fallback: if no specific list found, show all (avoids blocking user)
                        available_bonus_feats = available_feats
            except Exception as e:
                logger.warning(f"Failed to get available feats: {e}")

        # Class skills
        class_skills = []
        cross_class_skills = []
        if skill_manager:
            class_skills = list(skill_manager._get_class_skills(class_id))
            # All skills not in class_skills are cross-class
            all_skills = skill_manager.get_all_skill_ids() if hasattr(skill_manager, 'get_all_skill_ids') else []
            cross_class_skills = [s for s in all_skills if s not in class_skills]

        return {
            'can_level_up': total_level < 60,  # NWN2 max level
            'current_level': total_level,
            'new_level': new_level,
            'class_id': class_id,
            'class_name': self.field_mapper.get_field_value(class_data, 'label', f'Class {class_id}'),
            'new_class_level': new_class_level,
            'hp_gain': hp_gain,
            'skill_points': skill_points,
            'feat_slots': feat_slots,
            'has_ability_increase': has_ability_increase,
            'spell_slots_gained': spell_slots_gained,
            'available_feats': available_feats,
            'available_bonus_feats': available_bonus_feats,
            'available_spells': available_spells,
            'class_skills': class_skills,
            'cross_class_skills': cross_class_skills,
            'is_spellcaster': is_spellcaster,
            'current_abilities': current_abilities
        }

    def _calculate_feat_slots_for_level(
        self,
        total_level: int,
        class_id: int,
        class_level: int
    ) -> Dict[str, int]:
        """Calculate feat slots available at this level"""
        general = 0
        bonus = 0

        # General feats
        if total_level <= 20:
            # Heroic: 1, 3, 6, 9, 12, 15, 18
            if total_level == 1 or total_level % 3 == 0:
                general = 1
        else:
            # Epic: Every Odd Level (21, 23, 25...) - Verified NWN2 Rule
            if total_level % 2 != 0:
                general = 1

        # Bonus feats from class
        # 1. Check Standard Bonus Feat Table (Levels 1-20 usually)
        class_data = self.rules_service.get_by_id('classes', class_id)
        if class_data:
            # Check if using a bonus table
            bonus_feat_table_resref = self.field_mapper.get_field_value(class_data, 'bonus_feats_table', '')
            if bonus_feat_table_resref and bonus_feat_table_resref != '****':
                b_table = self.rules_service.get_table(bonus_feat_table_resref.lower())
                if b_table:
                    # Table is 0-indexed by class level
                    level_idx = max(0, class_level - 1)
                    if level_idx < len(b_table):
                        row = b_table[level_idx]
                        has_bonus = self.field_mapper._safe_int(
                            self.field_mapper.get_field_value(row, 'bonus', '0'), 0
                        )
                        if has_bonus > 0:
                            bonus += 1
            
            # 2. Check Epic Bonus Feats (Levels 21+)
            # NWN2 Hardcoded Intervals for Epic Class Feats
            if class_level > 20:
                # Get class label/name to identify logic (IDs can vary, but base classes are static)
                # Using ID checking for robustness if we have constants, but string label fallback
                # Base Class IDs: Fighter=4, Wizard=10, Rogue=7, Druid=3
                
                # Formula: (ClassLevel - 20) % Interval == 0
                
                interval = 0
                if class_id == 4: # Fighter: Every 2 levels (22, 24...)
                    interval = 2
                elif class_id == 10: # Wizard: Every 3 levels (23, 26...)
                    interval = 3
                elif class_id == 7 or class_id == 3: # Rogue, Druid: Every 4 levels (24, 28...)
                    interval = 4
                
                # TODO: Add other classes (Monk=5, etc) if needed. 
                # For now implementing verified ones.
                
                if interval > 0:
                    if (class_level - 20) % interval == 0:
                        bonus += 1

        return {'general': general, 'bonus': bonus}

    def get_class_label(self, class_id: int) -> Optional[str]:
        """Helper to get class label/name"""
        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
             return None
        
        class_label = self.field_mapper.get_field_value(class_data, 'label', '')
        if not class_label:
             class_label = self.field_mapper.get_field_value(class_data, 'name', '')
             
        return class_label

    def _get_bonus_feat_label(self, class_id: int, total_level: int) -> str:
        """Get a human-readable label for the bonus feat slot of a class."""
        if total_level > 20:
            return "Epic"

        if class_id == 4: # Fighter
            return "Combat"
        if class_id == 10: # Wizard
            return "Metamagic/Item Creation"
        if class_id == 7: # Rogue
            return "Special Ability"
        if class_id == 3: # Druid
            return "Nature"
        if class_id == 5: # Monk 
            return "Monk"
        if class_id == 6: # Paladin
            return "Paladin"
        if class_id == 8: # Ranger
            return "Ranger"
        
        # Prestige classes often have specific lists
        class_label = self.get_class_label(class_id)
        if class_label:
            return f"{class_label}"

        return "Bonus"

    def _get_bonus_feat_ids(self, class_id: int, total_level: int, class_level: int) -> List[int]:
        """Identify allowed feat IDs for bonus slots for a given class and level."""
        # 1. Handle Epic (Total level > 20)
        if total_level > 20:
            class_label = self.get_class_label(class_id)
            if class_label:
                # Epic bonus feat tables are usually cls_efeat_<class>
                table_name = f"cls_efeat_{class_label.lower()}"
                table = self.rules_service.get_table(table_name)
                if table:
                    allowed_ids = []
                    for row in table:
                        # Common column names for feat ID in epic tables: 'FeatIndex', 'FeatID', 'Feat'
                        feat_id = self.field_mapper._safe_int(self.field_mapper.get_field_value(row, 'feat_index', -1))
                        if feat_id == -1:
                            feat_id = self.field_mapper._safe_int(getattr(row, 'FeatID', -1))
                        
                        if feat_id != -1:
                            allowed_ids.append(feat_id)
                    return allowed_ids

        # 2. Handle Heroic (Total level <= 20)
        # Use CLS_FEAT_<class>.2da which contains the 'List' column (2=Bonus, 3=Both)
        class_data = self.rules_service.get_by_id('classes', class_id)
        if class_data:
            feats_table_resref = self.field_mapper.get_field_value(class_data, 'feats_table', '')
            if feats_table_resref and feats_table_resref != '****':
                feat_list_table = self.rules_service.get_table(feats_table_resref.lower())
                if feat_list_table:
                    allowed_ids = []
                    for row in feat_list_table:
                        # Extract feat index and list type
                        feat_id = self.field_mapper._safe_int(self.field_mapper.get_field_value(row, 'feat_index', -1))
                        if feat_id == -1:
                            feat_id = self.field_mapper._safe_int(getattr(row, 'FeatIndex', -1))
                        
                        if feat_id == -1: continue
                        
                        # List: 0=Auto, 1=General, 2=Bonus, 3=Both
                        lst = self.field_mapper._safe_int(getattr(row, 'List', 1)) 
                        if lst in [1, 2, 3]:
                            allowed_ids.append(feat_id)
                    
                    if allowed_ids:
                        # Some mods/versions have shifted IDs, ensure both 28 and 29 are treated as bonus if either is found
                        if 28 in allowed_ids and 29 not in allowed_ids:
                            allowed_ids.append(29)
                        return allowed_ids

        # Fallback to category-based filtering if table lookup fails
        allowed_categories = []
        if class_id == 4: # Fighter
            allowed_categories = [2] # Combat (FEAT_TYPE_MAP 'COMBAT': 2)
        elif class_id == 10: # Wizard
            allowed_categories = [8, 2048] # Metamagic, Item Creation
        elif class_id == 7: # Rogue
            allowed_categories = [64, 16] # Special/Divine?
        
        if allowed_categories:
            feat_table = self.rules_service.get_table('feat')
            feat_manager = self.character_manager.get_manager('feat')
            if feat_table and feat_manager:
                allowed_ids = []
                for row_index, feat_data in enumerate(feat_table):
                    feat_type = feat_manager._parse_feat_type(feat_data)
                    if feat_type in allowed_categories:
                        feat_id = getattr(feat_data, 'id', getattr(feat_data, 'row_index', row_index))
                        allowed_ids.append(feat_id)
                return allowed_ids

        return []

    def preview_level_up(
        self,
        class_id: int,
        ability_increase: Optional[str] = None,
        feats: List[int] = None,
        skills: Dict[int, int] = None,
        spells: List[Dict[str, int]] = None
    ) -> Dict[str, Any]:
        """
        Preview level-up without applying changes.
        Validates all selections and returns what would change.

        Args:
            class_id: Class to level up in
            ability_increase: Attribute to increase (Str, Dex, etc.)
            feats: List of feat IDs to add
            skills: Dict of skill_id -> points to allocate
            spells: List of {spell_level, spell_id} dicts

        Returns:
            Preview of changes with validation results
        """
        feats = feats or []
        skills = skills or {}
        spells = spells or []

        errors = []
        warnings = []

        requirements = self.get_level_up_requirements(class_id)
        # Recalculate slots specifically for validation to be safe
        # (Though requirements should be accurate)
        total_general_slots = requirements['feat_slots']['general']
        total_bonus_slots = requirements['feat_slots']['bonus']
        total_slots = total_general_slots + total_bonus_slots

        # Validate ability increase
        if requirements['has_ability_increase']:
            if not ability_increase:
                errors.append("Ability increase required at this level")
            elif ability_increase not in ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']:
                errors.append(f"Invalid ability: {ability_increase}")
        elif ability_increase:
            warnings.append("Ability increase not available at this level, will be ignored")

        # Validate feat count
        if len(feats) > total_slots:
            errors.append(f"Too many feats selected: {len(feats)} > {total_slots} available")

        # Validate feats and slot usage
        feat_manager = self.character_manager.get_manager('feat')
        feats_to_gain = []
        
        # Get allowed bonus IDs for strict validation
        allowed_bonus_ids = self._get_bonus_feat_ids(class_id, requirements['new_level'], requirements['new_class_level'])
        allowed_bonus_set = set(allowed_bonus_ids) if allowed_bonus_ids else set()
        
        bonus_compatible_count = 0
        for feat_id in feats:
            feat_info = None
            if feat_manager:
                feat_info = feat_manager.get_feat_info(feat_id) if hasattr(feat_manager, 'get_feat_info') else None
            
            if feat_info and not feat_info.get('can_take', True):
                errors.append(f"Cannot take feat: {feat_info.get('name', feat_id)}")
            feats_to_gain.append(feat_info or {'id': feat_id, 'name': f'Feat {feat_id}'})
            
            if feat_id in allowed_bonus_set:
                bonus_compatible_count += 1

        # Strict Slot Validation: Ensure bonus slots are used for valid bonus feats
        if total_bonus_slots > 0:
            general_feats_count = len(feats) - bonus_compatible_count
            if general_feats_count > total_general_slots:
                errors.append(
                    f"Invalid feat selection: {general_feats_count} general feats selected, "
                    f"but only {total_general_slots} general slots available "
                    f"(requires {total_bonus_slots} bonus feats from class list)."
                )

        # Validate skill points
        total_skill_points = sum(skills.values())
        if total_skill_points > requirements['skill_points']:
            errors.append(f"Skill points exceeded: {total_skill_points} > {requirements['skill_points']}")

        # Validate spells
        spells_to_learn = []
        spell_manager = self.character_manager.get_manager('spell')
        for spell_entry in spells:
            spell_id = spell_entry.get('spell_id')
            spell_level = spell_entry.get('spell_level')
            if spell_manager:
                spell_details = spell_manager.get_spell_details(spell_id)
                spells_to_learn.append(spell_details)
            else:
                spells_to_learn.append({'id': spell_id, 'level': spell_level})

        return {
            'valid': len(errors) == 0,
            'errors': errors,
            'warnings': warnings,
            'hp_gained': requirements['hp_gain'],
            'new_total_level': requirements['new_level'],
            'new_class_level': requirements['new_class_level'],
            'stats_preview': {},
            'feats_to_gain': feats_to_gain,
            'skills_to_update': skills,
            'spells_to_learn': spells_to_learn
        }

    def _record_ability_increase_in_history(self, ability: str):
        """Record ability increase in LvlStatList"""
        ability_map = {
            'Str': 0, 'Dex': 1, 'Con': 2,
            'Int': 3, 'Wis': 4, 'Cha': 5
        }
        ability_index = ability_map.get(ability)
        if ability_index is None:
            return

        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if lvl_stat_list:
            current_entry = lvl_stat_list[-1]
            current_entry[LVL_STAT_ABILITY] = ability_index
            self.gff.set(LVL_STAT_LIST, lvl_stat_list)
            logger.debug(f"Recorded ability increase ({ability}) in level history")

    def _calculate_spell_slots_gained(self, class_id: int, class_level: int) -> Dict[int, int]:
        """Calculate spell slots gained at this level"""
        spell_manager = self.character_manager.get_manager('spell')
        if not spell_manager:
            return {}

        # This would need to look up the spell gain table for the class
        # For now, return empty - spell manager handles this via events
        return {}

    def _get_available_spells_for_level(self, class_id: int, class_level: int) -> Dict[int, List[Dict[str, Any]]]:
        """Get spells available to learn at this level"""
        spell_manager = self.character_manager.get_manager('spell')
        if not spell_manager:
            return {}

        available_spells = {}
        try:
            # Get available spells for each spell level
            for spell_level in range(10):  # 0-9
                spells = spell_manager.get_available_spells(spell_level, class_id)
                if spells:
                    available_spells[spell_level] = spells
        except Exception as e:
            logger.warning(f"Failed to get available spells: {e}")

        return available_spells

    def _get_xp_table(self) -> List[int]:
        """Load XP thresholds from exptable.2da"""
        if not hasattr(self, '_xp_table_cache') or self._xp_table_cache is None:
            self._xp_table_cache = []
            try:
                exptable = self.rules_service.get_table('exptable')
                if exptable:
                    for row in exptable:
                        xp_val = getattr(row, 'XP', None)
                        if xp_val is not None:
                            try:
                                self._xp_table_cache.append(int(xp_val))
                            except (ValueError, TypeError):
                                pass
                    logger.debug(f"Loaded {len(self._xp_table_cache)} XP thresholds from exptable.2da")
            except Exception as e:
                logger.warning(f"Failed to load exptable.2da: {e}")

            if not self._xp_table_cache:
                logger.warning("Using fallback XP table (Level 1-60)")
                # Formula: level * (level - 1) * 500
                self._xp_table_cache = [i * (i - 1) * 500 for i in range(1, 61)]

            # Ensure the table is at least 60 levels long (standard for many mods/expansions)
            if len(self._xp_table_cache) < 60:
                logger.debug(f"Extending XP table from {len(self._xp_table_cache)} to 60 levels")
                current_len = len(self._xp_table_cache)
                for level in range(current_len + 1, 61):
                    # Standard NWN2 formula: level * (level - 1) * 500
                    self._xp_table_cache.append(level * (level - 1) * 500)
                    
        return self._xp_table_cache

    def get_experience(self) -> int:
        """Get current experience points"""
        return self.gff.get('Experience', 0)

    def set_experience(self, xp: int) -> Dict[str, Any]:
        """
        Set experience points

        Args:
            xp: New XP value (must be >= 0)

        Returns:
            Dict with old_xp, new_xp, old_level, new_level
        """
        if xp < 0:
            raise ValueError("XP cannot be negative")

        old_xp = self.get_experience()
        old_level = self.xp_to_level(old_xp)
        new_level = self.xp_to_level(xp)

        self.gff.set('Experience', xp)
        logger.info(f"Set XP from {old_xp} to {xp} (level {old_level} -> {new_level})")

        return {
            'old_xp': old_xp,
            'new_xp': xp,
            'old_level': old_level,
            'new_level': new_level,
            'level_changed': old_level != new_level
        }

    def xp_to_level(self, xp: int) -> int:
        """Convert XP to level"""
        xp_table = self._get_xp_table()
        for level, threshold in enumerate(xp_table, start=1):
            if xp < threshold:
                return max(1, level - 1)
        return len(xp_table)

    def level_to_xp(self, level: int) -> int:
        """Get minimum XP required for a level"""
        xp_table = self._get_xp_table()
        if level < 1:
            return 0
        if level > len(xp_table):
            level = len(xp_table)
        return xp_table[level - 1]

    def get_xp_for_next_level(self) -> Optional[int]:
        """Get XP needed for next level, or None if at max"""
        xp_table = self._get_xp_table()
        current_xp = self.get_experience()
        current_level = self.xp_to_level(current_xp)
        if current_level >= len(xp_table):
            return None
        return xp_table[current_level]

    def get_xp_progress(self) -> Dict[str, Any]:
        """Get XP progress info for UI"""
        xp_table = self._get_xp_table()
        current_xp = self.get_experience()
        current_level = self.xp_to_level(current_xp)
        total_level = self.get_total_level()
        max_level = len(xp_table)

        if current_level >= max_level:
            return {
                'current_xp': current_xp,
                'current_xp_level': current_level,
                'total_class_level': total_level,
                'next_level_xp': None,
                'xp_to_next': None,
                'current_level_min_xp': xp_table[max_level - 1],
                'next_level_min_xp': xp_table[max_level - 1],
                'progress_percent': 100
            }

        current_threshold = xp_table[current_level - 1] if current_level > 0 else 0
        next_threshold = xp_table[current_level]
        xp_in_level = current_xp - current_threshold
        xp_needed = next_threshold - current_threshold
        progress = (xp_in_level / xp_needed * 100) if xp_needed > 0 else 100

        return {
            'current_xp': current_xp,
            'current_xp_level': current_level,
            'total_class_level': total_level,
            'next_level_xp': next_threshold,
            'xp_to_next': next_threshold - current_xp,
            'current_level_min_xp': current_threshold,
            'next_level_min_xp': next_threshold,
            'progress_percent': round(progress, 1)
        }