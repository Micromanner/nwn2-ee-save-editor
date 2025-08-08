"""
Comprehensive tests for RaceManager class.
Tests cover data-driven race changes, ability modifiers, racial feats, size/speed changes,
validation, and integration with other managers.
"""
import pytest
import time
from unittest.mock import Mock, MagicMock, patch
from typing import Dict, Any

from character.managers.race_manager import RaceManager, RaceChangedEvent
from character.events import EventType, EventData
from character.character_manager import CharacterManager, GFFDataWrapper


@pytest.fixture
def mock_game_data_loader():
    """Create a mock DynamicGameDataLoader with realistic race data"""
    mock_loader = Mock()
    
    # Sample race data with various field naming conventions
    race_data = {
        0: Mock(  # Human
            label="Human",
            name="Human",
            str_adjust=0, dex_adjust=0, con_adjust=0,
            int_adjust=0, wis_adjust=0, cha_adjust=0,
            creature_size=4,  # Medium
            movement_rate=30,
            player_race=True,
            favored_class=0,  # Fighter
            Feat0=None, Feat1=None  # No racial feats
        ),
        1: Mock(  # Elf
            label="Elf",
            name="Elf", 
            str_adjust=0, dex_adjust=2, con_adjust=-2,
            int_adjust=0, wis_adjust=0, cha_adjust=0,
            creature_size=4,  # Medium
            movement_rate=30,
            player_race=True,
            favored_class=1,  # Wizard
            Feat0=1,  # Keen Senses
            Feat1=2,  # Weapon Proficiency
            Feat2=None
        ),
        2: Mock(  # Dwarf
            label="Dwarf",
            name="Dwarf",
            str_adjust=0, dex_adjust=0, con_adjust=2,
            int_adjust=0, wis_adjust=0, cha_adjust=-2,
            creature_size=4,  # Medium
            movement_rate=20,  # Slower
            player_race=True,
            favored_class=0,  # Fighter
            Feat0=3,  # Darkvision
            Feat1=4,  # Stonecunning
            Feat2=None
        ),
        3: Mock(  # Halfling (Small)
            label="Halfling",
            name="Halfling",
            str_adjust=-2, dex_adjust=2, con_adjust=0,
            int_adjust=0, wis_adjust=0, cha_adjust=0,
            creature_size=3,  # Small
            movement_rate=20,
            player_race=True,
            favored_class=2,  # Rogue
            Feat0=5,  # Small Size bonus
            Feat1=None
        ),
        99: Mock(  # Custom Race
            label="Custom Race",
            name="Custom Race",
            str_adjust=1, dex_adjust=1, con_adjust=1,
            int_adjust=1, wis_adjust=1, cha_adjust=1,
            creature_size=5,  # Large
            movement_rate=40,
            player_race=True,
            favored_class=0,
            racial_feats=[10, 11, 12]  # Different format
        ),
        100: Mock(  # Non-player race
            label="Dragon",
            name="Dragon",
            player_race=False,
            creature_size=6  # Huge
        )
    }
    
    def get_by_id_side_effect(table_name, race_id):
        if table_name == 'racialtypes':
            return race_data.get(race_id, None)
        return None
    
    mock_loader.get_by_id.side_effect = get_by_id_side_effect
    return mock_loader


@pytest.fixture
def sample_character_data():
    """Create sample character data for testing"""
    return {
        "Race": 0,  # Human
        "Subrace": "",
        "Str": 16,
        "Dex": 14,
        "Con": 15,
        "Int": 12,
        "Wis": 10,
        "Cha": 8,
        "CreatureSize": 4,
        "FirstName": {
            "type": "locstring",
            "substrings": [{"string": "Test", "language": 0, "gender": 0}]
        },
        "LastName": {
            "type": "locstring", 
            "substrings": [{"string": "Character", "language": 0, "gender": 0}]
        },
        "FeatList": []
    }


