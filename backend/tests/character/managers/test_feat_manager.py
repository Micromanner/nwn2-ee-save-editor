"""
Comprehensive tests for FeatManager class.
Tests cover feat addition/removal, prerequisite validation, event handling,
feat progression, protection mechanisms, and data-driven architecture.
"""
import pytest
import time
from unittest.mock import Mock, MagicMock, patch, PropertyMock
from typing import Dict, List, Any

from character.managers.feat_manager import FeatManager
from character.events import EventEmitter, EventType, ClassChangedEvent, LevelGainedEvent, FeatChangedEvent
from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader


class MockFeat:
    """Mock feat data for testing"""
    def __init__(self, id, label, name=None, feat_type=0, prereq_str=0, prereq_dex=0, 
                 prereq_feat1=0, prereq_feat2=0, required_class=-1, min_level=0):
        self.id = id
        self.label = label
        self.name = name or label
        self.type = feat_type
        self.feat_type = feat_type  # Alternative column name
        self.prereq_str = prereq_str
        self.prereqstr = prereq_str  # Alternative column name
        self.prereq_dex = prereq_dex
        self.prereqdex = prereq_dex  # Alternative column name
        self.prereq_feat1 = prereq_feat1
        self.prereqfeat1 = prereq_feat1  # Alternative column name
        self.prereq_feat2 = prereq_feat2
        self.prereqfeat2 = prereq_feat2  # Alternative column name
        self.required_class = required_class
        self.reqclass = required_class  # Alternative column name
        self.min_level = min_level
        self.minlevel = min_level  # Alternative column name


class MockClass:
    """Mock class data for testing"""
    def __init__(self, id, label, name=None):
        self.id = id
        self.label = label
        self.name = name or label


@pytest.fixture
def mock_game_data_loader():
    """Create comprehensive mock DynamicGameDataLoader with various feat types"""
    mock_loader = Mock(spec=DynamicGameDataLoader)
    
    # Mock feats data for get_by_id and get_table
    mock_feats = {
        # Basic feats (no prerequisites)
        1: MockFeat(1, 'WeaponFinesse', 'Weapon Finesse', feat_type=1),
        2: MockFeat(2, 'PowerAttack', 'Power Attack', feat_type=1),
        3: MockFeat(3, 'CombatExpertise', 'Combat Expertise', feat_type=1),
        
        # Feats with ability score requirements
        10: MockFeat(10, 'ImprovedCritical', 'Improved Critical', feat_type=1, prereq_dex=13),
        11: MockFeat(11, 'PowerCleave', 'Power Cleave', feat_type=1, prereq_str=15),
        
        # Feats with feat prerequisites
        20: MockFeat(20, 'CleaveGreat', 'Great Cleave', feat_type=1, prereq_feat1=2),  # Requires Power Attack
        21: MockFeat(21, 'WhirlwindAttack', 'Whirlwind Attack', feat_type=1, prereq_feat1=3, prereq_feat2=10),  # Requires Combat Expertise and Improved Critical
        
        # Feats with class/level requirements
        30: MockFeat(30, 'TurnUndead', 'Turn Undead', feat_type=2, required_class=3, min_level=1),  # Cleric only
        31: MockFeat(31, 'ScribeScroll', 'Scribe Scroll', feat_type=2, required_class=1, min_level=1),  # Wizard only
        
        # Progressive feats
        40: MockFeat(40, 'BarbarianRage', 'Barbarian Rage', feat_type=2),
        41: MockFeat(41, 'BarbarianRage2', 'Barbarian Rage 2', feat_type=2),
        42: MockFeat(42, 'BarbarianRage3', 'Barbarian Rage 3', feat_type=2),
        
        # Epithet/Custom feats (high IDs)
        10001: MockFeat(10001, 'CustomEpithet', 'Custom Epithet Feat', feat_type=3),
        10002: MockFeat(10002, 'ModdedFeat', 'Modded Special Feat', feat_type=1),
    }
    
    # Mock classes data
    mock_classes = {
        0: MockClass(0, 'FIGHTER', 'Fighter'),
        1: MockClass(1, 'WIZARD', 'Wizard'),
        2: MockClass(2, 'ROGUE', 'Rogue'),
        3: MockClass(3, 'CLERIC', 'Cleric'),
    }
    
    def mock_get_by_id(table_name, item_id):
        if table_name == 'feat':
            return mock_feats.get(item_id)
        elif table_name == 'classes':
            return mock_classes.get(item_id)
        return None
    
    def mock_get_table(table_name):
        if table_name == 'feat':
            return list(mock_feats.values())
        elif table_name == 'classes':
            return list(mock_classes.values())
        return []
    
    mock_loader.get_by_id = mock_get_by_id
    mock_loader.get_table = mock_get_table
    
    return mock_loader


