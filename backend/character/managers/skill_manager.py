"""Skill manager for skill points, ranks, and class skills."""

from typing import Dict, List, Set, Tuple, Optional, Any
from loguru import logger
import time

from ..events import EventEmitter, EventType, ClassChangedEvent, LevelGainedEvent, SkillPointsAwardedEvent
from gamedata.dynamic_loader.field_mapping_utility import field_mapper


class SkillManager(EventEmitter):
    """Manages skill points, ranks, modifiers, and class skill detection."""

    def __init__(self, character_manager):
        super().__init__()
        self.character_manager = character_manager
        self.game_rules_service = character_manager.rules_service
        self.gff = character_manager.gff
        self._register_event_handlers()
        self._skill_cache = {}
        self._class_skills_cache = {}

    def _register_event_handlers(self):
        """Register handlers for class changes and level gains."""
        self.character_manager.on(EventType.CLASS_CHANGED, self.on_class_changed)
        self.character_manager.on(EventType.LEVEL_GAINED, self.on_level_gained)

    def on_class_changed(self, event: ClassChangedEvent):
        """Reset skills and recalculate points when class changes."""
        logger.info(f"SkillManager handling class change: {event.old_class_id} -> {event.new_class_id}")

        if event.is_level_adjustment:
            logger.info("Skipping skill reset - this is a level adjustment, not a class swap")
            self._update_class_skills_cache(event.new_class_id)
            return

        total_skill_points = self.calculate_total_skill_points(event.new_class_id, event.level)
        self.reset_all_skills()
        self.gff.set('SkillPoints', total_skill_points)
        self._update_class_skills_cache(event.new_class_id)
        logger.info(f"Reset skills with {total_skill_points} points available")

    def on_level_gained(self, event: LevelGainedEvent):
        """Award skill points when a level is gained."""
        logger.info(f"SkillManager handling level gain: Level {event.new_level}")

        class_data = self.game_rules_service.get_by_id('classes', event.class_id)
        if not class_data:
            raise ValueError(f"Class {event.class_id} not found in game data")

        modifiers = self._get_ability_modifiers()
        skill_points_gained = self.calculate_skill_points_for_level(
            class_data, modifiers['INT'], is_first_level=(event.new_level == 1)
        )

        current_points = self.gff.get('SkillPoints', 0)
        self.gff.set('SkillPoints', current_points + skill_points_gained)

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
        """Calculate total skill points for a class at given level."""
        class_data = self.game_rules_service.get_by_id('classes', class_id)
        if not class_data:
            raise ValueError(f"Class {class_id} not found in game data")

        modifiers = self._get_ability_modifiers()
        int_modifier = modifiers['INT']

        base_skill_points = field_mapper.get_field_value(class_data, 'skill_point_base')
        if base_skill_points is None:
            raise ValueError(f"Class {class_id} missing skill_point_base field")
        base_skill_points = int(base_skill_points)

        racial_bonus_base = self._get_racial_skill_point_bonus_base()

        # NWN2: adjustments apply before level 1 multiplication
        points_per_level = base_skill_points + int_modifier + racial_bonus_base
        points_per_level = max(1, points_per_level)

        level_1_points = max(4, points_per_level * 4)

        if total_level <= 1:
            return level_1_points

        subsequent_levels = total_level - 1
        return level_1_points + (points_per_level * subsequent_levels)
    
    def calculate_skill_points_for_level(self, class_data, int_modifier: int, is_first_level: bool = False) -> int:
        """Calculate skill points gained for a single level."""
        base_skill_points = field_mapper.get_field_value(class_data, 'skill_point_base')
        if base_skill_points is None:
            raise ValueError("Class data missing skill_point_base field")
        base_skill_points = int(base_skill_points)

        racial_bonus_base = self._get_racial_skill_point_bonus_base()

        points = base_skill_points + int_modifier + racial_bonus_base
        points = max(1, points)

        if is_first_level:
            points *= 4
            points = max(4, points)

        return points
    
    def set_skill_rank(self, skill_id: int, ranks: int) -> bool:
        """Set ranks in a skill, returning True if successful."""
        skill_data = self.game_rules_service.get_by_id('skills', skill_id)
        if not skill_data:
            logger.warning(f"Invalid skill ID: {skill_id}")
            return False

        if ranks < 0:
            logger.warning(f"Cannot set negative ranks ({ranks}) in skill {skill_id}")
            return False

        available_points = self.gff.get('SkillPoints', 0)
        current_ranks = self.get_skill_ranks(skill_id)
        current_cost = self.calculate_skill_cost(skill_id, current_ranks)
        new_cost = self.calculate_skill_cost(skill_id, ranks)
        net_cost = new_cost - current_cost

        max_ranks = self.get_max_skill_ranks(skill_id)
        if ranks > max_ranks:
            logger.info(f"Setting {ranks} ranks in skill {skill_id} exceeds normal maximum of {max_ranks}")

        if net_cost > available_points:
            logger.info(f"Skill allocation uses {net_cost} points (have {available_points} available)")

        skill_list = self.gff.get('SkillList', [])

        # NWN2 uses positional format (index = skill ID) or legacy dict format
        if isinstance(skill_list, list) and len(skill_list) > 0 and isinstance(skill_list[0], dict):
            first_entry = skill_list[0]
            is_positional = 'Skill' not in first_entry

            if is_positional:
                while len(skill_list) <= skill_id:
                    skill_list.append({'Rank': 0})
                skill_list[skill_id] = {'Rank': ranks}
            else:
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
                        skill_list.remove(skill_entry)
        else:
            skill_list = [{'Rank': ranks if i == skill_id else 0} for i in range(skill_id + 1)]

        self.gff.set('SkillList', skill_list)

        rank_delta = ranks - current_ranks
        if rank_delta != 0:
            class_manager = self.character_manager.get_manager('class')
            if class_manager:
                class_manager.record_skill_change(skill_id, rank_delta)

        self.gff.set('SkillPoints', available_points - net_cost)

        if skill_id in self._skill_cache:
            del self._skill_cache[skill_id]

        logger.info(f"Set skill {skill_id} to {ranks} ranks")
        return True
    
    def get_skill_ranks(self, skill_id: int) -> int:
        """Get current ranks in a skill."""
        skill_list = self.gff.get('SkillList', [])

        # Positional format: index = skill ID
        if isinstance(skill_list, list) and 0 <= skill_id < len(skill_list):
            skill_entry = skill_list[skill_id]
            if isinstance(skill_entry, dict):
                return skill_entry.get('Rank', 0)

        # Legacy format: list of dicts with 'Skill' field
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
        """Get maximum ranks allowed in a skill based on level."""
        total_level = self._get_total_level()

        if self.is_class_skill(skill_id):
            return total_level + 3
        return (total_level + 3) // 2

    def is_class_skill(self, skill_id: int) -> bool:
        """Check if a skill is a class skill for any of the character's classes."""
        class_list = self.gff.get('ClassList', [])

        for class_entry in class_list:
            class_id = class_entry.get('Class')
            class_skills = self._get_class_skills(class_id)
            if skill_id in class_skills:
                return True

        return False

    def calculate_skill_cost(self, skill_id: int, ranks: int) -> int:
        """Calculate skill point cost (1 for class skills, 2 for cross-class, 1 with Able Learner)."""
        if ranks == 0:
            return 0

        if self.is_class_skill(skill_id):
            return ranks

        # Able Learner feat (ID 406) makes cross-class skills cost 1 point
        feat_manager = self.character_manager.get_manager('feat')
        if feat_manager and feat_manager.has_feat(406):
            return ranks

        return ranks * 2

    def calculate_skill_modifier(self, skill_id: int) -> int:
        """Calculate total skill modifier including ranks, ability, and equipment."""
        ranks = self.get_skill_ranks(skill_id)

        skill_data = self.game_rules_service.get_by_id('skills', skill_id)
        if not skill_data:
            raise ValueError(f"Skill {skill_id} not found in game data")

        key_ability = field_mapper.get_field_value(skill_data, 'key_ability')
        if not key_ability:
            raise ValueError(f"Skill {skill_id} missing key_ability field")
        key_ability = key_ability.upper()

        modifiers = self._get_ability_modifiers()
        ability_mod = modifiers.get(key_ability, 0)

        inventory_manager = self.character_manager.get_manager('inventory')
        equipment_skill_bonus = 0
        if inventory_manager:
            equipment_bonuses = inventory_manager.get_equipment_bonuses()
            skill_bonuses = equipment_bonuses.get('skills', {}) or {}
            skill_name = field_mapper.get_field_value(skill_data, 'label', '') or ''
            equipment_skill_bonus = skill_bonuses.get(skill_name, 0)

        return ranks + ability_mod + equipment_skill_bonus
    
    def reset_all_skills(self):
        """Reset all skill ranks to 0 and refund spent points."""
        logger.info("Resetting all skills")

        total_refund = self._calculate_spent_skill_points()
        skill_list = self.gff.get('SkillList', [])

        is_positional = skill_list and isinstance(skill_list[0], dict) and 'Skill' not in skill_list[0]

        if is_positional:
            for i in range(len(skill_list)):
                if isinstance(skill_list[i], dict):
                    skill_list[i]['Rank'] = 0
        else:
            for skill in skill_list:
                if isinstance(skill, dict) and 'Rank' in skill:
                    skill['Rank'] = 0

        self.gff.set('SkillList', skill_list)
        current_available = self.gff.get('SkillPoints', 0)
        self.gff.set('SkillPoints', current_available + total_refund)
        logger.info(f"Reset all skills, refunded {total_refund} points")

    def _get_class_skills(self, class_id: int) -> Set[int]:
        """Get set of class skill IDs for a class from cls_skill_* table."""
        if class_id in self._class_skills_cache:
            return self._class_skills_cache[class_id]

        class_skills = set()
        class_data = self.game_rules_service.get_by_id('classes', class_id)

        if not class_data:
            self._class_skills_cache[class_id] = class_skills
            return class_skills

        skills_table_name = field_mapper.get_field_value(class_data, 'skills_table')
        if not skills_table_name:
            logger.warning(f"Class {class_id} missing skills_table field")
            self._class_skills_cache[class_id] = class_skills
            return class_skills

        class_skills_table = self.game_rules_service.get_table(skills_table_name.lower())
        if not class_skills_table:
            logger.warning(f"Class skills table {skills_table_name} not found")
            self._class_skills_cache[class_id] = class_skills
            return class_skills

        for skill_entry in class_skills_table:
            is_class_skill = field_mapper.get_field_value(skill_entry, 'class_skill', '0')
            if is_class_skill == '1' or is_class_skill == 1:
                skill_id = field_mapper.get_field_value(skill_entry, 'skill_index')
                if skill_id is not None:
                    try:
                        skill_id_int = int(skill_id)
                        if skill_id_int >= 0:
                            class_skills.add(skill_id_int)
                    except (ValueError, TypeError):
                        continue

        self._class_skills_cache[class_id] = class_skills
        return class_skills

    def _update_class_skills_cache(self, primary_class_id: int):
        """Clear and repopulate class skills cache for all current classes."""
        self._class_skills_cache.clear()
        class_list = self.gff.get('ClassList', [])
        for class_entry in class_list:
            class_id = class_entry.get('Class')
            self._get_class_skills(class_id)

    def _get_racial_skill_point_bonus_base(self) -> int:
        """Get racial skill point bonus per level (NWN2 engine: humans get +1)."""
        race_manager = self.character_manager.get_manager('race')
        if not race_manager:
            raise RuntimeError("RaceManager not available")

        racial_props = race_manager.get_racial_properties()
        race_name = racial_props.get('race_name', '').lower()

        # NWN2 engine hardcodes human +1 skill/level (no 2DA field exists for this)
        if 'human' in race_name:
            return 1

        return 0

    def _get_ability_modifiers(self) -> Dict[str, int]:
        """Get ability modifiers from AbilityManager."""
        attr_manager = self.character_manager.get_manager('ability')
        if not attr_manager:
            raise RuntimeError("AbilityManager not available")
        return attr_manager.get_all_modifiers()

    def _get_total_level(self) -> int:
        """Get total character level from ClassManager."""
        class_manager = self.character_manager.get_manager('class')
        if not class_manager:
            raise RuntimeError("ClassManager not available")
        return class_manager.get_total_level()

    def get_skill_info(self, skill_id: int) -> Optional[Dict[str, Any]]:
        """Get detailed information about a skill, or None if not found."""
        if skill_id in self._skill_cache:
            return self._skill_cache[skill_id]

        skill_data = self.game_rules_service.get_by_id('skills', skill_id)
        if not skill_data:
            return None

        label = field_mapper.get_field_value(skill_data, 'label')
        key_ability = field_mapper.get_field_value(skill_data, 'key_ability')
        armor_check_val = field_mapper.get_field_value(skill_data, 'armor_check_penalty')
        
        tlk_name = None
        if hasattr(skill_data, 'Name'):
            raw_name = skill_data.Name
            if raw_name and not str(raw_name).isdigit():
                tlk_name = raw_name

        info = {
            'id': skill_id,
            'label': label or f'Skill_{skill_id}',
            'name': tlk_name or label or f'Skill_{skill_id}',
            'key_ability': key_ability or 'STR',
            'armor_check': int(armor_check_val or 0) > 0,
            'is_class_skill': self.is_class_skill(skill_id),
            'current_ranks': self.get_skill_ranks(skill_id),
            'max_ranks': self.get_max_skill_ranks(skill_id),
            'total_modifier': self.calculate_skill_modifier(skill_id)
        }
        self._skill_cache[skill_id] = info
        return info
    
    def get_skill_summary(self) -> Dict[str, Any]:
        """Get comprehensive skill summary including points, ranks, and categorized skills."""
        try:
            skill_list = self.gff.get('SkillList', [])
            available_points = self.gff.get('SkillPoints', 0)
            spent_points = self._calculate_spent_skill_points()

            total_level = self._get_total_level()
            primary_class = self.gff.get('ClassList', [{}])[0].get('Class', 0) if self.gff.get('ClassList') else 0
            total_available = self.calculate_total_skill_points(primary_class, total_level)
            overspent = max(0, spent_points - total_available)

            is_positional = skill_list and isinstance(skill_list[0], dict) and 'Skill' not in skill_list[0]

            if is_positional:
                skills_with_ranks = len([s for s in skill_list if isinstance(s, dict) and s.get('Rank', 0) > 0])
            else:
                skills_with_ranks = len([s for s in skill_list if isinstance(s, dict) and s.get('Skill') is not None and s.get('Rank', 0) > 0])

            current_level_gained = 0
            current_level_spent = 0

            lvl_stat_list = self.gff.get('LvlStatList', [])
            if lvl_stat_list and isinstance(lvl_stat_list, list):
                last_entry = lvl_stat_list[-1]
                recorded_gained = last_entry.get('SkillPoints', 0)

                class_id = last_entry.get('LvlStatClass', -1)
                class_data = self.game_rules_service.get_by_id('classes', class_id)
                modifiers = self._get_ability_modifiers()
                int_mod = modifiers.get('INT', 0)
                is_first_level = len(lvl_stat_list) == 1

                expected_gained = self.calculate_skill_points_for_level(class_data, int_mod, is_first_level)
                current_level_gained = max(recorded_gained, expected_gained)

                history_skill_list = last_entry.get('SkillList', [])
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

            all_skills = self.get_all_skills()
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
        """Get available skill points from GFF."""
        return self.gff.get('SkillPoints', 0)

    def _calculate_spent_skill_points(self) -> int:
        """Calculate total skill points spent on all skills."""
        skill_list = self.gff.get('SkillList', [])
        total_spent = 0

        is_positional = skill_list and isinstance(skill_list[0], dict) and 'Skill' not in skill_list[0]

        if is_positional:
            for skill_id, skill_entry in enumerate(skill_list):
                if isinstance(skill_entry, dict):
                    ranks = skill_entry.get('Rank', 0)
                    if ranks > 0:
                        cost = self.calculate_skill_cost(skill_id, ranks)
                        total_spent += cost
        else:
            for skill in skill_list:
                if isinstance(skill, dict):
                    skill_id = skill.get('Skill')
                    ranks = skill.get('Rank', 0)
                    if skill_id is not None and ranks > 0:
                        try:
                            skill_id = int(skill_id)
                        except (ValueError, TypeError):
                            continue
                        cost = self.calculate_skill_cost(skill_id, ranks)
                        total_spent += cost

        return total_spent

    def validate(self) -> Tuple[bool, List[str]]:
        """Validate skill configuration for corruption-causing issues only."""
        errors = []
        skill_list = self.gff.get('SkillList', [])

        is_positional = skill_list and isinstance(skill_list[0], dict) and 'Skill' not in skill_list[0]

        if is_positional:
            for skill_id, skill_entry in enumerate(skill_list):
                if not isinstance(skill_entry, dict):
                    errors.append(f"Invalid skill entry at position {skill_id}: not a dictionary")
                    continue

                ranks = skill_entry.get('Rank', 0)
                if ranks < 0:
                    errors.append(f"Skill {skill_id}: negative ranks ({ranks}) can cause save corruption")

                if ranks > 0:
                    skill_data = self.game_rules_service.get_by_id('skills', skill_id)
                    if not skill_data:
                        errors.append(f"Invalid skill ID {skill_id}: skill does not exist in game data")
        else:
            for skill in skill_list:
                if not isinstance(skill, dict):
                    errors.append("Invalid skill entry: not a dictionary")
                    continue

                skill_id = skill.get('Skill')
                if skill_id is None:
                    errors.append("Skill entry missing 'Skill' field")
                    continue

                try:
                    skill_id = int(skill_id)
                except (ValueError, TypeError):
                    errors.append(f"Invalid skill ID (not a number): {skill_id}")
                    continue

                ranks = skill.get('Rank', 0)
                if ranks < 0:
                    errors.append(f"Skill {skill_id}: negative ranks ({ranks}) can cause save corruption")

                if ranks > 0:
                    skill_data = self.game_rules_service.get_by_id('skills', skill_id)
                    if not skill_data:
                        errors.append(f"Invalid skill ID {skill_id}: skill does not exist in game data")

        return len(errors) == 0, errors

    def get_all_skills(self) -> List[Dict[str, Any]]:
        """Get list of all skills from game data with current character state."""
        skills = []
        skills_table = self.game_rules_service.get_table('skills')
        if not skills_table:
            return skills

        for skill_id, skill_data in enumerate(skills_table):
            if skill_id < 0:
                continue

            label = field_mapper.get_field_value(skill_data, 'label')
            key_ability = field_mapper.get_field_value(skill_data, 'key_ability')
            armor_check = field_mapper.get_field_value(skill_data, 'armor_check')
            untrained = field_mapper.get_field_value(skill_data, 'untrained')
            
            tlk_name = None
            if hasattr(skill_data, 'Name'):
                raw_name = skill_data.Name
                if raw_name and not str(raw_name).isdigit():
                    tlk_name = raw_name

            skill_info = {
                'id': skill_id,
                'name': tlk_name or label or f'Skill_{skill_id}',
                'description': field_mapper.get_field_value(skill_data, 'description') or '',
                'key_ability': key_ability or 'STR',
                'ranks': self.get_skill_ranks(skill_id),
                'modifier': self.calculate_skill_modifier(skill_id),
                'is_class_skill': self.is_class_skill(skill_id),
                'max_ranks': self.get_max_skill_ranks(skill_id),
                'armor_check_penalty': int(armor_check or 0) > 0,
                'untrained': int(untrained or 1) > 0
            }

            skills.append(skill_info)

        return skills

    def roll_skill_check(self, skill_id: int, take_10: bool = False, take_20: bool = False, 
                        circumstance_bonus: int = 0) -> Dict[str, Any]:
        """Simulate a d20 + skill modifier roll with options."""
        import random

        modifier = self.calculate_skill_modifier(skill_id)
        
        if take_20:
            roll = 20
        elif take_10:
            roll = 10
        else:
            roll = random.randint(1, 20)
            
        total = roll + modifier + circumstance_bonus

        skill_data = self.game_rules_service.get_by_id('skills', skill_id)
        skill_name = field_mapper.get_field_value(skill_data, 'label') if skill_data else None

        return {
            'skill_id': skill_id,
            'skill_name': skill_name or f'Skill_{skill_id}',
            'roll': roll,
            'modifier': modifier,
            'circumstance': circumstance_bonus,
            'total': total,
            'critical': roll == 20 and not (take_10 or take_20),
            'fumble': roll == 1 and not (take_10 or take_20),
            'breakdown': self._get_modifier_breakdown(skill_id)
        }
    
    def _get_modifier_breakdown(self, skill_id: int) -> Dict[str, int]:
        """Get detailed breakdown of skill modifier components."""
        breakdown = {}
        breakdown['ranks'] = self.get_skill_ranks(skill_id)

        skill_data = self.game_rules_service.get_by_id('skills', skill_id)
        if skill_data:
            key_ability = field_mapper.get_field_value(skill_data, 'key_ability')
            if key_ability:
                modifiers = self._get_ability_modifiers()
                breakdown['ability'] = modifiers.get(key_ability.upper(), 0)

        return breakdown
    
    def batch_set_skills(self, skills_dict: Dict[int, int]) -> List[Dict[str, Any]]:
        """Set multiple skills at once within a transaction."""
        results = []
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
        """Check if a skill is affected by armor check penalty."""
        skill_data = self.game_rules_service.get_by_id('skills', skill_id)
        if not skill_data:
            return False

        armor_check = field_mapper.get_field_value(skill_data, 'armor_check_penalty')
        return int(armor_check or 0) == 1

    def get_skill_prerequisites(self, skill_id: int) -> Dict[str, Any]:
        """Get prerequisites for a skill (NWN2 skills have no 2DA prerequisites)."""
        return {
            'skill_id': skill_id,
            'requirements': []
        }

    def export_skill_build(self) -> Dict[str, Any]:
        """Export current skill allocation for saving/sharing."""
        skill_list = self.gff.get('SkillList', [])
        total_level = self._get_total_level()
        primary_class = self.gff.get('ClassList', [{}])[0].get('Class', 0) if self.gff.get('ClassList') else 0

        build = {
            'character_level': total_level,
            'total_skill_points': self.calculate_total_skill_points(primary_class, total_level),
            'skills': {}
        }

        is_positional = skill_list and isinstance(skill_list[0], dict) and 'Skill' not in skill_list[0]

        if is_positional:
            for skill_id, skill_entry in enumerate(skill_list):
                if isinstance(skill_entry, dict):
                    ranks = skill_entry.get('Rank', 0)
                    if ranks > 0:
                        skill_data = self.game_rules_service.get_by_id('skills', skill_id)
                        skill_name = field_mapper.get_field_value(skill_data, 'label') if skill_data else None
                        build['skills'][skill_name or f'Skill_{skill_id}'] = {
                            'id': skill_id,
                            'ranks': ranks,
                            'is_class_skill': self.is_class_skill(skill_id),
                            'cost': self.calculate_skill_cost(skill_id, ranks)
                        }
        else:
            for skill_entry in skill_list:
                if isinstance(skill_entry, dict):
                    skill_id = skill_entry.get('Skill')
                    ranks = skill_entry.get('Rank', 0)
                    if skill_id is not None:
                        try:
                            skill_id = int(skill_id)
                        except (ValueError, TypeError):
                            continue
                        if skill_id >= 0 and ranks > 0:
                            skill_data = self.game_rules_service.get_by_id('skills', skill_id)
                            skill_name = field_mapper.get_field_value(skill_data, 'label') if skill_data else None
                            build['skills'][skill_name or f'Skill_{skill_id}'] = {
                                'id': skill_id,
                                'ranks': ranks,
                                'is_class_skill': self.is_class_skill(skill_id),
                                'cost': self.calculate_skill_cost(skill_id, ranks)
                            }

        return build

    def import_skill_build(self, build_data: Dict[str, Any]) -> bool:
        """Import a skill build, resetting existing skills first."""
        if 'skills' not in build_data:
            raise ValueError("Invalid skill build data: missing 'skills' key")

        txn = self.character_manager.begin_transaction()

        try:
            self.reset_all_skills()

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
        """Calculate unspent skill points (can be negative if overspending)."""
        total_level = self._get_total_level()
        primary_class = self.gff.get('ClassList', [{}])[0].get('Class', 0) if self.gff.get('ClassList') else 0

        total_available = self.calculate_total_skill_points(primary_class, total_level)
        total_spent = self._calculate_spent_skill_points()

        return total_available - total_spent

    def get_skill_spending_info(self) -> Dict[str, int]:
        """Get detailed skill point spending breakdown."""
        total_level = self._get_total_level()
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
            'remaining': available_points  # Alias for available_points
        }

    def update_skills(self, skills_dict: Dict[str, int]) -> Tuple[List[Any], List[str]]:
        """Update multiple skills, returning changes and validation errors."""
        changes = []
        validation_errors = []
        
        for skill_id_str, new_rank in skills_dict.items():
            try:
                skill_id = int(skill_id_str)
            except (ValueError, TypeError):
                validation_errors.append(f"Invalid skill ID: {skill_id_str}")
                continue
                
            skill_data = self.game_rules_service.get_by_id('skills', skill_id)
            if not skill_data:
                validation_errors.append(f"Skill {skill_id} does not exist")
                continue
                
            if new_rank < 0:
                validation_errors.append(f"Cannot set negative ranks ({new_rank}) for skill {skill_id}")
                continue
            
            old_rank = self.get_skill_ranks(skill_id)
            if self.set_skill_rank(skill_id, new_rank):
                skill_info = self.get_skill_info(skill_id)
                skill_name = skill_info['name'] if skill_info else f'Skill {skill_id}'
                
                changes.append({
                    'skill_id': skill_id,
                    'skill_name': skill_name,
                    'old_rank': old_rank,
                    'new_rank': new_rank,
                    'points_spent': self.calculate_skill_cost(skill_id, new_rank),
                    'new_total_modifier': self.calculate_skill_modifier(skill_id)
                })
            else:
                validation_errors.append(f"Failed to set skill {skill_id} to {new_rank} ranks")
                
        return changes, validation_errors


    def _extract_skills_summary(self) -> Dict[int, int]:
        """Extract skill_id -> rank mapping for rules validation."""
        skills = {}
        skill_list = self.gff.get('SkillList', [])

        is_positional = skill_list and isinstance(skill_list[0], dict) and 'Skill' not in skill_list[0]

        if is_positional:
            for skill_id, skill_entry in enumerate(skill_list):
                if isinstance(skill_entry, dict):
                    rank = skill_entry.get('Rank', 0)
                    if rank > 0:
                        skills[skill_id] = rank
        else:
            for skill in skill_list:
                if isinstance(skill, dict):
                    skill_id = skill.get('Skill', -1)
                    try:
                        skill_id = int(skill_id) if skill_id != -1 else -1
                    except (ValueError, TypeError):
                        continue
                    rank = skill.get('Rank', 0)
                    if skill_id >= 0:
                        skills[skill_id] = rank

        return skills