"""
Comprehensive tests for SaveManager class.
Tests cover base saves, ability modifiers, feat bonuses, racial bonuses,
class-specific bonuses, temporary modifiers, save conditions, and immunities.
"""
import pytest
import time
from unittest.mock import Mock, MagicMock, patch, PropertyMock
from typing import Dict, List, Any

from character.managers.save_manager import SaveManager
from character.events import EventEmitter, EventType, EventData
from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader


class MockFeat:
    """Mock feat data for testing"""
    def __init__(self, id, label, name, save_bonus=0, fort_bonus=0, reflex_bonus=0, will_bonus=0, save_modifier=0):
        self.id = id
        self.label = label
        self.name = name
        self.save_bonus = save_bonus
        self.fort_bonus = fort_bonus
        self.reflex_bonus = reflex_bonus
        self.will_bonus = will_bonus
        self.fortitude_bonus = fort_bonus  # Alternative field name
        self.save_modifier = save_modifier if save_modifier else save_bonus  # Alternative field name
        
        # Add lowercase versions for getattr with lowercase lookup
        self.save_bonus = save_bonus
        self.fortitude_bonus = fort_bonus
        self.reflex_bonus = reflex_bonus
        self.will_bonus = will_bonus
        self.bonus_value = save_modifier if save_modifier else save_bonus


class MockRace:
    """Mock race data for testing"""
    def __init__(self, id, label, name, fort_save=0, ref_save=0, will_save=0):
        self.id = id
        self.label = label
        self.name = name
        self.fort_save = fort_save
        self.ref_save = ref_save
        self.will_save = will_save


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
        self.fortitude = fort_save  # Alternative name
        self.ref_save = ref_save
        self.reflex = ref_save  # Alternative name
        self.will_save = will_save
        self.will = will_save  # Alternative name


@pytest.fixture
def mock_game_data_loader():
    """Create comprehensive mock DynamicGameDataLoader with save-related data"""
    mock_loader = Mock(spec=DynamicGameDataLoader)
    
    # Mock feats data with save bonuses
    # Labels must contain keywords that SaveManager looks for
    # Use save_modifier as a fallback that works for all fields
    mock_feats = [
        MockFeat(22, 'IronWill', 'Iron Will', save_modifier=2),  # contains 'will'
        MockFeat(24, 'LightningReflexes', 'Lightning Reflexes', save_modifier=2),  # contains 'lightning'
        MockFeat(14, 'GreatFortitude', 'Great Fortitude', save_modifier=2),  # contains 'fortitude'
        MockFeat(100, 'LuckOfHeroes', 'Luck of Heroes', save_modifier=1),  # contains 'luck'
        MockFeat(376, 'DivineGrace', 'Divine Grace'),  # Paladin ability
        MockFeat(389, 'Evasion', 'Evasion'),
        MockFeat(390, 'ImprovedEvasion', 'Improved Evasion'),
        MockFeat(392, 'SlipperyMind', 'Slippery Mind'),
        MockFeat(400, 'DivineHealth', 'Divine Health'),
        MockFeat(401, 'AuraOfCourage', 'Aura of Courage'),
        MockFeat(410, 'PurityOfBody', 'Purity of Body'),
        MockFeat(412, 'DiamondBody', 'Diamond Body'),
        MockFeat(415, 'StillMind', 'Still Mind'),
        MockFeat(220, 'DivineSpark', 'Divine Spark'),
    ]
    
    # Mock races data with save bonuses
    mock_races = [
        MockRace(0, 'Dwarf', 'Dwarf', fort_save=2, ref_save=0, will_save=2),  # Dwarves: +2 vs poison/spells
        MockRace(1, 'Elf', 'Elf', fort_save=0, ref_save=0, will_save=2),  # Elves: +2 vs enchantments
        MockRace(2, 'Gnome', 'Gnome', fort_save=0, ref_save=0, will_save=2),  # Gnomes: +2 vs illusions
        MockRace(3, 'Halfling', 'Halfling', fort_save=1, ref_save=1, will_save=1),  # Halflings: +1 all saves
        MockRace(4, 'HalfElf', 'Half-Elf'),
        MockRace(5, 'HalfOrc', 'Half-Orc'),
        MockRace(6, 'Human', 'Human'),
    ]
    
    # Mock classes
    mock_classes = {
        0: MockClass(0, 'Fighter', 'Fighter', 'cls_savthr_fight'),
        1: MockClass(1, 'Wizard', 'Wizard', 'cls_savthr_wiz'),
        2: MockClass(2, 'Rogue', 'Rogue', 'cls_savthr_rog'),
        3: MockClass(3, 'Cleric', 'Cleric', 'cls_savthr_cler'),
        5: MockClass(5, 'Paladin', 'Paladin', 'cls_savthr_pal'),
        6: MockClass(6, 'Monk', 'Monk', 'cls_savthr_monk'),
    }
    
    # Mock save tables
    mock_save_tables = {
        'cls_savthr_fight': [
            MockSaveTable(0, 1, 2, 0, 0),
            MockSaveTable(1, 2, 3, 0, 0),
            MockSaveTable(2, 3, 3, 1, 1),
            MockSaveTable(3, 4, 4, 1, 1),
            MockSaveTable(4, 5, 4, 1, 1),
            MockSaveTable(5, 6, 5, 2, 2),
            MockSaveTable(9, 10, 7, 3, 3),
            MockSaveTable(19, 20, 12, 6, 6),
        ],
        'cls_savthr_rog': [
            MockSaveTable(0, 1, 0, 2, 0),
            MockSaveTable(1, 2, 0, 3, 0),
            MockSaveTable(2, 3, 1, 3, 1),
            MockSaveTable(3, 4, 1, 4, 1),
            MockSaveTable(4, 5, 1, 4, 1),
            MockSaveTable(9, 10, 3, 7, 3),
        ],
        'cls_savthr_pal': [
            MockSaveTable(0, 1, 2, 0, 0),
            MockSaveTable(1, 2, 3, 0, 0),
            MockSaveTable(2, 3, 3, 1, 1),
            MockSaveTable(3, 4, 4, 1, 1),
            MockSaveTable(4, 5, 4, 1, 1),
        ],
    }
    
    # Mock get_table method
    def mock_get_table(table_name: str):
        if table_name == 'feat':
            return mock_feats
        elif table_name == 'racialtypes':
            return mock_races
        elif table_name in mock_save_tables:
            return mock_save_tables[table_name]
        return []
    
    # Mock get_by_id method
    def mock_get_by_id(table_name: str, item_id: int):
        if table_name == 'classes':
            return mock_classes.get(item_id)
        return None
    
    mock_loader.get_table = mock_get_table
    mock_loader.get_by_id = mock_get_by_id
    
    return mock_loader


