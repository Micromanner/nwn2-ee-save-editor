"""
Data-Driven Skill Manager - handles skill points, skill ranks, and class skills
Uses CharacterManager and DynamicGameDataLoader for all skill data access
"""

from typing import Dict, List, Set, Tuple, Optional, Any
from loguru import logger
import time

from ..events import EventEmitter, EventType, ClassChangedEvent, LevelGainedEvent, SkillPointsAwardedEvent
from gamedata.dynamic_loader.field_mapping_utility import field_mapper

# Using global loguru logger


class SkillManager(EventEmitter):
    """
    Data-Driven Skill Manager
    Uses CharacterManager as hub for all character data access
    """
    
    def __init__(self, character_manager):
        """
        Initialize the SkillManager
        
        Args:
            character_manager: Reference to parent CharacterManager
        """
        super().__init__()
        self.character_manager = character_manager
        self.game_rules_service = character_manager.rules_service
        self.gff = character_manager.gff
        
        # Register for events
        self._register_event_handlers()
        
        # Cache
        self._skill_cache = {}
        self._class_skills_cache = {}
    
    def _register_event_handlers(self):
        """Register handlers for relevant events"""
        self.character_manager.on(EventType.CLASS_CHANGED, self.on_class_changed)
        self.character_manager.on(EventType.LEVEL_GAINED, self.on_level_gained)
    
    def on_class_changed(self, event: ClassChangedEvent):
        """Handle class change event"""
        logger.info(f"SkillManager handling class change: {event.old_class_id} -> {event.new_class_id}")
        
        # Skip destructive operations for simple level adjustments (up/down in same class)
        if event.is_level_adjustment:
            logger.info("Skipping skill reset - this is a level adjustment, not a class swap")
            # Still update class skills cache in case level affects skill maximums
            self._update_class_skills_cache(event.new_class_id)
            return
        
        # Recalculate total skill points
        total_skill_points = self.calculate_total_skill_points(event.new_class_id, event.level)
        
        # Reset skills for redistribution
        self.reset_all_skills()
        
        # Update available skill points
        self.gff.set('SkillPoints', total_skill_points)
        
        # Update class skills list
        self._update_class_skills_cache(event.new_class_id)
        
        logger.info(f"Reset skills with {total_skill_points} points available")
    
    def on_level_gained(self, event: LevelGainedEvent):
        """Handle level gain event"""
        logger.info(f"SkillManager handling level gain: Level {event.new_level}")
        
        # Calculate skill points for this level
        class_data = self.game_rules_service.get_by_id('classes', event.class_id)
        if class_data:
            modifiers = self._calculate_ability_modifiers()
            skill_points_gained = self.calculate_skill_points_for_level(
                class_data, modifiers['INT'], is_first_level=(event.new_level == 1)
            )
            
            # Add to available points
            current_points = self.gff.get('SkillPoints', 0)
            self.gff.set('SkillPoints', current_points + skill_points_gained)
            
            # Emit skill points awarded event
            skill_event = SkillPointsAwardedEvent(
                event_type=EventType.SKILL_POINTS_AWARDED,
                source_manager='skill',
                timestamp=time.time(),
                class_id=event.class_id,
                level=event.new_level,
                points=skill_points_gained
            )
            self.character_manager.emit(skill_event)
            
            logger.info(f"Gained {skill_points_gained} skill points")
    
    
    def calculate_total_skill_points(self, class_id: int, total_level: int) -> int:
        """
        Calculate total skill points for a class at given level
        
        Args:
            class_id: The class ID
            total_level: Total character level
            
        Returns:
            Total skill points
        """
        class_data = self.game_rules_service.get_by_id('classes', class_id)
        if not class_data:
            return 0
        
        modifiers = self._calculate_ability_modifiers()
        int_modifier = modifiers['INT']
        
        # Determine points per level
        base_skill_points = field_mapper._safe_int(
            field_mapper.get_field_value(class_data, 'skill_point_base', 2), 2
        )
        
        # Calculate racial bonus base (per level)
        racial_bonus_base = self._get_racial_skill_point_bonus_base()
        
        # Base points per level (Class + Int + Race)
        # Note: In NWN2 adjustments apply before multiplication
        points_per_level = base_skill_points + int_modifier + racial_bonus_base
        points_per_level = max(1, points_per_level) # Minimum 1 point per level
        
        # Level 1 calculation (x4)
        level_1_points = max(4, points_per_level * 4)
        
        if total_level <= 1:
            return level_1_points
            
        # Subsequent levels
        subsequent_levels = total_level - 1
        total_points = level_1_points + (points_per_level * subsequent_levels)
        
        return total_points
    
    def calculate_skill_points_for_level(self, class_data, int_modifier: int, is_first_level: bool = False) -> int:
        """
        Calculate skill points gained for a single level
        
        Args:
            class_data: Class data object
            int_modifier: Intelligence modifier
            is_first_level: Whether this is the character's first level (x4 multiplier)
        """
        base_skill_points = field_mapper._safe_int(
            field_mapper.get_field_value(class_data, 'skill_point_base', 2), 2
        )
        
        # Get racial bonus
        racial_bonus_base = self._get_racial_skill_point_bonus_base()
        
        # Calculate base points
        points = base_skill_points + int_modifier + racial_bonus_base
        points = max(1, points) # Minimum 1
        
        # Apply multiplier
        if is_first_level:
            points *= 4
            points = max(4, points) # Minimum 4 at first level
            
        return points
    
    def set_skill_rank(self, skill_id: int, ranks: int) -> bool:
        """
        Set ranks in a skill
        
        Args:
            skill_id: The skill ID
            ranks: Number of ranks to set
            
        Returns:
            True if successful
        """
        # Validate skill ID exists (prevent crashes)
        skill_data = self.game_rules_service.get_by_id('skills', skill_id)
        if not skill_data:
            logger.warning(f"Invalid skill ID: {skill_id}")
            return False
            
        # Prevent negative ranks (GFF integrity)
        if ranks < 0:
            logger.warning(f"Cannot set negative ranks ({ranks}) in skill {skill_id}")
            return False
        
        # Calculate cost and points (for tracking purposes)
        available_points = self.gff.get('SkillPoints', 0)
        current_ranks = self.get_skill_ranks(skill_id)
        current_cost = self.calculate_skill_cost(skill_id, current_ranks)
        new_cost = self.calculate_skill_cost(skill_id, ranks)
        net_cost = new_cost - current_cost
        
        # Informational warnings (don't block action)
        max_ranks = self.get_max_skill_ranks(skill_id)
        if ranks > max_ranks:
            logger.info(f"Setting {ranks} ranks in skill {skill_id} exceeds normal maximum of {max_ranks}")
        
        if net_cost > available_points:
            logger.info(f"Skill allocation uses {net_cost} points (have {available_points} available) - overspending by {net_cost - available_points}")
        
        
        # Update skill list
        skill_list = self.gff.get('SkillList', [])
        
        # Check if we have positional format
        if isinstance(skill_list, list) and len(skill_list) > 0 and isinstance(skill_list[0], dict):
            # Check if it's positional format (no 'Skill' field in entries)
            first_entry = skill_list[0]
            is_positional = 'Skill' not in first_entry
            
            if is_positional:
                # Ensure list is long enough for skill_id
                while len(skill_list) <= skill_id:
                    skill_list.append({'Rank': 0})
                
                # Update the skill at the correct position
                skill_list[skill_id] = {'Rank': ranks}
            else:
                # Old format - find or create skill entry
                skill_entry = None
                for skill in skill_list:
                    stored_skill_id = skill.get('Skill')
                    if stored_skill_id is not None:
                        try:
                            stored_skill_id = int(stored_skill_id)
                        except (ValueError, TypeError):
                            continue
                        if stored_skill_id == skill_id:
                            skill_entry = skill
                            break
                
                if not skill_entry and ranks > 0:
                    skill_entry = {'Skill': skill_id}
                    skill_list.append(skill_entry)
                
                if skill_entry:
                    if ranks > 0:
                        skill_entry['Rank'] = ranks
                    else:
                        # Remove skill if ranks is 0
                        skill_list.remove(skill_entry)
        else:
            # Initialize as positional format if empty
            skill_list = []
            for i in range(skill_id + 1):
                skill_list.append({'Rank': ranks if i == skill_id else 0})
        
        self.gff.set('SkillList', skill_list)

        rank_delta = ranks - current_ranks
        if rank_delta != 0:
            class_manager = self.character_manager.get_manager('class')
            if class_manager:
                class_manager.record_skill_change(skill_id, rank_delta)

        # Update available points (can go negative - user freedom)
        self.gff.set('SkillPoints', available_points - net_cost)
        
        # CRITICAL FIX: Clear skill cache to prevent stale data
        if skill_id in self._skill_cache:
            del self._skill_cache[skill_id]
        
        logger.info(f"Set skill {skill_id} to {ranks} ranks")
        return True
    
    def get_skill_ranks(self, skill_id: int) -> int:
        """Get current ranks in a skill"""
        skill_list = self.gff.get('SkillList', [])
        
        # Handle positional format (index = skill ID)
        if isinstance(skill_list, list) and 0 <= skill_id < len(skill_list):
            skill_entry = skill_list[skill_id]
            if isinstance(skill_entry, dict):
                return skill_entry.get('Rank', 0)
        
        # Fallback to old format (list of dicts with 'Skill' field)
        for skill in skill_list:
            if isinstance(skill, dict):
                stored_skill_id = skill.get('Skill')
                if stored_skill_id is not None:
                    try:
                        stored_skill_id = int(stored_skill_id)
                    except (ValueError, TypeError):
                        continue
                    if stored_skill_id == skill_id:
                        return skill.get('Rank', 0)
        
        return 0
    
    def get_max_skill_ranks(self, skill_id: int) -> int:
        """Get maximum ranks allowed in a skill"""
        total_level = sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', []))
        
        # Class skills: level + 3
        # Cross-class skills: (level + 3) / 2
        if self.is_class_skill(skill_id):
            return total_level + 3
        else:
            return (total_level + 3) // 2
    
    def is_class_skill(self, skill_id: int) -> bool:
        """Check if a skill is a class skill"""
        # Check all current classes
        class_list = self.gff.get('ClassList', [])
        
        for class_entry in class_list:
            class_id = class_entry.get('Class')
            class_skills = self._get_class_skills(class_id)
            if skill_id in class_skills:
                return True
        
        return False
    
    def calculate_skill_cost(self, skill_id: int, ranks: int) -> int:
        """
        Calculate skill point cost for ranks.
        Rules:
        - Class Skills (for any class possessed): Cost 1 per rank.
        - Cross-Class Skills: Cost 2 per rank.
        - Able Learner Feat (ID 406): Cross-Class Skills cost 1 per rank.
        """
        if ranks == 0:
            return 0
            
        # Check if it's a class skill (Permanent Memory rule maintained by global check)
        if self.is_class_skill(skill_id):
            return ranks
            
        # Check for Able Learner feat (ID 406)
        # We need to access FeatManager securely
        feat_manager = self.character_manager.get_manager('feat')
        if feat_manager:
            # Check if character has Able Learner
            # Use raw GFF check or feat manager method if available
            # Doing raw check here for speed/independence or using feat manager
            if feat_manager.has_feat(406):
                return ranks
        
        # If neither Class Skill nor Able Learner, it's a Cross-Class skill
        return ranks * 2
    
    def calculate_skill_modifier(self, skill_id: int) -> int:
        """Calculate total skill modifier including ranks and ability bonus"""
        # Get base ranks
        ranks = self.get_skill_ranks(skill_id)

        # Get ability modifier using dynamic skill data
        skill_data = self.game_rules_service.get_by_id('skills', skill_id)
        key_ability = field_mapper.get_field_value(skill_data, 'key_ability', 'STR').upper() if skill_data else 'STR'

        modifiers = self._calculate_ability_modifiers()
        ability_mod = modifiers.get(key_ability, 0)

        # Get equipment bonuses from InventoryManager (with null safety)
        inventory_manager = self.character_manager.get_manager('inventory')
        if inventory_manager:
            equipment_bonuses = inventory_manager.get_equipment_bonuses()
            skill_bonuses = equipment_bonuses.get('skills', {}) or {}
            skill_name = field_mapper.get_field_value(skill_data, 'label', '') if skill_data else ''
            equipment_skill_bonus = skill_bonuses.get(skill_name, 0) if skill_bonuses else 0
        else:
            equipment_skill_bonus = 0

        # Check for skill synergies
        synergy_bonus = self._calculate_synergy_bonus(skill_id)

        # Check for armor check penalty
        armor_penalty = 0
        if skill_data:
            armor_check_penalty = field_mapper._safe_int(
                field_mapper.get_field_value(skill_data, 'armor_check_penalty', 0), 0
            )
            if armor_check_penalty > 0:
                armor_penalty = self._get_armor_check_penalty()

        total = ranks + ability_mod + synergy_bonus + equipment_skill_bonus - armor_penalty

        return total
    
    def reset_all_skills(self):
        """Reset all skill ranks to 0 and refund spent points"""
        logger.info("Resetting all skills")
        
        # Calculate total spent points before reset
        total_refund = self._calculate_spent_skill_points()
        
        # Get current skill list
        skill_list = self.gff.get('SkillList', [])
        
        # Check if we have positional format
        is_positional = False
        if skill_list and isinstance(skill_list[0], dict):
            is_positional = 'Skill' not in skill_list[0]
        
        if is_positional:
            # Reset all ranks to 0 in positional format
            for i in range(len(skill_list)):
                if isinstance(skill_list[i], dict):
                    skill_list[i]['Rank'] = 0
        else:
            # Reset all ranks to 0 in old format
            for skill in skill_list:
                if isinstance(skill, dict) and 'Rank' in skill:
                    skill['Rank'] = 0
        
        # Update the skill list
        self.gff.set('SkillList', skill_list)
        
        # Refund the spent points
        current_available = self.gff.get('SkillPoints', 0)
        self.gff.set('SkillPoints', current_available + total_refund)
        
        logger.info(f"Reset all skills, refunded {total_refund} points")
    
    def _get_class_skills(self, class_id: int) -> Set[int]:
        """Get set of class skills for a class using dynamic data"""
        if class_id in self._class_skills_cache:
            return self._class_skills_cache[class_id]
        
        class_skills = set()
        class_data = self.game_rules_service.get_by_id('classes', class_id)
        
        if class_data:
            # Get the skills table name from class data using FieldMappingUtility
            skills_table_name = field_mapper.get_field_value(class_data, 'skills_table', None)
            if skills_table_name:
                # Load the specific class skills table (e.g., "CLS_SKILL_BARD")
                # Convert to lowercase for table lookup (tables are stored in lowercase)
                class_skills_table = self.game_rules_service.get_table(skills_table_name.lower())
                if class_skills_table:
                    for skill_entry in class_skills_table:
                        # Check for ClassSkill field to ensure it's actually a class skill
                        is_class_skill = field_mapper.get_field_value(skill_entry, 'class_skill', '0')
                        if is_class_skill == '1' or is_class_skill == 1:
                            skill_id = field_mapper.get_field_value(skill_entry, 'skill_index', None)
                            if skill_id is not None:
                                skill_id_int = field_mapper._safe_int(skill_id, -1)
                                if skill_id_int >= 0:
                                    class_skills.add(skill_id_int)
                else:
                    logger.warning(f"Class skills table {skills_table_name} not found")
                
                # Log if no class skills were found
                if not class_skills:
                    class_label = field_mapper.get_field_value(class_data, 'label', 'Unknown')
                    logger.warning(f"No class skills found for class {class_label}")
        
        self._class_skills_cache[class_id] = class_skills
        return class_skills
    
    
    def _update_class_skills_cache(self, primary_class_id: int):
        """Update the class skills cache when primary class changes"""
        # Clear cache to force recalculation
        self._class_skills_cache.clear()
        
        # Pre-populate for current classes
        class_list = self.gff.get('ClassList', [])
        for class_entry in class_list:
            class_id = class_entry.get('Class')
            self._get_class_skills(class_id)
    
    def _get_racial_skill_point_bonus_base(self) -> int:
        """Get base racial skill point bonus per level"""
        race_manager = self.character_manager.get_manager('race')
        if race_manager:
            # Get racial properties which includes skill point bonuses
            racial_props = race_manager.get_racial_properties()
            race_name = racial_props.get('race_name', '').lower()
            
            # In D&D/NWN2, humans get +1 skill point per level
            if 'human' in race_name:
                return 1
            
            # TODO: Add other racial skill bonuses when race data includes them
        
        return 0

    def _calculate_ability_modifiers(self) -> Dict[str, int]:
        """Calculate ability modifiers using AbilityManager"""
        attr_manager = self.character_manager.get_manager('ability')
        if attr_manager:
            return attr_manager.get_all_modifiers()
        # Fallback if no AttributeManager available
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
    
    def _calculate_synergy_bonus(self, skill_id: int) -> int:
        """Calculate skill synergy bonuses"""
        synergy_bonus = 0
        
        # Example synergies (would need full synergy data)
        synergies = {
            # Jump gets +2 from 5 ranks in Tumble
            'jump': [('tumble', 5, 2)],
            # Diplomacy gets +2 from 5 ranks in Bluff
            'diplomacy': [('bluff', 5, 2)],
        }
        
        # This is simplified - would need skill name mapping
        return synergy_bonus
    
    def _get_armor_check_penalty(self) -> int:
        """Get armor check penalty from equipped armor"""
        # Would need to check equipped items
        # For now, return 0
        return 0
    
    def get_skill_info(self, skill_id: int) -> Optional[Dict[str, Any]]:
        """Get detailed information about a skill"""
        if skill_id in self._skill_cache:
            return self._skill_cache[skill_id]
        
        skill_data = self.game_rules_service.get_by_id('skills', skill_id)
        if skill_data:
            info = {
                'id': skill_id,
                'label': field_mapper.get_field_value(skill_data, 'label', 'Unknown'),
                'name': field_mapper.get_field_value(skill_data, 'name', 'Unknown'),
                'key_ability': field_mapper.get_field_value(skill_data, 'key_ability', 'STR'),
                'armor_check': field_mapper._safe_int(
                    field_mapper.get_field_value(skill_data, 'armor_check_penalty', 0), 0
                ) > 0,
                'is_class_skill': self.is_class_skill(skill_id),
                'current_ranks': self.get_skill_ranks(skill_id),
                'max_ranks': self.get_max_skill_ranks(skill_id),
                'total_modifier': self.calculate_skill_modifier(skill_id)
            }
            self._skill_cache[skill_id] = info
            return info
        
        return None
    
    def get_skill_summary(self) -> Dict[str, Any]:
        """Get summary of character's skills"""
        try:
            skill_list = self.gff.get('SkillList', [])
            available_points = self.gff.get('SkillPoints', 0)
            spent_points = self._calculate_spent_skill_points()
            
            # Calculate total available points for comparison
            total_level = sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', []))
            primary_class = self.gff.get('ClassList', [{}])[0].get('Class', 0) if self.gff.get('ClassList') else 0
            total_available = self.calculate_total_skill_points(primary_class, total_level)
            overspent = max(0, spent_points - total_available)
            
            # Check format first
            is_positional = False
            if skill_list and isinstance(skill_list[0], dict):
                is_positional = 'Skill' not in skill_list[0]
            
            # Calculate skills_with_ranks based on format
            if is_positional:
                skills_with_ranks = len([s for s in skill_list if isinstance(s, dict) and s.get('Rank', 0) > 0])
            else:
                skills_with_ranks = len([s for s in skill_list if isinstance(s, dict) and s.get('Skill') is not None and s.get('Rank', 0) > 0])
            
            # Current Level Logic
            current_level_gained = 0
            current_level_spent = 0
            
            lvl_stat_list = self.gff.get('LvlStatList', [])
            if lvl_stat_list and isinstance(lvl_stat_list, list):
                last_entry = lvl_stat_list[-1]
                
                # Points gained (stored in history) - failover to calculation if 0
                recorded_gained = last_entry.get('SkillPoints', 0)
                
                # Calculate expected points for this level
                class_id = last_entry.get('LvlStatClass', -1)
                class_data = self.game_rules_service.get_by_id('classes', class_id)
                modifiers = self._calculate_ability_modifiers()
                int_mod = modifiers.get('INT', 0)
                is_first_level = len(lvl_stat_list) == 1
                
                expected_gained = self.calculate_skill_points_for_level(class_data, int_mod, is_first_level)
                
                # Use expected if recorded is 0 (fix for "Overdrawn" issue on saves with missing history data)
                # Otherwise trust the GFF (in case of manual edits/bonuses not covered by rules)
                current_level_gained = max(recorded_gained, expected_gained)
                
                # Points spent (calculate from history SkillList)
                history_skill_list = last_entry.get('SkillList', [])
                
                # record_skill_change creates a LIST of dicts: [{'Rank': 0}, {'Rank': 1}...]
                # So index is skill ID.
                for skill_id, skill_entry in enumerate(history_skill_list):
                    ranks_added = skill_entry.get('Rank', 0)
                    if ranks_added > 0:
                        cost = self.calculate_skill_cost(skill_id, ranks_added)
                        current_level_spent += cost

            current_level_balance = current_level_gained - current_level_spent
            current_level_available = max(0, current_level_balance)
            current_level_overdrawn = max(0, -current_level_balance)

            summary = {
                'available_points': available_points,
                'total_available': total_available,
                'spent_points': spent_points,
                'overspent': overspent,
                'total_ranks': sum(s.get('Rank', 0) for s in skill_list if isinstance(s, dict)),
                'skills_with_ranks': skills_with_ranks,
                'current_level_gained': current_level_gained,
                'current_level_spent': current_level_spent,
                'current_level_available': current_level_available,
                'current_level_overdrawn': current_level_overdrawn,
                'class_skills': [],
                'cross_class_skills': []
            }
            
            
            # Get all skills using the working get_all_skills method
            logger.debug("About to call get_all_skills() from get_skill_summary()")
            all_skills = self.get_all_skills()
            logger.debug(f"get_all_skills() returned {len(all_skills)} skills")
            for skill in all_skills:
                skill_info = {
                    'id': skill['id'],
                    'label': skill['name'],  # Add required label field
                    'name': skill['name'],
                    'key_ability': skill['key_ability'],
                    'current_ranks': skill['ranks'],
                    'max_ranks': skill['max_ranks'],
                    'total_modifier': skill['modifier'],
                    'is_class_skill': skill['is_class_skill'],
                    'armor_check': skill.get('armor_check_penalty', False)
                }
                
                if skill['is_class_skill']:
                    summary['class_skills'].append(skill_info)
                else:
                    summary['cross_class_skills'].append(skill_info)
            
            return summary
        except Exception as e:
            import traceback
            logger.error(f"Error in get_skill_summary: {e}")
            logger.error(f"Traceback: {traceback.format_exc()}")
            # Return minimal safe summary
            return {
                'available_points': 0,
                'total_available': 0,
                'spent_points': 0,
                'overspent': 0,
                'total_ranks': 0,
                'skills_with_ranks': 0,
                'class_skills': [],
                'cross_class_skills': [],
                'error': str(e)
            }
    
    def _calculate_available_skill_points(self) -> int:
        """Get available skill points (called by views)"""
        return self.gff.get('SkillPoints', 0)
    
    def _calculate_spent_skill_points(self) -> int:
        """Calculate total spent skill points (called by views)"""
        skill_list = self.gff.get('SkillList', [])
        total_spent = 0
        
        # Check if we have positional format
        is_positional = False
        if skill_list and isinstance(skill_list[0], dict):
            is_positional = 'Skill' not in skill_list[0]
        
        if is_positional:
            # Handle positional format (index = skill ID)
            for skill_id, skill_entry in enumerate(skill_list):
                if isinstance(skill_entry, dict):
                    ranks = skill_entry.get('Rank', 0)
                    if ranks > 0:
                        # Calculate cost based on whether it's a class skill
                        cost = self.calculate_skill_cost(skill_id, ranks)
                        total_spent += cost
        else:
            # Handle old format
            for skill in skill_list:
                if isinstance(skill, dict):
                    skill_id = skill.get('Skill')
                    ranks = skill.get('Rank', 0)
                    
                    if skill_id is not None and ranks > 0:
                        # Ensure skill_id is an integer
                        try:
                            skill_id = int(skill_id)
                        except (ValueError, TypeError):
                            continue
                        # Calculate cost based on whether it's a class skill
                        cost = self.calculate_skill_cost(skill_id, ranks)
                        total_spent += cost
        
        return total_spent
    
    def validate(self) -> Tuple[bool, List[str]]:
        """Validate current skill configuration - only check for corruption issues"""
        errors = []
        
        skill_list = self.gff.get('SkillList', [])
        
        # Check if we have positional format
        is_positional = False
        if skill_list and isinstance(skill_list[0], dict):
            is_positional = 'Skill' not in skill_list[0]
        
        # Check each skill for corruption issues only
        if is_positional:
            # Handle positional format
            for skill_id, skill_entry in enumerate(skill_list):
                if not isinstance(skill_entry, dict):
                    errors.append(f"Invalid skill entry at position {skill_id}: not a dictionary")
                    continue
                
                ranks = skill_entry.get('Rank', 0)
                
                # Only check for corruption issues
                if ranks < 0:
                    errors.append(f"Skill {skill_id}: negative ranks ({ranks}) can cause save corruption")
                
                # Verify skill ID exists (prevent crash on load)
                if ranks > 0:
                    skill_data = self.game_rules_service.get_by_id('skills', skill_id)
                    if not skill_data:
                        errors.append(f"Invalid skill ID {skill_id}: skill does not exist in game data")
        else:
            # Handle old format
            for skill in skill_list:
                if not isinstance(skill, dict):
                    errors.append(f"Invalid skill entry: not a dictionary")
                    continue
                    
                skill_id = skill.get('Skill')
                if skill_id is None:
                    errors.append(f"Skill entry missing 'Skill' field")
                    continue
                
                # Ensure skill_id is an integer
                try:
                    skill_id = int(skill_id)
                except (ValueError, TypeError):
                    errors.append(f"Invalid skill ID (not a number): {skill_id}")
                    continue
                    
                ranks = skill.get('Rank', 0)
                
                # Only check for corruption issues
                if ranks < 0:
                    errors.append(f"Skill {skill_id}: negative ranks ({ranks}) can cause save corruption")
                
                # Verify skill ID exists (prevent crash on load)
                if ranks > 0:
                    skill_data = self.game_rules_service.get_by_id('skills', skill_id)
                    if not skill_data:
                        errors.append(f"Invalid skill ID {skill_id}: skill does not exist in game data")
        
        return len(errors) == 0, errors
    
    def get_all_skills(self) -> List[Dict[str, Any]]:
        """
        Get list of all skills with current ranks and modifiers
        
        Returns:
            List of skill dictionaries with complete info
        """
        skills = []
        
        # Get all skills from game data
        skills_table = self.game_rules_service.get_table('skills')
        if not skills_table:
            return skills
        
        for skill_id, skill_data in enumerate(skills_table):
            # Use table index as skill ID (standard for 2DA files)
            if skill_id < 0:
                continue
            
            skill_info = {
                'id': skill_id,
                'name': field_mapper.get_field_value(skill_data, 'label', f'Skill {skill_id}'),
                'description': field_mapper.get_field_value(skill_data, 'description', ''),
                'key_ability': field_mapper.get_field_value(skill_data, 'key_ability', 'STR'),
                'ranks': self.get_skill_ranks(skill_id),
                'modifier': self.calculate_skill_modifier(skill_id),
                'is_class_skill': self.is_class_skill(skill_id),
                'max_ranks': self.get_max_skill_ranks(skill_id),
                'armor_check_penalty': field_mapper._safe_int(
                    field_mapper.get_field_value(skill_data, 'armor_check', 0), 0
                ) > 0,
                'untrained': field_mapper._safe_int(
                    field_mapper.get_field_value(skill_data, 'untrained', 1), 1
                ) > 0
            }
            
            skills.append(skill_info)
        
        return skills
    
    def roll_skill_check(self, skill_id: int) -> Dict[str, Any]:
        """
        Roll d20 + skill modifier (simulated)
        
        Args:
            skill_id: The skill ID
            
        Returns:
            Dict with roll result and breakdown
        """
        import random
        
        modifier = self.calculate_skill_modifier(skill_id)
        roll = random.randint(1, 20)
        total = roll + modifier
        
        # Get skill name
        skill_data = self.game_rules_service.get_by_id('skills', skill_id)
        skill_name = field_mapper.get_field_value(skill_data, 'label', f'Skill {skill_id}') if skill_data else f'Skill {skill_id}'
        
        return {
            'skill_id': skill_id,
            'skill_name': skill_name,
            'roll': roll,
            'modifier': modifier,
            'total': total,
            'critical': roll == 20,
            'fumble': roll == 1,
            'breakdown': self._get_modifier_breakdown(skill_id)
        }
    
    def _get_modifier_breakdown(self, skill_id: int) -> Dict[str, int]:
        """Get detailed breakdown of skill modifier"""
        breakdown = {}
        
        # Get ranks
        ranks = self.get_skill_ranks(skill_id)
        breakdown['ranks'] = ranks
        
        # Get ability modifier
        skill_data = self.game_rules_service.get_by_id('skills', skill_id)
        if skill_data:
            key_ability = field_mapper.get_field_value(skill_data, 'key_ability', 'STR')
            if key_ability:
                # Map to character's actual attribute field
                ability_mapping = {'STR': 'Str', 'DEX': 'Dex', 'CON': 'Con', 
                                 'INT': 'Int', 'WIS': 'Wis', 'CHA': 'Cha'}
                if key_ability in ability_mapping:
                    attr_value = self.gff.get(ability_mapping[key_ability], 10)
                    ability_mod = (attr_value - 10) // 2
                    breakdown['ability'] = ability_mod
        
        # Armor check penalty if applicable
        if skill_data:
            armor_check = field_mapper._safe_int(
                field_mapper.get_field_value(skill_data, 'armor_check', 0), 0
            )
            if armor_check > 0:
                armor_penalty = self._get_armor_check_penalty()
                if armor_penalty < 0:
                    breakdown['armor_penalty'] = armor_penalty
        
        # Synergy bonuses
        synergy_bonus = self._calculate_synergy_bonus(skill_id)
        if synergy_bonus > 0:
            breakdown['synergy'] = synergy_bonus
        
        # TODO: Add other bonuses (feats, items, etc.)
        
        return breakdown
    
    def batch_set_skills(self, skills_dict: Dict[int, int]) -> List[Dict[str, Any]]:
        """
        Set multiple skills at once
        
        Args:
            skills_dict: Dict mapping skill_id to ranks
            
        Returns:
            List of results for each skill
        """
        results = []
        
        # Start transaction
        txn = self.character_manager.begin_transaction()
        
        try:
            for skill_id, ranks in skills_dict.items():
                try:
                    success = self.set_skill_rank(skill_id, ranks)
                    results.append({
                        'skill_id': skill_id,
                        'ranks': ranks,
                        'success': success,
                        'error': None
                    })
                except Exception as e:
                    results.append({
                        'skill_id': skill_id,
                        'ranks': ranks,
                        'success': False,
                        'error': str(e)
                    })
            
            self.character_manager.commit_transaction()
            
        except Exception as e:
            self.character_manager.rollback_transaction()
            raise
        
        return results
    
    def is_armor_check_skill(self, skill_id: int) -> bool:
        """
        Check if a skill is affected by armor check penalty
        
        Args:
            skill_id: Skill ID
            
        Returns:
            True if skill has armor check penalty
        """
        skill_data = self.game_rules_service.get_by_id('skills', skill_id)
        if not skill_data:
            return False
            
        # Check the ArmorCheckPenalty field using FieldMappingUtility
        armor_check = field_mapper._safe_int(
            field_mapper.get_field_value(skill_data, 'armor_check_penalty', 0), 0
        )
        return armor_check == 1
    
    def get_skill_prerequisites(self, skill_id: int) -> Dict[str, Any]:
        """
        Get prerequisites for a skill (some skills have requirements)
        
        Args:
            skill_id: The skill ID
            
        Returns:
            Dict with prerequisites
        """
        prerequisites = {
            'skill_id': skill_id,
            'requirements': []
        }
        
        # Most skills don't have prerequisites in D&D/NWN2
        # but some special cases exist
        
        # Knowledge skills might require literacy
        skill_data = self.game_rules_service.get_by_id('skills', skill_id)
        if skill_data:
            skill_name = field_mapper.get_field_value(skill_data, 'label', '')
            
            # Spellcraft requires ability to cast spells
            if 'spellcraft' in skill_name.lower():
                prerequisites['requirements'].append({
                    'type': 'ability',
                    'description': 'Must be able to cast spells or have Use Magic Device'
                })
            
            # Use Magic Device has no prerequisites but is special
            elif 'use magic device' in skill_name.lower():
                prerequisites['requirements'].append({
                    'type': 'note',
                    'description': 'Cannot Take 10 on checks'
                })
        
        return prerequisites
    
    def export_skill_build(self) -> Dict[str, Any]:
        """
        Export current skill allocation for saving/sharing
        
        Returns:
            Dict with skill build data
        """
        skill_list = self.gff.get('SkillList', [])
        
        build = {
            'character_level': sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', [])),
            'total_skill_points': self.calculate_total_skill_points(
                self.gff.get('ClassList', [{}])[0].get('Class', 0),
                sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', []))
            ),
            'skills': {}
        }
        
        # Check if we have positional format
        is_positional = False
        if skill_list and isinstance(skill_list[0], dict):
            is_positional = 'Skill' not in skill_list[0]
        
        if is_positional:
            # Handle positional format
            for skill_id, skill_entry in enumerate(skill_list):
                if isinstance(skill_entry, dict):
                    ranks = skill_entry.get('Rank', 0)
                    if ranks > 0:
                        skill_data = self.game_rules_service.get_by_id('skills', skill_id)
                        skill_name = field_mapper.get_field_value(skill_data, 'label', f'Skill {skill_id}') if skill_data else f'Skill {skill_id}'
                        build['skills'][skill_name] = {
                            'id': skill_id,
                            'ranks': ranks,
                            'is_class_skill': self.is_class_skill(skill_id),
                            'cost': self.calculate_skill_cost(skill_id, ranks)
                        }
        else:
            # Handle old format
            for skill_entry in skill_list:
                if isinstance(skill_entry, dict):
                    skill_id = skill_entry.get('Skill')
                    ranks = skill_entry.get('Rank', 0)
                    if skill_id is not None:
                        # Ensure skill_id is an integer
                        try:
                            skill_id = int(skill_id)
                        except (ValueError, TypeError):
                            continue
                        if skill_id >= 0 and ranks > 0:
                            skill_data = self.game_rules_service.get_by_id('skills', skill_id)
                            skill_name = field_mapper.get_field_value(skill_data, 'label', f'Skill {skill_id}') if skill_data else f'Skill {skill_id}'
                            build['skills'][skill_name] = {
                                'id': skill_id,
                                'ranks': ranks,
                                'is_class_skill': self.is_class_skill(skill_id),
                                'cost': self.calculate_skill_cost(skill_id, ranks)
                            }
        
        return build
    
    def import_skill_build(self, build_data: Dict[str, Any]) -> bool:
        """
        Import a skill build
        
        Args:
            build_data: Skill build data to import
            
        Returns:
            True if successful
        """
        if 'skills' not in build_data:
            raise ValueError("Invalid skill build data")
        
        # Start transaction
        txn = self.character_manager.begin_transaction()
        
        try:
            # Reset all skills first
            self.reset_all_skills()
            
            # Apply new skills
            for skill_name, skill_info in build_data['skills'].items():
                skill_id = skill_info.get('id')
                ranks = skill_info.get('ranks', 0)
                
                if skill_id is not None:
                    self.set_skill_rank(skill_id, ranks)
            
            self.character_manager.commit_transaction()
            return True
            
        except Exception as e:
            self.character_manager.rollback_transaction()
            logger.error(f"Failed to import skill build: {e}")
            raise
    
    def get_unspent_points(self) -> int:
        """
        Calculate unspent skill points (can be negative if overspending)
        
        Returns:
            Number of unspent skill points (negative if overspending)
        """
        # Calculate total available points
        total_level = sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', []))
        primary_class = self.gff.get('ClassList', [{}])[0].get('Class', 0) if self.gff.get('ClassList') else 0
        
        total_available = self.calculate_total_skill_points(primary_class, total_level)
        total_spent = self._calculate_spent_skill_points()
        
        return total_available - total_spent
    
    def get_skill_spending_info(self) -> Dict[str, int]:
        """
        Get detailed skill point spending information
        
        Returns:
            Dict with spending breakdown
        """
        total_level = sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', []))
        primary_class = self.gff.get('ClassList', [{}])[0].get('Class', 0) if self.gff.get('ClassList') else 0
        
        total_available = self.calculate_total_skill_points(primary_class, total_level)
        spent_points = self._calculate_spent_skill_points()
        available_points = self.gff.get('SkillPoints', 0)
        overspent = max(0, spent_points - total_available)
        
        return {
            'total_available': total_available,
            'spent_points': spent_points, 
            'available_points': available_points,
            'overspent': overspent,
            'unspent': max(0, total_available - spent_points)
        }

    def _extract_skills_summary(self) -> Dict[int, int]:
        """Extract skills summary for rules validation"""
        skills = {}
        skill_list = self.gff.get('SkillList', [])
        
        # Check if we have positional format
        is_positional = False
        if skill_list and isinstance(skill_list[0], dict):
            is_positional = 'Skill' not in skill_list[0]
        
        if is_positional:
            # Handle positional format (index = skill ID)
            for skill_id, skill_entry in enumerate(skill_list):
                if isinstance(skill_entry, dict):
                    rank = skill_entry.get('Rank', 0)
                    if rank > 0:
                        skills[skill_id] = rank
        else:
            # Handle old format (list of dicts with 'Skill' field)
            for skill in skill_list:
                if isinstance(skill, dict):
                    skill_id = skill.get('Skill', -1)
                    # Ensure skill_id is an integer
                    try:
                        skill_id = int(skill_id) if skill_id != -1 else -1
                    except (ValueError, TypeError):
                        continue
                    rank = skill.get('Rank', 0)
                    if skill_id >= 0:
                        skills[skill_id] = rank
        
        return skills