@pytest.fixture
def mock_character_manager(sample_character_data, mock_game_data_loader):
    """Create a mock CharacterManager for testing"""
    mock_cm = Mock(spec=CharacterManager)
    mock_cm.character_data = sample_character_data
    mock_cm.gff = GFFDataWrapper(sample_character_data)
    mock_cm.game_data_loader = mock_game_data_loader
    
    # Mock character model
    mock_cm.character_model = Mock()
    mock_cm.character_model.race_id = 0
    mock_cm.character_model.race_name = "Human"
    mock_cm.character_model.subrace_id = 0
    mock_cm.character_model.subrace_name = ""
    
    # Mock manager retrieval
    mock_cm.get_manager.return_value = None
    mock_cm.emit = Mock()
    
    # Mock the helper methods
    mock_cm.get_racial_saves = Mock(return_value={'fortitude': 0, 'reflex': 0, 'will': 0})
    mock_cm.has_class_by_name = Mock(return_value=False)
    mock_cm.get_class_level_by_name = Mock(return_value=0)
    mock_cm.has_feat_by_name = Mock(return_value=False)
    
    return mock_cm


@pytest.fixture
def race_manager(mock_character_manager):
    """Create a RaceManager instance for testing"""
    return RaceManager(mock_character_manager)


@pytest.fixture
def mock_attribute_manager():
    """Create a mock AttributeManager"""
    mock_am = Mock()
    mock_am.set_attribute = Mock()
    return mock_am


@pytest.fixture
def mock_feat_manager():
    """Create a mock FeatManager"""
    mock_fm = Mock()
    mock_fm.has_feat = Mock(return_value=False)
    mock_fm.add_feat = Mock()
    mock_fm.remove_feat = Mock()
    mock_fm.get_feat_info = Mock(return_value={"name": "Test Feat"})
    return mock_fm


class TestRaceManagerInitialization:
    """Test RaceManager initialization and setup"""
    
    def test_initialization(self, race_manager, mock_character_manager):
        """Test RaceManager initialization"""
        assert race_manager.character_manager == mock_character_manager
        assert race_manager.gff == mock_character_manager.gff
        assert race_manager.game_data_loader == mock_character_manager.game_data_loader
        assert len(race_manager._race_data_cache) == 0
        
        # Check original race is cached
        assert race_manager._original_race['race_id'] == 0
        assert race_manager._original_race['subrace'] == ""
        assert 'attributes' in race_manager._original_race
    
    def test_original_race_attributes_cached(self, race_manager):
        """Test that original race attributes are properly cached"""
        attrs = race_manager._original_race['attributes']
        assert attrs['Str'] == 16
        assert attrs['Dex'] == 14
        assert attrs['Con'] == 15
        assert attrs['Int'] == 12
        assert attrs['Wis'] == 10
        assert attrs['Cha'] == 8


class TestRaceDataRetrieval:
    """Test race data retrieval and caching"""
    
    def test_get_race_data_valid_id(self, race_manager):
        """Test getting valid race data"""
        race_data = race_manager._get_race_data(0)
        assert race_data is not None
        assert hasattr(race_data, 'label')
        assert hasattr(race_data, 'name')
    
    def test_get_race_data_invalid_id(self, race_manager):
        """Test getting invalid race data"""
        race_data = race_manager._get_race_data(999)
        assert race_data is None
    
    def test_race_data_caching(self, race_manager):
        """Test that race data is properly cached"""
        # First call
        race_data1 = race_manager._get_race_data(0)
        assert 0 in race_manager._race_data_cache
        
        # Second call should use cache
        race_data2 = race_manager._get_race_data(0) 
        assert race_data1 is race_data2
        
        # Verify game data loader was only called once
        race_manager.game_data_loader.get_by_id.assert_called_once_with('racialtypes', 0)