@pytest.fixture
def mock_character_manager():
    """Create a mock CharacterManager with required methods"""
    manager = Mock()
    
    # Mock custom content detection
    manager.custom_content = {
        'feat_10001': {
            'type': 'feat',
            'id': 10001,
            'protected': True
        }
    }
    
    # Mock methods
    manager.detect_epithet_feats = Mock(return_value=[10001])
    manager.get_class_feats_for_level = Mock(return_value=[])
    manager.on = Mock()  # Event registration
    manager.emit = Mock()  # Event emission
    
    return manager


@pytest.fixture
def mock_gff_data():
    """Create mock GFF data wrapper"""
    gff_data = {
        'FeatList': [],
        'ClassList': [{'Class': 0, 'ClassLevel': 5}],  # Fighter level 5
        'Str': 16, 'Dex': 14, 'Con': 15, 'Int': 12, 'Wis': 13, 'Cha': 10
    }
    
    mock_gff = Mock()
    mock_gff.get = lambda key, default=None: gff_data.get(key, default)
    mock_gff.set = lambda key, value: gff_data.update({key: value})
    
    return mock_gff


@pytest.fixture
def feat_manager(mock_character_manager, mock_game_data_loader, mock_gff_data):
    """Create FeatManager instance with mocked dependencies"""
    mock_character_manager.game_data_loader = mock_game_data_loader
    mock_character_manager.gff = mock_gff_data
    
    return FeatManager(mock_character_manager)


class TestFeatManagerInitialization:
    """Test FeatManager initialization and setup"""
    
    def test_initialization(self, mock_character_manager, mock_game_data_loader, mock_gff_data):
        """Test basic FeatManager initialization"""
        mock_character_manager.game_data_loader = mock_game_data_loader
        mock_character_manager.gff = mock_gff_data
        
        feat_manager = FeatManager(mock_character_manager)
        
        assert feat_manager.character_manager == mock_character_manager
        assert feat_manager.game_data_loader == mock_game_data_loader
        assert feat_manager.gff == mock_gff_data
        assert isinstance(feat_manager._feat_cache, dict)
        assert isinstance(feat_manager._protected_feats, set)
    
    def test_event_handler_registration(self, feat_manager, mock_character_manager):
        """Test that event handlers are properly registered"""
        # Should register for class changed and level gained events
        expected_calls = [
            ((EventType.CLASS_CHANGED, feat_manager.on_class_changed),),
            ((EventType.LEVEL_GAINED, feat_manager.on_level_gained),)
        ]
        
        # Verify that on() was called with the expected handlers
        assert mock_character_manager.on.call_count >= 2
    
    def test_protected_feats_initialization(self, feat_manager):
        """Test that protected feats are properly initialized"""
        # Should include epithet feats from detection
        assert 10001 in feat_manager._protected_feats


class TestCoreFeatManagement:
    """Test core feat addition, removal, and querying functionality"""
    
    def test_add_feat_success(self, feat_manager):
        """Test successful feat addition"""
        result = feat_manager.add_feat(1, source='manual')  # WeaponFinesse
        
        assert result is True
        feat_list = feat_manager.gff.get('FeatList', [])
        assert {'Feat': 1} in feat_list
        
        # Should emit FeatChangedEvent
        feat_manager.character_manager.emit.assert_called_once()
        event = feat_manager.character_manager.emit.call_args[0][0]
        assert isinstance(event, FeatChangedEvent)
        assert event.feat_id == 1
        assert event.action == 'added'
        assert event.source == 'manual'
    
    def test_add_feat_duplicate_prevention(self, feat_manager):
        """Test that duplicate feats are not added"""
        # Add feat first time
        feat_manager.add_feat(1, source='manual')
        feat_manager.character_manager.emit.reset_mock()
        
        # Try to add same feat again
        result = feat_manager.add_feat(1, source='manual')
        
        assert result is False
        # Should not emit event for duplicate
        feat_manager.character_manager.emit.assert_not_called()
    
    def test_add_feat_prerequisite_validation_failure(self, feat_manager):
        """Test that feats with unmet prerequisites are rejected"""
        # Try to add Great Cleave without Power Attack prerequisite
        result = feat_manager.add_feat(20, source='manual')
        
        assert result is False
        feat_list = feat_manager.gff.get('FeatList', [])
        assert not any(f.get('Feat') == 20 for f in feat_list)
    
    def test_add_feat_bypass_validation_for_class_source(self, feat_manager):
        """Test that prerequisite validation is bypassed for class/level sources"""
        # Should succeed even without prerequisites for class-granted feats
        result = feat_manager.add_feat(20, source='class')
        
        assert result is True
        feat_list = feat_manager.gff.get('FeatList', [])
        assert {'Feat': 20} in feat_list
    
    def test_remove_feat_success(self, feat_manager):
        """Test successful feat removal"""
        # Add feat first
        feat_manager.add_feat(1, source='manual')
        feat_manager.character_manager.emit.reset_mock()
        
        # Remove feat
        result = feat_manager.remove_feat(1)
        
        assert result is True
        feat_list = feat_manager.gff.get('FeatList', [])
        assert not any(f.get('Feat') == 1 for f in feat_list)
        
        # Should emit FeatChangedEvent
        feat_manager.character_manager.emit.assert_called_once()
        event = feat_manager.character_manager.emit.call_args[0][0]
        assert isinstance(event, FeatChangedEvent)
        assert event.feat_id == 1
        assert event.action == 'removed'
    
    def test_remove_feat_protection(self, feat_manager):
        """Test that protected feats cannot be removed without force"""
        # Try to remove protected feat (epithet feat)
        result = feat_manager.remove_feat(10001, force=False)
        
        assert result is False
        feat_manager.character_manager.emit.assert_not_called()
    
    def test_remove_feat_force_override(self, feat_manager):
        """Test that force=True bypasses protection"""
        # Add protected feat first
        feat_manager.gff.get('FeatList').append({'Feat': 10001})
        
        # Force remove protected feat
        result = feat_manager.remove_feat(10001, force=True)
        
        assert result is True
        feat_list = feat_manager.gff.get('FeatList', [])
        assert not any(f.get('Feat') == 10001 for f in feat_list)
    
    def test_remove_feat_not_present(self, feat_manager):
        """Test removing feat that character doesn't have"""
        result = feat_manager.remove_feat(999)  # Non-existent feat
        
        assert result is False
        feat_manager.character_manager.emit.assert_not_called()
    
    def test_has_feat_present(self, feat_manager):
        """Test has_feat returns True for present feats"""
        feat_manager.add_feat(1)
        
        assert feat_manager.has_feat(1) is True
    
    def test_has_feat_absent(self, feat_manager):
        """Test has_feat returns False for absent feats"""
        assert feat_manager.has_feat(999) is False
    
    def test_has_feat_empty_list(self, feat_manager):
        """Test has_feat with empty feat list"""
        # Ensure feat list is empty
        feat_manager.gff.set('FeatList', [])
        
        assert feat_manager.has_feat(1) is False


