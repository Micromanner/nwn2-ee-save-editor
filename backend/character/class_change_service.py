"""
Data-Driven Character Class Change Service
Uses CharacterManager and DynamicGameDataLoader for all character data access
"""

from typing import Dict, List, Optional, Tuple, Any, TYPE_CHECKING
from django.db import transaction
from django.db.models.signals import pre_save
import random
import logging
from dataclasses import dataclass, field, asdict

from .models import Character, CharacterClass, CharacterFeat

if TYPE_CHECKING:
    from .character_manager import CharacterManager

logger = logging.getLogger(__name__)


@dataclass
class ClassChangeResult:
    """Structured result of a class change operation"""
    old_class_id: int
    new_class_id: int
    character_level: int
    changes_made: List[str]
    hp_change: Optional[Tuple[int, int]] = None  # (old_hp, new_hp)
    bab_change: Optional[Tuple[int, int]] = None  # (old_bab, new_bab)
    save_changes: Optional[Dict[str, Tuple[int, int]]] = None  # {save_type: (old, new)}
    feats_added: List[int] = field(default_factory=list)
    feats_removed: List[int] = field(default_factory=list)
    skill_points_change: Optional[Tuple[int, int]] = None  # (old, new)
    spell_changes: Optional[Dict[str, Any]] = None
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for backward compatibility"""
        return asdict(self)


class ClassChangeService:
    """
    Data-Driven Class Change Service
    Uses CharacterManager as hub for all character data access
    """
    
    def __init__(self, character_manager: 'CharacterManager'):
        """
        Initialize with CharacterManager instance
        
        Args:
            character_manager: CharacterManager instance providing data access
        """
        self.character_manager = character_manager
        self.game_data_loader = character_manager.game_data_loader
        self.gff = character_manager.gff
    
    @transaction.atomic
    def change_class(self, character: Character, new_class_id: int, 
                    preserve_level: bool = True, cheat_mode: bool = False) -> Dict[str, Any]:
        """
        Change a character's class
        
        Args:
            character: The character to modify
            new_class_id: The new class ID
            preserve_level: If True, keep the same total level
            cheat_mode: If True, bypass validation
            
        Returns:
            Dict with details of changes made
        """
        try:
            logger.debug(f"Starting class change for character {character.id} ({character.first_name} {character.last_name}) to class {new_class_id}")
            
            # Validate the class change using CharacterManager
            if not cheat_mode:
                is_valid, error_msg = self.character_manager.validate_alignment_for_class(new_class_id)
                if not is_valid:
                    logger.warning(f"Class change validation failed: {error_msg}")
                    raise ValueError(f"Class change not allowed: {error_msg}")
            
            # Get class info from dynamic data loader
            new_class = self.game_data_loader.get_by_id('classes', new_class_id)
            if not new_class:
                logger.error(f"Invalid class ID: {new_class_id}")
                raise ValueError(f"Invalid class ID: {new_class_id}")
                
            # Get current state
            old_classes = list(character.classes.all())
            total_level = sum(c.class_level for c in old_classes) or 1
            old_class_id = old_classes[0].class_id if old_classes else 0
            old_class = self.game_data_loader.get_by_id('classes', old_class_id)
            
            # Get class name safely using dynamic data
            old_class_name = 'None'
            if old_class:
                old_class_name = getattr(old_class, 'name', getattr(old_class, 'label', f'Class {old_class_id}'))
            
            logger.info(f"Character {character.first_name} current class: {old_class_name} (level {total_level})")
            
            # Initialize result object
            result = ClassChangeResult(
                old_class_id=old_class_id,
                new_class_id=new_class_id,
                character_level=total_level,
                changes_made=[],
                save_changes={}
            )
            
            # 1. Update classes
            try:
                character.classes.all().delete()
            except Exception as e:
                logger.error(f"Failed to delete old classes: {e}")
                raise
            CharacterClass.objects.create(
                character=character,
                class_id=new_class_id,
                class_level=total_level
            )
            character.character_class = new_class_id
            # Get new class name safely
            new_class_name = getattr(new_class, 'name', getattr(new_class, 'label', f'Class {new_class_id}'))
            result.changes_made.append(f"Changed class to {new_class_name}")
            logger.debug(f"Updated character class to {new_class_name}")
            
            # 2. Recalculate hit points
            # Always recalculate HP when changing class
            # Use CharacterManager's ability score calculation
            abilities = self.character_manager.get_ability_scores()
            con_modifier = (abilities.get('constitution', 10) - 10) // 2
            
            # Calculate new HP using dynamic data
            hit_die = getattr(new_class, 'hit_die', 4)  # Default d4 if not found
            base_hp = hit_die  # Max at level 1
            if total_level > 1:
                avg_roll = (hit_die + 1) // 2
                base_hp += avg_roll * (total_level - 1)
            
            new_max_hp = base_hp + (con_modifier * total_level)
            new_max_hp = max(1, new_max_hp)
            
            old_max_hp = character.max_hit_points
            if old_max_hp != new_max_hp:
                character.max_hit_points = new_max_hp
                character.hit_points = new_max_hp
                character.current_hit_points = new_max_hp
                
                result.hp_change = (old_max_hp, new_max_hp)
                result.changes_made.append(f"Hit points changed from {old_max_hp} to {new_max_hp}")
            
            # 3. Recalculate BAB using dynamic data
            attack_bonus_table = getattr(new_class, 'attack_bonus_table', 'low')
            
            # Calculate BAB based on table type
            if attack_bonus_table.lower() in ['high', 'warrior']:
                new_bab = total_level  # 1 per level
            elif attack_bonus_table.lower() in ['medium', 'priest']:
                new_bab = (total_level * 3) // 4  # 3/4 per level
            else:  # low, wizard
                new_bab = total_level // 2  # 1/2 per level
            
            if character.base_attack_bonus != new_bab:
                old_bab = character.base_attack_bonus
                character.base_attack_bonus = new_bab
                result.bab_change = (old_bab, new_bab)
                result.changes_made.append(f"BAB changed from {old_bab} to {new_bab}")
            
            # 4. Recalculate saves using dynamic data
            saving_throw_table = getattr(new_class, 'saving_throw_table', 'low')
            
            # Calculate base saves based on table type
            if saving_throw_table.lower() in ['high', 'good']:
                # Good saves: 2 + (level // 2)
                fort_base = ref_base = will_base = 2 + (total_level // 2)
            else:  # low, poor
                # Poor saves: level // 3
                fort_base = ref_base = will_base = total_level // 3
            
            # Get ability modifiers from CharacterManager
            dex_modifier = (abilities.get('dexterity', 10) - 10) // 2
            wis_modifier = (abilities.get('wisdom', 10) - 10) // 2
            
            # Calculate total saves
            fort_save = fort_base + con_modifier + character.fortbonus
            ref_save = ref_base + dex_modifier + character.refbonus
            will_save = will_base + wis_modifier + character.willbonus
            
            # Update saves
            if character.fortitude_save != fort_save:
                old_fort = character.fortitude_save
                character.fortitude_save = fort_save
                result.save_changes['fortitude'] = (old_fort, fort_save)
                result.changes_made.append(f"Fortitude save changed to {fort_save}")
            if character.reflex_save != ref_save:
                old_ref = character.reflex_save
                character.reflex_save = ref_save
                result.save_changes['reflex'] = (old_ref, ref_save)
                result.changes_made.append(f"Reflex save changed to {ref_save}")
            if character.will_save != will_save:
                old_will = character.will_save
                character.will_save = will_save
                result.save_changes['will'] = (old_will, will_save)
                result.changes_made.append(f"Will save changed to {will_save}")
            
            # 5. Recalculate skill points using dynamic data
            int_modifier = (abilities.get('intelligence', 10) - 10) // 2
            skill_points_per_level = getattr(new_class, 'skill_points', 2)  # Default 2 if not found
            
            # Calculate total skill points: (base + int_mod) * level, with level 1 getting 4x
            base_points = skill_points_per_level + int_modifier
            base_points = max(1, base_points)  # Minimum 1 per level
            total_skill_points = base_points * 4 + base_points * (total_level - 1)
            
            if not cheat_mode:
                # Clear skills for redistribution
                character.skills.all().delete()
                
            old_skill_points = character.skill_points
            character.skill_points = total_skill_points
            result.skill_points_change = (old_skill_points, total_skill_points)
            result.changes_made.append(f"Skill points reset to {total_skill_points}")
            
            # 6. Handle class-specific feats
            if not cheat_mode:
                # Remove ALL old class feats using dynamic data
                if old_class:
                    # Get all feats that were granted by the old class
                    old_class_feats = []
                    feat_table = getattr(old_class, 'feat_table', '')
                    if feat_table:
                        # Get class feats from dynamic data (simplified for now)
                        # In full implementation, this would query the class feat tables
                        pass  # Skip complex feat removal for now
                    
                    if old_class_feats:
                        removed = character.feats.filter(feat_id__in=old_class_feats).delete()
                        if removed[0] > 0:
                            result.feats_removed.extend(old_class_feats)
                            old_class_name_safe = getattr(old_class, 'name', getattr(old_class, 'label', 'Unknown'))
                            result.changes_made.append(f"Removed {removed[0]} {old_class_name_safe}-specific feats")
                
                # Grant new class feats using dynamic data
                new_feats_granted = 0
                feat_table = getattr(new_class, 'feat_table', '')
                logger.debug(f"Checking feats for {new_class_name} (feat table: {feat_table})")
                
                # Simplified feat granting for now - in full implementation, 
                # this would query the class feat tables from dynamic data
                # For now, just log that feats would be granted
                logger.debug(f"Would grant class feats for {new_class_name} levels 1-{total_level}")
                
                if new_feats_granted > 0:
                    result.changes_made.append(f"Granted {new_feats_granted} {new_class_name} class feats")
            
            # 7. Handle spellcasting using dynamic data
            old_is_caster = old_class and getattr(old_class, 'spellcaster', 0) > 0
            new_is_caster = getattr(new_class, 'spellcaster', 0) > 0
            
            if old_is_caster and not new_is_caster:
                character.spells.all().delete()
                result.spell_changes = {'action': 'removed_all', 'reason': 'no_longer_caster'}
                result.changes_made.append("Removed all spells (no longer a spellcaster)")
            elif not old_is_caster and new_is_caster:
                # Set up spell slots for new caster using dynamic data
                # Simplified for now - in full implementation would calculate from spell progression tables
                result.spell_changes = {'action': 'new_caster', 'class_name': new_class_name}
                result.changes_made.append("Now a spellcaster - spells need to be selected")
            elif old_is_caster and new_is_caster:
                old_spell_ability = getattr(old_class, 'spell_ability', 0)
                new_spell_ability = getattr(new_class, 'spell_ability', 0)
                if old_spell_ability != new_spell_ability:
                    character.spells.all().delete()
                    result.spell_changes = {'action': 'changed_spell_list', 'old_ability': old_spell_ability, 'new_ability': new_spell_ability}
                    result.changes_made.append("Cleared spell list (different spell list for new class)")
        
            # Save all changes
            if cheat_mode:
                # In cheat mode, temporarily disconnect validation signal
                from character.models import validate_character_data_on_save
                
                # Disconnect the validation signal
                pre_save.disconnect(validate_character_data_on_save, sender=Character)
                try:
                    character.save()
                finally:
                    # Always reconnect the signal
                    pre_save.connect(validate_character_data_on_save, sender=Character)
            else:
                character.save()
            
            logger.info(f"Successfully changed {character.first_name} {character.last_name} from {old_class_name} to {new_class_name}")
            logger.debug(f"Changes made: {result.changes_made}")
            
            # Return as dictionary for backward compatibility
            return_dict = result.to_dict()
            # Add some legacy fields that tests might expect
            return_dict['old_class'] = result.old_class_id
            return_dict['new_class'] = result.new_class_id
            return_dict['level'] = result.character_level
            return_dict['changes'] = result.changes_made
            
            return return_dict
            
        except ValueError as e:
            # Re-raise ValueError as it's expected for validation failures
            raise
        except Exception as e:
            logger.error(f"Unexpected error during class change: {e}", exc_info=True)
            raise RuntimeError(f"Failed to change class: {str(e)}")
    
    def validate_multiclass(self, character: Character, new_class_id: int,
                          cheat_mode: bool = False) -> Tuple[bool, List[str]]:
        """
        Validate if a character can add a new class (multiclass)
        
        Returns (is_valid, list_of_errors)
        """
        if cheat_mode:
            return True, []
            
        errors = []
        
        # Get new class data from dynamic loader
        new_class = self.game_data_loader.get_by_id('classes', new_class_id)
        if not new_class:
            errors.append(f"Invalid class ID: {new_class_id}")
            return False, errors
        
        # Check if already has this class
        existing_classes = {c.class_id for c in character.classes.all()}
        if new_class_id in existing_classes:
            errors.append("Already has levels in this class")
            
        # Check multiclass limit (3 in NWN2)
        if len(existing_classes) >= 3:
            errors.append("Maximum of 3 classes allowed")
            
        # Validate alignment restrictions using CharacterManager
        is_valid, error_msg = self.character_manager.validate_alignment_for_class(new_class_id)
        if not is_valid:
            errors.append(error_msg)
        
        # Check prestige class requirements
        if hasattr(new_class, 'prestige_class') and new_class.prestige_class:
            # Check minimum level requirement (usually level 5 for prestige classes)
            total_level = sum(c.class_level for c in character.classes.all())
            class_name = getattr(new_class, 'name', getattr(new_class, 'label', f'Class {new_class_id}'))
            if total_level < 5:
                errors.append(f"{class_name} requires at least level 5")
            
            # Check BAB requirement
            prereq_bab = getattr(new_class, 'prereq_bab', 0)
            if prereq_bab > 0:
                class_name = getattr(new_class, 'name', getattr(new_class, 'label', f'Class {new_class_id}'))
                if character.base_attack_bonus < prereq_bab:
                    errors.append(f"{class_name} requires BAB +{prereq_bab} (current: +{character.base_attack_bonus})")
            
            # Check skill requirements using dynamic data
            prereq_skills = getattr(new_class, 'prereq_skills', [])
            if prereq_skills:
                for skill_req in prereq_skills:
                    # Simplified skill checking for now
                    # In full implementation, would check against dynamic skill data
                    pass
            
            # Check feat requirements using dynamic data  
            prereq_feats = getattr(new_class, 'prereq_feats', [])
            if prereq_feats:
                character_feats = {f.feat_id for f in character.feats.all()}
                for feat_id in prereq_feats:
                    if feat_id not in character_feats:
                        feat_data = self.game_data_loader.get_by_id('feat', feat_id)
                        feat_name = getattr(feat_data, 'name', getattr(feat_data, 'label', f'Feat {feat_id}')) if feat_data else f'Feat {feat_id}'
                        errors.append(f"{class_name} requires feat: {feat_name}")
            
            # Check spell level requirements using dynamic data
            prereq_spell_level = getattr(new_class, 'prereq_spell_level', 0)
            if prereq_spell_level > 0:
                # Simplified spell level checking for now
                # In full implementation, would check spell progression tables
                errors.append(f"{class_name} requires ability to cast level {prereq_spell_level} spells (check manually)")
        
        # Check for special multiclass restrictions using dynamic data
        # Simplified restriction checking for now
        if new_class_id == 6:  # Monk (if this ID is still valid)
            for char_class in character.classes.all():
                class_data = self.game_data_loader.get_by_id('classes', char_class.class_id)
                if class_data:
                    class_label = getattr(class_data, 'label', '').upper()
                    if class_label in ['PALADIN', 'FIGHTER', 'CLERIC']:
                        errors.append("Monks cannot multiclass with heavy armor classes")
                        break
        
        # Check if any existing classes forbid multiclassing with new class
        for char_class in character.classes.all():
            if char_class.class_id == 6 and new_class_id in [1, 3, 4]:  # Monk with Fighter/Paladin/Cleric
                errors.append(f"Monks cannot multiclass with {class_name}")
        
        return len(errors) == 0, errors
    
    def add_class_level(self, character: Character, class_id: int,
                       cheat_mode: bool = False) -> Dict[str, Any]:
        """
        Add a level in a specific class (for multiclassing or leveling up)
        
        This handles:
        - Adding a new class if character doesn't have it
        - Incrementing level if they already have the class
        - Rolling HP for the new level
        - Calculating new skill points
        - Updating BAB and saves
        - Granting class features for the new level
        """
        try:
            logger.debug(f"Adding level in class {class_id} for character {character.id}")
            
            # Get class data from dynamic loader
            class_data = self.game_data_loader.get_by_id('classes', class_id)
            if not class_data:
                raise ValueError(f"Invalid class ID: {class_id}")
            
            # Check if character already has this class
            existing_class = character.classes.filter(class_id=class_id).first()
            
            if existing_class:
                # Level up existing class
                existing_class.class_level += 1
                existing_class.save()
                new_level = existing_class.class_level
                class_name = getattr(class_data, 'name', getattr(class_data, 'label', f'Class {class_id}'))
                logger.info(f"Leveled up {character.first_name}'s {class_name} to level {new_level}")
            else:
                # Add new class (multiclassing)
                is_valid, errors = self.validate_multiclass(character, class_id, cheat_mode)
                if not is_valid:
                    raise ValueError(f"Cannot multiclass: {', '.join(errors)}")
                
                CharacterClass.objects.create(
                    character=character,
                    class_id=class_id,
                    class_level=1
                )
                new_level = 1
                logger.info(f"Added new class {class_name} to {character.first_name}")
            
            # Calculate new total level
            total_level = sum(c.class_level for c in character.classes.all())
            
            # Get ability modifiers from CharacterManager
            abilities = self.character_manager.get_ability_scores()
            con_modifier = (abilities.get('constitution', 10) - 10) // 2
            int_modifier = (abilities.get('intelligence', 10) - 10) // 2
            
            # Roll HP for new level using dynamic data
            hit_die = getattr(class_data, 'hit_die', 4)
            if total_level == 1:
                hp_gain = hit_die  # Max HP at level 1
            else:
                hp_gain = random.randint(1, hit_die)
            
            # Add constitution modifier
            hp_gain += con_modifier
            hp_gain = max(1, hp_gain)  # Minimum 1 HP per level
            
            old_hp = character.max_hit_points
            character.max_hit_points += hp_gain
            character.hit_points = character.max_hit_points
            character.current_hit_points = character.max_hit_points
            
            # Update BAB using dynamic data
            old_bab = character.base_attack_bonus
            # Recalculate total BAB from all classes
            total_bab = 0
            for char_class in character.classes.all():
                class_info = self.game_data_loader.get_by_id('classes', char_class.class_id)
                if class_info:
                    attack_bonus_table = getattr(class_info, 'attack_bonus_table', 'low')
                    # Calculate BAB based on table type
                    if attack_bonus_table.lower() in ['high', 'warrior']:
                        class_bab = char_class.class_level  # 1 per level
                    elif attack_bonus_table.lower() in ['medium', 'priest']:
                        class_bab = (char_class.class_level * 3) // 4  # 3/4 per level
                    else:  # low, wizard
                        class_bab = char_class.class_level // 2  # 1/2 per level
                    total_bab += class_bab
            character.base_attack_bonus = total_bab
            
            # Update saves using dynamic data
            # Recalculate total saves from all classes
            dex_modifier = (abilities.get('dexterity', 10) - 10) // 2
            wis_modifier = (abilities.get('wisdom', 10) - 10) // 2
            
            # For multiclass, take the best base save from any class
            best_fort = best_ref = best_will = 0
            for char_class in character.classes.all():
                class_info = self.game_data_loader.get_by_id('classes', char_class.class_id)
                if class_info:
                    saving_throw_table = getattr(class_info, 'saving_throw_table', 'low')
                    # Calculate base saves based on table type
                    if saving_throw_table.lower() in ['high', 'good']:
                        base_save = 2 + (char_class.class_level // 2)
                    else:  # low, poor
                        base_save = char_class.class_level // 3
                    
                    best_fort = max(best_fort, base_save)
                    best_ref = max(best_ref, base_save)
                    best_will = max(best_will, base_save)
            
            # Add ability modifiers and misc bonuses
            character.fortitude_save = best_fort + con_modifier + character.fortbonus
            character.reflex_save = best_ref + dex_modifier + character.refbonus
            character.will_save = best_will + wis_modifier + character.willbonus
            
            # Calculate skill points for this level using dynamic data
            skill_points_per_level = getattr(class_data, 'skill_points', 2)
            base_points = skill_points_per_level + int_modifier
            base_points = max(1, base_points)  # Minimum 1 per level
            
            if total_level == 1:
                # First level gets 4x skill points
                skill_points_gained = base_points * 4
            else:
                # Regular skill point gain
                skill_points_gained = base_points
            character.skill_points += skill_points_gained
            
            # Grant class features for the new level using dynamic data
            feats_granted = []
            # Simplified feat granting for now - in full implementation would query feat tables
            feat_table = getattr(class_data, 'feat_table', '')
            if feat_table:
                # Would check feat tables for feats granted at this level
                pass
            
            # Handle spell slots if caster using dynamic data
            spell_changes = None
            is_caster = getattr(class_data, 'spellcaster', 0) > 0
            if is_caster:
                spell_ability = getattr(class_data, 'spell_ability', 0)
                spell_changes = {
                    'is_caster': True,
                    'spell_ability': spell_ability,
                    'level': new_level
                }
            
            # Save character
            character.save()
            
            result = {
                'class_id': class_id,
                'class_name': class_name,
                'new_level': new_level,
                'total_level': total_level,
                'hp_gained': hp_gain,
                'old_hp': old_hp,
                'new_hp': character.max_hit_points,
                'bab_change': (old_bab, character.base_attack_bonus),
                'skill_points_gained': skill_points_gained,
                'feats_granted': feats_granted,
                'spell_changes': spell_changes
            }
            
            logger.info(f"Successfully added level for {character.first_name}: {result}")
            return result
            
        except Exception as e:
            logger.error(f"Failed to add class level: {e}", exc_info=True)
            raise