class TestRaceChange:
    """Test race change functionality"""
    
    def test_basic_race_change(self, race_manager):
        """Test basic race change from Human to Elf"""
        changes = race_manager.change_race(1, "Wood Elf")
        
        # Verify race was changed
        assert race_manager.gff.get('Race') == 1
        assert race_manager.gff.get('Subrace') == "Wood Elf"
        
        # Verify changes structure
        assert changes['old_race']['id'] == 0
        assert changes['old_race']['name'] == "Human"
        assert changes['new_race']['id'] == 1
        assert changes['new_race']['name'] == "Elf"
        assert changes['new_race']['subrace'] == "Wood Elf"
    
    def test_ability_modifier_changes(self, race_manager):
        """Test ability modifiers are correctly applied"""
        # Change from Human (no mods) to Elf (+2 Dex, -2 Con)
        changes = race_manager.change_race(1)
        
        # Verify ability changes
        assert race_manager.gff.get('Dex') == 16  # 14 + 2
        assert race_manager.gff.get('Con') == 13  # 15 - 2
        assert race_manager.gff.get('Str') == 16  # Unchanged
        
        # Verify changes are recorded
        ability_changes = changes['ability_changes']
        dex_change = next(c for c in ability_changes if c['attribute'] == 'Dex')
        assert dex_change['old_value'] == 14
        assert dex_change['new_value'] == 16
        assert dex_change['modifier_applied'] == 2
        
        con_change = next(c for c in ability_changes if c['attribute'] == 'Con')
        assert con_change['old_value'] == 15
        assert con_change['new_value'] == 13
        assert con_change['modifier_applied'] == -2
    
    def test_size_change(self, race_manager):
        """Test size changes from Medium to Small"""
        # Change from Human (Medium) to Halfling (Small)
        changes = race_manager.change_race(3)
        
        assert race_manager.gff.get('CreatureSize') == 3
        assert changes['size_change']['old'] == 4
        assert changes['size_change']['new'] == 3
        assert changes['size_change']['old_name'] == "Medium"
        assert changes['size_change']['new_name'] == "Small"
    
    def test_speed_change(self, race_manager):
        """Test movement speed changes"""
        # Change from Human (30ft) to Dwarf (20ft)
        changes = race_manager.change_race(2)
        
        assert changes['speed_change']['old'] == 30
        assert changes['speed_change']['new'] == 20
    
    def test_racial_feats_added(self, race_manager, mock_feat_manager):
        """Test racial feats are added during race change"""
        race_manager.character_manager.get_manager.return_value = mock_feat_manager
        
        # Change to Elf (has racial feats)
        changes = race_manager.change_race(1)
        
        # Verify feats were attempted to be added
        assert mock_feat_manager.add_feat.called
        
        # Verify changes structure exists
        assert 'feat_changes' in changes
        assert 'added' in changes['feat_changes']
    
    def test_character_model_updated(self, race_manager):
        """Test character model is updated during race change"""
        race_manager.change_race(1, "Wood Elf")
        
        char_model = race_manager.character_manager.character_model
        assert char_model.race_id == 1
        assert char_model.race_name == "Elf"
        assert char_model.subrace_id == 0
        assert char_model.subrace_name == "Wood Elf"
    
    def test_event_emission(self, race_manager):
        """Test that race change events are emitted"""
        race_manager.change_race(1, "Wood Elf")
        
        # Verify event was emitted
        race_manager.character_manager.emit.assert_called_once()
        event = race_manager.character_manager.emit.call_args[0][0]
        assert isinstance(event, RaceChangedEvent)
        assert event.old_race_id == 0
        assert event.new_race_id == 1
        assert event.old_subrace == ""
        assert event.new_subrace == "Wood Elf"
    
    def test_invalid_race_change(self, race_manager):
        """Test error handling for invalid race change"""
        with pytest.raises(ValueError, match="Unknown race ID: 999"):
            race_manager.change_race(999)