class TestPrerequisiteValidation:
    """Test feat prerequisite validation logic"""
    
    def test_validate_no_prerequisites(self, feat_manager):
        """Test validation passes for feats with no prerequisites"""
        is_valid, errors = feat_manager.validate_feat_prerequisites(1)  # WeaponFinesse
        
        assert is_valid is True
        assert errors == []
    
    def test_validate_ability_score_requirements_pass(self, feat_manager):
        """Test validation passes when ability score requirements are met"""
        # Character has STR 16, DEX 14
        is_valid, errors = feat_manager.validate_feat_prerequisites(10)  # Improved Critical (DEX 13 req)
        
        assert is_valid is True
        assert errors == []
    
    def test_validate_ability_score_requirements_fail(self, feat_manager):
        """Test validation fails when ability score requirements are not met"""
        # Character has STR 16, but feat requires STR 15+ (should pass)
        # Let's test with a feat that requires higher stats
        is_valid, errors = feat_manager.validate_feat_prerequisites(11)  # Power Cleave (STR 15 req)
        
        assert is_valid is True  # Character has STR 16, so should pass
        assert errors == []
        
        # Test with insufficient stats by mocking lower STR
        feat_manager.gff.set('Str', 10)  # Lower than required
        is_valid, errors = feat_manager.validate_feat_prerequisites(11)
        
        assert is_valid is False
        assert any('STR 15' in error for error in errors)
    
    def test_validate_feat_prerequisites_pass(self, feat_manager):
        """Test validation passes when feat prerequisites are met"""
        # Add Power Attack first
        feat_manager.add_feat(2)  # Power Attack
        
        # Now validate Great Cleave which requires Power Attack
        is_valid, errors = feat_manager.validate_feat_prerequisites(20)
        
        assert is_valid is True
        assert errors == []
    
    def test_validate_feat_prerequisites_fail(self, feat_manager):
        """Test validation fails when feat prerequisites are not met"""
        # Try to validate Great Cleave without Power Attack
        is_valid, errors = feat_manager.validate_feat_prerequisites(20)
        
        assert is_valid is False
        assert any('PowerAttack' in error or 'Power Attack' in error for error in errors)
    
    def test_validate_multiple_feat_prerequisites(self, feat_manager):
        """Test validation with multiple feat prerequisites"""
        # Whirlwind Attack requires both Combat Expertise and Improved Critical
        is_valid, errors = feat_manager.validate_feat_prerequisites(21)
        
        assert is_valid is False
        assert len(errors) == 2  # Should have 2 missing prerequisites
    
    def test_validate_class_requirements_pass(self, feat_manager):
        """Test validation passes when class requirements are met"""
        # Character is Fighter (class 0), but Turn Undead requires Cleric (class 3)
        # Let's test with a feat that allows Fighter
        feat_manager.gff.set('ClassList', [{'Class': 3, 'ClassLevel': 1}])  # Make character a Cleric
        
        is_valid, errors = feat_manager.validate_feat_prerequisites(30)  # Turn Undead
        
        assert is_valid is True
        assert errors == []
    
    def test_validate_class_requirements_fail(self, feat_manager):
        """Test validation fails when class requirements are not met"""
        # Character is Fighter, but Turn Undead requires Cleric
        is_valid, errors = feat_manager.validate_feat_prerequisites(30)
        
        assert is_valid is False
        assert any('CLERIC' in error or 'Cleric' in error for error in errors)
    
    def test_validate_level_requirements_pass(self, feat_manager):
        """Test validation passes when level requirements are met"""
        # Character is level 5, feat requires level 1
        is_valid, errors = feat_manager.validate_feat_prerequisites(30)  # Turn Undead (min level 1)
        
        # Will fail due to class requirement, but level should not be in errors
        is_valid, errors = feat_manager.validate_feat_prerequisites(30)
        assert not any('level' in error.lower() for error in errors)
    
    def test_validate_level_requirements_fail(self, feat_manager):
        """Test validation fails when level requirements are not met"""
        # Set character to level 1, then test a hypothetical high-level feat
        feat_manager.gff.set('ClassList', [{'Class': 0, 'ClassLevel': 1}])
        
        # Create a mock feat with high level requirement
        mock_feat = MockFeat(999, 'EpicFeat', min_level=21)
        feat_manager.game_data_loader.get_by_id = Mock(return_value=mock_feat)
        
        is_valid, errors = feat_manager.validate_feat_prerequisites(999)
        
        assert is_valid is False
        assert any('level 21' in error for error in errors)
    
    def test_validate_unknown_feat_allowed(self, feat_manager):
        """Test that unknown feats (custom content) are allowed"""
        # Mock returning None for unknown feat
        feat_manager.game_data_loader.get_by_id = Mock(return_value=None)
        
        is_valid, errors = feat_manager.validate_feat_prerequisites(99999)
        
        assert is_valid is True
        assert errors == []