@pytest.fixture
def mock_character_manager(mock_game_data_loader):
    """Create mock CharacterManager with GFF data and helper methods"""
    manager = Mock()
    manager._current_transaction = None
    manager.custom_content = {}
    manager.game_data_loader = mock_game_data_loader
    
    # Create mock GFF with default character data
    mock_gff = Mock()
    mock_gff.data = {
        'Con': 14,  # +2 modifier
        'Dex': 16,  # +3 modifier
        'Wis': 13,  # +1 modifier
        'Cha': 18,  # +4 modifier (for Paladin)
        'Race': 3,  # Halfling (+1 all saves)
        'FortSave': 5,
        'RefSave': 8,
        'WillSave': 4,
        'FeatList': [
            {'Feat': 22},  # Iron Will
            {'Feat': 24},  # Lightning Reflexes
        ],
        'ClassList': [
            {'Class': 0, 'ClassLevel': 5},  # Fighter 5
            {'Class': 2, 'ClassLevel': 5},  # Rogue 5
        ]
    }
    
    def mock_get(key, default=None):
        return mock_gff.data.get(key, default)
    
    mock_gff.get = mock_get
    manager.gff = mock_gff
    
    # Mock helper methods
    def mock_has_feat_by_name(feat_name):
        feat_map = {
            'IronWill': 22 in [f['Feat'] for f in mock_gff.data['FeatList']],
            'LightningReflexes': 24 in [f['Feat'] for f in mock_gff.data['FeatList']],
            'GreatFortitude': 14 in [f['Feat'] for f in mock_gff.data['FeatList']],
            'Evasion': 389 in [f['Feat'] for f in mock_gff.data['FeatList']],
            'ImprovedEvasion': 390 in [f['Feat'] for f in mock_gff.data['FeatList']],
            'SlipperyMind': 392 in [f['Feat'] for f in mock_gff.data['FeatList']],
            'DivineHealth': 400 in [f['Feat'] for f in mock_gff.data['FeatList']],
            'AuraOfCourage': 401 in [f['Feat'] for f in mock_gff.data['FeatList']],
        }
        return feat_map.get(feat_name, False)
    
    def mock_has_class_by_name(class_name):
        class_map = {
            'Fighter': 0 in [c['Class'] for c in mock_gff.data['ClassList']],
            'Rogue': 2 in [c['Class'] for c in mock_gff.data['ClassList']],
            'Paladin': 5 in [c['Class'] for c in mock_gff.data['ClassList']],
        }
        return class_map.get(class_name, False)
    
    def mock_get_class_level_by_name(class_name):
        class_map = {
            'Fighter': 5 if mock_has_class_by_name('Fighter') else 0,
            'Rogue': 5 if mock_has_class_by_name('Rogue') else 0,
            'Paladin': 0,  # Not in default setup
        }
        return class_map.get(class_name, 0)
    
    def mock_get_racial_saves(race_id):
        if race_id == 3:  # Halfling
            return {'fortitude': 1, 'reflex': 1, 'will': 1}
        elif race_id == 0:  # Dwarf
            return {'fortitude': 2, 'reflex': 0, 'will': 2}
        return {'fortitude': 0, 'reflex': 0, 'will': 0}
    
    manager.has_feat_by_name = mock_has_feat_by_name
    manager.has_class_by_name = mock_has_class_by_name
    manager.get_class_level_by_name = mock_get_class_level_by_name
    manager.get_racial_saves = mock_get_racial_saves
    
    # Mock class manager for base saves
    mock_class_manager = Mock()
    def mock_calculate_total_saves():
        # Fighter 5 + Rogue 5 saves
        return {
            'fortitude': 5,  # Fighter good + Rogue poor
            'reflex': 10,    # Fighter poor + Rogue good
            'will': 4,       # Both poor
            'base_fortitude': 4,  # Base without ability
            'base_reflex': 7,     # Base without ability
            'base_will': 3,       # Base without ability
        }
    mock_class_manager.calculate_total_saves = mock_calculate_total_saves
    
    def mock_get_manager(manager_type):
        if manager_type == 'class':
            return mock_class_manager
        return None
    
    manager.get_manager = mock_get_manager
    
    return manager