class TestRaceChangeWithExistingModifiers:
    """Test race changes when character already has racial modifiers"""
    
    def test_remove_old_modifiers(self, race_manager):
        """Test old racial modifiers are removed"""
        # Start with Elf (has modifiers)
        race_manager.gff.set('Race', 1)
        race_manager.gff.set('Dex', 16)  # Already has +2 Dex
        race_manager.gff.set('Con', 13)  # Already has -2 Con
        
        # Change to Human (no modifiers)
        changes = race_manager.change_race(0)
        
        # Verify old modifiers were removed
        assert race_manager.gff.get('Dex') == 14  # 16 - 2
        assert race_manager.gff.get('Con') == 15  # 13 + 2
        
        # Verify changes recorded removal
        ability_changes = changes['ability_changes']
        dex_change = next(c for c in ability_changes if c['attribute'] == 'Dex')
        assert 'modifier_removed' in dex_change
        assert dex_change['modifier_removed'] == 2


class TestRacialAbilityModifiers:
    """Test racial ability modifier extraction"""
    
    def test_get_racial_ability_modifiers_human(self, race_manager):
        """Test Human has no ability modifiers"""
        modifiers = race_manager._get_racial_ability_modifiers(0)
        
        for attr in ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']:
            assert modifiers[attr] == 0
    
    def test_get_racial_ability_modifiers_elf(self, race_manager):
        """Test Elf ability modifiers"""
        modifiers = race_manager._get_racial_ability_modifiers(1)
        
        assert modifiers['Str'] == 0
        assert modifiers['Dex'] == 2
        assert modifiers['Con'] == -2
        assert modifiers['Int'] == 0
        assert modifiers['Wis'] == 0
        assert modifiers['Cha'] == 0
    
    def test_get_racial_ability_modifiers_custom_race(self, race_manager):
        """Test custom race with all +1 modifiers"""
        modifiers = race_manager._get_racial_ability_modifiers(99)
        
        for attr in ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']:
            assert modifiers[attr] == 1


class TestRacialFeats:
    """Test racial feat extraction"""
    
    def test_get_racial_feats_individual_fields(self, race_manager):
        """Test extracting feats from individual Feat0, Feat1 fields"""
        feats = race_manager._get_racial_feats(1)  # Elf
        # Should find at least one feat, exact count may vary based on Mock structure
        assert len(feats) >= 1
        # Check that valid feat IDs are returned
        assert all(isinstance(feat_id, int) and feat_id > 0 for feat_id in feats)
    
    def test_get_racial_feats_array_format(self, race_manager):
        """Test extracting feats from array format"""
        feats = race_manager._get_racial_feats(99)  # Custom race
        assert 10 in feats
        assert 11 in feats
        assert 12 in feats
        assert len(feats) == 3
    
    def test_get_racial_feats_no_feats(self, race_manager):
        """Test race with no racial feats"""
        feats = race_manager._get_racial_feats(0)  # Human
        assert len(feats) == 0
    
    def test_get_racial_feats_invalid_race(self, race_manager):
        """Test getting feats for invalid race"""
        feats = race_manager._get_racial_feats(999)
        assert len(feats) == 0