class TestFeatInfo:
    """Test feat information retrieval and caching"""
    
    def test_get_feat_info_known_feat(self, feat_manager):
        """Test getting info for known feat"""
        info = feat_manager.get_feat_info(1)  # WeaponFinesse
        
        assert info is not None
        assert info['id'] == 1
        assert info['label'] == 'WeaponFinesse'
        assert info['name'] == 'WeaponFinesse'  # Name and label are the same in our mock
        assert info['type'] == 1
        assert 'protected' in info
        assert 'custom' in info
    
    def test_get_feat_info_caching(self, feat_manager):
        """Test that feat info is properly cached"""
        # Get info twice
        info1 = feat_manager.get_feat_info(1)
        info2 = feat_manager.get_feat_info(1)
        
        # Should be same object (cached)
        assert info1 is info2
        assert 1 in feat_manager._feat_cache
    
    def test_get_feat_info_unknown_feat(self, feat_manager):
        """Test getting info for unknown feat"""
        # Mock returning None for unknown feat
        feat_manager.game_data_loader.get_by_id = Mock(return_value=None)
        
        info = feat_manager.get_feat_info(99999)
        
        assert info is not None
        assert info['id'] == 99999
        assert 'Unknown' in info['label']
        assert info['protected'] is True  # Unknown feats are protected
        assert info['custom'] is True
    
    def test_get_feat_info_custom_content_detection(self, feat_manager):
        """Test custom content detection in feat info"""
        # Mock the ContentManager to return custom content
        mock_content_manager = Mock()
        mock_content_manager.is_custom_content.return_value = True
        feat_manager._get_content_manager = Mock(return_value=mock_content_manager)
        
        info = feat_manager.get_feat_info(10002)  # ModdedFeat
        
        assert info['custom'] is True
        mock_content_manager.is_custom_content.assert_called_with('feat', 10002)