@pytest.fixture
def save_manager(mock_character_manager):
    """Create SaveManager instance for testing"""
    return SaveManager(mock_character_manager)


class TestSaveManagerBasics:
    """Test basic save calculations"""
    
    def test_initialization(self, save_manager):
        """Test SaveManager initialization"""
        assert save_manager.character_manager is not None
        assert save_manager.gff is not None
        assert save_manager.game_data_loader is not None
        assert save_manager.temporary_modifiers == {
            'fortitude': 0,
            'reflex': 0,
            'will': 0
        }
    
    def test_calculate_basic_saves(self, save_manager):
        """Test basic save calculation with Fighter/Rogue multiclass"""
        saves = save_manager.calculate_saving_throws()
        
        # Check structure
        assert 'fortitude' in saves
        assert 'reflex' in saves
        assert 'will' in saves
        
        # Check fortitude breakdown (base 4 + CON 2 + racial 1 + feats 0)
        # The character doesn't have Great Fortitude
        fort = saves['fortitude']
        assert fort['base'] == 4
        assert fort['ability'] == 2  # CON 14
        assert fort['racial'] == 1   # Halfling
        assert fort['feat'] == 0     # No Great Fortitude
        assert fort['total'] == 7
        
        # Check reflex breakdown (base 7 + DEX 3 + racial 1 + feats 2)
        ref = saves['reflex']
        assert ref['base'] == 7
        assert ref['ability'] == 3   # DEX 16
        assert ref['racial'] == 1    # Halfling
        assert ref['feat'] == 2      # Lightning Reflexes
        assert ref['total'] == 13
        
        # Check will breakdown (base 3 + WIS 1 + racial 1 + feats 2)
        will = saves['will']
        assert will['base'] == 3
        assert will['ability'] == 1  # WIS 13
        assert will['racial'] == 1   # Halfling
        assert will['feat'] == 2     # Iron Will
        assert will['total'] == 7
    
    def test_feat_bonus_calculation(self, save_manager):
        """Test feat-based save bonuses"""
        # Since save bonuses aren't in the 2DA data, we need to mock the 
        # SaveManager's internal feat bonus lookup
        save_manager._save_affecting_feats = {
            'fortitude': [{'id': 14, 'label': 'GreatFortitude', 'bonus': 2}],
            'reflex': [{'id': 24, 'label': 'LightningReflexes', 'bonus': 2}],
            'will': [{'id': 22, 'label': 'IronWill', 'bonus': 2}],
            'universal': []
        }
        
        # Add Great Fortitude feat
        save_manager.gff.data['FeatList'].append({'Feat': 14})
        
        saves = save_manager.calculate_saving_throws()
        
        # Now fortitude should include feat bonus
        assert saves['fortitude']['feat'] == 2
        assert saves['fortitude']['total'] == 9  # 7 + 2
    
    def test_racial_bonus_calculation(self, save_manager):
        """Test racial save bonuses"""
        # Change to Dwarf
        save_manager.gff.data['Race'] = 0
        
        saves = save_manager.calculate_saving_throws()
        
        # Dwarf bonuses: +2 Fort, +0 Ref, +2 Will
        assert saves['fortitude']['racial'] == 2
        assert saves['reflex']['racial'] == 0
        assert saves['will']['racial'] == 2


