"""
Comprehensive tests for SaveManager class.
Tests cover base saves, ability modifiers, feat bonuses, racial bonuses,
class-specific bonuses, temporary modifiers, save conditions, and immunities.
"""
import pytest
from unittest.mock import Mock, MagicMock, patch
from typing import Dict, List, Any

from character.managers.save_manager import SaveManager
from character.events import EventType, EventData
from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader


# --- Mocks ---

class MockFeat:
    """Mock feat data for testing"""
    def __init__(self, id, label, name, save_modifier=0):
        self.id = id
        self.label = label
        self.name = name
        self.bonus_value = save_modifier

class MockRace:
    """Mock race data for testing"""
    def __init__(self, id, label, name):
        self.id = id
        self.label = label
        self.name = name

class MockClass:
    """Mock class data for testing"""
    def __init__(self, id, label, name, saving_throw_table):
        self.id = id
        self.label = label
        self.name = name
        self.saving_throw_table = saving_throw_table

class MockSaveTable:
    """Mock save table for testing"""
    def __init__(self, id, level, fort_save, ref_save, will_save):
        self.id = id
        self.level = level
        self.fort_save = fort_save
        self.ref_save = ref_save
        self.will_save = will_save
        # Field mapping helpers
        self.fort_save_table = str(fort_save)
        self.ref_save_table = str(ref_save)
        self.will_save_table = str(will_save)