class TestEventHandling:
    """Test event handling for class changes and level gains"""
    
    def test_on_class_changed_feat_removal(self, feat_manager):
        """Test feat removal during class change"""
        # Add some feats first
        feat_manager.add_feat(1)  # WeaponFinesse
        feat_manager.add_feat(2)  # PowerAttack
        
        # Mock class feats for level calculation
        feat_manager.character_manager.get_class_feats_for_level.return_value = [
            {'feat_id': 2, 'list_type': 0}  # PowerAttack is auto-granted
        ]
        
        # Create class changed event
        event = ClassChangedEvent(
            event_type=EventType.CLASS_CHANGED,
            source_manager='class',
            timestamp=time.time(),
            old_class_id=0,  # Fighter
            new_class_id=1,  # Wizard
            level=5,
            preserve_feats=[1]  # Preserve WeaponFinesse
        )
        
        feat_manager.on_class_changed(event)
        
        # Should have called methods to handle class change
        assert feat_manager.character_manager.get_class_feats_for_level.called
    
    def test_class_specific_feat_removal_stormlord(self, feat_manager):
        """Test removal of Stormlord-specific feats when changing from Stormlord class"""
        # Add a mock Stormlord class and feat
        stormlord_class_id = 56  # Common Stormlord class ID
        stormlord_feat_id = 500  # Mock Stormlord feat
        
        # Create mock Stormlord class
        mock_stormlord_class = MockClass(stormlord_class_id, 'STORMLORD', 'Storm Lord')
        
        # Create mock Stormlord feat that requires Stormlord class
        mock_stormlord_feat = MockFeat(
            stormlord_feat_id, 
            'StormlordLightning', 
            'Stormlord Lightning Strike',
            feat_type=2,
            required_class=stormlord_class_id
        )
        
        # Update mocks to include Stormlord data
        feat_manager.game_rules_service.get_by_id.side_effect = lambda table, id: {
            ('classes', stormlord_class_id): mock_stormlord_class,
            ('feat', stormlord_feat_id): mock_stormlord_feat,
            ('classes', 0): MockClass(0, 'FIGHTER', 'Fighter'),
            ('classes', 1): MockClass(1, 'WIZARD', 'Wizard'),
            ('feat', 1): MockFeat(1, 'WeaponFinesse', 'Weapon Finesse'),
        }.get((table, id))
        
        # Set up character with Stormlord class and feat
        feat_manager.gff.get.side_effect = lambda key, default=None: {
            'ClassList': [{'Class': stormlord_class_id, 'ClassLevel': 5}],
            'FeatList': [{'Feat': stormlord_feat_id}]
        }.get(key, default)
        
        # Test the class-specific feat check
        remaining_classes = {0}  # Fighter class remains
        is_class_specific = feat_manager._is_class_specific_feat(
            stormlord_feat_id, 
            stormlord_class_id, 
            remaining_classes
        )
        
        assert is_class_specific == True, "Stormlord feat should be identified as class-specific"
    
    def test_protected_feats_not_removed(self, feat_manager):
        """Test that protected feats (quest, epithet, racial) are not removed during class change"""
        quest_feat_id = 9999  # Mock quest feat
        
        # Mock a quest feat
        mock_quest_feat = MockFeat(quest_feat_id, 'QuestReward', 'Special Quest Reward')
        
        feat_manager.game_rules_service.get_by_id.side_effect = lambda table, id: {
            ('feat', quest_feat_id): mock_quest_feat,
            ('classes', 0): MockClass(0, 'FIGHTER', 'Fighter'),
        }.get((table, id))
        
        # Mark as protected
        feat_manager._protected_feats.add(quest_feat_id)
        
        # Set up character data
        feat_manager.gff.get.side_effect = lambda key, default=None: {
            'ClassList': [{'Class': 0, 'ClassLevel': 5}],
            'FeatList': [{'Feat': quest_feat_id}]
        }.get(key, default)
        
        # Test that protected feat is not considered for removal
        remaining_classes = {1}  # Different class
        is_class_specific = feat_manager._is_class_specific_feat(
            quest_feat_id, 
            0,  # Remove fighter
            remaining_classes
        )
        
        # Even if it was class-specific, it should be protected
        assert feat_manager.is_feat_protected(quest_feat_id) == True, "Quest feat should be protected"
    
    def test_on_level_gained_feat_addition(self, feat_manager):
        """Test feat addition during level gain"""
        # Mock class data and feats for new level
        mock_class = MockClass(0, 'FIGHTER', 'Fighter')
        feat_manager.game_data_loader.get_by_id.return_value = mock_class
        
        feat_manager.character_manager.get_class_feats_for_level.return_value = [
            {'feat_id': 2, 'list_type': 0}  # Auto-granted PowerAttack
        ]
        
        # Create level gained event
        event = LevelGainedEvent(
            event_type=EventType.LEVEL_GAINED,
            source_manager='class',
            timestamp=time.time(),
            class_id=0,
            new_level=2,
            total_level=2
        )
        
        feat_manager.on_level_gained(event)
        
        # Should have added the auto-granted feat
        feat_list = feat_manager.gff.get('FeatList', [])
        assert {'Feat': 2} in feat_list
    
    def test_on_level_gained_feat_progression(self, feat_manager):
        """Test feat progression during level gain"""
        # Add base rage feat
        feat_manager.add_feat(40)  # BarbarianRage
        feat_manager.character_manager.emit.reset_mock()
        
        # Mock progression detection
        with patch.object(feat_manager, '_check_feat_progression', return_value=40):
            # Mock class data
            mock_class = MockClass(0, 'BARBARIAN', 'Barbarian')
            feat_manager.game_data_loader.get_by_id.return_value = mock_class
            
            feat_manager.character_manager.get_class_feats_for_level.return_value = [
                {'feat_id': 41, 'list_type': 0}  # Auto-granted BarbarianRage2
            ]
            
            # Create level gained event
            event = LevelGainedEvent(
                event_type=EventType.LEVEL_GAINED,
                source_manager='class',
                timestamp=time.time(),
                class_id=0,
                new_level=5,
                total_level=5
            )
            
            feat_manager.on_level_gained(event)
            
            # Should have removed old feat and added new one
            feat_list = feat_manager.gff.get('FeatList', [])
            assert not any(f.get('Feat') == 40 for f in feat_list)  # Old rage removed
            assert {'Feat': 41} in feat_list  # New rage added
    
    def test_on_level_gained_unknown_class(self, feat_manager):
        """Test level gained with unknown class ID"""
        # Mock returning None for unknown class
        feat_manager.game_data_loader.get_by_id.return_value = None
        
        event = LevelGainedEvent(
            event_type=EventType.LEVEL_GAINED,
            source_manager='class',
            timestamp=time.time(),
            class_id=999,  # Unknown class
            new_level=2,
            total_level=2
        )
        
        # Should not crash
        feat_manager.on_level_gained(event)
        
        # Should not have added any feats
        feat_list = feat_manager.gff.get('FeatList', [])
        assert len(feat_list) == 0


