"""
Character ViewSet - Base CRUD operations for characters
Handles character creation, deletion, import/export, and basic queries
"""

from rest_framework import viewsets, status
from rest_framework.decorators import action
from rest_framework.response import Response
from rest_framework.parsers import MultiPartParser, FormParser
from django.core.files.storage import default_storage
from django.core.files.base import ContentFile
from django.db import transaction
import os
import tempfile
import logging

from character.models import Character
from character.services import CharacterImportService
from character.factory import get_or_create_character_manager, invalidate_character_cache
from character.validators import CharacterValidator
from character.custom_content import CustomContentDetector
from parsers.resource_manager import ResourceManager
from ..serializers import (
    CharacterListSerializer, CharacterDetailSerializer,
    CharacterUpdateSerializer, FileUploadSerializer
)
from parsers.gff import GFFParser, GFFWriter
from .base_character_view import BaseCharacterViewSet

logger = logging.getLogger(__name__)


class CharacterViewSet(BaseCharacterViewSet, viewsets.ModelViewSet):
    """
    API endpoint for character management - basic CRUD operations
    """
    queryset = Character.objects.all()

    def get_serializer_class(self):
        if self.action == 'list':
            return CharacterListSerializer
        elif self.action in ['update', 'partial_update']:
            return CharacterUpdateSerializer
        return CharacterDetailSerializer

    def get_queryset(self):
        queryset = Character.objects.all()

        # Filter by companion status
        is_companion = self.request.query_params.get('is_companion', None)
        if is_companion is not None:
            queryset = queryset.filter(is_companion=is_companion.lower() == 'true')

        # Search by name
        search = self.request.query_params.get('search', None)
        if search:
            queryset = queryset.filter(
                first_name__icontains=search
            ) | queryset.filter(
                last_name__icontains=search
            )

        return queryset


    def update(self, request, *args, **kwargs):
        """Override update to also save changes to in-memory data"""
        # Perform standard update
        response = super().update(request, *args, **kwargs)
        
        # If update was successful, update in-memory data  
        if response.status_code == 200:
            try:
                character, session = self._get_character_session(kwargs.get('pk'))
                manager = session.character_manager
                
                # Update in-memory GFF data with the changed fields
                updated_fields = request.data
                gff_field_map = {
                    'strength': 'Str',
                    'dexterity': 'Dex', 
                    'constitution': 'Con',
                    'intelligence': 'Int',
                    'wisdom': 'Wis',
                    'charisma': 'Cha',
                    'hit_points': 'CurrentHitPoints',
                    'max_hit_points': 'MaxHitPoints',
                    'experience': 'Experience', 
                    'gold': 'Gold',
                    'law_chaos': 'LawfulChaotic',
                    'good_evil': 'GoodEvil',
                    'first_name': 'FirstName',
                    'last_name': 'LastName',
                }
                
                for field, value in updated_fields.items():
                    gff_field = gff_field_map.get(field, field)
                    manager.gff.set(gff_field, value)
                
                response.data['has_unsaved_changes'] = session.has_unsaved_changes()
                
                # Cache invalidation is handled automatically by the in-memory system
                
            except Exception as e:
                logger.error(f"Failed to update in-memory data for character {kwargs.get('pk')}: {e}")
                response.data['memory_update_warning'] = f"Database updated but in-memory sync failed: {str(e)}"
        
        return response

    def partial_update(self, request, *args, **kwargs):
        """Override partial_update to also save changes to in-memory data"""
        # Perform standard update
        response = super().partial_update(request, *args, **kwargs)
        
        # If update was successful, update in-memory data
        if response.status_code == 200:
            try:
                character, session = self._get_character_session(kwargs.get('pk'))
                manager = session.character_manager
                
                # Update in-memory GFF data with the changed fields
                updated_fields = request.data
                gff_field_map = {
                    'strength': 'Str',
                    'dexterity': 'Dex',
                    'constitution': 'Con', 
                    'intelligence': 'Int',
                    'wisdom': 'Wis',
                    'charisma': 'Cha',
                    'hit_points': 'CurrentHitPoints',
                    'max_hit_points': 'MaxHitPoints',
                    'experience': 'Experience',
                    'gold': 'Gold',
                    'law_chaos': 'LawfulChaotic',
                    'good_evil': 'GoodEvil',
                    'first_name': 'FirstName',
                    'last_name': 'LastName',
                }
                
                for field, value in updated_fields.items():
                    gff_field = gff_field_map.get(field, field)
                    manager.gff.set(gff_field, value)
                
                response.data['has_unsaved_changes'] = session.has_unsaved_changes()
                
                # Cache invalidation is handled automatically by the in-memory system
                
            except Exception as e:
                logger.error(f"Failed to update in-memory data for character {kwargs.get('pk')}: {e}")
                response.data['memory_update_warning'] = f"Database updated but in-memory sync failed: {str(e)}"
        
        return response

    @action(detail=True, methods=['get'])
    def character_state(self, request, pk=None):
        """Get comprehensive character state with all subsystem information"""
        try:
            character, manager = self._get_character_manager(pk)

            # Get comprehensive state directly from managers (in-memory system is fast enough)
            state = {
                'summary': manager.get_character_summary(),
                'classes': manager.get_manager('class').get_class_summary() if 'class' in manager._managers else {},
                'combat': manager.get_manager('combat').get_combat_summary() if 'combat' in manager._managers else {},
                'skills': manager.get_manager('skill').get_skill_summary() if 'skill' in manager._managers else {},
                'feats': manager.get_manager('feat').get_feat_summary() if 'feat' in manager._managers else {},
                'spells': manager.get_manager('spell').get_spell_summary() if 'spell' in manager._managers else {},
                'inventory': manager.get_manager('inventory').get_inventory_summary() if 'inventory' in manager._managers else {},
                'attributes': manager.get_manager('attribute').get_attribute_summary() if 'attribute' in manager._managers else {},
                'saves': manager.get_manager('save').get_save_summary() if 'save' in manager._managers else {},
            }
            
            # Add non-cacheable data (always fresh)
            state['custom_content'] = {
                'count': len(manager.custom_content),
                'summary': CustomContentDetector(manager.game_data_loader).get_protection_summary(
                    manager.custom_content
                )
            }

            return Response(state, status=status.HTTP_200_OK)

        except Exception as e:
            return self._handle_character_error(pk, e, "character_state")

    @action(detail=True, methods=['post'])
    def duplicate(self, request, pk=None):
        """Create a copy of the character"""
        original = self.get_object()

        # Create new character
        new_char = Character.objects.create(
            owner=None,
            file_name=f"Copy of {original.file_name}",
            first_name=original.first_name,
            last_name=original.last_name,
            age=original.age,
            gender=original.gender,
            deity=original.deity,
            race_id=original.race_id,
            race_name=original.race_name,
            subrace_id=original.subrace_id,
            subrace_name=original.subrace_name,
            law_chaos=original.law_chaos,
            good_evil=original.good_evil,
            experience=original.experience,
            character_level=original.character_level,
            strength=original.strength,
            dexterity=original.dexterity,
            constitution=original.constitution,
            intelligence=original.intelligence,
            wisdom=original.wisdom,
            charisma=original.charisma,
            hit_points=original.hit_points,
            max_hit_points=original.max_hit_points,
            armor_class=original.armor_class,
            fortitude_save=original.fortitude_save,
            reflex_save=original.reflex_save,
            will_save=original.will_save,
            gold=original.gold,
            is_companion=original.is_companion
        )

        # Copy related data
        for cls in original.classes.all():
            new_char.classes.create(
                class_id=cls.class_id,
                class_name=cls.class_name,
                class_level=cls.class_level,
                domain1_id=cls.domain1_id,
                domain1_name=cls.domain1_name,
                domain2_id=cls.domain2_id,
                domain2_name=cls.domain2_name
            )

        for feat in original.feats.all():
            new_char.feats.create(
                feat_id=feat.feat_id,
                feat_name=feat.feat_name
            )

        for skill in original.skills.all():
            new_char.skills.create(
                skill_id=skill.skill_id,
                skill_name=skill.skill_name,
                rank=skill.rank
            )

        serializer = CharacterDetailSerializer(new_char)
        return Response(serializer.data, status=status.HTTP_201_CREATED)

    @action(detail=True, methods=['get'])
    def validate(self, request, pk=None):
        """Validate character data against game rules"""
        character = self.get_object()

        rm = ResourceManager('nwn2_ee_data')
        try:
            validator = CharacterValidator(rm)
            errors = validator.validate_character(character)

            if errors:
                return Response({
                    'valid': False,
                    'errors': errors
                }, status=status.HTTP_200_OK)
            else:
                return Response({
                    'valid': True,
                    'errors': {}
                }, status=status.HTTP_200_OK)

        finally:
            rm.close()

    @action(detail=True, methods=['get'])
    def raw_data(self, request, pk=None):
        """Get raw parsed data from character file"""
        try:
            character, manager = self._get_character_manager(pk)

            # Get raw data from the manager's GFF element
            raw_data = manager.gff.to_dict()

            # Convert bytes to base64 for JSON serialization
            def convert_bytes(obj):
                if isinstance(obj, bytes):
                    return {'_type': 'bytes', 'length': len(obj), 'data': obj.hex()[:100] + '...'}
                elif isinstance(obj, dict):
                    return {k: convert_bytes(v) for k, v in obj.items()}
                elif isinstance(obj, list):
                    return [convert_bytes(item) for item in obj]
                return obj

            serializable_data = convert_bytes(raw_data)

            return Response({
                'file_path': character.file_path,
                'file_type': 'SAVEGAME' if character.is_savegame else ('ROS' if character.is_companion else 'BIC'),
                'data': serializable_data
            })

        except Exception as e:
            return self._handle_character_error(pk, e, "raw_data")

    @action(detail=True, methods=['get'])
    def field_structure(self, request, pk=None):
        """Get the field structure of a character file with field names and types"""
        try:
            character, manager = self._get_character_manager(pk)

            # Parse the character file
            from parsers.gff import GFFParser, GFFFieldType

            def extract_field_info(element, path=''):
                """Recursively extract field information"""
                field_info = {
                    'path': path,
                    'label': element.label,
                    'type': GFFFieldType(element.type).name,
                    'type_id': element.type,
                }

                # Add value for simple types
                if element.type not in [GFFFieldType.STRUCT, GFFFieldType.LIST]:
                    field_info['value'] = element.value

                # Add children for complex types
                if element.type == GFFFieldType.STRUCT:
                    field_info['fields'] = []
                    for field in element.value:
                        child_path = f"{path}.{field.label}" if path else field.label
                        field_info['fields'].append(extract_field_info(field, child_path))
                elif element.type == GFFFieldType.LIST:
                    field_info['count'] = len(element.value)
                    # Show structure of first item if exists
                    if element.value:
                        field_info['item_structure'] = extract_field_info(
                            element.value[0],
                            f"{path}[0]" if path else "[0]"
                        )

                return field_info

            # Get GFF element from manager
            char_data = manager.gff

            # Extract field structure
            structure = extract_field_info(char_data)

            # Also extract all top-level field names
            field_names = [field.label for field in char_data.value]

            # For savegames, we need to handle metadata differently
            if character.is_savegame:
                file_type = 'SAVEGAME'
                file_version = 'V3.2'  # Savegames use standard GFF V3.2
            else:
                # For regular files, parser has this info
                parser = GFFParser()
                # We need to quickly read just the header to get file type/version
                with open(character.file_path, 'rb') as f:
                    header = f.read(56)
                    file_type = header[0:4].decode('ascii', errors='ignore').strip()
                    file_version = header[4:8].decode('ascii', errors='ignore').strip()
            
            response_data = {
                'character_id': character.id,
                'file_name': character.file_name,
                'file_type': file_type,
                'file_version': file_version,
                'top_level_fields': sorted(field_names),
                'field_count': len(field_names),
                'structure': structure
            }

            return Response(response_data, status=status.HTTP_200_OK)

        except Exception as e:
            return self._handle_character_error(pk, e, "field_structure")

    @action(detail=True, methods=['get'])
    def alignment(self, request, pk=None):
        """Get character alignment"""
        try:
            character, manager = self._get_character_manager(pk)

            # Get alignment from CharacterManager
            law_chaos = manager.gff.get('LawfulChaotic', 50)
            good_evil = manager.gff.get('GoodEvil', 50)

            return Response({
                'lawChaos': law_chaos,
                'goodEvil': good_evil
            }, status=status.HTTP_200_OK)

        except Exception as e:
            return self._handle_character_error(pk, e, "alignment")

    @action(detail=True, methods=['post'])
    def update_alignment(self, request, pk=None):
        """Update character alignment"""
        try:
            law_chaos = request.data.get('lawChaos')
            good_evil = request.data.get('goodEvil')
            
            if law_chaos is None or good_evil is None:
                return Response(
                    {'error': 'Both lawChaos and goodEvil values are required'},
                    status=status.HTTP_400_BAD_REQUEST
                )
            
            # Validate range (0-100)
            if not (0 <= law_chaos <= 100) or not (0 <= good_evil <= 100):
                return Response(
                    {'error': 'Alignment values must be between 0 and 100'},
                    status=status.HTTP_400_BAD_REQUEST
                )

            character, session = self._get_character_session(pk)
            manager = session.character_manager
            
            # Update alignment in memory
            manager.gff.set('LawfulChaotic', law_chaos)
            manager.gff.set('GoodEvil', good_evil)
            
            # Update character model
            character.law_chaos = law_chaos
            character.good_evil = good_evil
            character.save(update_fields=['law_chaos', 'good_evil'])

            return Response({
                'lawChaos': law_chaos,
                'goodEvil': good_evil,
                'alignment_string': character.alignment,
                'has_unsaved_changes': session.has_unsaved_changes()
            }, status=status.HTTP_200_OK)

        except Exception as e:
            return self._handle_character_error(pk, e, "update_alignment")