class TestSizeAndSpeed:
    """Test size and speed extraction"""
    
    def test_get_race_size(self, race_manager):
        """Test race size extraction"""
        assert race_manager._get_race_size(0) == 4  # Human - Medium
        assert race_manager._get_race_size(3) == 3  # Halfling - Small
        assert race_manager._get_race_size(99) == 5  # Custom - Large
    
    def test_get_race_size_fallback(self, race_manager):
        """Test size fallback for invalid race"""
        assert race_manager._get_race_size(999) == 4  # Default to Medium
    
    def test_get_base_speed(self, race_manager):
        """Test base speed extraction"""
        assert race_manager._get_base_speed(0) == 30  # Human
        assert race_manager._get_base_speed(2) == 20  # Dwarf
        assert race_manager._get_base_speed(99) == 40  # Custom
    
    def test_get_base_speed_fallback(self, race_manager):
        """Test speed fallback based on size"""
        # Mock a small race without speed data
        mock_small_race = Mock()
        mock_small_race.creature_size = 3
        # Remove speed attributes to test fallback
        for attr in ['movement_rate', 'base_speed', 'speed', 'MovementRate']:
            if hasattr(mock_small_race, attr):
                delattr(mock_small_race, attr)
        
        def mock_get_by_id(table, race_id):
            if race_id == 998:
                return mock_small_race
            elif race_id == 997:
                mock_medium_race = Mock()
                mock_medium_race.creature_size = 4
                for attr in ['movement_rate', 'base_speed', 'speed', 'MovementRate']:
                    if hasattr(mock_medium_race, attr):
                        delattr(mock_medium_race, attr)
                return mock_medium_race
            return None
        
        race_manager.game_data_loader.get_by_id.side_effect = mock_get_by_id
        
        speed = race_manager._get_base_speed(998)
        assert speed == 20  # Small races default to 20ft
        
        speed = race_manager._get_base_speed(997)
        assert speed == 30  # Medium+ races default to 30ft
    
    def test_get_size_name(self, race_manager):
        """Test size name mapping"""
        assert race_manager._get_size_name(3) == "Small"
        assert race_manager._get_size_name(4) == "Medium"
        assert race_manager._get_size_name(5) == "Large"
        assert race_manager._get_size_name(99) == "Unknown"


class TestRaceNames:
    """Test race name extraction"""
    
    def test_get_race_name_valid(self, race_manager):
        """Test getting race names for valid races"""
        assert race_manager._get_race_name(0) == "Human"
        assert race_manager._get_race_name(1) == "Elf"
        assert race_manager._get_race_name(2) == "Dwarf"
    
    def test_get_race_name_invalid(self, race_manager):
        """Test fallback race name for invalid race"""
        assert race_manager._get_race_name(999) == "Race_999"


class TestRaceProperties:
    """Test comprehensive race property extraction"""
    
    def test_get_racial_properties_human(self, race_manager):
        """Test getting properties for Human"""
        props = race_manager.get_racial_properties()
        
        assert props['race_id'] == 0
        assert props['race_name'] == "Human"
        assert props['subrace'] == ""
        assert props['size'] == 4
        assert props['size_name'] == "Medium"
        assert props['base_speed'] == 30
        assert props['favored_class'] == 0
        
        # All ability modifiers should be 0
        for attr in ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']:
            assert props['ability_modifiers'][attr] == 0
        
        assert len(props['racial_feats']) == 0
    
    def test_get_racial_properties_elf(self, race_manager):
        """Test getting properties for Elf after race change"""
        race_manager.change_race(1, "Wood Elf")
        props = race_manager.get_racial_properties()
        
        assert props['race_id'] == 1
        assert props['race_name'] == "Elf"
        assert props['subrace'] == "Wood Elf"
        assert props['ability_modifiers']['Dex'] == 2
        assert props['ability_modifiers']['Con'] == -2
        # Should have at least one racial feat
        assert len(props['racial_feats']) >= 1


class TestRaceValidation:
    """Test race change validation"""
    
    def test_validate_race_change_valid(self, race_manager):
        """Test validation for valid race change"""
        is_valid, errors = race_manager.validate_race_change(1)
        assert is_valid is True
        assert len(errors) == 0
    
    def test_validate_race_change_invalid_id(self, race_manager):
        """Test validation for invalid race ID"""
        is_valid, errors = race_manager.validate_race_change(999)
        assert is_valid is False
        assert "Unknown race ID: 999" in errors
    
    def test_validate_race_change_non_player_race(self, race_manager):
        """Test validation allows non-player race (validation cleanup)"""
        is_valid, errors = race_manager.validate_race_change(100)
        assert is_valid is True  # Now allows non-player races
        assert len(errors) == 0  # No restrictions