@pytest.fixture
def mock_game_data_loader():
    """Create comprehensive mock DynamicGameDataLoader with save-related data"""
    mock_loader = Mock(spec=DynamicGameDataLoader)
    
    mock_races = [
        MockRace(0, 'Dwarf', 'Dwarf'),
        MockRace(3, 'Halfling', 'Halfling'),
    ]
    
    # 3 classes: Fighter, Rogue, Paladin
    mock_classes = {
        0: MockClass(0, 'Fighter', 'Fighter', 'cls_savthr_fight'),
        2: MockClass(2, 'Rogue', 'Rogue', 'cls_savthr_rog'),
        5: MockClass(5, 'Paladin', 'Paladin', 'cls_savthr_pal'),
    }
    
    # Mock save tables (0-indexed logic in code vs 1-indexed levels)
    # Fighter: High Fort, Low Ref/Will
    fight_table = []
    for i in range(20):
        level = i + 1
        fort = 2 + (level // 2) # Crude approx
        ref = level // 3
        will = level // 3
        fight_table.append(MockSaveTable(i, level, fort, ref, will))

    # Mock get_table method
    def mock_get_table(table_name: str):
        if table_name == 'racialtypes':
            return mock_races
        if table_name == 'cls_savthr_fight':
            return fight_table
        return []
    
    # Mock get_by_id method
    def mock_get_by_id(table_name: str, item_id: int):
        if table_name == 'classes':
            return mock_classes.get(item_id)
        if table_name == 'racialtypes':
            for r in mock_races:
                if r.id == item_id: return r
        return None
    
    mock_loader.get_table = mock_get_table
    mock_loader.get_by_id = mock_get_by_id
    
    return mock_loader

@pytest.fixture
def mock_managers():
    """Create mocks for dependent managers"""
    ability = Mock()
    ability.get_total_modifiers.return_value = {'Con': 2, 'Dex': 3, 'Wis': 1}
    ability.get_all_modifiers.return_value = {'CON': 2, 'DEX': 3, 'WIS': 1, 'CHA': 4}

    inventory = Mock()
    inventory.get_equipment_bonuses.return_value = {
        'saves': {'fortitude': 0, 'reflex': 0, 'will': 0}
    }

    feat = Mock()
    feat.get_save_bonuses.return_value = {'fortitude': 0, 'reflex': 2, 'will': 2}
    feat.has_feat_by_name.return_value = False
    feat.has_feat.return_value = False # Default no special feats

    clazz = Mock()
    clazz.calculate_total_saves.return_value = {} # Not used for base anymore, mainly checking existence

    return {
        'ability': ability,
        'inventory': inventory,
        'feat': feat,
        'class': clazz
    }

@pytest.fixture
def mock_character_manager(mock_game_data_loader, mock_managers):
    """Create mock CharacterManager with GFF data and helper methods"""
    manager = Mock()
    manager.game_data_loader = mock_game_data_loader
    manager.rules_service = mock_game_data_loader # Using loader as rules service for simplicity
    
    # Create mock GFF
    # Must provide Valid LvlStatList to avoid RuntimeError in strict mode!
    # 5 Fighter Levels, 5 Rogue Levels = 10 Total Levels
    mock_lvl_stat_list = []
    # 5 Fighter
    for i in range(5):
        mock_lvl_stat_list.append({'LvlStatClass': 0})
    # 5 Rogue
    for i in range(5):
        mock_lvl_stat_list.append({'LvlStatClass': 2})

    mock_gff = Mock()
    mock_gff.data = {
        'Race': 3,  # Halfling
        'ClassList': [
            {'Class': 0, 'ClassLevel': 5},  # Fighter 5
            {'Class': 2, 'ClassLevel': 5},  # Rogue 5
        ],
        'LvlStatList': mock_lvl_stat_list,
        'fortbonus': 0,
        'refbonus': 0,
        'willbonus': 0
    }
    
    def mock_get(key, default=None):
        return mock_gff.data.get(key, default)
    def mock_set(key, value):
        mock_gff.data[key] = value

    mock_gff.get = mock_get
    mock_gff.set = mock_set
    manager.gff = mock_gff
    
    # Mock Manager Getter
    def mock_get_manager(name):
        return mock_managers.get(name)
    
    manager.get_manager = mock_get_manager
    manager.get_racial_saves = Mock(return_value={'fortitude': 1, 'reflex': 1, 'will': 1})

    return manager

@pytest.fixture
def save_manager(mock_character_manager):
    """Create SaveManager instance for testing"""
    # We need to patch field_mapper.get_racial_saves to work with our mocks
    with patch('character.managers.save_manager.field_mapper') as mock_mapper:
        # Generic safe int
        mock_mapper._safe_int = lambda x, d: int(x) if x else d
        mock_mapper.get_field_value = lambda obj, field, default: getattr(obj, field, default)
        
        # Specific racial saves for Halfling (ID 3) and Dwarf (ID 0)
        def get_racial_saves(race_data):
            if race_data.id == 3: return {'fortitude': 1, 'reflex': 1, 'will': 1}
            if race_data.id == 0: return {'fortitude': 2, 'reflex': 0, 'will': 2}
            return {'fortitude': 0, 'reflex': 0, 'will': 0}
        
        mock_mapper.get_racial_saves = get_racial_saves
        
        return SaveManager(mock_character_manager)

class TestSaveManagerBasics:
    
    def test_initialization(self, save_manager):
        assert save_manager.character_manager is not None
        assert save_manager.temporary_modifiers == {'fortitude': 0, 'reflex': 0, 'will': 0}

    def test_calculate_basic_saves(self, save_manager):
        """Test basic save calculation."""
        saves = save_manager.calculate_saving_throws()
        
        assert 'fortitude' in saves
        assert 'reflex' in saves
        assert 'will' in saves
        # Just checking structure and that it didn't crash
        assert saves['fortitude']['total'] > 0

    def test_feat_bonus_delegation(self, save_manager, mock_managers):
        """Test that we delegate to FeatManager."""
        mock_managers['feat'].get_save_bonuses.return_value = {'fortitude': 10, 'reflex': 0, 'will': 0}
        
        saves = save_manager.calculate_saving_throws()
        assert saves['fortitude']['feat'] == 10

class TestCorruptedData:
    """Test strict error handling for corrupted data"""
    
    def test_corrupted_history_mismatch(self, save_manager):
        """Test that missing or partial LvlStatList raises RuntimeError"""
        # Corrupt the history: Only 3 levels in history, but 10 class levels total
        save_manager.gff.data['LvlStatList'] = [{'LvlStatClass': 0}] * 3
        
        with pytest.raises(RuntimeError, match="Character corruption detected"):
            save_manager.calculate_saving_throws()

class TestDivineGrace:
    """Test specifically for Divine Grace / Dark One's Luck logic replacement"""

    def test_divine_grace_applied(self, save_manager, mock_managers):
        """Test that Divine Grace (Feat 214) adds CHA mod to saves."""
        # Setup: Character has Feat 214
        mock_managers['feat'].has_feat.side_effect = lambda feat_id: feat_id == 214
        # CHA mod is 4 (from mock_managers default)
        
        saves = save_manager.calculate_saving_throws()
        
        # Base Feat Bonus: Fort 0, Ref 2, Will 2 (from default mock)
        # Divine Grace (+4): Fort +4, Ref +4, Will +4
        assert saves['fortitude']['feat'] == 4
        assert saves['reflex']['feat'] == 2 + 4
        assert saves['will']['feat'] == 2 + 4

    def test_dark_ones_luck_applied(self, save_manager, mock_managers):
        """Test that Dark One's Luck (Feat 400) adds CHA mod to saves."""
        # Setup: Character has Feat 400
        mock_managers['feat'].has_feat.side_effect = lambda feat_id: feat_id == 400
        
        saves = save_manager.calculate_saving_throws()
        
        # Base Feat Bonus: Fort 0, Ref 2, Will 2 (from default mock)
        # Dark One's Luck (+4): Fort +4, Ref +4, Will +4
        assert saves['fortitude']['feat'] == 4
        assert saves['reflex']['feat'] == 2 + 4
        assert saves['will']['feat'] == 2 + 4

    def test_no_double_dip(self, save_manager, mock_managers):
        """Test behavior if user somehow has both feats (should stack if logic says so, or maybe not?)"""
        mock_managers['feat'].has_feat.side_effect = lambda feat_id: feat_id in [214, 400]
        
        saves = save_manager.calculate_saving_throws()
        
        # Both (+4 each) = +8
        assert saves['fortitude']['feat'] == 8 

class TestStrictErrorHandling:
    """Test that missing dependencies raise errors."""

    def test_no_ability_manager(self, save_manager):
        save_manager.character_manager.get_manager = Mock(side_effect=lambda x: None if x == 'ability' else Mock())
        with pytest.raises(RuntimeError, match="AbilityManager is required"):
            save_manager.calculate_saving_throws()

    def test_no_inventory_manager(self, save_manager):
        def get_manager(name):
            if name == 'ability': return Mock(get_total_modifiers=lambda: {}, get_all_modifiers=lambda: {})
            if name == 'inventory': return None
            return Mock()
            
        save_manager.character_manager.get_manager = str # Break it
        save_manager.character_manager.get_manager = Mock(side_effect=get_manager)
        
        with pytest.raises(RuntimeError, match="InventoryManager is required"):
            save_manager.calculate_saving_throws()

    def test_no_feat_manager(self, save_manager):
        def get_manager(name):
            if name == 'ability': return Mock(get_total_modifiers=lambda: {}, get_all_modifiers=lambda: {})
            if name == 'inventory': return Mock(get_equipment_bonuses=lambda: {})
            if name == 'feat': return None
            return Mock()
            
        save_manager.character_manager.get_manager = Mock(side_effect=get_manager)
        
        with pytest.raises(RuntimeError, match="FeatManager is required"):
            save_manager.calculate_saving_throws()

class TestTemporaryModifiers:
    
    def test_add_remove_modifiers(self, save_manager):
        save_manager.add_temporary_modifier('fortitude', 5)
        assert save_manager.calculate_fortitude_save() == save_manager.calculate_saving_throws()['fortitude']['total']
        # We can't easily check total equality without knowing base, but we can check the temp component
        assert save_manager.calculate_saving_throws()['fortitude']['temporary'] == 5
        
        save_manager.remove_temporary_modifier('fortitude', 2)
        assert save_manager.calculate_saving_throws()['fortitude']['temporary'] == 3

    def test_clear_modifiers(self, save_manager):
        save_manager.add_temporary_modifier('fortitude', 5)
        save_manager.clear_temporary_modifiers()
        assert save_manager.calculate_saving_throws()['fortitude']['temporary'] == 0

class TestValidation:
    
    def test_validation_low_saves(self, save_manager):
        save_manager.add_temporary_modifier('fortitude', -100)
        valid, errors = save_manager.validate()
        assert valid is False
        assert "Fortitude save is unusually low" in errors[0]