class TestFeatProgression:
    """Test feat progression detection and replacement"""
    
    def test_check_feat_progression_numeric_pattern(self, feat_manager):
        """Test progression detection with numeric patterns (Rage -> Rage2)"""
        # Add base rage feat
        feat_manager.gff.set('FeatList', [{'Feat': 40}])  # BarbarianRage
        
        # Check if Rage2 should replace Rage
        old_feat_id = feat_manager._check_feat_progression(41, 0)  # BarbarianRage2
        
        assert old_feat_id == 40  # Should detect BarbarianRage as the one to replace
    
    def test_check_feat_progression_no_progression(self, feat_manager):
        """Test that non-progressive feats return None"""
        # Test with a feat that doesn't have progression
        old_feat_id = feat_manager._check_feat_progression(1, 0)  # WeaponFinesse
        
        assert old_feat_id is None
    
    def test_check_feat_progression_no_existing_feat(self, feat_manager):
        """Test progression detection when character doesn't have base feat"""
        # Character doesn't have BarbarianRage, try to add BarbarianRage2
        old_feat_id = feat_manager._check_feat_progression(41, 0)
        
        assert old_feat_id is None
    
    def test_check_feat_progression_base_to_numbered(self, feat_manager):
        """Test progression from base feat to numbered version"""
        # Add base rage feat with different naming
        feat_manager.gff.set('FeatList', [{'Feat': 40}])  # BarbarianRage
        
        # Mock feat data with proper labels
        feat_manager.game_data_loader.get_by_id = Mock(side_effect=lambda table, id: {
            ('feat', 41): MockFeat(41, 'BarbarianRage2'),
            ('feat', 40): MockFeat(40, 'BarbarianRage'),
        }.get((table, id)))
        
        old_feat_id = feat_manager._check_feat_progression(41, 0)
        
        assert old_feat_id == 40
    
    def test_check_feat_progression_unknown_feat(self, feat_manager):
        """Test progression detection with unknown feat"""
        # Mock returning None for unknown feat
        feat_manager.game_data_loader.get_by_id.return_value = None
        
        old_feat_id = feat_manager._check_feat_progression(99999, 0)
        
        assert old_feat_id is None


class TestProtectionMechanisms:
    """Test feat protection and custom content handling"""
    
    def test_is_feat_protected_epithet_feat(self, feat_manager):
        """Test that epithet feats are protected"""
        assert feat_manager.is_feat_protected(10001) is True
    
    def test_is_feat_protected_normal_feat(self, feat_manager):
        """Test that normal feats are not protected"""
        assert feat_manager.is_feat_protected(1) is False
    
    def test_update_protected_feats_custom_content(self, feat_manager):
        """Test that custom content feats are marked as protected"""
        # Add custom content feat to character manager
        feat_manager.character_manager.custom_content['feat_20000'] = {
            'type': 'feat',
            'id': 20000,
            'protected': True
        }
        
        feat_manager._update_protected_feats()
        
        assert 20000 in feat_manager._protected_feats
    
    def test_protection_during_removal(self, feat_manager):
        """Test that protection prevents feat removal"""
        # Add protected feat
        feat_manager.gff.get('FeatList').append({'Feat': 10001})
        
        # Try to remove without force
        result = feat_manager.remove_feat(10001, force=False)
        
        assert result is False
        # Feat should still be present
        feat_list = feat_manager.gff.get('FeatList', [])
        assert {'Feat': 10001} in feat_list


