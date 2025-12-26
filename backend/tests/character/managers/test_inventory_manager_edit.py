import pytest
from unittest.mock import Mock, MagicMock
from typing import Dict, Any, List, Optional, Tuple

from character.managers.inventory_manager import InventoryManager
from character.events import EventEmitter, EventType
from character.services.item_property_decoder import ItemPropertyDecoder

class MockGFF:
    def __init__(self, data):
        self.data = data
    def get(self, key, default=None):
        return self.data.get(key, default)
    def set(self, key, value):
        self.data[key] = value

@pytest.fixture
def mock_character_manager():
    manager = Mock()
    manager.rules_service = Mock()
    # Mock for _get_item_name
    manager.rules_service.get_by_id.return_value = {'label': 'Test Item', 'stacking': 1}
    
    mock_gff = MockGFF({
        'ItemList': [
            {'BaseItem': 1, 'Tag': 'item1', 'StackSize': 1},
            {'BaseItem': 2, 'Tag': 'item2', 'StackSize': 5}
        ],
        'Equip_ItemList': [
            {'BaseItem': 10, 'Tag': 'head_item', '__struct_id__': 0x0001}
        ]
    })
    manager.gff = mock_gff
    
    return manager

@pytest.fixture
def inventory_manager(mock_character_manager):
    # We want to test the actual methods, so we use a real InventoryManager
    # with a mock character_manager dependency.
    manager = InventoryManager(mock_character_manager)
    
    # Mock the internal property_decoder for specific tests
    manager.property_decoder = MagicMock(spec=ItemPropertyDecoder)
    
    return manager

class TestInventoryManagerEdit:
    def test_update_inventory_item(self, inventory_manager):
        new_data = {'BaseItem': 1, 'Tag': 'item1_updated', 'StackSize': 2}
        success, message = inventory_manager.update_item(item_index=0, slot=None, item_data=new_data)
        
        assert success is True
        assert inventory_manager.gff.get('ItemList')[0]['Tag'] == 'item1_updated'
        assert inventory_manager.gff.get('ItemList')[0]['StackSize'] == 2

    def test_update_equipped_item(self, inventory_manager):
        new_data = {'BaseItem': 10, 'Tag': 'head_updated'}
        success, message = inventory_manager.update_item(item_index=None, slot='head', item_data=new_data)
        
        assert success is True
        equipped = inventory_manager.get_equipped_item('head')
        assert equipped['Tag'] == 'head_updated'
        assert equipped['__struct_id__'] == 0x0001

    def test_add_item_by_base_type(self, inventory_manager):
        # Mock base item data
        inventory_manager.game_rules_service.get_by_id.return_value = {'label': 'Sword', 'stacking': 1}
        
        initial_count = len(inventory_manager.gff.get('ItemList'))
        success, new_item, message = inventory_manager.add_item_by_base_type(base_item_id=13)
        
        assert success is True
        assert len(inventory_manager.gff.get('ItemList')) == initial_count + 1
        assert inventory_manager.gff.get('ItemList')[-1]['BaseItem'] == 13
        assert 'Sword' in inventory_manager.gff.get('ItemList')[-1]['LocalizedName']['substrings'][0]['value']

    def test_get_item_editor_metadata(self, inventory_manager):
        # Mock decoder metadata
        inventory_manager.property_decoder.get_editor_property_metadata.return_value = [{'id': 1, 'label': 'AC Bonus'}]
        inventory_manager.property_decoder._ability_map = {0: 'Str'}
        inventory_manager.property_decoder._save_map = {0: 'fort'}
        inventory_manager.property_decoder._get_all_damage_types.return_value = {0: 'fire'}
        inventory_manager.property_decoder._get_all_immunity_types.return_value = {0: 'poison'}
        
        # Mock rules table for skills and races
        inventory_manager.game_rules_service.get_table.side_effect = lambda x: [{'Label': 'Skill1'}] if x == 'skills' else [{'Label': 'Race1'}]
        
        metadata = inventory_manager.get_item_editor_metadata()
        
        assert 'property_types' in metadata
        assert metadata['abilities'][0] == 'Str'
        assert metadata['skills'][0] == 'Skill1'
        assert metadata['racial_groups'][0] == 'Race1'
