"""
Game Rules Service - Combines data access with dynamic rule detection
Provides business logic and validation using data from DynamicGameDataLoader and RuleDetector
"""
from typing import Dict, List, Tuple, Optional, Any
from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader
from .rule_detector import RuleDetector


class GameRulesService:
    """
    Game rules service that combines data access from GameDataLoader
    with dynamic rule detection from RuleDetector.
    
    Now uses composition instead of inheritance to avoid creating multiple
    DynamicGameDataLoader instances.
    """
    
    def __init__(self, resource_manager: Optional[Any] = None, load_mode: str = 'full'):
        # Get singleton DynamicGameDataLoader instance, passing the ResourceManager
        # This ensures the singleton uses our shared ResourceManager
        self._loader = get_dynamic_game_data_loader(resource_manager=resource_manager)
        
        # Initialize RuleDetector with the loader's ResourceManager
        self.rule_detector = RuleDetector(self._loader.rm)
        
        # Store ResourceManager reference for compatibility
        self.rm = self._loader.rm
    
    # Delegate data access methods to the loader
    def get_table(self, table_name: str) -> List[Any]:
        """Get all instances for a table."""
        return self._loader.get_table(table_name)
    
    def get_by_id(self, table_name: str, row_id: int) -> Optional[Any]:
        """Get a specific row by ID."""
        return self._loader.get_by_id(table_name, row_id)
    
    def set_module_context(self, module_path: str) -> bool:
        """Set module context for loading module-specific overrides."""
        return self._loader.set_module_context(module_path)
    
    def get_stats(self) -> Dict[str, Any]:
        """Get statistics about loaded data."""
        return self._loader.get_stats()
    
    def get_validation_report(self) -> Optional[Any]:
        """Get the relationship validation report if available."""
        return self._loader.get_validation_report()
    
    def get_table_relationships(self, table_name: str) -> Dict[str, Any]:
        """Get relationship information for a specific table."""
        return self._loader.get_table_relationships(table_name)
    
    def validate_class_change(self, character_data: Dict[str, Any], new_class_id: int, 
                            cheat_mode: bool = False) -> Tuple[bool, List[str]]:
        """
        Validate if a character can change to a new class using dynamic rules
        
        Args:
            character_data: Character information
            new_class_id: The class ID to change to
            cheat_mode: If True, bypass all checks
            
        Returns:
            Tuple of (is_valid, list_of_error_messages)
        """
        if cheat_mode:
            return True, []
        
        errors = []
        new_class = self.get_by_id('classes', new_class_id)
        if not new_class:
            errors.append(f"Invalid class ID: {new_class_id}")
            return False, errors
        
        # Check if there's a prerequisite table for this class
        class_label = new_class.label.lower()
        pres_table = self.rule_detector.find_class_progression_table(class_label, 'pres')
        
        if pres_table:
            # Get all prerequisites from the table
            pres_2da = self.rm.get_2da_with_overrides(pres_table)
            if pres_2da:
                for i in range(pres_2da.get_resource_count()):
                    row = pres_2da.get_row_dict(i)
                    if not row:
                        continue
                    reqs = self.rule_detector.get_requirements(pres_table, row)
                    
                    # Check each requirement type
                    if 'min_level' in reqs and character_data.get('level', 1) < reqs['min_level']:
                        errors.append(f"{new_class.name} requires level {reqs['min_level']}")
                    
                    if 'prereq_feats' in reqs:
                        char_feats = set(character_data.get('feats', []))
                        
                        # Check "all of" feats
                        for feat_id in reqs['prereq_feats'].get('all_of', []):
                            if feat_id not in char_feats:
                                feat = self.get_by_id('feat', feat_id)
                                feat_name = feat.name if feat else f"Feat {feat_id}"
                                errors.append(f"{new_class.name} requires {feat_name}")
                        
                        # Check "one of" feats
                        one_of = reqs['prereq_feats'].get('one_of', [])
                        if one_of and not any(f in char_feats for f in one_of):
                            errors.append(f"{new_class.name} requires one of the prerequisite feats")
                    
                    if 'required_skills' in reqs:
                        char_skills = character_data.get('skills', {})
                        for skill_req in reqs['required_skills']:
                            skill_id = skill_req['id']
                            required_ranks = skill_req['ranks']
                            char_ranks = char_skills.get(skill_id, 0)
                            
                            if char_ranks < required_ranks:
                                skill = self.get_by_id('skills', skill_id)
                                skill_name = skill.name if skill else f"Skill {skill_id}"
                                errors.append(f"{new_class.name} requires {skill_name} rank {required_ranks}")
        
        # Check alignment restrictions from classes.2da
        class_row = self.rm.get_2da_with_overrides('classes').get_row_dict(new_class_id)
        if class_row:
            class_reqs = self.rule_detector.get_requirements('classes', class_row)
            
            if 'alignment_restrict' in class_reqs:
                # This would need proper alignment checking logic
                # For now, just note that there are alignment restrictions
                if class_reqs['alignment_restrict'] != 0:
                    errors.append(f"{new_class.name} has alignment restrictions")
        
        return len(errors) == 0, errors
    
    def get_available_feats(self, character_data: Dict[str, Any]) -> List[int]:
        """
        Get all feats available to a character using dynamic rule detection
        
        Args:
            character_data: Character information including level, attributes, etc.
            
        Returns:
            List of available feat IDs
        """
        return self.rule_detector.get_available_feats(character_data)
    
    def get_spell_caster_info(self, spell_id: int) -> Dict[str, int]:
        """
        Get which classes can cast a spell without hardcoding
        
        Args:
            spell_id: Spell ID to check
            
        Returns:
            Dict of class_name -> spell_level
        """
        spell_classes = self.rule_detector.get_spell_classes(spell_id)
        
        # Map column names to class names using classes.2da
        result = {}
        for col_name, spell_level in spell_classes.items():
            # Try to find the class that uses this column
            # This is still a bit of mapping, but it's data-driven
            if col_name == 'Wiz_Sorc':
                result['Wizard'] = spell_level
                result['Sorcerer'] = spell_level
            elif col_name == 'Spirit_Shaman':
                result['Spirit Shaman'] = spell_level
            else:
                # Use the column name as-is for standard classes
                result[col_name] = spell_level
        
        return result
    
    def calculate_ability_modifiers(self, character_data: Dict[str, Any]) -> Dict[str, int]:
        """
        Calculate ability modifiers without hardcoding attribute names
        
        Args:
            character_data: Character data with attributes
            
        Returns:
            Dict of attribute -> modifier
        """
        modifiers = {}
        
        # Find all attribute-like keys in character data
        # This approach doesn't hardcode STR, DEX, etc.
        for key, value in character_data.items():
            # Check if this looks like an attribute (3 letters, all caps or title case)
            if len(key) == 3 and key.isalpha() and isinstance(value, (int, float)):
                # D&D modifier calculation
                modifiers[key.upper()] = (int(value) - 10) // 2
        
        return modifiers
    
    def get_class_progression(self, class_id: int, progression_type: str, level: int) -> Any:
        """
        Get class progression data without hardcoding table names
        
        Args:
            class_id: Class ID
            progression_type: Type of progression (feat, skill, savthr, etc.)
            level: Character level
            
        Returns:
            Progression data for the specified level
        """
        class_data = self.get_by_id('classes', class_id)
        if not class_data:
            return None
        
        # Find the progression table
        table_name = self.rule_detector.find_class_progression_table(
            class_data.label, 
            progression_type
        )
        
        if not table_name:
            return None
        
        # Get the data from the table
        table = self.rm.get_2da_with_overrides(table_name)
        if not table or level - 1 >= table.get_resource_count():
            return None
        
        return table.get_row_dict(level - 1)