class TestRaceManagerSummary:
    """Test race summary functionality"""
    
    def test_get_race_summary_human(self, race_manager):
        """Test race summary for Human"""
        summary = race_manager.get_race_summary()
        
        assert summary['race_id'] == 0
        assert summary['race_name'] == "Human"
        assert summary['ability_modifier_string'] == "None"
    
    def test_get_race_summary_elf(self, race_manager):
        """Test race summary for Elf with modifiers"""
        race_manager.change_race(1)
        summary = race_manager.get_race_summary()
        
        assert summary['race_id'] == 1
        assert summary['race_name'] == "Elf"
        assert "Dex +2" in summary['ability_modifier_string']
        assert "Con -2" in summary['ability_modifier_string']


class TestRaceManagerValidation:
    """Test RaceManager validation"""
    
    def test_validate_valid_race(self, race_manager):
        """Test validation with valid race configuration"""
        is_valid, errors = race_manager.validate()
        assert is_valid is True
        assert len(errors) == 0
    
    def test_validate_invalid_race_id(self, race_manager):
        """Test validation with invalid race ID"""
        race_manager.gff.set('Race', 999)
        is_valid, errors = race_manager.validate()
        assert is_valid is False
        assert "Invalid race ID: 999" in errors
    
    def test_validate_size_mismatch(self, race_manager):
        """Test validation allows size mismatch (validation cleanup)"""
        race_manager.gff.set('Race', 3)  # Halfling (Small)
        race_manager.gff.set('CreatureSize', 4)  # Medium
        is_valid, errors = race_manager.validate()
        assert is_valid is True  # Now allows size mismatches
        assert len(errors) == 0  # No size restrictions


class TestRaceManagerReverting:
    """Test race manager reverting functionality"""
    
    def test_revert_to_original_race(self, race_manager):
        """Test reverting to original race"""
        # Change race
        race_manager.change_race(1, "Wood Elf")
        assert race_manager.gff.get('Race') == 1
        
        # Revert
        changes = race_manager.revert_to_original_race()
        assert race_manager.gff.get('Race') == 0
        assert race_manager.gff.get('Subrace') == ""
        
        # Verify it's a proper change operation
        assert changes['old_race']['id'] == 1
        assert changes['new_race']['id'] == 0


class TestIntegrationWithManagers:
    """Test integration with other managers"""
    
    def test_integration_with_attribute_manager(self, race_manager, mock_attribute_manager):
        """Test integration with AttributeManager"""
        race_manager.character_manager.get_manager.return_value = mock_attribute_manager
        
        # Change race with ability modifiers
        race_manager.change_race(1)  # Elf: +2 Dex, -2 Con
        
        # Verify attribute manager was called
        mock_attribute_manager.set_attribute.assert_any_call('Dex', 16, validate=False)
        mock_attribute_manager.set_attribute.assert_any_call('Con', 13, validate=False)
    
    def test_integration_with_feat_manager_preserve_feats(self, race_manager, mock_feat_manager):
        """Test feat integration with preserve_feats=True"""
        race_manager.character_manager.get_manager.return_value = mock_feat_manager
        
        # Change race with preserve_feats=True (default)
        race_manager.change_race(1, preserve_feats=True)
        
        # Should not remove any feats
        mock_feat_manager.remove_feat.assert_not_called()
        
        # Should add racial feats
        mock_feat_manager.add_feat.assert_called()
    
    def test_integration_with_feat_manager_no_preserve(self, race_manager, mock_feat_manager):
        """Test feat integration with preserve_feats=False"""
        race_manager.character_manager.get_manager.return_value = mock_feat_manager
        mock_feat_manager.has_feat.return_value = True
        
        # Start with Elf, change to Human
        race_manager.gff.set('Race', 1)
        race_manager.change_race(0, preserve_feats=False)
        
        # Should remove old racial feats
        mock_feat_manager.remove_feat.assert_called()