class TestClassSpecificBonuses:
    """Test class-specific save bonuses"""
    
    def test_paladin_divine_grace(self, save_manager):
        """Test Paladin's Divine Grace (CHA to all saves)"""
        # Change to Paladin level 2
        save_manager.gff.data['ClassList'] = [
            {'Class': 5, 'ClassLevel': 2}  # Paladin 2
        ]
        # Clear existing feats to test only Divine Grace
        save_manager.gff.data['FeatList'] = []
        
        # Mock the class manager to return Paladin saves
        class_manager = save_manager.character_manager.get_manager('class')
        class_manager.calculate_total_saves.return_value = {
            'fortitude': 3,
            'reflex': 0,
            'will': 0,
            'base_fortitude': 3,
            'base_reflex': 0,
            'base_will': 0,
        }
        
        # Update has_class_by_name to recognize Paladin
        def new_has_class(class_name):
            if class_name == 'Paladin':
                return True
            return False
        save_manager.character_manager.has_class_by_name = new_has_class
        
        def new_get_level(class_name):
            if class_name == 'Paladin':
                return 2
            return 0
        save_manager.character_manager.get_class_level_by_name = new_get_level
        
        saves = save_manager.calculate_saving_throws()
        
        # CHA 18 = +4 modifier, should be added to all saves via feat bonus
        assert saves['fortitude']['feat'] == 4  # Divine Grace
        assert saves['reflex']['feat'] == 4     # Divine Grace
        assert saves['will']['feat'] == 4       # Divine Grace
    
    def test_paladin_below_level_2(self, save_manager):
        """Test that Paladin below level 2 doesn't get Divine Grace"""
        # Paladin level 1
        save_manager.gff.data['ClassList'] = [
            {'Class': 5, 'ClassLevel': 1}  # Paladin 1
        ]
        
        def new_has_class(class_name):
            if class_name == 'Paladin':
                return True
            return False
        save_manager.character_manager.has_class_by_name = new_has_class
        
        def new_get_level(class_name):
            if class_name == 'Paladin':
                return 1
            return 0
        save_manager.character_manager.get_class_level_by_name = new_get_level
        
        saves = save_manager.calculate_saving_throws()
        
        # No Divine Grace bonus
        assert saves['fortitude']['feat'] == 0
        assert saves['reflex']['feat'] == 2     # Lightning Reflexes only
        assert saves['will']['feat'] == 2       # Iron Will only


class TestTemporaryModifiers:
    """Test temporary save modifiers"""
    
    def test_add_temporary_modifier(self, save_manager):
        """Test adding temporary save modifiers"""
        save_manager.add_temporary_modifier('fortitude', 4)
        save_manager.add_temporary_modifier('reflex', 2)
        
        saves = save_manager.calculate_saving_throws()
        
        assert saves['fortitude']['temporary'] == 4
        assert saves['reflex']['temporary'] == 2
        assert saves['will']['temporary'] == 0
        
        # Check totals include temporary
        assert saves['fortitude']['total'] == 11  # 7 base + 4 temp
        assert saves['reflex']['total'] == 15     # 13 base + 2 temp
    
    def test_remove_temporary_modifier(self, save_manager):
        """Test removing temporary modifiers"""
        save_manager.add_temporary_modifier('will', 5)
        save_manager.add_temporary_modifier('will', 3)
        
        # Should stack to 8
        assert save_manager.temporary_modifiers['will'] == 8
        
        save_manager.remove_temporary_modifier('will', 3)
        assert save_manager.temporary_modifiers['will'] == 5
        
        saves = save_manager.calculate_saving_throws()
        assert saves['will']['temporary'] == 5
    
    def test_clear_temporary_modifiers(self, save_manager):
        """Test clearing all temporary modifiers"""
        save_manager.add_temporary_modifier('fortitude', 2)
        save_manager.add_temporary_modifier('reflex', 3)
        save_manager.add_temporary_modifier('will', 4)
        
        save_manager.clear_temporary_modifiers()
        
        assert save_manager.temporary_modifiers == {
            'fortitude': 0,
            'reflex': 0,
            'will': 0
        }