class TestDataDrivenArchitecture:
    """Test data-driven architecture compatibility"""
    
    def test_dynamic_column_access_primary_names(self, feat_manager):
        """Test accessing feat data with primary column names"""
        info = feat_manager.get_feat_info(10)  # Improved Critical
        
        # Should successfully access data regardless of column naming
        assert info is not None
        assert info['type'] == 1
    
    def test_dynamic_column_access_alternative_names(self, feat_manager):
        """Test accessing feat data with alternative column names"""
        # Create feat with only alternative column names
        mock_feat = Mock()
        mock_feat.id = 999
        mock_feat.label = 'TestFeat'
        mock_feat.name = 'Test Feat'
        mock_feat.feat_type = 2  # Only feat_type, not type
        del mock_feat.type  # Remove primary name
        
        feat_manager.game_data_loader.get_by_id = Mock(return_value=mock_feat)
        
        info = feat_manager.get_feat_info(999)
        
        assert info['type'] == 2  # Should find feat_type as fallback
    
    def test_prerequisite_validation_column_flexibility(self, feat_manager):
        """Test prerequisite validation with flexible column names"""
        # Create feat with alternative column names
        mock_feat = MockFeat(999, 'TestFeat', prereq_str=15)
        
        feat_manager.game_data_loader.get_by_id = Mock(return_value=mock_feat)
        
        # Should work with either column naming convention
        is_valid, errors = feat_manager.validate_feat_prerequisites(999)
        
        # Character has STR 16, so should pass
        assert is_valid is True
    
    def test_unknown_columns_ignored(self, feat_manager):
        """Test that unknown columns don't break functionality"""
        # Create feat with extra unknown columns
        mock_feat = Mock()
        mock_feat.id = 999
        mock_feat.label = 'TestFeat'
        mock_feat.type = 1
        mock_feat.unknown_column = 'should_be_ignored'
        mock_feat.mod_specific_data = [1, 2, 3]
        
        feat_manager.game_data_loader.get_by_id = Mock(return_value=mock_feat)
        
        # Should not crash and should work normally
        info = feat_manager.get_feat_info(999)
        assert info is not None
        assert info['type'] == 1


class TestQueryAndSummaryMethods:
    """Test feat querying and summary generation"""
    
    def test_get_available_feats_all(self, feat_manager):
        """Test getting all available feats"""
        available = feat_manager.get_available_feats()
        
        # Should return feats that character can take
        assert len(available) > 0
        # Should not include feats character already has
        feat_ids = [f['id'] for f in available]
        assert all(not feat_manager.has_feat(fid) for fid in feat_ids)
    
    def test_get_available_feats_by_type(self, feat_manager):
        """Test getting available feats filtered by type"""
        available = feat_manager.get_available_feats(feat_type=1)  # General feats only
        
        # Should only return general feats
        assert all(f['type'] == 1 for f in available)
    
    def test_get_available_feats_excludes_invalid_prerequisites(self, feat_manager):
        """Test that feats with unmet prerequisites are excluded"""
        available = feat_manager.get_available_feats()
        
        # Great Cleave should not be available without Power Attack
        available_ids = [f['id'] for f in available]
        assert 20 not in available_ids  # Great Cleave requires Power Attack
    
    def test_get_available_feats_includes_valid_prerequisites(self, feat_manager):
        """Test that feats with met prerequisites are included"""
        # Add Power Attack first
        feat_manager.add_feat(2)
        
        available = feat_manager.get_available_feats()
        available_ids = [f['id'] for f in available]
        
        # Great Cleave should now be available
        assert 20 in available_ids
    
    def test_get_feat_summary_categorization(self, feat_manager):
        """Test feat summary categorization"""
        # Add protected feat to the GFF directly to bypass validation
        feat_manager.gff.get('FeatList').append({'Feat': 10001})
        
        # Add normal feats
        feat_manager.add_feat(1)     # General feat
        feat_manager.add_feat(2)     # Another general feat
        
        summary = feat_manager.get_feat_summary()
        
        assert summary['total'] == 3
        assert len(summary['general_feats']) >= 2  # Both feat 1 and 2 are general
        assert len(summary['protected']) >= 1       # Protected feat 10001
    
    def test_get_feat_summary_empty_list(self, feat_manager):
        """Test feat summary with empty feat list"""
        feat_manager.gff.set('FeatList', [])
        
        summary = feat_manager.get_feat_summary()
        
        assert summary['total'] == 0
        assert len(summary['general_feats']) == 0
        assert len(summary['class_feats']) == 0
        assert len(summary['custom_feats']) == 0
        assert len(summary['protected']) == 0