class TestEdgeCases:
    """Test edge cases and error handling"""
    
    def test_race_change_same_race(self, race_manager):
        """Test changing to the same race"""
        changes = race_manager.change_race(0, "Different Subrace")
        
        # Should still work and update subrace
        assert changes['old_race']['id'] == 0
        assert changes['new_race']['id'] == 0
        assert race_manager.gff.get('Subrace') == "Different Subrace"
    
    def test_malformed_race_data(self, race_manager):
        """Test handling of malformed race data"""
        # Mock race data with missing fields
        mock_race = Mock()
        del mock_race.str_adjust  # Remove expected field
        race_manager.game_data_loader.get_by_id.return_value = mock_race
        
        # Should handle gracefully
        modifiers = race_manager._get_racial_ability_modifiers(999)
        assert all(mod == 0 for mod in modifiers.values())
    
    def test_no_character_model(self, race_manager):
        """Test race change without character model"""
        # Remove the character_model attribute entirely to test hasattr check
        if hasattr(race_manager.character_manager, 'character_model'):
            delattr(race_manager.character_manager, 'character_model')
        
        # Should not crash
        changes = race_manager.change_race(1)
        assert changes['new_race']['id'] == 1
    
    def test_favored_class_extraction(self, race_manager):
        """Test favored class extraction"""
        # Test with valid favored class
        favored = race_manager._get_favored_class(0)  # Human
        assert favored == 0
        
        # Test with invalid race
        favored = race_manager._get_favored_class(999)
        assert favored is None


class TestPerformance:
    """Test performance-related scenarios"""
    
    def test_multiple_race_changes_performance(self, race_manager):
        """Test performance with multiple race changes"""
        races = [0, 1, 2, 3, 0, 1, 2, 3]
        
        start_time = time.time()
        for race_id in races:
            race_manager.change_race(race_id)
        end_time = time.time()
        
        # Should complete quickly due to caching
        assert end_time - start_time < 1.0
        
        # Verify caching worked
        assert len(race_manager._race_data_cache) == 4
    
    def test_large_feat_list_handling(self, race_manager):
        """Test handling of races with many feats"""
        # Mock race with many feats
        mock_race = Mock()
        mock_race.label = "Feat Heavy Race"
        mock_race.racial_feats = list(range(1, 101))  # 1-100 to avoid 0 being filtered
        
        def mock_get_by_id(table, race_id):
            if race_id == 998:
                return mock_race
            return race_manager.game_data_loader.get_by_id(table, race_id)
        
        race_manager.game_data_loader.get_by_id.side_effect = mock_get_by_id
        
        feats = race_manager._get_racial_feats(998)
        assert len(feats) == 100
        assert all(i in feats for i in range(1, 101))


@pytest.mark.parametrize("race_id,expected_name", [
    (0, "Human"),
    (1, "Elf"), 
    (2, "Dwarf"),
    (3, "Halfling"),
    (99, "Custom Race"),
    (999, "Race_999")
])
def test_race_names_parametrized(race_manager, race_id, expected_name):
    """Test race name extraction for various races"""
    name = race_manager._get_race_name(race_id)
    assert name == expected_name


@pytest.mark.parametrize("from_race,to_race,expected_size_change", [
    (0, 3, True),   # Human to Halfling (Medium to Small)
    (3, 0, True),   # Halfling to Human (Small to Medium)
    (0, 1, False),  # Human to Elf (both Medium)
    (1, 2, False),  # Elf to Dwarf (both Medium)
])
def test_size_changes_parametrized(race_manager, from_race, to_race, expected_size_change):
    """Test size changes between different races"""
    if from_race != 0:
        race_manager.change_race(from_race)
    
    changes = race_manager.change_race(to_race)
    
    if expected_size_change:
        assert changes['size_change'] is not None
    else:
        assert changes['size_change'] is None