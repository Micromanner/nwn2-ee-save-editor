"""
Data-Driven Skill Manager - handles skill points, skill ranks, and class skills
Uses CharacterManager and DynamicGameDataLoader for all skill data access
"""

from typing import Dict, List, Set, Tuple, Optional, Any
import logging
import time

from ..events import EventEmitter, EventType, ClassChangedEvent, LevelGainedEvent
from gamedata.dynamic_loader.field_mapping_utility import field_mapper

logger = logging.getLogger(__name__)


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
        self.game_data_loader = character_manager.game_data_loader
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
        class_data = self.game_data_loader.get_by_id('classes', event.class_id)
        if class_data:
            modifiers = self._calculate_ability_modifiers()
            skill_points_gained = self.calculate_skill_points_for_level(
                class_data, modifiers['INT']
            )
            
            # Add to available points
            current_points = self.gff.get('SkillPoints', 0)
            self.gff.set('SkillPoints', current_points + skill_points_gained)
            
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
        class_data = self.game_data_loader.get_by_id('classes', class_id)
        if not class_data:
            return 0
        
        modifiers = self._calculate_ability_modifiers()
        int_modifier = modifiers['INT']
        
        # Get base skill points per level from class
        base_skill_points = getattr(class_data, 'skill_point_base', 2)
        
        # First level gets 4x skill points
        first_level_points = (base_skill_points + int_modifier) * 4
        first_level_points = max(4, first_level_points)  # Minimum 4 at level 1
        
        # Other levels get normal skill points
        if total_level > 1:
            per_level_points = base_skill_points + int_modifier
            per_level_points = max(1, per_level_points)  # Minimum 1 per level
            
            other_levels_points = per_level_points * (total_level - 1)
            total_points = first_level_points + other_levels_points
        else:
            total_points = first_level_points
        
        # Add human bonus if applicable
        race_id = self.gff.get('Race', 0)
        if race_id == 6:  # Human
            total_points += total_level  # +1 per level
        
        return total_points
    
    def calculate_skill_points_for_level(self, class_data, int_modifier: int) -> int:
        """Calculate skill points gained for a single level"""
        base_skill_points = getattr(class_data, 'skill_point_base', 2)
        base_points = base_skill_points + int_modifier
        base_points = max(1, base_points)  # Minimum 1
        
        # Add human bonus
        race_id = self.gff.get('Race', 0)
        if race_id == 6:  # Human
            base_points += 1
        
        return base_points
    
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
        skill_data = self.game_data_loader.get_by_id('skills', skill_id)
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
                    if skill.get('Skill') == skill_id:
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
        
        # Update available points (can go negative - user freedom)
        self.gff.set('SkillPoints', available_points - net_cost)
        
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
            if isinstance(skill, dict) and skill.get('Skill') == skill_id:
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
        """Calculate skill point cost for ranks - removed cross-class penalties for user freedom"""
        if ranks == 0:
            return 0
        
        # All skills cost 1 point per rank - no cross-class penalties
        # This allows users to freely allocate skill points without restrictions
        return ranks
    
    def calculate_skill_modifier(self, skill_id: int) -> int:
        """Calculate total skill modifier including ranks and ability bonus"""
        # Get base ranks
        ranks = self.get_skill_ranks(skill_id)
        
        # Get ability modifier using dynamic skill data
        skill_data = self.game_data_loader.get_by_id('skills', skill_id)
        key_ability = getattr(skill_data, 'KeyAbility', 'STR').upper() if skill_data else 'STR'
        
        modifiers = self._calculate_ability_modifiers()
        ability_mod = modifiers.get(key_ability, 0)
        
        # Check for skill synergies
        synergy_bonus = self._calculate_synergy_bonus(skill_id)
        
        # Check for armor check penalty
        armor_penalty = 0
        if skill_data and getattr(skill_data, 'armor_check_penalty', 0) > 0:
            armor_penalty = self._get_armor_check_penalty()
        
        total = ranks + ability_mod + synergy_bonus - armor_penalty
        
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
        class_data = self.game_data_loader.get_by_id('classes', class_id)
        
        if class_data:
            # Get the skills table name from class data
            skills_table_name = getattr(class_data, 'SkillsTable', None)
            if skills_table_name:
                # Load the specific class skills table (e.g., "cls_skill_bard")
                class_skills_table = self.game_data_loader.get_table(skills_table_name.lower())
                if class_skills_table:
                    for skill_entry in class_skills_table:
                        # Check for ClassSkill field to ensure it's actually a class skill
                        is_class_skill = getattr(skill_entry, 'ClassSkill', '0')
                        if is_class_skill == '1' or is_class_skill == 1:
                            skill_id = getattr(skill_entry, 'SkillIndex', None)
                            if skill_id is not None:
                                try:
                                    class_skills.add(int(skill_id))
                                except (ValueError, TypeError):
                                    pass
                else:
                    logger.warning(f"Class skills table {skills_table_name} not found")
                
                # Log if no class skills were found
                if not class_skills:
                    logger.warning(f"No class skills found for class {field_mapper.get_field_value(class_data, 'label', 'Unknown')}")
        
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
    
    def _calculate_ability_modifiers(self) -> Dict[str, int]:
        """Calculate ability modifiers"""
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
        
        skill_data = self.game_data_loader.get_by_id('skills', skill_id)
        if skill_data:
            info = {
                'id': skill_id,
                'label': field_mapper.get_field_value(skill_data, 'label', 'Unknown'),
                'name': getattr(skill_data, 'name', 'Unknown'),
                'key_ability': getattr(skill_data, 'KeyAbility', 'STR'),
                'armor_check': getattr(skill_data, 'armor_check_penalty', 0) > 0,
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
            
            summary = {
                'available_points': available_points,
                'total_available': total_available,
                'spent_points': spent_points,
                'overspent': overspent,
                'total_ranks': sum(s.get('Rank', 0) for s in skill_list if isinstance(s, dict)),
                'skills_with_ranks': skills_with_ranks,
                'class_skills': [],
                'cross_class_skills': []
            }
            
            
            if is_positional:
                # Handle positional format (index = skill ID)
                for skill_id, skill_entry in enumerate(skill_list):
                    if not isinstance(skill_entry, dict):
                        continue
                    
                    rank = skill_entry.get('Rank', 0)
                    if rank > 0:
                        skill_info = self.get_skill_info(skill_id)
                        if skill_info:
                            if skill_info['is_class_skill']:
                                summary['class_skills'].append(skill_info)
                            else:
                                summary['cross_class_skills'].append(skill_info)
            else:
                # Handle old format (list of dicts with 'Skill' field)
                for skill in skill_list:
                    # Validate skill entry
                    if not isinstance(skill, dict):
                        logger.warning(f"Invalid skill entry in SkillList: {skill}")
                        continue
                        
                    skill_id = skill.get('Skill')
                    if skill_id is None:
                        logger.warning(f"Skill entry missing 'Skill' field: {skill}")
                        continue
                        
                    skill_info = self.get_skill_info(skill_id)
                    
                    if skill_info:
                        if skill_info['is_class_skill']:
                            summary['class_skills'].append(skill_info)
                        else:
                            summary['cross_class_skills'].append(skill_info)
            
            return summary
        except Exception as e:
            logger.error(f"Error in get_skill_summary: {e}")
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
                    skill_data = self.game_data_loader.get_by_id('skills', skill_id)
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
                    
                ranks = skill.get('Rank', 0)
                
                # Only check for corruption issues
                if ranks < 0:
                    errors.append(f"Skill {skill_id}: negative ranks ({ranks}) can cause save corruption")
                
                # Verify skill ID exists (prevent crash on load)
                if ranks > 0:
                    skill_data = self.game_data_loader.get_by_id('skills', skill_id)
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
        skills_table = self.game_data_loader.get_table('skills')
        if not skills_table:
            return skills
        
        for skill_data in skills_table:
            skill_id = field_mapper.get_field_value(skill_data, 'id', -1)
            if skill_id < 0:
                continue
            
            skill_info = {
                'id': skill_id,
                'name': field_mapper.get_field_value(skill_data, 'label', f'Skill {skill_id}'),
                'description': field_mapper.get_field_value(skill_data, 'description', ''),
                'key_ability': getattr(skill_data, 'KeyAbility', ''),
                'ranks': self.get_skill_ranks(skill_id),
                'modifier': self.calculate_skill_modifier(skill_id),
                'is_class_skill': self.is_class_skill(skill_id),
                'max_ranks': self.get_max_skill_ranks(skill_id),
                'armor_check_penalty': field_mapper.get_field_value(skill_data, 'armor_check', 0) > 0,
                'untrained': field_mapper.get_field_value(skill_data, 'untrained', 1) > 0
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
        skill_data = self.game_data_loader.get_by_id('skills', skill_id)
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
        skill_data = self.game_data_loader.get_by_id('skills', skill_id)
        if skill_data:
            key_ability = getattr(skill_data, 'KeyAbility', '')
            if key_ability:
                # Map to character's actual attribute field
                ability_mapping = {'STR': 'Str', 'DEX': 'Dex', 'CON': 'Con', 
                                 'INT': 'Int', 'WIS': 'Wis', 'CHA': 'Cha'}
                if key_ability in ability_mapping:
                    attr_value = self.gff.get(ability_mapping[key_ability], 10)
                    ability_mod = (attr_value - 10) // 2
                    breakdown['ability'] = ability_mod
        
        # Armor check penalty if applicable
        if skill_data and field_mapper.get_field_value(skill_data, 'armor_check', 0) > 0:
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
        skill_data = self.game_data_loader.get_by_id('skills', skill_id)
        if not skill_data:
            return False
            
        # Check the ArmorCheckPenalty field
        armor_check = getattr(skill_data, 'ArmorCheckPenalty', 0)
        # Convert string to int if needed
        try:
            return int(armor_check) == 1
        except (ValueError, TypeError):
            return bool(armor_check)
    
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
        skill_data = self.game_data_loader.get_by_id('skills', skill_id)
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
                        skill_data = self.game_data_loader.get_by_id('skills', skill_id)
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
                    if skill_id is not None and skill_id >= 0 and ranks > 0:
                        skill_data = self.game_data_loader.get_by_id('skills', skill_id)
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