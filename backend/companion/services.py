"""
Companion import service for .ros files
"""

from typing import Dict, Any
from django.db import transaction
from parsers.gff import GFFParser, GFFFieldType
from parsers.resource_manager import ResourceManager
from .models import Companion, CompanionClass, CompanionFeat, CompanionSkill


class CompanionImportService:
    """Service to import companions from .ros files"""
    
    def __init__(self, resource_manager: ResourceManager):
        self.rm = resource_manager
        self.parser = GFFParser()
        
    @transaction.atomic
    def import_companion(self, file_path: str, owner=None) -> Companion:
        """Import a companion from a .ros file"""
        
        # Parse the file
        with open(file_path, 'rb') as f:
            gff_data = self.parser.load(f)
            
        # Extract all fields
        fields = self._extract_fields(gff_data)
        
        # Create companion instance
        companion = Companion(
            owner=owner,
            file_name=file_path.split('/')[-1],
            file_path=file_path
        )
        
        # Map core fields
        core_field_mapping = {
            'FirstName': 'first_name',
            'LastName': 'last_name',
            'Tag': 'tag',
            'RosterTag': 'roster_tag',
            'Age': 'age',
            'Gender': 'gender',
            'Race': 'race_id',
            'Subrace': 'subrace_id',
            'Deity': 'deity',
            'LawfulChaotic': 'law_chaos',
            'GoodEvil': 'good_evil',
            'Str': 'strength',
            'Dex': 'dexterity',
            'Con': 'constitution',
            'Int': 'intelligence',
            'Wis': 'wisdom',
            'Cha': 'charisma',
            'HitPoints': 'hit_points',
            'MaxHitPoints': 'max_hit_points',
            'CurrentHitPoints': 'current_hit_points',
            'ArmorClass': 'armor_class',
            'BaseAttackBonus': 'base_attack_bonus',
            'FortSaveThrow': 'fortitude_save',
            'RefSaveThrow': 'reflex_save',
            'WillSaveThrow': 'will_save',
        }
        
        # Map NPC-specific fields
        npc_field_mapping = {
            'ActionList': 'action_list',
            'EffectList': 'effect_list',
            'PersonalRepList': 'personal_rep_list',
            'VarTable': 'var_table',
        }
        
        # Process core fields
        for gff_field, model_field in core_field_mapping.items():
            if gff_field in fields:
                value = self._process_field_value(fields[gff_field])
                setattr(companion, model_field, value)
                
        # Process NPC-specific fields
        for gff_field, model_field in npc_field_mapping.items():
            if gff_field in fields:
                value = self._process_list_field(fields[gff_field])
                setattr(companion, model_field, value)
                
        # Store remaining fields in additional_data
        additional_data = {}
        all_mapped = set(core_field_mapping.keys()) | set(npc_field_mapping.keys())
        for field_name, field_data in fields.items():
            if field_name not in all_mapped:
                additional_data[field_name] = self._serialize_field(field_data)
                
        companion.additional_data = additional_data
        companion.save()
        
        # Import related data
        self._import_classes(companion, fields.get('ClassList'))
        self._import_feats(companion, fields.get('FeatList'))
        self._import_skills(companion, fields.get('SkillList'))
        
        return companion
        
    def _extract_fields(self, gff_data) -> Dict[str, Any]:
        """Extract all fields from GFF data"""
        fields = {}
        for field in gff_data.value:
            fields[field.label] = field
        return fields
        
    def _process_field_value(self, field):
        """Process a single field value"""
        if field.type == GFFFieldType.LOCSTRING:
            # Extract the actual string
            if field.value.substrings:
                return field.value.substrings[0].string
            return ""
        elif field.type in [GFFFieldType.BYTE, GFFFieldType.CHAR, GFFFieldType.WORD,
                           GFFFieldType.SHORT, GFFFieldType.DWORD, GFFFieldType.INT,
                           GFFFieldType.FLOAT]:
            return field.value
        elif field.type == GFFFieldType.STRING:
            return field.value or ""
        else:
            return self._serialize_field(field)
            
    def _process_list_field(self, field):
        """Process a list field into JSON-serializable format"""
        if not field or field.type != GFFFieldType.LIST:
            return []
        
        result = []
        for item in field.value:
            if hasattr(item, 'value'):
                item_data = {}
                for subfield in item.value:
                    item_data[subfield.label] = self._serialize_field(subfield)
                result.append(item_data)
        return result
        
    def _serialize_field(self, field):
        """Serialize any field type for JSON storage"""
        if field.type == GFFFieldType.LOCSTRING:
            return {
                'type': 'locstring',
                'string_ref': field.value.string_ref,
                'strings': [{'lang': s.language, 'text': s.string} 
                           for s in field.value.substrings]
            }
        elif field.type == GFFFieldType.LIST:
            return self._process_list_field(field)
        elif field.type == GFFFieldType.STRUCT:
            struct_data = {}
            for subfield in field.value:
                struct_data[subfield.label] = self._serialize_field(subfield)
            return struct_data
        else:
            return field.value
            
    def _import_classes(self, companion, class_list_field):
        """Import companion classes"""
        if not class_list_field or class_list_field.type != GFFFieldType.LIST:
            return
            
        for class_struct in class_list_field.value:
            class_data = {}
            for field in class_struct.value:
                if field.label == 'Class':
                    class_data['class_id'] = field.value
                elif field.label == 'ClassLevel':
                    class_data['class_level'] = field.value
                    
            if 'class_id' in class_data:
                CompanionClass.objects.create(
                    companion=companion,
                    **class_data
                )
                
    def _import_feats(self, companion, feat_list_field):
        """Import companion feats"""
        if not feat_list_field or feat_list_field.type != GFFFieldType.LIST:
            return
            
        for feat_struct in feat_list_field.value:
            for field in feat_struct.value:
                if field.label == 'Feat':
                    CompanionFeat.objects.create(
                        companion=companion,
                        feat_id=field.value
                    )
                    
    def _import_skills(self, companion, skill_list_field):
        """Import companion skills"""
        if not skill_list_field or skill_list_field.type != GFFFieldType.LIST:
            return
            
        for skill_struct in skill_list_field.value:
            if skill_struct.type != GFFFieldType.STRUCT:
                continue
                
            skill_id = None
            rank = 0
            
            for field in skill_struct.value:
                if field.label == 'Skill':
                    skill_id = field.value
                elif field.label == 'Rank':
                    rank = field.value
            
            if skill_id is not None and rank > 0:
                CompanionSkill.objects.create(
                    companion=companion,
                    skill_id=skill_id,
                    rank=rank
                )
