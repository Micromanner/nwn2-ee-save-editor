"""
Combat ViewSet - All combat-related endpoints
Handles BAB, AC, attack bonuses, damage, and combat statistics
"""

from rest_framework import viewsets, status
from rest_framework.decorators import action
from rest_framework.response import Response
from django.shortcuts import get_object_or_404
import logging

from character.models import Character
from character.factory import get_or_create_character_manager
from gamedata.middleware import get_character_manager, set_character_manager, clear_character_manager
from parsers.gff import GFFParser, GFFWriter
from parsers.savegame_handler import SaveGameHandler
from .base_character_view import BaseCharacterViewSet
from io import BytesIO
import os

logger = logging.getLogger(__name__)


class CombatViewSet(BaseCharacterViewSet):
    """
    ViewSet for combat-related operations
    All endpoints are nested under /api/characters/{id}/combat/
    """
    
    @action(detail=False, methods=['get'], url_path='state')
    def combat_state(self, request, character_pk=None):
        """Get current combat statistics for the combat editor"""
        try:
            character, manager = self._get_character_manager(character_pk)
            combat_manager = manager.get_manager('combat')
            
            state = {
                'combat_summary': combat_manager.get_combat_summary(),
                'base_attack_bonus': combat_manager.calculate_base_attack_bonus(),
                'armor_class': {
                    'total': combat_manager.calculate_armor_class(),
                    'base': 10,
                    'dex_modifier': combat_manager._get_dex_modifier(),
                    'armor_bonus': combat_manager._get_armor_bonus(),
                    'shield_bonus': combat_manager._get_shield_bonus(),
                    'natural_armor': combat_manager._get_natural_armor_bonus()
                },
                'attack_bonuses': {
                    'melee': combat_manager.calculate_melee_attack_bonus(),
                    'ranged': combat_manager.calculate_ranged_attack_bonus()
                },
                'damage_bonuses': {
                    'melee': combat_manager.calculate_melee_damage_bonus(),
                    'ranged': combat_manager.calculate_ranged_damage_bonus()
                }
            }
            
            return Response(state, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "combat_state")
    
    @action(detail=False, methods=['get'], url_path='bab')
    def base_attack_bonus(self, request, character_pk=None):
        """Get detailed base attack bonus breakdown"""
        try:
            character, manager = self._get_character_manager(character_pk)
            combat_manager = manager.get_manager('combat')
            
            bab_info = {
                'total_bab': combat_manager.calculate_base_attack_bonus(),
                'class_breakdown': combat_manager.get_class_bab_breakdown(),
                'attack_sequence': combat_manager.get_attack_sequence(),
                'iterative_attacks': combat_manager.get_iterative_attacks()
            }
            
            return Response(bab_info, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "base_attack_bonus")
     
    @action(detail=False, methods=['get'], url_path='ac')
    def armor_class(self, request, character_pk=None):
        """Get detailed armor class breakdown"""
        try:
            character, manager = self._get_character_manager(character_pk)
            combat_manager = manager.get_manager('combat')
            
            ac_info = {
                'total_ac': combat_manager.calculate_armor_class(),
                'base_ac': 10,
                'dex_modifier': combat_manager._get_dex_modifier(),
                'armor_bonus': combat_manager._get_armor_bonus(),
                'shield_bonus': combat_manager._get_shield_bonus(),
                'natural_armor': combat_manager._get_natural_armor_bonus(),
                'deflection_bonus': combat_manager._get_deflection_bonus(),
                'dodge_bonus': combat_manager._get_dodge_bonus(),
                'size_modifier': combat_manager._get_size_ac_modifier(),
                'touch_ac': combat_manager.calculate_touch_ac(),
                'flat_footed_ac': combat_manager.calculate_flat_footed_ac()
            }
            
            return Response(ac_info, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "armor_class")
       
    @action(detail=False, methods=['get'], url_path='attacks')
    def attack_bonuses(self, request, character_pk=None):
        """Get detailed attack bonus breakdown"""
        try:
            character, manager = self._get_character_manager(character_pk)
            combat_manager = manager.get_manager('combat')
            
            attack_info = {
                'melee': {
                    'total': combat_manager.calculate_melee_attack_bonus(),
                    'base_attack_bonus': combat_manager.calculate_base_attack_bonus(),
                    'ability_modifier': combat_manager._get_str_modifier(),
                    'size_modifier': combat_manager._get_size_attack_modifier(),
                    'weapon_focus': combat_manager._get_weapon_focus_bonus(),
                    'other_bonuses': combat_manager._get_misc_attack_bonuses()
                },
                'ranged': {
                    'total': combat_manager.calculate_ranged_attack_bonus(),
                    'base_attack_bonus': combat_manager.calculate_base_attack_bonus(),
                    'ability_modifier': combat_manager._get_dex_modifier(),
                    'size_modifier': combat_manager._get_size_attack_modifier(),
                    'weapon_focus': combat_manager._get_weapon_focus_bonus(),
                    'other_bonuses': combat_manager._get_misc_attack_bonuses()
                }
            }
            
            return Response(attack_info, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "attack_bonuses")
    
    @action(detail=False, methods=['post'], url_path='update-ac')
    def update_natural_armor(self, request, character_pk=None):
        """Update character's natural armor bonus"""
        natural_ac = request.data.get('natural_ac')
        
        if natural_ac is None:
            return Response(
                {'error': 'natural_ac field is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        try:
            # Clamp value to reasonable range
            natural_ac = max(0, min(20, int(natural_ac)))
            
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            
            # Update the GFF field directly
            old_value = manager.character_data.get('NaturalAC', 0)
            manager.character_data['NaturalAC'] = natural_ac
            
            # Also update the GFF element
            manager.gff.set('NaturalAC', natural_ac)
            
            # Get updated combat stats
            combat_manager = manager.get_manager('combat')
            updated_ac = combat_manager.calculate_armor_class()
            
            return Response({
                'field': 'NaturalAC',
                'old_value': old_value,
                'new_value': natural_ac,
                'updated_ac': updated_ac,
                'has_unsaved_changes': session.has_unsaved_changes()
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "update_natural_armor")
    
    @action(detail=False, methods=['get'], url_path='damage')
    def damage_bonuses(self, request, character_pk=None):
        """Get detailed damage bonus breakdown"""
        try:
            character, manager = self._get_character_manager(character_pk)
            combat_manager = manager.get_manager('combat')
            
            damage_info = {
                'melee': {
                    'total': combat_manager.calculate_melee_damage_bonus(),
                    'ability_modifier': combat_manager._get_str_modifier(),
                    'weapon_specialization': combat_manager._get_weapon_specialization_bonus(),
                    'enhancement_bonus': combat_manager._get_weapon_enhancement_bonus(),
                    'other_bonuses': combat_manager._get_misc_damage_bonuses()
                },
                'ranged': {
                    'total': combat_manager.calculate_ranged_damage_bonus(),
                    'ability_modifier': 0,  # Ranged weapons don't usually add Str
                    'weapon_specialization': combat_manager._get_weapon_specialization_bonus(),
                    'enhancement_bonus': combat_manager._get_weapon_enhancement_bonus(),
                    'other_bonuses': combat_manager._get_misc_damage_bonuses()
                }
            }
            
            return Response(damage_info, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "damage_bonuses")
        
    @action(detail=False, methods=['get'], url_path='weapons')
    def equipped_weapons(self, request, character_pk=None):
        """Get information about equipped weapons"""
        try:
            character, manager = self._get_character_manager(character_pk)
            combat_manager = manager.get_manager('combat')
            
            weapons_info = {
                'main_hand': combat_manager.get_main_hand_weapon(),
                'off_hand': combat_manager.get_off_hand_weapon(),
                'ranged': combat_manager.get_ranged_weapon(),
                'two_weapon_fighting': combat_manager.is_two_weapon_fighting(),
                'weapon_finesse': combat_manager.can_use_weapon_finesse()
            }
            
            return Response(weapons_info, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "equipped_weapons")
       
    @action(detail=False, methods=['post'], url_path='simulate')
    def simulate_attack(self, request, character_pk=None):
        """Simulate an attack roll (for testing/preview)"""
        attack_type = request.data.get('type', 'melee')  # 'melee' or 'ranged'
        target_ac = request.data.get('target_ac', 10)
        
        try:
            character, manager = self._get_character_manager(character_pk)
            combat_manager = manager.get_manager('combat')
            
            if attack_type == 'ranged':
                result = combat_manager.simulate_ranged_attack(target_ac)
            else:
                result = combat_manager.simulate_melee_attack(target_ac)
            
            return Response(result, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "simulate_attack")
    
    @action(detail=False, methods=['get'], url_path='defensive')
    def defensive_stats(self, request, character_pk=None):
        """Get defensive combat statistics"""
        try:
            character, manager = self._get_character_manager(character_pk)
            combat_manager = manager.get_manager('combat')
            
            defensive_info = {
                'armor_class': combat_manager.calculate_armor_class(),
                'touch_ac': combat_manager.calculate_touch_ac(),
                'flat_footed_ac': combat_manager.calculate_flat_footed_ac(),
                'damage_reduction': combat_manager.get_damage_reduction(),
                'spell_resistance': combat_manager.get_spell_resistance(),
                'concealment': combat_manager.get_concealment(),
                'miss_chance': combat_manager.get_miss_chance()
            }
            
            return Response(defensive_info, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "defensive_stats")
    
