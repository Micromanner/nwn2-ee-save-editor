"""
Character creation service that modifies template .bic files
"""
import os
import shutil
from typing import Dict, Any, Optional, List
from django.db import transaction

from .models import Character, CharacterClass, CharacterFeat, CharacterSkill
from parsers.gff import GFFParser, GFFWriter, GFFFieldType
from parsers.resource_manager import ResourceManager
from gamedata.services.game_rules_service import GameRulesService


class CharacterCreationService:
    """Service to create new characters by modifying template .bic files"""
    
    def __init__(self, resource_manager: Optional[ResourceManager] = None):
        self.rm = resource_manager or ResourceManager()
        self.game_rules = GameRulesService(self.rm)
        
    def create_character(self, character_data: Dict[str, Any], template_path: str, output_path: str) -> Character:
        """
        Create a new character by modifying a template .bic file
        
        Args:
            character_data: Dictionary with character creation data
            template_path: Path to template .bic file
            output_path: Where to save the new .bic file
            
        Returns:
            Character model instance
        """
        # Copy template to output location
        shutil.copy2(template_path, output_path)
        
        # Parse the template
        parser = GFFParser()
        gff_data = parser.parse(template_path)
        
        # Update only the fields we're changing
        self._update_basic_info(gff_data, character_data)
        self._update_abilities(gff_data, character_data)
        self._update_alignment(gff_data, character_data)
        self._update_race(gff_data, character_data)
        self._update_classes(gff_data, character_data)
        self._update_skills(gff_data, character_data)
        self._update_feats(gff_data, character_data)
        self._update_appearance(gff_data, character_data)
        
        # Calculate derived values
        self._calculate_saves(gff_data, character_data)
        self._calculate_hit_points(gff_data, character_data)
        self._calculate_skill_points(gff_data, character_data)
        
        # Write the modified data
        writer = GFFWriter()
        writer.write(output_path, gff_data)
        
        # Import to database
        from .services import CharacterImportService
        import_service = CharacterImportService(self.rm)
        character = import_service.import_character(output_path)
        
        return character
    
    def _update_basic_info(self, gff_data: Dict[str, Any], character_data: Dict[str, Any]):
        """Update basic character information"""
        if 'firstName' in character_data:
            gff_data['FirstName'] = {
                'type': GFFFieldType.LOCSTRING,
                'value': {
                    'string_ref': -1,
                    'substrings': [{
                        'string': character_data['firstName'],
                        'language': 0,
                        'gender': 0
                    }]
                }
            }
            
        if 'lastName' in character_data:
            gff_data['LastName'] = {
                'type': GFFFieldType.LOCSTRING,
                'value': {
                    'string_ref': -1,
                    'substrings': [{
                        'string': character_data['lastName'],
                        'language': 0,
                        'gender': 0
                    }]
                }
            }
            
        if 'age' in character_data:
            gff_data['Age'] = {'type': GFFFieldType.INT, 'value': character_data['age']}
            
        if 'gender' in character_data:
            gff_data['Gender'] = {'type': GFFFieldType.BYTE, 'value': character_data['gender']}
            
        if 'deity' in character_data:
            gff_data['Deity'] = {'type': GFFFieldType.STRING, 'value': character_data['deity']}
    
    def _update_abilities(self, gff_data: Dict[str, Any], character_data: Dict[str, Any]):
        """Update ability scores"""
        ability_map = {
            'strength': 'Str',
            'dexterity': 'Dex',
            'constitution': 'Con',
            'intelligence': 'Int',
            'wisdom': 'Wis',
            'charisma': 'Cha'
        }
        
        for key, gff_key in ability_map.items():
            if key in character_data:
                gff_data[gff_key] = {'type': GFFFieldType.BYTE, 'value': character_data[key]}
    
    def _update_alignment(self, gff_data: Dict[str, Any], character_data: Dict[str, Any]):
        """Update alignment"""
        if 'lawChaos' in character_data:
            gff_data['LawfulChaotic'] = {'type': GFFFieldType.BYTE, 'value': character_data['lawChaos']}
            
        if 'goodEvil' in character_data:
            gff_data['GoodEvil'] = {'type': GFFFieldType.BYTE, 'value': character_data['goodEvil']}
    
    def _update_race(self, gff_data: Dict[str, Any], character_data: Dict[str, Any]):
        """Update race and subrace"""
        if 'raceId' in character_data:
            gff_data['Race'] = {'type': GFFFieldType.BYTE, 'value': character_data['raceId']}
            
        if 'subraceId' in character_data:
            gff_data['Subrace'] = {'type': GFFFieldType.WORD, 'value': character_data['subraceId']}
    
    def _update_classes(self, gff_data: Dict[str, Any], character_data: Dict[str, Any]):
        """Update character classes"""
        if 'classes' not in character_data:
            return
            
        class_list = []
        total_level = 0
        
        for i, cls_data in enumerate(character_data['classes']):
            class_struct = {
                'type': GFFFieldType.STRUCT,
                'value': {
                    'Class': {'type': GFFFieldType.INT, 'value': cls_data['classId']},
                    'ClassLevel': {'type': GFFFieldType.SHORT, 'value': cls_data['level']}
                }
            }
            
            # Add domains for divine casters
            if 'domains' in cls_data and cls_data['domains']:
                if len(cls_data['domains']) > 0 and cls_data['domains'][0] is not None:
                    class_struct['value']['Domain1'] = {'type': GFFFieldType.BYTE, 'value': cls_data['domains'][0]}
                if len(cls_data['domains']) > 1 and cls_data['domains'][1] is not None:
                    class_struct['value']['Domain2'] = {'type': GFFFieldType.BYTE, 'value': cls_data['domains'][1]}
            
            class_list.append(class_struct)
            total_level += cls_data['level']
        
        gff_data['ClassList'] = {'type': GFFFieldType.LIST, 'value': class_list}
        
        # Update total character level
        gff_data['CharacterLevel'] = {'type': GFFFieldType.INT, 'value': total_level}
        
        # Set primary class (highest level)
        primary_class = max(character_data['classes'], key=lambda x: x['level'])
        gff_data['Class'] = {'type': GFFFieldType.INT, 'value': primary_class['classId']}
        gff_data['ClassLevel'] = {'type': GFFFieldType.SHORT, 'value': primary_class['level']}
    
    def _update_skills(self, gff_data: Dict[str, Any], character_data: Dict[str, Any]):
        """Update skill ranks"""
        if 'skills' not in character_data:
            return
            
        # Get all skills from 2DA to ensure proper ordering
        skills_2da = self.rm.get_2da_with_overrides('skills')
        if not skills_2da:
            return
            
        skill_list = []
        
        for i in range(skills_2da.get_resource_count()):
            rank = character_data['skills'].get(str(i), 0)  # Skills might be keyed as strings
            if rank > 0:  # Only add skills with ranks
                skill_struct = {
                    'type': GFFFieldType.STRUCT,
                    'value': {
                        'Skill': {'type': GFFFieldType.BYTE, 'value': i},
                        'Rank': {'type': GFFFieldType.BYTE, 'value': rank}
                    }
                }
                skill_list.append(skill_struct)
        
        gff_data['SkillList'] = {'type': GFFFieldType.LIST, 'value': skill_list}
    
    def _update_feats(self, gff_data: Dict[str, Any], character_data: Dict[str, Any]):
        """Update feat list"""
        if 'feats' not in character_data:
            return
            
        feat_list = []
        
        for feat_id in character_data['feats']:
            feat_struct = {
                'type': GFFFieldType.STRUCT,
                'value': {
                    'Feat': {'type': GFFFieldType.WORD, 'value': feat_id}
                }
            }
            feat_list.append(feat_struct)
        
        gff_data['FeatList'] = {'type': GFFFieldType.LIST, 'value': feat_list}
    
    def _update_appearance(self, gff_data: Dict[str, Any], character_data: Dict[str, Any]):
        """Update appearance fields"""
        appearance_fields = {
            'appearanceType': ('Appearance_Type', GFFFieldType.INT),
            'hairStyle': ('Appearance_Hair', GFFFieldType.INT),
            'headModel': ('Appearance_Head', GFFFieldType.INT),
            'portraitId': ('Portrait', GFFFieldType.STRING),
        }
        
        for key, (gff_key, field_type) in appearance_fields.items():
            if key in character_data:
                gff_data[gff_key] = {'type': field_type, 'value': character_data[key]}
        
        # Handle color fields (RGBA structs)
        if 'hairColor' in character_data and isinstance(character_data['hairColor'], dict):
            gff_data['Tint_Hair'] = {
                'type': GFFFieldType.STRUCT,
                'value': {
                    'r': {'type': GFFFieldType.BYTE, 'value': character_data['hairColor'].get('r', 0)},
                    'g': {'type': GFFFieldType.BYTE, 'value': character_data['hairColor'].get('g', 0)},
                    'b': {'type': GFFFieldType.BYTE, 'value': character_data['hairColor'].get('b', 0)},
                    'a': {'type': GFFFieldType.BYTE, 'value': character_data['hairColor'].get('a', 255)}
                }
            }
    
    def _calculate_saves(self, gff_data: Dict[str, Any], character_data: Dict[str, Any]):
        """Calculate saving throws based on classes and abilities"""
        # Get ability modifiers
        con_mod = (character_data.get('constitution', 10) - 10) // 2
        dex_mod = (character_data.get('dexterity', 10) - 10) // 2
        wis_mod = (character_data.get('wisdom', 10) - 10) // 2
        
        # Base saves from classes
        fort_base = 0
        ref_base = 0
        will_base = 0
        
        for cls_data in character_data.get('classes', []):
            class_id = cls_data['classId']
            level = cls_data['level']
            
            # Get class save progression from 2DA
            class_data = self.game_rules.get_by_id('classes', class_id)
            if class_data:
                # High save progression = 2 + (level // 2)
                # Low save progression = level // 3
                
                if hasattr(class_data, 'fortSave') and class_data.fortSave == 'high':
                    fort_base += 2 + (level // 2)
                else:
                    fort_base += level // 3
                    
                if hasattr(class_data, 'refSave') and class_data.refSave == 'high':
                    ref_base += 2 + (level // 2)
                else:
                    ref_base += level // 3
                    
                if hasattr(class_data, 'willSave') and class_data.willSave == 'high':
                    will_base += 2 + (level // 2)
                else:
                    will_base += level // 3
        
        # Total saves
        gff_data['FortSaveThrow'] = {'type': GFFFieldType.CHAR, 'value': fort_base + con_mod}
        gff_data['RefSaveThrow'] = {'type': GFFFieldType.CHAR, 'value': ref_base + dex_mod}
        gff_data['WillSaveThrow'] = {'type': GFFFieldType.CHAR, 'value': will_base + wis_mod}
    
    def _calculate_hit_points(self, gff_data: Dict[str, Any], character_data: Dict[str, Any]):
        """Calculate hit points based on classes and constitution"""
        con_mod = (character_data.get('constitution', 10) - 10) // 2
        total_hp = 0
        
        for i, cls_data in enumerate(character_data.get('classes', [])):
            class_id = cls_data['classId']
            level = cls_data['level']
            
            # Get hit die from class
            class_data = self.game_rules.get_by_id('classes', class_id)
            if class_data and hasattr(class_data, 'hitDie'):
                hit_die = class_data.hitDie
                
                # First level gets max HP
                if i == 0:
                    total_hp += hit_die + con_mod
                    level -= 1
                
                # Average for remaining levels
                avg_roll = (hit_die // 2) + 1
                total_hp += level * (avg_roll + con_mod)
            else:
                # Default d8 hit die
                if i == 0:
                    total_hp += 8 + con_mod
                    level -= 1
                total_hp += level * (5 + con_mod)
        
        # Minimum 1 HP per level
        total_level = sum(cls['level'] for cls in character_data.get('classes', []))
        total_hp = max(total_hp, total_level)
        
        gff_data['HitPoints'] = {'type': GFFFieldType.SHORT, 'value': total_hp}
        gff_data['MaxHitPoints'] = {'type': GFFFieldType.SHORT, 'value': total_hp}
        gff_data['CurrentHitPoints'] = {'type': GFFFieldType.SHORT, 'value': total_hp}
    
    def _calculate_skill_points(self, gff_data: Dict[str, Any], character_data: Dict[str, Any]):
        """Calculate available skill points"""
        int_mod = (character_data.get('intelligence', 10) - 10) // 2
        total_points = 0
        
        # Human race gets +1 skill point per level
        race_bonus = 1 if character_data.get('raceId') == 6 else 0
        
        for cls_data in character_data.get('classes', []):
            class_id = cls_data['classId']
            level = cls_data['level']
            
            # Get skill points from class
            class_data = self.game_rules.get_by_id('classes', class_id)
            if class_data and hasattr(class_data, 'skillPointBase'):
                base_points = class_data.skillPointBase
            else:
                base_points = 2  # Default
            
            # First level gets 4x skill points
            if total_points == 0:
                total_points += (base_points + int_mod + race_bonus) * 4
                level -= 1
            
            # Remaining levels
            total_points += level * max(1, base_points + int_mod + race_bonus)
        
        # Subtract spent points
        spent_points = sum(character_data.get('skills', {}).values())
        available_points = total_points - spent_points
        
        gff_data['SkillPoints'] = {'type': GFFFieldType.SHORT, 'value': available_points}
    
    def get_template_paths(self) -> Dict[str, str]:
        """Get available template .bic files"""
        templates = {}
        
        # Check for templates in various locations
        template_dirs = [
            os.path.join(os.path.dirname(__file__), '..', 'templates', 'characters'),
            os.path.join(self.rm.nwn2_docs, 'localvault', 'templates'),
            os.path.join(self.rm.nwn2_home, 'templates')
        ]
        
        for template_dir in template_dirs:
            if os.path.exists(template_dir):
                for filename in os.listdir(template_dir):
                    if filename.endswith('.bic'):
                        name = os.path.splitext(filename)[0]
                        templates[name] = os.path.join(template_dir, filename)
        
        # If no templates found, we can use any existing character
        if not templates:
            localvault = os.path.join(self.rm.nwn2_docs, 'localvault')
            if os.path.exists(localvault):
                for filename in os.listdir(localvault):
                    if filename.endswith('.bic'):
                        templates['default'] = os.path.join(localvault, filename)
                        break
        
        return templates