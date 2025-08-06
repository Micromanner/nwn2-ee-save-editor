"""
Character validation logic
"""
from typing import Dict, List, Optional
from django.core.exceptions import ValidationError
from .models import Character
from parsers.resource_manager import ResourceManager


class CharacterValidator:
    """Validates character data against game rules"""
    
    def __init__(self, resource_manager: ResourceManager):
        self.rm = resource_manager
        
    def validate_character(self, character: Character) -> Dict[str, List[str]]:
        """
        Validate a character and return any errors
        
        Returns:
            Dict mapping field names to lists of error messages
        """
        errors = {}
        
        # Validate ability scores
        self._validate_abilities(character, errors)
        
        # Validate alignment
        self._validate_alignment(character, errors)
        
        # Validate race
        self._validate_race(character, errors)
        
        # Validate classes
        self._validate_classes(character, errors)
        
        # Validate level
        self._validate_level(character, errors)
        
        # Validate hit points
        self._validate_hit_points(character, errors)
        
        # Validate saves
        self._validate_saves(character, errors)
        
        # Validate skills
        self._validate_skills(character, errors)
        
        # Validate feats
        self._validate_feats(character, errors)
        
        return errors
        
    def _validate_abilities(self, character: Character, errors: Dict[str, List[str]]):
        """Validate ability scores are within valid ranges"""
        abilities = {
            'strength': character.strength,
            'dexterity': character.dexterity,
            'constitution': character.constitution,
            'intelligence': character.intelligence,
            'wisdom': character.wisdom,
            'charisma': character.charisma
        }
        
        for ability, value in abilities.items():
            if value < 3:
                if ability not in errors:
                    errors[ability] = []
                errors[ability].append(f"{ability.capitalize()} cannot be less than 3")
            elif value > 50:  # Reasonable upper limit
                if ability not in errors:
                    errors[ability] = []
                errors[ability].append(f"{ability.capitalize()} cannot be greater than 50")
                
    def _validate_alignment(self, character: Character, errors: Dict[str, List[str]]):
        """Validate alignment values"""
        if character.law_chaos < 0 or character.law_chaos > 100:
            if 'law_chaos' not in errors:
                errors['law_chaos'] = []
            errors['law_chaos'].append("Law/Chaos must be between 0 and 100")
            
        if character.good_evil < 0 or character.good_evil > 100:
            if 'good_evil' not in errors:
                errors['good_evil'] = []
            errors['good_evil'].append("Good/Evil must be between 0 and 100")
            
        # Check class alignment restrictions
        for char_class in character.classes.all():
            class_2da = self.rm.get_2da('classes')
            if class_2da and char_class.class_id < class_2da.get_resource_count():
                align_restrict = class_2da.get_int(char_class.class_id, 'AlignRestrict')
                align_type = class_2da.get_int(char_class.class_id, 'AlignRstrctType')
                
                if align_restrict and align_restrict > 0:
                    alignment = character.alignment
                    valid = self._check_alignment_restriction(
                        alignment, align_restrict, align_type
                    )
                    
                    if not valid:
                        if 'alignment' not in errors:
                            errors['alignment'] = []
                        errors['alignment'].append(
                            f"{char_class.class_name} has alignment restrictions"
                        )
                        
    def _check_alignment_restriction(self, alignment: str, 
                                    restrict: int, restrict_type: int) -> bool:
        """Check if alignment meets class restrictions"""
        # Alignment bits:
        # 0x01 = Lawful, 0x02 = Neutral (L-C), 0x04 = Chaotic
        # 0x10 = Good, 0x20 = Neutral (G-E), 0x40 = Evil
        
        alignment_map = {
            'Lawful Good': 0x11,
            'Neutral Good': 0x12,
            'Chaotic Good': 0x14,
            'Lawful Neutral': 0x21,
            'True Neutral': 0x22,
            'Chaotic Neutral': 0x24,
            'Lawful Evil': 0x41,
            'Neutral Evil': 0x42,
            'Chaotic Evil': 0x44
        }
        
        align_bits = alignment_map.get(alignment, 0)
        
        if restrict_type == 0x1:  # Alignment must match one of the bits
            return (align_bits & restrict) != 0
        elif restrict_type == 0x3:  # Alignment must not match any bits
            return (align_bits & restrict) == 0
            
        return True
        
    def _validate_race(self, character: Character, errors: Dict[str, List[str]]):
        """Validate race exists"""
        races = self.rm.get_2da('racialtypes')
        if races and character.race_id >= races.get_resource_count():
            if 'race_id' not in errors:
                errors['race_id'] = []
            errors['race_id'].append("Invalid race ID")
            
    def _validate_classes(self, character: Character, errors: Dict[str, List[str]]):
        """Validate character classes"""
        classes_2da = self.rm.get_2da('classes')
        if not classes_2da:
            return
            
        total_level = 0
        for char_class in character.classes.all():
            # Check valid class ID
            if char_class.class_id >= classes_2da.get_resource_count():
                if 'classes' not in errors:
                    errors['classes'] = []
                errors['classes'].append(f"Invalid class ID: {char_class.class_id}")
                continue
                
            # Check class level
            if char_class.class_level < 1 or char_class.class_level > 30:
                if 'classes' not in errors:
                    errors['classes'] = []
                errors['classes'].append(
                    f"{char_class.class_name} level must be between 1 and 30"
                )
                
            total_level += char_class.class_level
            
        # Verify total level matches character level
        if total_level != character.character_level:
            if 'character_level' not in errors:
                errors['character_level'] = []
            errors['character_level'].append(
                f"Character level ({character.character_level}) doesn't match "
                f"sum of class levels ({total_level})"
            )
            
    def _validate_level(self, character: Character, errors: Dict[str, List[str]]):
        """Validate character level"""
        if character.character_level < 1:
            if 'character_level' not in errors:
                errors['character_level'] = []
            errors['character_level'].append("Character level must be at least 1")
        elif character.character_level > 30:
            if 'character_level' not in errors:
                errors['character_level'] = []
            errors['character_level'].append("Character level cannot exceed 30")
            
    def _validate_hit_points(self, character: Character, errors: Dict[str, List[str]]):
        """Validate hit points"""
        if character.hit_points < 1:
            if 'hit_points' not in errors:
                errors['hit_points'] = []
            errors['hit_points'].append("Hit points must be at least 1")
            
        if character.hit_points > character.max_hit_points:
            if 'hit_points' not in errors:
                errors['hit_points'] = []
            errors['hit_points'].append("Hit points cannot exceed maximum hit points")
            
        # Check if max HP is reasonable for level
        min_possible = character.character_level  # 1 HP per level minimum
        max_possible = character.character_level * 12 + 200  # Barbarian d12 + high CON
        
        if character.max_hit_points < min_possible:
            if 'max_hit_points' not in errors:
                errors['max_hit_points'] = []
            errors['max_hit_points'].append(
                f"Maximum hit points too low for level {character.character_level}"
            )
        elif character.max_hit_points > max_possible:
            if 'max_hit_points' not in errors:
                errors['max_hit_points'] = []
            errors['max_hit_points'].append(
                f"Maximum hit points too high for level {character.character_level}"
            )
            
    def _validate_saves(self, character: Character, errors: Dict[str, List[str]]):
        """Validate saving throws"""
        # Saves should be reasonable for level
        min_save = 0
        max_save = character.character_level + 20  # Base + ability mod + items
        
        saves = {
            'fortitude_save': character.fortitude_save,
            'reflex_save': character.reflex_save,
            'will_save': character.will_save
        }
        
        for save_name, value in saves.items():
            if value < min_save:
                if save_name not in errors:
                    errors[save_name] = []
                errors[save_name].append(f"{save_name.replace('_', ' ').title()} cannot be negative")
            elif value > max_save:
                if save_name not in errors:
                    errors[save_name] = []
                errors[save_name].append(f"{save_name.replace('_', ' ').title()} seems too high")
                
    def _validate_skills(self, character: Character, errors: Dict[str, List[str]]):
        """Validate skill ranks"""
        skills_2da = self.rm.get_2da('skills')
        if not skills_2da:
            return
            
        for skill in character.skills.all():
            # Check valid skill ID
            if skill.skill_id >= skills_2da.get_resource_count():
                if 'skills' not in errors:
                    errors['skills'] = []
                errors['skills'].append(f"Invalid skill ID: {skill.skill_id}")
                continue
                
            # Check rank limits
            max_rank = character.character_level + 3  # Cross-class max
            if skill.rank < 0:
                if 'skills' not in errors:
                    errors['skills'] = []
                errors['skills'].append(f"{skill.skill_name} rank cannot be negative")
            elif skill.rank > max_rank:
                if 'skills' not in errors:
                    errors['skills'] = []
                errors['skills'].append(
                    f"{skill.skill_name} rank ({skill.rank}) exceeds maximum "
                    f"for level {character.character_level}"
                )
                
    def _validate_feats(self, character: Character, errors: Dict[str, List[str]]):
        """Validate feats"""
        feat_2da = self.rm.get_2da('feat')
        if not feat_2da:
            return
            
        feat_ids = set()
        for feat in character.feats.all():
            # Check valid feat ID
            if feat.feat_id >= feat_2da.get_resource_count():
                if 'feats' not in errors:
                    errors['feats'] = []
                errors['feats'].append(f"Invalid feat ID: {feat.feat_id}")
                continue
                
            # Check for duplicates
            if feat.feat_id in feat_ids:
                if 'feats' not in errors:
                    errors['feats'] = []
                errors['feats'].append(f"Duplicate feat: {feat.feat_name}")
            feat_ids.add(feat.feat_id)
            
            # TODO: Check feat prerequisites