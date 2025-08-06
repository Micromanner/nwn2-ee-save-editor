"""
Inventory ViewSet - All inventory-related endpoints
Handles items, equipment, and encumbrance
"""

from rest_framework import status
from rest_framework.decorators import action
from rest_framework.response import Response
import logging

from .base_character_view import BaseCharacterViewSet

logger = logging.getLogger(__name__)


class InventoryViewSet(BaseCharacterViewSet):
    """
    ViewSet for inventory-related operations
    All endpoints are nested under /api/characters/{id}/inventory/
    """
    @action(detail=False, methods=['get'], url_path='state')
    def inventory_state(self, request, character_pk=None):
        """Get current inventory state for the inventory editor"""
        try:
            character, manager = self._get_character_manager(character_pk)
            inventory_manager = manager.get_manager('inventory')
            
            encumbrance_data = inventory_manager.calculate_encumbrance()
            
            state = {
                'inventory_summary': inventory_manager.get_inventory_summary(),
                'equipped_items': inventory_manager.get_equipment_summary_by_slot(),
                'carry_capacity': {
                    'current_weight': encumbrance_data.get('total_weight', 0),
                    'light_load': encumbrance_data.get('light_load', 0),
                    'medium_load': encumbrance_data.get('medium_load', 0),
                    'heavy_load': encumbrance_data.get('heavy_load', 0),
                    'encumbrance_level': encumbrance_data.get('encumbrance_level', 'light')
                }
            }
            
            return Response(state, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "inventory_state")
    
    @action(detail=False, methods=['post'], url_path='manage')
    def manage_inventory(self, request, character_pk=None):
        """
        Add, remove, or move items in inventory
        """
        action = request.data.get('action')  # 'add', 'remove', 'move', 'equip', 'unequip'
        item_data = request.data.get('item_data', {})
        
        if not action:
            return Response(
                {'error': 'action is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            inventory_manager = manager.get_manager('inventory')
            
            if action == 'add':
                # Add new item
                base_item = item_data.get('base_item')
                stack_size = item_data.get('stack_size', 1)
                properties = item_data.get('properties', [])
                
                if base_item is None:
                    return Response(
                        {'error': 'base_item is required for add action'},
                        status=status.HTTP_400_BAD_REQUEST
                    )
                
                inventory_manager.add_to_inventory({
                    'BaseItem': base_item,
                    'StackSize': stack_size,
                    'PropertiesList': properties
                })
                message = 'Item added to inventory'
                
            elif action == 'remove':
                item_index = item_data.get('item_index')
                if item_index is None:
                    return Response(
                        {'error': 'item_index is required for remove action'},
                        status=status.HTTP_400_BAD_REQUEST
                    )
                inventory_manager.remove_from_inventory(item_index)
                message = 'Item removed from inventory'
                
            elif action == 'equip':
                item_data_to_equip = item_data.get('item')
                slot = item_data.get('slot')
                if not item_data_to_equip or slot is None:
                    return Response(
                        {'error': 'item and slot are required for equip action'},
                        status=status.HTTP_400_BAD_REQUEST
                    )
                success, errors = inventory_manager.equip_item(item_data_to_equip, slot)
                if not success:
                    return Response(
                        {'error': f'Cannot equip item: {"; ".join(errors)}'},
                        status=status.HTTP_400_BAD_REQUEST
                    )
                message = 'Item equipped'
                
            elif action == 'unequip':
                slot = item_data.get('slot')
                if slot is None:
                    return Response(
                        {'error': 'slot is required for unequip action'},
                        status=status.HTTP_400_BAD_REQUEST
                    )
                inventory_manager.unequip_item(slot)
                message = 'Item unequipped'
                
            else:
                return Response(
                    {'error': f'Unknown action: {action}'},
                    status=status.HTTP_400_BAD_REQUEST
                )
            
            # Get updated inventory
            inventory_summary = inventory_manager.get_inventory_summary()
            
            return Response({
                'message': message,
                'inventory_summary': inventory_summary,
                'has_unsaved_changes': session.has_unsaved_changes()
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "manage_inventory")