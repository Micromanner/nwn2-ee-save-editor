"""
Attribute ViewSet - All attribute/ability score related endpoints
Handles STR, DEX, CON, INT, WIS, CHA and their modifiers
"""

from rest_framework import status
from rest_framework.decorators import action
from rest_framework.response import Response
import logging

from .base_character_view import BaseCharacterViewSet

logger = logging.getLogger(__name__)


class AttributeViewSet(BaseCharacterViewSet):
    """
    ViewSet for attribute-related operations
    All endpoints are nested under /api/characters/{id}/attributes/
    """
    
    
    @action(detail=False, methods=['get'], url_path='state')
    def attributes_state(self, request, character_pk=None):
        """Get current attributes and modifiers for the attributes editor"""
        try:
            character, manager = self._get_character_manager(character_pk)
            attr_manager = manager.get_manager('attribute')
            
            # Get comprehensive attribute state
            state = {
                'base_attributes': attr_manager.get_attributes(),
                'attribute_modifiers': attr_manager.get_all_modifiers(),
                'point_buy_cost': attr_manager.calculate_point_buy_total(),
                'racial_modifiers': attr_manager.get_racial_modifiers(),
                'item_modifiers': {},  # TODO: Get from inventory
                'total_modifiers': {},
                'derived_stats': {
                    'hit_points': {
                        'current': manager.character_data.get('CurrentHitPoints', 1),
                        'maximum': manager.character_data.get('MaxHitPoints', 1)
                    }
                }
            }
            
            # Calculate total modifiers
            for attr in ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']:
                base = state['base_attributes'][attr]
                modifier = (base - 10) // 2
                state['total_modifiers'][attr] = modifier
            
            # Add combat and save summaries if managers available
            if 'combat' in manager._managers:
                state['combat_stats'] = manager.get_manager('combat').get_combat_summary()
            if 'save' in manager._managers:
                state['saving_throws'] = manager.get_manager('save').get_save_summary()
            
            return Response(state, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "attributes_state")
    
    @action(detail=False, methods=['post'], url_path='update')
    def change_attributes(self, request, character_pk=None):
        """
        Change character attributes (STR, DEX, CON, INT, WIS, CHA)
        Handles cascading effects like HP, saves, and combat bonuses
        """
        attributes = request.data.get('attributes', {})
        
        if not attributes:
            return Response(
                {'error': 'attributes field is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            
            # Track all changes
            all_changes = {
                'attribute_changes': [],
                'cascading_effects': []
            }
            
            # Apply each attribute change
            attr_manager = manager.get_manager('attribute')
            for attr_name, new_value in attributes.items():
                if attr_name in ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']:
                    old_value = manager.character_data.get(attr_name, 10)
                    if old_value != new_value:
                        result = attr_manager.set_attribute(attr_name, new_value)
                        all_changes['attribute_changes'].append({
                            'attribute': attr_name,
                            'old_value': old_value,
                            'new_value': new_value,
                            'cascading': result
                        })
            
            # Get updated summaries
            all_changes['updated_attributes'] = attr_manager.get_attributes()
            all_changes['updated_combat'] = manager.get_manager('combat').get_combat_summary() if 'combat' in manager._managers else {}
            all_changes['updated_saves'] = manager.get_manager('save').get_save_summary() if 'save' in manager._managers else {}
            
            # Keep changes in memory - no auto-save
            # Session retains all changes for future calculations
            
            all_changes['saved'] = False  # Changes kept in memory only
            all_changes['has_unsaved_changes'] = session.has_unsaved_changes()
            return Response(all_changes, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "change_attributes")
    
    @action(detail=True, methods=['post'], url_path='set')
    def set_attribute(self, request, character_pk=None, pk=None):
        """Set a specific attribute to a value"""
        new_value = request.data.get('value')
        
        if new_value is None:
            return Response(
                {'error': 'value field is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        # Map URL pk to attribute name
        attr_mapping = {
            'str': 'Str', 'strength': 'Str',
            'dex': 'Dex', 'dexterity': 'Dex',
            'con': 'Con', 'constitution': 'Con',
            'int': 'Int', 'intelligence': 'Int',
            'wis': 'Wis', 'wisdom': 'Wis',
            'cha': 'Cha', 'charisma': 'Cha'
        }
        
        attr_name = attr_mapping.get(pk.lower())
        if not attr_name:
            return Response(
                {'error': f'Invalid attribute: {pk}'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            attr_manager = manager.get_manager('attribute')
            
            old_value = manager.character_data.get(attr_name, 10)
            result = attr_manager.set_attribute(attr_name, new_value)
            
            # Keep changes in memory - no auto-save
            # Session retains all changes for future calculations
            
            return Response({
                'attribute': attr_name,
                'old_value': old_value,
                'new_value': new_value,
                'cascading_effects': result,
                'saved': False,  # Changes kept in memory only
                'has_unsaved_changes': session.has_unsaved_changes()
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "set_attribute")
    
    @action(detail=False, methods=['post'], url_path='point-buy')
    def set_point_buy(self, request, character_pk=None):
        """Set attributes using point-buy system"""
        point_buy_data = request.data.get('attributes', {})
        total_points = request.data.get('total_points', 32)  # Standard NWN2 point buy
        
        if not point_buy_data:
            return Response(
                {'error': 'attributes field is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            attr_manager = manager.get_manager('attribute')
            
            # Validate point buy
            cost = attr_manager.calculate_point_buy_cost(point_buy_data)
            if cost > total_points:
                return Response(
                    {'error': f'Point buy cost {cost} exceeds available points {total_points}'},
                    status=status.HTTP_400_BAD_REQUEST
                )
            
            # Apply all attributes
            changes = []
            for attr_name, new_value in point_buy_data.items():
                if attr_name in ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']:
                    old_value = manager.character_data.get(attr_name, 10)
                    if old_value != new_value:
                        attr_manager.set_attribute(attr_name, new_value)
                        changes.append({
                            'attribute': attr_name,
                            'old_value': old_value,
                            'new_value': new_value
                        })
            
            # Keep changes in memory - no auto-save
            # Session retains all changes for future calculations
            
            return Response({
                'changes': changes,
                'total_cost': cost,
                'remaining_points': total_points - cost,
                'saved': False,  # Changes kept in memory only
                'has_unsaved_changes': session.has_unsaved_changes()
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "set_point_buy")
    
    @action(detail=False, methods=['post'], url_path='roll')
    def roll_attributes(self, request, character_pk=None):
        """Roll attributes using 4d6 drop lowest method"""
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            attr_manager = manager.get_manager('attribute')
            
            # Roll new attributes
            rolled_stats = attr_manager.roll_attributes()
            
            # Apply the rolled stats
            changes = []
            for attr_name, new_value in rolled_stats.items():
                old_value = manager.character_data.get(attr_name, 10)
                attr_manager.set_attribute(attr_name, new_value)
                changes.append({
                    'attribute': attr_name,
                    'old_value': old_value,
                    'new_value': new_value,
                    'rolls': rolled_stats.get(f'{attr_name}_rolls', [])
                })
            
            # Keep changes in memory - no auto-save
            # Session retains all changes for future calculations
            
            return Response({
                'changes': changes,
                'total_rolled': sum(rolled_stats.values()),
                'saved': False,  # Changes kept in memory only
                'has_unsaved_changes': session.has_unsaved_changes()
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "roll_attributes")
    

    @action(detail=False, methods=['get'], url_path='modifiers')
    def get_modifiers(self, request, character_pk=None):
        """Get detailed breakdown of all attribute modifiers"""
        try:
            character, manager = self._get_character_manager(character_pk)
            attr_manager = manager.get_manager('attribute')
            
            modifiers = {
                'base_modifiers': attr_manager.get_all_modifiers(),
                'racial_modifiers': attr_manager.get_racial_modifiers(),
                'enhancement_modifiers': attr_manager.get_enhancement_modifiers(),
                'item_modifiers': attr_manager.get_item_modifiers(),
                'temporary_modifiers': attr_manager.get_temporary_modifiers(),
                'total_modifiers': attr_manager.get_total_modifiers()
            }
            
            return Response(modifiers, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "get_modifiers")