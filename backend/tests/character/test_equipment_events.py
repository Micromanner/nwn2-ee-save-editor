"""
Test equipment event system - verifies event emissions and dependent subsystem updates
"""
import pytest
import sys
from pathlib import Path
from unittest.mock import Mock, MagicMock, patch

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from character.character_manager import CharacterManager, GFFDataWrapper
from character.managers.inventory_manager import InventoryManager
from character.managers.combat_manager import CombatManager
from character.events import EventType
from parsers.gff import GFFElement, GFFFieldType


def create_mock_gff_element(equipped_items=None):
    """
    Create a mock GFF element with proper Equip_ItemList structure.

    Args:
        equipped_items: List of (bitmask, item_dict) tuples for equipped items
    """
    if equipped_items is None:
        equipped_items = []

    equip_list = Mock()
    equip_list.label = 'Equip_ItemList'
    equip_list.value = []

    for bitmask, item_dict in equipped_items:
        item_element = Mock()
        item_element.id = bitmask
        item_element.label = ''
        item_element.type = GFFFieldType.STRUCT
        item_element.to_dict = Mock(return_value=item_dict.copy())

        base_item_field = Mock()
        base_item_field.type = GFFFieldType.INT
        base_item_field.label = 'BaseItem'
        base_item_field.value = item_dict.get('BaseItem', 0)

        item_element.value = [base_item_field]
        equip_list.value.append(item_element)

    gff_element = Mock()
    gff_element.value = [equip_list]

    return gff_element


@pytest.fixture
def mock_gff():
    """Create a mock GFF wrapper with equipment slots"""
    mock = MagicMock(spec=GFFDataWrapper)
    mock.get.return_value = [None] * 14
    mock._data = {'Equip_ItemList': []}
    return mock


@pytest.fixture
def mock_character_manager():
    """Create a mock CharacterManager"""
    char_mgr = Mock(spec=CharacterManager)
    char_mgr._event_listeners = {}
    char_mgr._event_history = []

    char_mgr.rules_service = Mock()
    char_mgr.gff_element = create_mock_gff_element()

    def mock_on(event_type, callback):
        if event_type not in char_mgr._event_listeners:
            char_mgr._event_listeners[event_type] = []
        char_mgr._event_listeners[event_type].append(callback)

    def mock_emit(event_type, data):
        char_mgr._event_history.append((event_type, data))
        if event_type in char_mgr._event_listeners:
            for callback in char_mgr._event_listeners[event_type]:
                callback(data)

    char_mgr.on = mock_on
    char_mgr.emit = mock_emit

    return char_mgr


@pytest.fixture
def inventory_manager(mock_gff, mock_character_manager):
    """Create InventoryManager with mocked dependencies"""
    mock_character_manager.gff = mock_gff

    with patch.object(InventoryManager, '_build_proficiency_mappings'), \
         patch.object(InventoryManager, '_update_proficiency_cache'):
        manager = InventoryManager(mock_character_manager)
        manager.check_item_id_exists = Mock(return_value=(True, []))
        return manager


def test_equip_item_emits_event(inventory_manager):
    """Test that equip_item() emits ITEM_EQUIPPED event"""
    item_data = {
        'BaseItem': 123,
        'StackSize': 1,
        'Identified': 1
    }

    success, warnings = inventory_manager.equip_item(item_data, 'head')

    assert success is True

    event_history = inventory_manager.character_manager._event_history
    assert len(event_history) >= 1

    event_type, event_data = event_history[-1]
    assert event_type == EventType.ITEM_EQUIPPED
    assert event_data['slot'] == 'head'


def test_equip_item_with_swap_emits_event(mock_gff, mock_character_manager):
    """Test that equipping an item when slot is occupied emits event with swapped item"""
    existing_item = {'BaseItem': 999, 'StackSize': 1}
    new_item = {'BaseItem': 123, 'StackSize': 1}

    mock_character_manager.gff_element = create_mock_gff_element([
        (0x0001, existing_item)
    ])
    mock_character_manager.gff = mock_gff
    mock_gff._data = {'Equip_ItemList': [existing_item]}

    with patch.object(InventoryManager, '_build_proficiency_mappings'), \
         patch.object(InventoryManager, '_update_proficiency_cache'):
        inventory_manager = InventoryManager(mock_character_manager)
        inventory_manager.check_item_id_exists = Mock(return_value=(True, []))
        inventory_manager.add_to_inventory = Mock()

        success, warnings = inventory_manager.equip_item(new_item, 'head')

        assert success is True

        event_history = inventory_manager.character_manager._event_history
        assert len(event_history) >= 1

        event_type, event_data = event_history[-1]
        assert event_type == EventType.ITEM_EQUIPPED
        assert event_data['slot'] == 'head'

        inventory_manager.add_to_inventory.assert_called_once_with(existing_item)


