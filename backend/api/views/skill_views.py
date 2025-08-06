"""
Skill ViewSet - All skill-related endpoints
Handles skill points, ranks, modifiers, and skill checks
"""

from rest_framework import status
from rest_framework.decorators import action
from rest_framework.response import Response
import logging

from .base_character_view import BaseCharacterViewSet

logger = logging.getLogger(__name__)


class SkillViewSet(BaseCharacterViewSet):
    """
    ViewSet for skill-related operations
    All endpoints are nested under /api/characters/{id}/skills/
    """
    @action(detail=False, methods=['get'], url_path='state')
    def skills_state(self, request, character_pk=None):
        """Get current skills state for the skills editor"""
        try:
            character, manager = self._get_character_manager(character_pk)
            skill_manager = manager.get_manager('skill')
            
            # Get all skills with current ranks and modifiers
            all_skills = []
            skills_table = manager.game_data_loader.get_table('skills')
            if skills_table:
                for skill_id, skill_data in enumerate(skills_table):
                    # In NWN2, the skill ID is the index in the skills table
                    if skill_id >= 0:
                        # Get individual components of skill modifier
                        ranks = skill_manager.get_skill_ranks(skill_id)
                        total_modifier = skill_manager.calculate_skill_modifier(skill_id)
                        
                        # Calculate ability modifier component
                        key_ability = getattr(skill_data, 'KeyAbility', 'STR').upper()
                        modifiers = skill_manager._calculate_ability_modifiers()
                        ability_modifier = modifiers.get(key_ability, 0)
                        
                        skill_info = {
                            'id': skill_id,
                            'name': getattr(skill_data, 'Label', f'Skill {skill_id}'),
                            'rank': ranks,
                            'max_rank': skill_manager.get_max_skill_ranks(skill_id),
                            'total_modifier': total_modifier,
                            'ability_modifier': ability_modifier,
                            'is_class_skill': skill_manager.is_class_skill(skill_id),
                            'ability': getattr(skill_data, 'KeyAbility', 'None'),
                            'armor_check_penalty': skill_manager.is_armor_check_skill(skill_id)
                        }
                        all_skills.append(skill_info)
            
            state = {
                'skills': all_skills,
                'skill_points': {
                    'available': skill_manager._calculate_available_skill_points(),
                    'spent': skill_manager._calculate_spent_skill_points()
                },
                'summary': skill_manager.get_skill_summary()
            }
            
            return Response(state, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "skills_state")
    
    @action(detail=False, methods=['post'], url_path='update')
    def update_skills(self, request, character_pk=None):
        """Update skill ranks for character (in-memory only - use savegame endpoints to save)"""
        skill_updates = request.data.get('skills', {})
        
        logger.info(f"Skill update request for character {character_pk}: {skill_updates}")
        
        if not skill_updates:
            return Response(
                {'error': 'skills field is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            skill_manager = manager.get_manager('skill')
            
            # Track changes
            changes = []
            
            # Update each skill
            for skill_id_str, new_rank in skill_updates.items():
                skill_id = int(skill_id_str)
                old_rank = skill_manager.get_skill_ranks(skill_id)
                
                if old_rank != new_rank:
                    # Validate rank
                    max_rank = skill_manager.get_max_skill_ranks(skill_id)
                    if new_rank > max_rank:
                        return Response(
                            {'error': f'Skill {skill_id} rank {new_rank} exceeds maximum {max_rank}'},
                            status=status.HTTP_400_BAD_REQUEST
                        )
                    
                    skill_manager.set_skill_rank(skill_id, new_rank)
                    
                    # Get skill name for change tracking
                    skill_data = manager.game_data_loader.get_by_id('skills', skill_id)
                    skill_name = (getattr(skill_data, 'Label', f'Skill {skill_id}')
                                  if skill_data else f'Skill {skill_id}')
                    
                    changes.append({
                        'skill_id': skill_id,
                        'skill_name': skill_name,
                        'old_rank': old_rank,
                        'new_rank': new_rank
                    })
            
            # Get updated summary
            skill_summary = skill_manager.get_skill_summary()
            
            return Response({
                'changes': changes,
                'skill_summary': skill_summary,
                'has_unsaved_changes': session.has_unsaved_changes()
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "update_skills")
    
    @action(detail=False, methods=['post'], url_path='batch')
    def batch_update(self, request, character_pk=None):
        """Batch update multiple skills at once"""
        skills_dict = request.data.get('skills', {})
        
        if not skills_dict:
            return Response(
                {'error': 'skills field is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            skill_manager = manager.get_manager('skill')
            
            # Convert string keys to integers
            skills_dict = {int(k): v for k, v in skills_dict.items()}
            
            # Use batch update method
            results = skill_manager.batch_set_skills(skills_dict)
            
            # Keep changes in memory - no auto-save
            
            return Response({
                'results': results,
                'summary': skill_manager.get_skill_summary(),
                'saved': False
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "batch_update")
    
    @action(detail=False, methods=['post'], url_path='reset')
    def reset_skills(self, request, character_pk=None):
        """Reset all skills to 0 and refund points"""
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            skill_manager = manager.get_manager('skill')
            
            # Get current spent points before reset
            spent_before = skill_manager._calculate_spent_skill_points()
            
            # Reset all skills
            skill_manager.reset_all_skills()
            
            # Get points after reset
            available_after = skill_manager._calculate_available_skill_points()
            
            # Keep changes in memory - no auto-save
            
            return Response({
                'message': 'All skills reset successfully',
                'points_refunded': spent_before,
                'available_points': available_after,
                'saved': False
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "reset_skills")
    
    @action(detail=True, methods=['get'], url_path='check')
    def skill_check(self, request, character_pk=None, pk=None):
        """Simulate a skill check (d20 + modifiers)"""
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            skill_manager = manager.get_manager('skill')
            
            skill_id = int(pk)
            result = skill_manager.roll_skill_check(skill_id)
            
            return Response(result, status=status.HTTP_200_OK)
            
        except ValueError:
            return Response(
                {'error': 'Invalid skill ID'},
                status=status.HTTP_400_BAD_REQUEST
            )
        except Exception as e:
            return self._handle_character_error(character_pk, e, "skill_check")
    
    @action(detail=True, methods=['get'], url_path='prerequisites')
    def skill_prerequisites(self, request, character_pk=None, pk=None):
        """Get prerequisites for a specific skill"""
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            skill_manager = manager.get_manager('skill')
            
            skill_id = int(pk)
            prerequisites = skill_manager.get_skill_prerequisites(skill_id)
            
            return Response(prerequisites, status=status.HTTP_200_OK)
            
        except ValueError:
            return Response(
                {'error': 'Invalid skill ID'},
                status=status.HTTP_400_BAD_REQUEST
            )
        except Exception as e:
            return self._handle_character_error(character_pk, e, "skill_prerequisites")
    
    @action(detail=False, methods=['get'], url_path='export')
    def export_build(self, request, character_pk=None):
        """Export current skill build for saving/sharing"""
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            skill_manager = manager.get_manager('skill')
            
            build = skill_manager.export_skill_build()
            
            return Response(build, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "export_build")
    
    @action(detail=False, methods=['post'], url_path='import')
    def import_build(self, request, character_pk=None):
        """Import a skill build"""
        build_data = request.data
        
        if not build_data or 'skills' not in build_data:
            return Response(
                {'error': 'Invalid skill build data'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            skill_manager = manager.get_manager('skill')
            
            # Import the build
            success = skill_manager.import_skill_build(build_data)
            
            if success:
                # Keep changes in memory - no auto-save
                
                return Response({
                    'message': 'Skill build imported successfully',
                    'summary': skill_manager.get_skill_summary(),
                    'saved': False
                }, status=status.HTTP_200_OK)
            else:
                return Response(
                    {'error': 'Failed to import skill build'},
                    status=status.HTTP_400_BAD_REQUEST
                )
                
        except Exception as e:
            return self._handle_character_error(character_pk, e, "import_build")