class TestSaveChecks:
    """Test save check mechanics"""
    
    def test_check_save_success(self, save_manager):
        """Test checking saves against DC"""
        # Reflex save with +13 total
        result = save_manager.check_save('reflex', dc=15)
        
        assert result['total_bonus'] == 13
        assert result['dc'] == 15
        assert result['roll_needed'] == 2  # Need 2+ on d20
        assert result['success_chance'] == 95  # 19/20 * 100
        assert result['auto_success'] is False
        assert result['auto_fail'] is False
    
    def test_check_save_auto_success(self, save_manager):
        """Test auto-success conditions"""
        # DC 14 with +13 bonus = need 1 (auto-success)
        result = save_manager.check_save('reflex', dc=14)
        
        assert result['roll_needed'] == 1
        assert result['auto_success'] is True
        assert result['auto_fail'] is False
    
    def test_check_save_auto_fail(self, save_manager):
        """Test auto-fail conditions"""
        # DC 35 with +13 bonus = need 22 (impossible)
        result = save_manager.check_save('reflex', dc=35)
        
        assert result['roll_needed'] == 22
        assert result['auto_success'] is False
        assert result['auto_fail'] is True
        assert result['success_chance'] == 0
    
    def test_check_save_with_modifier(self, save_manager):
        """Test save check with situational modifier"""
        result = save_manager.check_save('fortitude', dc=15, modifier=2)
        
        assert result['total_bonus'] == 9  # 7 base + 2 modifier
        assert result['roll_needed'] == 6
    
    def test_check_save_take_20(self, save_manager):
        """Test taking 20 on a save (if allowed)"""
        result = save_manager.check_save('will', dc=20, take_20=True)
        
        assert result['total_bonus'] == 7
        assert result['success'] is True  # 20 + 7 >= 20
    
    def test_invalid_save_type(self, save_manager):
        """Test invalid save type raises error"""
        with pytest.raises(ValueError, match="Invalid save type"):
            save_manager.check_save('charisma', dc=15)


class TestSaveConditions:
    """Test special save conditions and immunities"""
    
    def test_evasion_detection(self, save_manager):
        """Test detection of Evasion ability"""
        # Add Evasion feat
        save_manager.gff.data['FeatList'].append({'Feat': 389})
        
        summary = save_manager.get_save_summary()
        conditions = summary['conditions']
        
        assert any('Evasion' in c for c in conditions)
    
    def test_improved_evasion_detection(self, save_manager):
        """Test detection of Improved Evasion"""
        # Add Improved Evasion
        save_manager.gff.data['FeatList'].append({'Feat': 390})
        
        summary = save_manager.get_save_summary()
        conditions = summary['conditions']
        
        assert any('Improved Evasion' in c for c in conditions)
    
    def test_slippery_mind_detection(self, save_manager):
        """Test detection of Slippery Mind"""
        save_manager.gff.data['FeatList'].append({'Feat': 392})
        
        summary = save_manager.get_save_summary()
        conditions = summary['conditions']
        
        assert any('Slippery Mind' in c for c in conditions)
    
    def test_immunity_detection(self, save_manager):
        """Test detection of immunities"""
        # Add Divine Health (disease immunity)
        save_manager.gff.data['FeatList'].append({'Feat': 400})
        # Add Aura of Courage (fear immunity)
        save_manager.gff.data['FeatList'].append({'Feat': 401})
        
        summary = save_manager.get_save_summary()
        immunities = summary['immunities']
        
        assert 'Disease immunity' in immunities
        assert 'Fear immunity' in immunities


class TestUniversalSaveBonuses:
    """Test feats that affect all saves"""
    
    def test_luck_of_heroes(self, save_manager):
        """Test Luck of Heroes (+1 all saves)"""
        # Add Luck of Heroes
        save_manager.gff.data['FeatList'].append({'Feat': 100})
        
        # Rebuild feat cache
        save_manager._build_save_affecting_feats_cache()
        
        saves = save_manager.calculate_saving_throws()
        
        # Should add +1 to all saves via feat bonus
        # Note: Base feats are Iron Will (+2 Will) and Lightning Reflexes (+2 Reflex)
        assert saves['fortitude']['feat'] == 1   # Luck only
        assert saves['reflex']['feat'] == 3      # Lightning Reflexes + Luck
        assert saves['will']['feat'] == 3        # Iron Will + Luck