def test_unequip_item_emits_event(mock_gff, mock_character_manager):
    """Test that unequip_item() emits ITEM_UNEQUIPPED event"""
    item_data = {'BaseItem': 123, 'StackSize': 1}

    mock_character_manager.gff_element = create_mock_gff_element([
        (0x0001, item_data)
    ])
    mock_character_manager.gff = mock_gff
    mock_gff._data = {'Equip_ItemList': [item_data]}

    with patch.object(InventoryManager, '_build_proficiency_mappings'), \
         patch.object(InventoryManager, '_update_proficiency_cache'):
        inventory_manager = InventoryManager(mock_character_manager)
        inventory_manager.add_to_inventory = Mock()

        result = inventory_manager.unequip_item('head')

        assert result == item_data

        event_history = inventory_manager.character_manager._event_history
        assert len(event_history) >= 1

        event_type, event_data = event_history[-1]
        assert event_type == EventType.ITEM_UNEQUIPPED
        assert event_data['slot'] == 'head'


def test_unequip_empty_slot_no_event(inventory_manager, mock_gff):
    """Test that unequipping an empty slot does NOT emit event"""
    mock_gff.get.return_value = [None] * 14

    initial_event_count = len(inventory_manager.character_manager._event_history)

    result = inventory_manager.unequip_item('head')

    assert result is None
    assert len(inventory_manager.character_manager._event_history) == initial_event_count


def test_combat_manager_listens_to_equipment_events():
    """Test that CombatManager subscribes to ITEM_EQUIPPED and ITEM_UNEQUIPPED events"""
    char_mgr = Mock(spec=CharacterManager)
    event_subscriptions = []

    def mock_on(event_type, callback):
        event_subscriptions.append(event_type)

    char_mgr.on = mock_on
    char_mgr.get_manager = Mock(return_value=Mock())

    mock_gff = MagicMock(spec=GFFDataWrapper)
    mock_gff.get.return_value = 0
    char_mgr.gff = mock_gff
    char_mgr.rules_service = Mock()

    combat_mgr = CombatManager(char_mgr)

    assert EventType.ITEM_EQUIPPED in event_subscriptions
    assert EventType.ITEM_UNEQUIPPED in event_subscriptions


def test_equipment_event_invalidates_ac_cache():
    """Test that equipment changes invalidate AC cache in CombatManager"""
    char_mgr = Mock(spec=CharacterManager)
    listeners = {}

    def mock_on(event_type, callback):
        if event_type not in listeners:
            listeners[event_type] = []
        listeners[event_type].append(callback)

    def mock_emit(event_type, data):
        if event_type in listeners:
            for callback in listeners[event_type]:
                callback(data)

    char_mgr.on = mock_on
    char_mgr.emit = mock_emit
    char_mgr.get_manager = Mock(return_value=Mock())

    mock_gff = MagicMock(spec=GFFDataWrapper)
    mock_gff.get.return_value = 0
    char_mgr.gff = mock_gff
    char_mgr.rules_service = Mock()

    combat_mgr = CombatManager(char_mgr)

    combat_mgr._ac_cache = {"total": 15}
    assert combat_mgr._ac_cache is not None

    char_mgr.emit(EventType.ITEM_EQUIPPED, {'slot': 'chest', 'item': {}})

    assert combat_mgr._ac_cache is None


def test_multiple_equipment_changes_emit_multiple_events(inventory_manager, mock_gff):
    """Test that multiple equipment changes emit separate events"""
    item1 = {'BaseItem': 123}
    item2 = {'BaseItem': 456}

    mock_gff.get.return_value = [None] * 14
    inventory_manager.add_to_inventory = Mock()

    initial_count = len(inventory_manager.character_manager._event_history)

    inventory_manager.equip_item(item1, 'head')
    inventory_manager.equip_item(item2, 'chest')

    event_history = inventory_manager.character_manager._event_history
    new_events_count = len(event_history) - initial_count
    assert new_events_count == 2

    assert event_history[-2][0] == EventType.ITEM_EQUIPPED
    assert event_history[-1][0] == EventType.ITEM_EQUIPPED


def test_all_equipment_slots_emit_correct_indices(inventory_manager, mock_gff):
    """Test that all equipment slots map to correct indices in events"""
    expected_slots = {
        'head': 0,
        'chest': 1,
        'boots': 2,
        'gloves': 3,
        'right_hand': 4,
        'left_hand': 5,
        'cloak': 6,
        'left_ring': 7,
        'right_ring': 8,
        'neck': 9,
        'belt': 10,
        'arrows': 11,
        'bullets': 12,
        'bolts': 13,
    }

    mock_gff.get.return_value = [None] * 14
    item = {'BaseItem': 123}

    for slot, expected_index in expected_slots.items():
        initial_count = len(inventory_manager.character_manager._event_history)

        inventory_manager.equip_item(item, slot)

        assert len(inventory_manager.character_manager._event_history) > initial_count, f"Event should be emitted for slot {slot}"
        event_type, event_data = inventory_manager.character_manager._event_history[-1]
        assert event_type == EventType.ITEM_EQUIPPED, f"Last event should be ITEM_EQUIPPED for slot {slot}"
        assert event_data['slot_index'] == expected_index, f"Slot {slot} should map to index {expected_index}"