class TestIntegrationScenarios:
    """Test integration scenarios and edge cases"""
    
    def test_add_class_feats_integration(self, feat_manager):
        """Test adding class feats integration"""
        # Mock class data
        mock_class = MockClass(0, 'FIGHTER', 'Fighter')
        feat_manager.game_data_loader.get_by_id.return_value = mock_class
        
        # Mock feats for levels 1-5
        feat_manager.character_manager.get_class_feats_for_level.return_value = [
            {'feat_id': 2, 'list_type': 0}  # Auto-granted PowerAttack
        ]
        
        feat_manager._add_class_feats(0, 5)
        
        # Should have added the feat
        feat_list = feat_manager.gff.get('FeatList', [])
        assert {'Feat': 2} in feat_list
    
    def test_remove_class_feats_integration(self, feat_manager):
        """Test removing class feats integration"""
        # Add some feats first
        feat_manager.add_feat(1)  # WeaponFinesse (preserve)
        feat_manager.add_feat(2)  # PowerAttack (remove)
        
        # Mock class data
        mock_class = MockClass(0, 'FIGHTER', 'Fighter')
        feat_manager.game_data_loader.get_by_id.return_value = mock_class
        
        # Mock feats for removal
        feat_manager.character_manager.get_class_feats_for_level.return_value = [
            {'feat_id': 2, 'list_type': 0}  # PowerAttack should be removed
        ]
        
        feat_manager._remove_class_feats(0, 5, preserve_list=[1])
        
        # PowerAttack should be removed, WeaponFinesse preserved
        feat_list = feat_manager.gff.get('FeatList', [])
        assert {'Feat': 1} in feat_list  # Preserved
        assert not any(f.get('Feat') == 2 for f in feat_list)  # Removed
    
    def test_validation_comprehensive(self, feat_manager):
        """Test comprehensive character validation"""
        # Add valid feats
        feat_manager.add_feat(1)  # WeaponFinesse
        feat_manager.add_feat(2)  # PowerAttack
        
        is_valid, errors = feat_manager.validate()
        
        assert is_valid is True
        assert errors == []
    
    def test_validation_duplicate_detection(self, feat_manager):
        """Test validation detects duplicate feats"""
        # Manually add duplicate feats
        feat_manager.gff.set('FeatList', [
            {'Feat': 1},
            {'Feat': 1}  # Duplicate
        ])
        
        is_valid, errors = feat_manager.validate()
        
        assert is_valid is False
        assert any('duplicate' in error.lower() for error in errors)
    
    def test_validation_prerequisite_checking(self, feat_manager):
        """Test validation checks all feat prerequisites"""
        # Add feat that character shouldn't have (missing prerequisites)
        feat_manager.gff.set('FeatList', [
            {'Feat': 20}  # Great Cleave without Power Attack
        ])
        
        is_valid, errors = feat_manager.validate()
        
        assert is_valid is False
        assert len(errors) > 0


class TestEdgeCasesAndErrorHandling:
    """Test edge cases and error handling"""
    
    def test_empty_feat_list_operations(self, feat_manager):
        """Test operations with empty feat list"""
        feat_manager.gff.set('FeatList', [])
        
        # All operations should work without crashing
        assert feat_manager.has_feat(1) is False
        assert feat_manager.remove_feat(1) is False
        
        summary = feat_manager.get_feat_summary()
        assert summary['total'] == 0
    
    def test_malformed_feat_list_entries(self, feat_manager):
        """Test handling of malformed feat list entries"""
        # Add malformed entries
        feat_manager.gff.set('FeatList', [
            {'Feat': 1},        # Valid
            {},                 # Missing Feat key
            {'Feat': None},     # None value
            {'NotFeat': 2}      # Wrong key
        ])
        
        # Should handle gracefully
        summary = feat_manager.get_feat_summary()
        assert summary['total'] == 4  # Should count all entries
        
        # has_feat should work
        assert feat_manager.has_feat(1) is True
        assert feat_manager.has_feat(2) is False
    
    def test_missing_game_data_graceful_handling(self, feat_manager):
        """Test graceful handling when game data is missing"""
        # Create fresh mocks that return empty data
        mock_loader = Mock()
        mock_loader.get_table.return_value = []
        mock_loader.get_by_id.return_value = None
        feat_manager.game_data_loader = mock_loader
        
        # Clear the feat cache to force fresh lookup
        feat_manager._feat_cache.clear()
        
        # Should not crash
        available = feat_manager.get_available_feats()
        assert available == []
        
        info = feat_manager.get_feat_info(999)
        assert info['custom'] is True  # Unknown feats marked as custom
    
    def test_large_feat_list_performance(self, feat_manager):
        """Test performance with large feat lists"""
        # Create large feat list
        large_feat_list = [{'Feat': i} for i in range(1, 1000)]
        feat_manager.gff.set('FeatList', large_feat_list)
        
        # Operations should still be reasonably fast
        start_time = time.time()
        
        # Test various operations
        feat_manager.has_feat(500)
        feat_manager.get_feat_summary()
        feat_manager.validate()
        
        end_time = time.time()
        
        # Should complete within reasonable time (1 second)
        assert end_time - start_time < 1.0
    
    def test_concurrent_modification_safety(self, feat_manager):
        """Test safety during concurrent modifications"""
        # This is more of a design test - ensure we don't have issues
        # with list modifications during iteration
        
        feat_manager.add_feat(1)
        feat_manager.add_feat(2)
        
        # Should be able to remove feat while others exist
        result = feat_manager.remove_feat(1)
        assert result is True
        
        # List should be in consistent state
        feat_list = feat_manager.gff.get('FeatList', [])
        assert {'Feat': 2} in feat_list
        assert not any(f.get('Feat') == 1 for f in feat_list)


if __name__ == '__main__':
    pytest.main([__file__])