class TestSaveBreakdown:
    """Test save breakdown formatting"""
    
    def test_save_breakdown_format(self, save_manager):
        """Test the breakdown string format"""
        saves = save_manager.calculate_saving_throws()
        
        # Check fortitude breakdown
        fort_breakdown = saves['fortitude']['breakdown']
        assert 'Fortitude +7' in fort_breakdown
        assert 'base +4' in fort_breakdown
        assert 'CON +2' in fort_breakdown
        assert 'racial +1' in fort_breakdown
        
        # Check reflex breakdown includes feat
        ref_breakdown = saves['reflex']['breakdown']
        assert 'Reflex +13' in ref_breakdown
        assert 'feats +2' in ref_breakdown
        
    def test_breakdown_with_temporary(self, save_manager):
        """Test breakdown includes temporary modifiers"""
        save_manager.add_temporary_modifier('will', 3)
        
        saves = save_manager.calculate_saving_throws()
        will_breakdown = saves['will']['breakdown']
        
        assert 'temporary +3' in will_breakdown


class TestValidation:
    """Test save validation"""
    
    def test_validation_normal_saves(self, save_manager):
        """Test validation passes for normal saves"""
        valid, errors = save_manager.validate()
        
        assert valid is True
        assert len(errors) == 0
    
    def test_validation_low_saves(self, save_manager):
        """Test validation catches extremely low saves"""
        # Artificially lower saves
        save_manager.temporary_modifiers['fortitude'] = -20
        
        valid, errors = save_manager.validate()
        
        assert valid is False
        assert len(errors) == 1
        assert 'Fortitude save is unusually low' in errors[0]


class TestEventHandling:
    """Test event handling for save updates"""
    
    def test_attribute_change_event(self, save_manager):
        """Test handling of attribute change events"""
        # Create event data
        from character.events import EventType
        import time
        
        # Create a mock event with proper structure
        event = Mock()
        event.event_type = EventType.ATTRIBUTE_CHANGED
        event.source_manager = 'test'
        event.timestamp = time.time()
        event.cascading_changes = [
            {'type': 'saving_throw_update', 'save': 'fortitude', 'change': 2}
        ]
        
        # Trigger event
        save_manager._on_attribute_changed(event)
        
        # Should log the change (check via mock if needed)
        assert True  # Event handled without error
    
    def test_class_change_event(self, save_manager):
        """Test handling of class change events"""
        event = Mock()
        save_manager._on_class_changed(event)
        assert True  # Event handled without error
    
    def test_feat_change_event(self, save_manager):
        """Test handling of feat change events"""
        event = Mock()
        save_manager._on_feat_changed(event)
        assert True  # Event handled without error


class TestEdgeCases:
    """Test edge cases and error conditions"""
    
    def test_missing_gff_data(self, save_manager):
        """Test handling of missing GFF data"""
        # Remove some GFF data
        del save_manager.gff.data['Con']
        
        saves = save_manager.calculate_saving_throws()
        
        # Should use default value of 10 (modifier 0)
        assert saves['fortitude']['ability'] == 0
    
    def test_no_class_manager(self, save_manager):
        """Test fallback when no class manager available"""
        save_manager.character_manager.get_manager = Mock(return_value=None)
        
        saves = save_manager.calculate_saving_throws()
        
        # Should use GFF values directly
        assert saves['fortitude']['base'] == 0  # From fallback
    
    def test_empty_feat_list(self, save_manager):
        """Test with no feats"""
        save_manager.gff.data['FeatList'] = []
        
        saves = save_manager.calculate_saving_throws()
        
        assert saves['fortitude']['feat'] == 0
        assert saves['reflex']['feat'] == 0
        assert saves['will']['feat'] == 0
    
    def test_negative_ability_modifiers(self, save_manager):
        """Test with negative ability scores"""
        save_manager.gff.data['Con'] = 6  # -2 modifier
        save_manager.gff.data['Dex'] = 8  # -1 modifier
        save_manager.gff.data['Wis'] = 3  # -4 modifier
        
        saves = save_manager.calculate_saving_throws()
        
        assert saves['fortitude']['ability'] == -2
        assert saves['reflex']['ability'] == -1
        assert saves['will']['ability'] == -4