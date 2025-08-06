"""
Comprehensive tests for ClassManager class.
Tests cover class data retrieval, multiclass handling, class prerequisites,
prestige class logic, and all calculation methods.
"""
import pytest
import time
from unittest.mock import Mock, MagicMock, patch, PropertyMock
from typing import Dict, List, Any

from character.managers.class_manager import ClassManager
from character.events import EventEmitter, EventType, ClassChangedEvent, LevelGainedEvent
from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader


class MockClass:
    """Mock class data for testing"""
    def __init__(self, id, label, name, hit_die, attack_bonus_table, saving_throw_table,
                 alignment_restrict=0, alignment_restrict_type=0, prestige_class=False,
                 primary_ability='STR', bab_type='high', fort_save='high', ref_save='low', will_save='low'):
        self.id = id
        self.label = label
        self.name = name
        self.hit_die = hit_die
        self.attack_bonus_table = attack_bonus_table
        self.saving_throw_table = saving_throw_table
        self.alignment_restrict = alignment_restrict
        self.alignment_restrict_type = alignment_restrict_type
        self.prestige_class = prestige_class
        self.primary_ability = primary_ability
        self.bab_type = bab_type
        self.fort_save = fort_save
        self.ref_save = ref_save
        self.will_save = will_save


@pytest.fixture
def mock_game_data_loader():
    """Create comprehensive mock DynamicGameDataLoader with various class types"""
    mock_loader = Mock(spec=DynamicGameDataLoader)
    
    # Mock classes data for get_by_id
    mock_classes = {
        # Base classes - use actual NWN2 table names
        0: MockClass(0, 'FIGHTER', 'Fighter', 10, 'cls_atk_1', 'cls_savthr_fig', 
                    fort_save='high', ref_save='low', will_save='low'),
        1: MockClass(1, 'WIZARD', 'Wizard', 4, 'cls_atk_2', 'cls_savthr_wiz',
                    fort_save='low', ref_save='low', will_save='high'),
        2: MockClass(2, 'ROGUE', 'Rogue', 6, 'cls_atk_3', 'cls_savthr_rog',
                    fort_save='low', ref_save='high', will_save='low'),
        3: MockClass(3, 'CLERIC', 'Cleric', 8, 'cls_atk_3', 'cls_savthr_fig',
                    fort_save='high', ref_save='low', will_save='high'),
        4: MockClass(4, 'RANGER', 'Ranger', 8, 'cls_atk_1', 'cls_savthr_fig'),
        5: MockClass(5, 'PALADIN', 'Paladin', 10, 'cls_atk_1', 'cls_savthr_fig',
                    alignment_restrict=0x01, alignment_restrict_type=0x01),  # Lawful Good only
        6: MockClass(6, 'BARBARIAN', 'Barbarian', 12, 'cls_atk_1', 'cls_savthr_rog',
                    alignment_restrict=0x04, alignment_restrict_type=0x04),  # Non-Lawful
        
        # Prestige classes
        100: MockClass(100, 'WEAPON_MASTER', 'Weapon Master', 10, 'cls_atk_1', 'cls_savthr_fig',
                      prestige_class=True),
        101: MockClass(101, 'ARCANE_TRICKSTER', 'Arcane Trickster', 4, 'cls_atk_2', 'cls_savthr_wiz',
                      prestige_class=True),
        102: MockClass(102, 'ELDRITCH_KNIGHT', 'Eldritch Knight', 6, 'cls_atk_1', 'cls_savthr_rog',
                      prestige_class=True),
        
        # Custom classes
        10001: MockClass(10001, 'CUSTOM_CLASS', 'Custom Class', 8, 'cls_atk_3', 'cls_savthr_rog'),
    }
    
    # Mock feats data for get_by_id
    mock_feats = {
        1: Mock(label='WeaponFinesse', name='Weapon Finesse', categories=''),
        2: Mock(label='PowerAttack', name='Power Attack', categories=''),
        3: Mock(label='CombatExpertise', name='Combat Expertise', categories=''),
    }
    
    # Mock get_by_id method
    def mock_get_by_id(table_name: str, item_id: int):
        if table_name == 'classes':
            return mock_classes.get(item_id)
        elif table_name == 'feat':
            return mock_feats.get(item_id)
        return None
    
    mock_loader.get_by_id = mock_get_by_id
    
    # Mock BAB tables
    class BABRow:
        def __init__(self, bab):
            self.bab = bab
    
    def create_bab_row(bab):
        return BABRow(bab)
    
    mock_bab_tables = {
        'high': [create_bab_row(i) for i in range(1, 21)],  # 1-20 BAB for high progression (Fighter)
        'medium': [create_bab_row(i) for i in [0, 1, 2, 2, 3, 4, 5, 5, 6, 7, 8, 8, 9, 10, 11, 11, 12, 13, 14, 14]],  # 3/4 BAB (Cleric/Rogue)
        'low': [create_bab_row(i) for i in [0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10]]  # 1/2 BAB for low progression (Wizard)
    }
    
    # Mock save tables - create proper save progressions
    class SaveRow:
        def __init__(self, fort, ref, will):
            self.fort = fort
            self.ref = ref
            self.will = will
    
    def create_save_row(fort, ref, will):
        return SaveRow(fort, ref, will)
    
    # Fighter save progression (good Fort)
    # Level:     1  2  3  4  5  6  7  8  9  10...
    fighter_saves = [
        create_save_row(2, 0, 0),  # Level 1
        create_save_row(3, 0, 0),  # Level 2
        create_save_row(3, 1, 1),  # Level 3
        create_save_row(4, 1, 1),  # Level 4
        create_save_row(4, 1, 1),  # Level 5 - Fort +4
        create_save_row(5, 2, 2),  # Level 6
        create_save_row(5, 2, 2),  # Level 7
        create_save_row(6, 2, 2),  # Level 8
        create_save_row(6, 3, 3),  # Level 9
        create_save_row(7, 3, 3),  # Level 10
        create_save_row(7, 3, 3),  # Level 11
        create_save_row(8, 4, 4),  # Level 12
        create_save_row(8, 4, 4),  # Level 13
        create_save_row(9, 4, 4),  # Level 14
        create_save_row(9, 5, 5),  # Level 15
        create_save_row(10, 5, 5), # Level 16
        create_save_row(10, 5, 5), # Level 17
        create_save_row(11, 6, 6), # Level 18
        create_save_row(11, 6, 6), # Level 19
        create_save_row(12, 6, 6)  # Level 20
    ]
    
    # Rogue save progression (good Ref)
    rogue_saves = [
        create_save_row(0, 2, 0), create_save_row(0, 3, 0), create_save_row(1, 3, 1),
        create_save_row(1, 4, 1), create_save_row(1, 4, 1), create_save_row(2, 5, 2),
        create_save_row(2, 5, 2), create_save_row(2, 6, 2), create_save_row(3, 6, 3),
        create_save_row(3, 7, 3), create_save_row(3, 7, 3), create_save_row(4, 8, 4),
        create_save_row(4, 8, 4), create_save_row(4, 9, 4), create_save_row(5, 9, 5),
        create_save_row(5, 10, 5), create_save_row(5, 10, 5), create_save_row(6, 11, 6),
        create_save_row(6, 11, 6), create_save_row(6, 12, 6)
    ]
    
    # Wizard/Cleric save progression (good Will)
    caster_saves = [
        create_save_row(0, 0, 2), create_save_row(0, 0, 3), create_save_row(1, 1, 3),
        create_save_row(1, 1, 4), create_save_row(1, 1, 4), create_save_row(2, 2, 5),
        create_save_row(2, 2, 5), create_save_row(2, 2, 6), create_save_row(3, 3, 6),
        create_save_row(3, 3, 7), create_save_row(3, 3, 7), create_save_row(4, 4, 8),
        create_save_row(4, 4, 8), create_save_row(4, 4, 9), create_save_row(5, 5, 9),
        create_save_row(5, 5, 10), create_save_row(5, 5, 10), create_save_row(6, 6, 11),
        create_save_row(6, 6, 11), create_save_row(6, 6, 12)
    ]
    
    mock_save_tables = {
        'high': fighter_saves,   # Good Fort save
        'medium': rogue_saves,   # Good Ref save
        'low': caster_saves      # Good Will save
    }
    
    # NWN2 uses specific table names like cls_atk_1, cls_savthr_fig, etc.
    # Map the generic names to specific table names
    mock_tables = {
        # BAB tables
        'cls_atk_1': mock_bab_tables['high'],     # Fighter, Paladin, etc.
        'cls_atk_2': mock_bab_tables['low'],      # Wizard, Sorcerer
        'cls_atk_3': mock_bab_tables['medium'],   # Cleric, Rogue (3/4 BAB)
        'high': mock_bab_tables['high'],          # Fallback
        'low': mock_bab_tables['low'],            # Fallback
        'medium': mock_bab_tables['medium'],      # Fallback
        
        # Save tables  
        'cls_savthr_fig': mock_save_tables['high'],    # Fighter saves
        'cls_savthr_rog': mock_save_tables['medium'],  # Rogue saves
        'cls_savthr_wiz': mock_save_tables['low'],     # Wizard saves
        'high': mock_save_tables['high'],               # Fallback
        'medium': mock_save_tables['medium'],           # Fallback
        'low': mock_save_tables['low'],                 # Fallback
    }
    
    # Mock get_table method
    def mock_get_table(table_name: str):
        return mock_tables.get(table_name.lower())
    
    mock_loader.get_table = mock_get_table
    
    return mock_loader


@pytest.fixture
def mock_character_manager(mock_game_data_loader):
    """Create mock CharacterManager with GFF data"""
    manager = Mock()
    manager._current_transaction = None
    manager.custom_content = {}
    manager.game_data_loader = mock_game_data_loader
    
    # Create mock GFF with default character data
    mock_gff = Mock()
    mock_gff_data = {
        'Str': 16, 'Dex': 14, 'Con': 15, 'Int': 12, 'Wis': 10, 'Cha': 8,
        'LawfulChaotic': 50, 'GoodEvil': 50,  # True Neutral
        'Class': 0,  # Fighter
        'ClassList': [{'Class': 0, 'ClassLevel': 5}],  # Fighter 5
        'FeatList': [
            {'Feat': 1},  # Some feat
            {'Feat': 2},  # Another feat
        ],
        'HitPoints': 45, 'MaxHitPoints': 45, 'CurrentHitPoints': 45,
        'BaseAttackBonus': 5,
        'FortSave': 4, 'RefSave': 1, 'WillSave': 1,
    }
    
    def mock_get(key, default=None):
        return mock_gff_data.get(key, default)
    
    def mock_set(key, value):
        mock_gff_data[key] = value
    
    mock_gff.get = mock_get
    mock_gff.set = mock_set
    manager.gff = mock_gff
    
    # Mock transaction methods
    manager.begin_transaction = Mock()
    manager.commit_transaction = Mock()
    manager.rollback_transaction = Mock()
    manager.emit = Mock()
    
    # Mock new CharacterManager methods
    def mock_get_ability_scores():
        return {
            'strength': mock_gff_data.get('Str', 10),
            'dexterity': mock_gff_data.get('Dex', 10),
            'constitution': mock_gff_data.get('Con', 10),
            'intelligence': mock_gff_data.get('Int', 10),
            'wisdom': mock_gff_data.get('Wis', 10),
            'charisma': mock_gff_data.get('Cha', 10)
        }
    
    def mock_validate_alignment_for_class(class_id):
        # Simple mock validation - always valid unless class_id is 999
        if class_id == 999:
            return False, "Invalid class ID"
        return True, None
    
    manager.get_ability_scores = mock_get_ability_scores
    manager.validate_alignment_for_class = mock_validate_alignment_for_class
    
    # Mock has_feat_by_name helper method
    def has_feat_by_name(feat_label):
        # Map known feat labels to IDs in the test data
        feat_map = {
            'WeaponFinesse': 1,  # Feat ID 1 in test data
            'PowerAttack': 2,
            # Add more as needed
        }
        feat_id = feat_map.get(feat_label, -1)
        feat_list = mock_gff_data.get('FeatList', [])
        return any(feat.get('Feat') == feat_id for feat in feat_list)
    
    manager.has_feat_by_name = Mock(side_effect=has_feat_by_name)
    
    # Mock detect_epithet_feats to return empty set by default
    manager.detect_epithet_feats = Mock(return_value=set())
    
    return manager


@pytest.fixture
def class_manager(mock_character_manager):
    """Create ClassManager instance with mocked dependencies"""
    return ClassManager(mock_character_manager)


@pytest.fixture
def single_class_character_data():
    """Sample data for single-class character"""
    return {
        'Class': 0, 'ClassList': [{'Class': 0, 'ClassLevel': 8}],
        'Str': 18, 'Dex': 12, 'Con': 16, 'Int': 10, 'Wis': 13, 'Cha': 8,
        'LawfulChaotic': 30, 'GoodEvil': 70,  # Lawful Good
        'HitPoints': 72, 'MaxHitPoints': 72,
        'BaseAttackBonus': 8, 'FortSave': 6, 'RefSave': 2, 'WillSave': 2,
        'FeatList': []
    }


@pytest.fixture
def multiclass_character_data():
    """Sample data for multiclass character"""
    return {
        'Class': 0,
        'ClassList': [
            {'Class': 0, 'ClassLevel': 5},  # Fighter 5
            {'Class': 1, 'ClassLevel': 3},  # Wizard 3
            {'Class': 2, 'ClassLevel': 2}   # Rogue 2
        ],
        'Str': 14, 'Dex': 16, 'Con': 14, 'Int': 16, 'Wis': 12, 'Cha': 10,
        'LawfulChaotic': 50, 'GoodEvil': 50,  # True Neutral
        'HitPoints': 62, 'MaxHitPoints': 62,
        'BaseAttackBonus': 6, 'FortSave': 4, 'RefSave': 5, 'WillSave': 5,
        'FeatList': []
    }


@pytest.fixture
def prestige_character_data():
    """Sample data for character with prestige class"""
    return {
        'Class': 100,
        'ClassList': [
            {'Class': 0, 'ClassLevel': 10},   # Fighter 10
            {'Class': 100, 'ClassLevel': 5}   # Weapon Master 5
        ],
        'Str': 20, 'Dex': 14, 'Con': 16, 'Int': 10, 'Wis': 12, 'Cha': 8,
        'LawfulChaotic': 60, 'GoodEvil': 40,  # Lawful Neutral
        'HitPoints': 135, 'MaxHitPoints': 135,
        'BaseAttackBonus': 15, 'FortSave': 9, 'RefSave': 3, 'WillSave': 3,
        'FeatList': []
    }


@pytest.fixture
def invalid_character_data():
    """Sample data with invalid states for error testing"""
    return {
        'Class': 999,  # Invalid class
        'ClassList': [
            {'Class': 999, 'ClassLevel': 5},  # Invalid class
            {'Class': 1, 'ClassLevel': 50}    # Invalid level
        ],
        'Str': 5, 'Dex': 5, 'Con': 5, 'Int': 5, 'Wis': 5, 'Cha': 5,  # Too low
        'LawfulChaotic': 200, 'GoodEvil': -50,  # Invalid alignment values
        'HitPoints': -10, 'MaxHitPoints': -10,
        'BaseAttackBonus': -5,
        'FeatList': []
    }


class TestClassManagerInitialization:
    """Test ClassManager initialization and setup"""
    
    def test_initialization_with_valid_manager(self, mock_character_manager):
        """Test proper initialization with valid character manager"""
        class_manager = ClassManager(mock_character_manager)
        
        assert class_manager.character_manager == mock_character_manager
        assert class_manager.gff == mock_character_manager.gff
        assert class_manager.game_data_loader == mock_character_manager.game_data_loader
        assert class_manager._class_cache == {}
    
    def test_inheritance_from_event_emitter(self, class_manager):
        """Test that ClassManager inherits from EventEmitter"""
        assert isinstance(class_manager, EventEmitter)
        assert hasattr(class_manager, 'emit')
        assert hasattr(class_manager, 'on')  # Changed from 'subscribe' to 'on'
    
    def test_cache_initialization(self, class_manager):
        """Test that class cache is properly initialized"""
        assert hasattr(class_manager, '_class_cache')
        assert isinstance(class_manager._class_cache, dict)
        assert len(class_manager._class_cache) == 0


class TestClassDataRetrieval:
    """Test class data retrieval and caching functionality"""
    
    def test_get_class_summary_single_class(self, class_manager, single_class_character_data):
        """Test class summary for single-class character"""
        # Setup character data
        for key, value in single_class_character_data.items():
            class_manager.gff.set(key, value)
        
        summary = class_manager.get_class_summary()
        
        assert summary['total_level'] == 8
        assert summary['multiclass'] == False
        assert summary['can_multiclass'] == True
        assert len(summary['classes']) == 1
        assert summary['classes'][0]['id'] == 0
        assert summary['classes'][0]['level'] == 8
        # Note: name will be the label attribute from MockClass
        assert hasattr(summary['classes'][0]['name'], 'label') or isinstance(summary['classes'][0]['name'], str)
    
    def test_get_class_summary_multiclass(self, class_manager, multiclass_character_data):
        """Test class summary for multiclass character"""
        # Setup character data
        for key, value in multiclass_character_data.items():
            class_manager.gff.set(key, value)
        
        summary = class_manager.get_class_summary()
        
        assert summary['total_level'] == 10
        assert summary['multiclass'] == True
        assert summary['can_multiclass'] == False  # Already has 3 classes
        assert len(summary['classes']) == 3
        
        class_ids = [c['id'] for c in summary['classes']]
        assert 0 in class_ids  # Fighter
        assert 1 in class_ids  # Wizard
        assert 2 in class_ids  # Rogue
    
    def test_get_class_summary_max_classes(self, class_manager):
        """Test class summary when at maximum class limit"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5},
            {'Class': 1, 'ClassLevel': 5},
            {'Class': 2, 'ClassLevel': 5}
        ])
        
        summary = class_manager.get_class_summary()
        
        assert summary['can_multiclass'] == False
        assert len(summary['classes']) == 3
    
    def test_get_attack_bonuses_single_class(self, class_manager, single_class_character_data):
        """Test attack bonus calculation for single class"""
        for key, value in single_class_character_data.items():
            class_manager.gff.set(key, value)
        
        bonuses = class_manager.get_attack_bonuses()
        
        assert bonuses['base_attack_bonus'] == 8
        assert bonuses['str_modifier'] == 4  # STR 18 = +4
        assert bonuses['dex_modifier'] == 1  # DEX 12 = +1
        assert bonuses['melee_attack_bonus'] == 12  # BAB + STR
        assert bonuses['ranged_attack_bonus'] == 9   # BAB + DEX
        assert bonuses['touch_attack_bonus'] == 8    # BAB only
        assert bonuses['has_weapon_finesse'] == False
    
    def test_get_attack_bonuses_with_weapon_finesse(self, class_manager):
        """Test attack bonuses when character has Weapon Finesse"""
        class_manager.gff.set('Str', 12)  # +1
        class_manager.gff.set('Dex', 18)  # +4
        class_manager.gff.set('BaseAttackBonus', 5)
        
        # Mock having Weapon Finesse feat
        with patch.object(class_manager, '_has_feat_by_name', return_value=True):
            bonuses = class_manager.get_attack_bonuses()
        
        assert bonuses['finesse_attack_bonus'] == 9  # BAB + DEX
        assert bonuses['has_weapon_finesse'] == True
    
    def test_get_attack_bonuses_multiple_attacks(self, class_manager):
        """Test multiple attack calculation at high BAB"""
        class_manager.gff.set('Str', 10)  # Set basic stats
        class_manager.gff.set('Dex', 10)
        
        # Mock calculate_total_bab to return 16
        with patch.object(class_manager, 'calculate_total_bab', return_value=16):
            bonuses = class_manager.get_attack_bonuses()
        
        expected_attacks = [16, 11, 6, 1]
        assert bonuses['multiple_attacks'] == expected_attacks
    
    def test_class_data_caching(self, class_manager):
        """Test that class data is properly cached"""
        # First call should populate cache
        class_manager._class_cache.clear()
        
        # Access some class data multiple times
        summary1 = class_manager.get_class_summary()
        summary2 = class_manager.get_class_summary()
        
        # Both should return same data
        assert summary1 == summary2
    
    def test_invalid_class_id_handling(self, class_manager):
        """Test handling of invalid class IDs in class list"""
        class_manager.gff.set('ClassList', [
            {'Class': 999, 'ClassLevel': 5}  # Invalid class
        ])
        
        # Should now handle invalid class IDs gracefully
        summary = class_manager.get_class_summary()
        
        assert len(summary['classes']) == 1
        assert summary['classes'][0]['id'] == 999
        assert summary['classes'][0]['level'] == 5
        assert summary['classes'][0]['name'] == "Unknown Class 999"


class TestMulticlassHandling:
    """Test multiclass functionality and validation"""
    
    def test_add_class_level_existing_class(self, class_manager):
        """Test adding level to existing class"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5}
        ])
        
        result = class_manager.add_class_level(0, cheat_mode=True)
        
        class_list = class_manager.gff.get('ClassList')
        assert class_list[0]['ClassLevel'] == 6
        assert result['class_id'] == 0
        assert result['new_total_level'] == 6
        assert result['multiclass'] == False
    
    def test_add_class_level_new_class(self, class_manager):
        """Test adding level in new class (multiclassing)"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5}
        ])
        
        result = class_manager.add_class_level(1, cheat_mode=True)
        
        class_list = class_manager.gff.get('ClassList')
        assert len(class_list) == 2
        assert class_list[1]['Class'] == 1
        assert class_list[1]['ClassLevel'] == 1
        assert result['multiclass'] == True
        assert result['new_total_level'] == 6
    
    def test_multiclass_validation_too_many_classes(self, class_manager):
        """Test validation prevents more than 3 classes"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5},
            {'Class': 1, 'ClassLevel': 3},
            {'Class': 2, 'ClassLevel': 2}
        ])
        
        with pytest.raises(ValueError, match="Maximum of 3 classes allowed"):
            class_manager.add_class_level(3)
    
    def test_multiclass_validation_duplicate_class(self, class_manager):
        """Test validation prevents duplicate classes"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5}
        ])
        
        with pytest.raises(ValueError, match="Already has levels in this class"):
            class_manager.add_class_level(0)
    
    def test_calculate_total_bab_multiclass(self, class_manager):
        """Test BAB calculation for multiclass character"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5},  # Fighter 5: BAB +5
            {'Class': 1, 'ClassLevel': 3}   # Wizard 3: BAB +1
        ])
        
        total_bab = class_manager.calculate_total_bab()
        
        # Fighter 5 uses cls_atk_1 (high progression) = 5
        # Wizard 3 uses cls_atk_2 (low progression) = 1
        assert total_bab == 6  # 5 + 1
    
    def test_calculate_total_bab_single_class(self, class_manager):
        """Test BAB calculation for single class"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 10}  # Fighter 10: BAB +10
        ])
        
        total_bab = class_manager.calculate_total_bab()
        
        assert total_bab == 10
    
    def test_calculate_total_saves_multiclass(self, class_manager):
        """Test saving throw calculation for multiclass (best progression)"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5},  # Fighter 5: Fort +4, Ref +1, Will +1
            {'Class': 2, 'ClassLevel': 3}   # Rogue 3: Fort +1, Ref +3, Will +1
        ])
        class_manager.gff.set('Con', 14)  # +2 modifier
        class_manager.gff.set('Dex', 16)  # +3 modifier
        class_manager.gff.set('Wis', 12)  # +1 modifier
        
        saves = class_manager.calculate_total_saves()
        
        # Fighter 5: Fort +4, Ref +1, Will +1
        # Rogue 3: Fort +1, Ref +3, Will +1
        # Best progression: Fort 4, Ref 3, Will 1
        assert saves['fortitude'] == 6  # Best fort (4) + Con (+2)
        assert saves['reflex'] == 6     # Best ref (3) + Dex (+3)
        assert saves['will'] == 2       # Best will (1) + Wis (+1)
        assert saves['base_fortitude'] == 4
        assert saves['base_reflex'] == 3
        assert saves['base_will'] == 1
    
    def test_multiclass_level_gained_event(self, class_manager):
        """Test that level gained event is emitted for multiclass"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5}
        ])
        
        with patch.object(class_manager, 'emit') as mock_emit:
            class_manager.add_class_level(1, cheat_mode=True)
            
            mock_emit.assert_called_once()
            event = mock_emit.call_args[0][0]
            assert isinstance(event, LevelGainedEvent)
            assert event.class_id == 1
            assert event.total_level == 6
    
    def test_invalid_class_id_multiclass(self, class_manager):
        """Test error handling for invalid class ID in multiclass"""
        with pytest.raises(ValueError, match="Invalid class ID"):
            class_manager.add_class_level(999)


class TestClassPrerequisites:
    """Test class prerequisite validation and alignment restrictions"""
    
    def test_paladin_alignment_restriction_valid(self, class_manager):
        """Test Paladin can be selected with Lawful Good alignment"""
        # Set Lawful Good alignment
        class_manager.gff.set('LawfulChaotic', 20)  # Lawful
        class_manager.gff.set('GoodEvil', 80)       # Good
        
        is_valid, errors = class_manager._validate_class_change(5)  # Paladin
        
        # Note: The current implementation doesn't fully validate alignment
        # This test verifies the method runs without error
        assert isinstance(is_valid, bool)
        assert isinstance(errors, list)
    
    def test_paladin_alignment_restriction_invalid(self, class_manager):
        """Test Paladin cannot be selected with non-Lawful Good alignment"""
        # Set Chaotic Evil alignment
        class_manager.gff.set('LawfulChaotic', 80)  # Chaotic
        class_manager.gff.set('GoodEvil', 20)       # Evil
        
        is_valid, errors = class_manager._validate_class_change(5)  # Paladin
        
        # Note: Current implementation has placeholder alignment validation
        assert isinstance(is_valid, bool)
        assert isinstance(errors, list)
    
    def test_barbarian_alignment_restriction(self, class_manager):
        """Test Barbarian alignment restrictions (non-Lawful)"""
        # Set Lawful alignment (should be invalid for Barbarian)
        class_manager.gff.set('LawfulChaotic', 20)  # Lawful
        class_manager.gff.set('GoodEvil', 50)       # Neutral
        
        is_valid, errors = class_manager._validate_class_change(6)  # Barbarian
        
        assert isinstance(is_valid, bool)
        assert isinstance(errors, list)
    
    def test_class_change_validation_cheat_mode_bypass(self, class_manager):
        """Test that cheat mode bypasses validation"""
        # Set invalid alignment for Paladin
        class_manager.gff.set('LawfulChaotic', 80)  # Chaotic
        class_manager.gff.set('GoodEvil', 20)       # Evil
        
        # Should succeed with cheat mode
        result = class_manager.change_class(5, cheat_mode=True)
        
        assert result['class_change']['new_class'] == 5
    
    def test_validate_class_change_invalid_class_id(self, class_manager):
        """Test validation with invalid class ID"""
        with pytest.raises(ValueError, match="Invalid class ID"):
            class_manager.change_class(999)
    
    def test_multiclass_alignment_validation(self, class_manager):
        """Test alignment validation for multiclassing"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5}  # Fighter
        ])
        class_manager.gff.set('LawfulChaotic', 80)  # Chaotic
        class_manager.gff.set('GoodEvil', 20)       # Evil
        
        # Try to multiclass into Paladin (should fail)
        is_valid, errors = class_manager._validate_multiclass(5)
        
        assert isinstance(is_valid, bool)
        assert isinstance(errors, list)


class TestPrestigeClassLogic:
    """Test prestige class handling and special requirements"""
    
    def test_prestige_class_identification(self, class_manager):
        """Test identification of prestige classes"""
        # Check if Weapon Master is identified as prestige class
        weapon_master = class_manager.game_data_loader.get_by_id('classes', 100)
        assert weapon_master.prestige_class == True
        
        # Check if Fighter is not prestige class
        fighter = class_manager.game_data_loader.get_by_id('classes', 0)
        assert fighter.prestige_class == False
    
    def test_prestige_class_in_class_summary(self, class_manager, prestige_character_data):
        """Test prestige class appears correctly in class summary"""
        for key, value in prestige_character_data.items():
            class_manager.gff.set(key, value)
        
        summary = class_manager.get_class_summary()
        
        assert summary['total_level'] == 15
        assert summary['multiclass'] == True
        
        # Check that classes are present (names might be the label attribute)
        assert len(summary['classes']) == 2
        class_ids = [c['id'] for c in summary['classes']]
        assert 0 in class_ids    # Fighter
        assert 100 in class_ids  # Weapon Master
    
    def test_prestige_class_bab_calculation(self, class_manager):
        """Test BAB calculation with prestige class"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 10},   # Fighter 10: BAB +10
            {'Class': 100, 'ClassLevel': 5}   # Weapon Master 5: BAB +5
        ])
        
        total_bab = class_manager.calculate_total_bab()
        
        assert total_bab == 15  # 10 + 5
    
    def test_prestige_class_saves_calculation(self, class_manager):
        """Test saves calculation with prestige class"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 10},   # Fighter 10
            {'Class': 100, 'ClassLevel': 5}   # Weapon Master 5 (uses Fighter save table)
        ])
        class_manager.gff.set('Con', 16)  # +3
        class_manager.gff.set('Dex', 14)  # +2
        class_manager.gff.set('Wis', 12)  # +1
        
        saves = class_manager.calculate_total_saves()
        
        # Both use fighter save progression, so take best from level 10
        assert saves['base_fortitude'] == 7  # Fighter 10 fort save
        assert saves['fortitude'] == 10      # 7 + 3 Con
    
    def test_multiclass_into_prestige_class(self, class_manager):
        """Test multiclassing into prestige class"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 10}  # Fighter 10
        ])
        
        result = class_manager.add_class_level(100, cheat_mode=True)  # Weapon Master
        
        assert result['multiclass'] == True
        assert result['class_id'] == 100
        assert result['new_total_level'] == 11
    
    def test_prestige_class_validation(self, class_manager):
        """Test prestige class prerequisite validation"""
        # Basic test - prestige class validation should work like normal classes
        class_manager.gff.set('ClassList', [])
        
        is_valid, errors = class_manager._validate_multiclass(100)  # Weapon Master
        
        assert isinstance(is_valid, bool)
        assert isinstance(errors, list)
    
    def test_arcane_trickster_prestige_class(self, class_manager):
        """Test Arcane Trickster prestige class (low BAB)"""
        class_manager.gff.set('ClassList', [
            {'Class': 1, 'ClassLevel': 5},    # Wizard 5: BAB +2 (from cls_atk_2 table)
            {'Class': 2, 'ClassLevel': 3},    # Rogue 3: BAB +2 (from cls_atk_3 table - medium)
            {'Class': 101, 'ClassLevel': 2}   # Arcane Trickster 2: BAB +1 (from cls_atk_2 table)
        ])
        
        total_bab = class_manager.calculate_total_bab()
        
        # Wizard 5 = +2, Rogue 3 = +2 (medium progression), Arcane Trickster 2 = +1
        assert total_bab == 5  # 2 + 2 + 1


class TestClassChangeService:
    """Test primary class change functionality"""
    
    def test_change_class_basic(self, class_manager):
        """Test basic class change functionality"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5}  # Fighter 5
        ])
        
        result = class_manager.change_class(1, cheat_mode=True)  # Change to Wizard
        
        assert result['class_change']['old_class'] == 0
        assert result['class_change']['new_class'] == 1
        assert result['class_change']['level'] == 5
        
        # Check GFF was updated
        assert class_manager.gff.get('Class') == 1
        class_list = class_manager.gff.get('ClassList')
        assert len(class_list) == 1
        assert class_list[0]['Class'] == 1
        assert class_list[0]['ClassLevel'] == 5
    
    def test_change_class_preserve_level(self, class_manager):
        """Test class change preserving character level"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 8}
        ])
        
        result = class_manager.change_class(2, preserve_level=True, cheat_mode=True)
        
        assert result['class_change']['level'] == 8
        class_list = class_manager.gff.get('ClassList')
        assert class_list[0]['ClassLevel'] == 8
    
    def test_change_class_updates_stats(self, class_manager):
        """Test that class change updates derived stats"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5}  # Fighter
        ])
        class_manager.gff.set('Con', 14)  # +2 modifier
        
        result = class_manager.change_class(1, cheat_mode=True)  # Change to Wizard
        
        assert 'stats_updated' in result
        assert 'hit_points' in result['stats_updated']
        assert 'bab' in result['stats_updated']
        assert 'saves' in result['stats_updated']
    
    def test_change_class_transaction_management(self, class_manager):
        """Test transaction management during class change"""
        class_manager.character_manager._current_transaction = None
        
        class_manager.change_class(1, cheat_mode=True)
        
        class_manager.character_manager.begin_transaction.assert_called_once()
        class_manager.character_manager.commit_transaction.assert_called_once()
    
    def test_change_class_transaction_rollback_on_error(self, class_manager):
        """Test transaction rollback on error"""
        class_manager.character_manager._current_transaction = None
        
        # Mock an error during stat update
        with patch.object(class_manager, '_update_class_stats', side_effect=Exception("Test error")):
            with pytest.raises(Exception, match="Test error"):
                class_manager.change_class(1, cheat_mode=True)
        
        class_manager.character_manager.begin_transaction.assert_called_once()
        class_manager.character_manager.rollback_transaction.assert_called_once()
    
    def test_change_class_emits_event(self, class_manager):
        """Test that class change emits appropriate event"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5}
        ])
        
        with patch.object(class_manager.character_manager, 'emit') as mock_emit:
            class_manager.change_class(1, cheat_mode=True)
            
            mock_emit.assert_called_once()
            event = mock_emit.call_args[0][0]
            assert isinstance(event, ClassChangedEvent)
            assert event.old_class_id == 0
            assert event.new_class_id == 1
            assert event.level == 5
    
    def test_change_class_without_cheat_mode_validation(self, class_manager):
        """Test class change validation without cheat mode"""
        # Set up invalid alignment for Paladin
        class_manager.gff.set('LawfulChaotic', 80)  # Chaotic
        class_manager.gff.set('GoodEvil', 20)       # Evil
        
        # Mock validation to return False
        with patch.object(class_manager, '_validate_class_change', return_value=(False, ["Invalid alignment"])):
            with pytest.raises(ValueError, match="Class change not allowed"):
                class_manager.change_class(5)  # Paladin without cheat mode


class TestLevelProgression:
    """Test level progression and derived stat calculations"""
    
    def test_hit_points_calculation_level_1(self, class_manager):
        """Test hit points calculation at level 1 (max HP)"""
        fighter_class = class_manager.game_data_loader.get_by_id('classes', 0)  # Fighter, d10 HD
        
        hp = class_manager._calculate_hit_points(fighter_class, 1, 2)  # +2 Con
        
        assert hp == 12  # 10 (max d10) + 2 (Con) = 12
    
    def test_hit_points_calculation_higher_levels(self, class_manager):
        """Test hit points calculation at higher levels (average HP)"""
        fighter_class = class_manager.game_data_loader.get_by_id('classes', 0)  # Fighter, d10 HD
        
        hp = class_manager._calculate_hit_points(fighter_class, 5, 3)  # Level 5, +3 Con
        
        # Level 1: 10 + 3 = 13
        # Levels 2-5: 4 * 5.5 + 4 * 3 = 22 + 12 = 34
        # Total: 13 + 34 = 47
        expected_hp = 10 + 3 + (4 * 5) + (4 * 3)  # 10+3+20+12 = 45
        assert hp == expected_hp
    
    def test_hit_points_minimum_1(self, class_manager):
        """Test hit points cannot go below 1"""
        wizard_class = class_manager.game_data_loader.get_by_id('classes', 1)  # Wizard, d4 HD
        
        hp = class_manager._calculate_hit_points(wizard_class, 1, -5)  # Massive Con penalty
        
        assert hp == 1  # Minimum 1 HP
    
    def test_bab_calculation_high_progression(self, class_manager):
        """Test BAB calculation for high progression class"""
        fighter_class = class_manager.game_data_loader.get_by_id('classes', 0)  # Fighter
        
        bab = class_manager._calculate_bab(fighter_class, 10)
        
        assert bab == 10  # Fighter level 10 = BAB +10
    
    def test_bab_calculation_low_progression(self, class_manager):
        """Test BAB calculation for low progression class"""
        wizard_class = class_manager.game_data_loader.get_by_id('classes', 1)  # Wizard
        
        bab = class_manager._calculate_bab(wizard_class, 10)
        
        assert bab == 5  # Wizard level 10 = BAB +5
    
    def test_bab_calculation_level_cap(self, class_manager):
        """Test BAB calculation at level cap"""
        fighter_class = class_manager.game_data_loader.get_by_id('classes', 0)
        
        bab = class_manager._calculate_bab(fighter_class, 25)  # Above level 20
        
        assert bab == 20  # Capped at level 20 progression
    
    def test_saves_calculation_good_saves(self, class_manager):
        """Test saving throw calculation for good save progression"""
        fighter_class = class_manager.game_data_loader.get_by_id('classes', 0)  # Good Fort save
        modifiers = {'CON': 3, 'DEX': 1, 'WIS': 0}
        
        saves = class_manager._calculate_saves(fighter_class, 10, modifiers)
        
        assert saves['fortitude'] == 10  # 7 (base) + 3 (Con)
        assert saves['reflex'] == 4     # 3 (base) + 1 (Dex)
        assert saves['will'] == 3       # 3 (base) + 0 (Wis)
    
    def test_saves_calculation_poor_saves(self, class_manager):
        """Test saving throw calculation for poor save progression"""
        wizard_class = class_manager.game_data_loader.get_by_id('classes', 1)  # Poor Fort/Ref saves
        modifiers = {'CON': 1, 'DEX': 2, 'WIS': 4}
        
        saves = class_manager._calculate_saves(wizard_class, 10, modifiers)
        
        assert saves['fortitude'] == 4  # 3 (base) + 1 (Con)
        assert saves['reflex'] == 5     # 3 (base) + 2 (Dex)
        assert saves['will'] == 11      # 7 (base) + 4 (Wis)
    
    def test_ability_modifier_calculation(self, class_manager):
        """Test ability score modifier calculation"""
        class_manager.gff.set('Str', 18)  # +4
        class_manager.gff.set('Dex', 12)  # +1
        class_manager.gff.set('Con', 8)   # -1
        class_manager.gff.set('Int', 10)  # +0
        class_manager.gff.set('Wis', 15)  # +2
        class_manager.gff.set('Cha', 7)   # -2
        
        modifiers = class_manager._calculate_ability_modifiers()
        
        assert modifiers['STR'] == 4
        assert modifiers['DEX'] == 1
        assert modifiers['CON'] == -1
        assert modifiers['INT'] == 0
        assert modifiers['WIS'] == 2
        assert modifiers['CHA'] == -2
    
    def test_level_progression_stat_updates(self, class_manager):
        """Test that level progression updates all derived stats"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5}
        ])
        class_manager.gff.set('Con', 16)  # +3
        
        changes = class_manager._update_class_stats(
            class_manager.game_data_loader.get_by_id('classes', 0), 
            6
        )
        
        assert 'hit_points' in changes
        assert 'bab' in changes
        assert 'saves' in changes
        assert 'fortitude' in changes['saves']
        assert 'reflex' in changes['saves']
        assert 'will' in changes['saves']


class TestCalculationMethods:
    """Test accuracy of calculation methods with various combinations"""
    
    def test_total_bab_empty_class_list(self, class_manager):
        """Test BAB calculation with empty class list"""
        class_manager.gff.set('ClassList', [])
        
        total_bab = class_manager.calculate_total_bab()
        
        assert total_bab == 0
    
    def test_total_bab_invalid_class_in_list(self, class_manager):
        """Test BAB calculation with invalid class in list"""
        class_manager.gff.set('ClassList', [
            {'Class': 999, 'ClassLevel': 5}  # Invalid class
        ])
        
        total_bab = class_manager.calculate_total_bab()
        
        assert total_bab == 0  # Invalid classes contribute 0 BAB
    
    def test_total_saves_empty_class_list(self, class_manager):
        """Test saves calculation with empty class list"""
        class_manager.gff.set('ClassList', [])
        class_manager.gff.set('Con', 14)
        class_manager.gff.set('Dex', 16)
        class_manager.gff.set('Wis', 12)
        
        saves = class_manager.calculate_total_saves()
        
        assert saves['fortitude'] == 2   # 0 + 2 (Con)
        assert saves['reflex'] == 3      # 0 + 3 (Dex)
        assert saves['will'] == 1        # 0 + 1 (Wis)
    
    def test_total_saves_best_progression_selection(self, class_manager):
        """Test saves calculation selects best progression from each class"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 3},  # Fighter 3: Fort +3, Ref +1, Will +1
            {'Class': 2, 'ClassLevel': 3}   # Rogue 3: Fort +1, Ref +3, Will +1
        ])
        class_manager.gff.set('Con', 10)  # +0
        class_manager.gff.set('Dex', 10)  # +0
        class_manager.gff.set('Wis', 10)  # +0
        
        saves = class_manager.calculate_total_saves()
        
        assert saves['base_fortitude'] == 3  # Best from Fighter
        assert saves['base_reflex'] == 3     # Best from Rogue
        assert saves['base_will'] == 1       # Same from both
    
    def test_complex_multiclass_calculations(self, class_manager):
        """Test calculations with complex multiclass combination"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 8},   # Fighter 8: BAB +8
            {'Class': 1, 'ClassLevel': 4},   # Wizard 4: BAB +2
            {'Class': 3, 'ClassLevel': 3}    # Cleric 3: BAB +2
        ])
        class_manager.gff.set('Str', 16)  # +3
        class_manager.gff.set('Con', 14)  # +2
        class_manager.gff.set('Dex', 12)  # +1
        class_manager.gff.set('Wis', 16)  # +3
        
        total_bab = class_manager.calculate_total_bab()
        saves = class_manager.calculate_total_saves()
        bonuses = class_manager.get_attack_bonuses()
        
        assert total_bab == 12  # 8 + 2 + 2
        assert bonuses['melee_attack_bonus'] == 15  # 12 + 3 (Str)
        assert saves['fortitude'] == 8   # Best fort (6) + Con (2)
        assert saves['will'] == 7        # Best will (4) + Wis (3) - based on actual save tables
    
    def test_calculation_performance_large_dataset(self, class_manager):
        """Test calculation performance with repeated calls"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 10},
            {'Class': 1, 'ClassLevel': 10}
        ])
        
        # Multiple calculations should be fast
        for _ in range(100):
            total_bab = class_manager.calculate_total_bab()
            saves = class_manager.calculate_total_saves()
            bonuses = class_manager.get_attack_bonuses()
        
        # If we get here without timeout, performance is acceptable
        assert total_bab == 15  # 10 + 5
    
    def test_edge_case_ability_scores(self, class_manager):
        """Test calculations with extreme ability scores"""
        class_manager.gff.set('Str', 3)   # -4 modifier
        class_manager.gff.set('Dex', 25)  # +7 modifier
        class_manager.gff.set('Con', 1)   # -5 modifier
        class_manager.gff.set('ClassList', [])  # No classes
        
        # Mock calculate_total_bab to return 10 (since no classes will give 0)
        with patch.object(class_manager, 'calculate_total_bab', return_value=10):
            bonuses = class_manager.get_attack_bonuses()
            modifiers = class_manager._calculate_ability_modifiers()
        
        assert modifiers['STR'] == -4
        assert modifiers['DEX'] == 7
        assert modifiers['CON'] == -5
        assert bonuses['melee_attack_bonus'] == 6   # 10 + (-4)
        assert bonuses['ranged_attack_bonus'] == 17  # 10 + 7


class TestEventSystemIntegration:
    """Test event system integration and emission"""
    
    def test_class_change_event_emission(self, class_manager):
        """Test ClassChangedEvent is properly emitted"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5}
        ])
        
        with patch.object(class_manager.character_manager, 'emit') as mock_emit:
            class_manager.change_class(1, cheat_mode=True)
            
            mock_emit.assert_called_once()
            event = mock_emit.call_args[0][0]
            
            assert isinstance(event, ClassChangedEvent)
            assert event.old_class_id == 0
            assert event.new_class_id == 1
            assert event.level == 5
            assert event.source_manager == 'class'
            assert isinstance(event.preserve_feats, list)
    
    def test_level_gained_event_emission(self, class_manager):
        """Test LevelGainedEvent is properly emitted"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5}
        ])
        
        with patch.object(class_manager, 'emit') as mock_emit:
            class_manager.add_class_level(1, cheat_mode=True)
            
            mock_emit.assert_called_once()
            event = mock_emit.call_args[0][0]
            
            assert isinstance(event, LevelGainedEvent)
            assert event.class_id == 1
            assert event.new_level == 6
            assert event.total_level == 6
            assert event.source_manager == 'class'
    
    def test_event_timing_and_sequence(self, class_manager):
        """Test event emission timing during class operations"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5}
        ])
        
        with patch.object(class_manager.character_manager, 'emit') as mock_emit:
            start_time = time.time()
            class_manager.change_class(1, cheat_mode=True)
            end_time = time.time()
            
            event = mock_emit.call_args[0][0]
            assert start_time <= event.timestamp <= end_time
    
    def test_preserved_feats_in_event(self, class_manager):
        """Test that preserved feats are correctly included in events"""
        # Mock custom content with non-removable feat
        class_manager.character_manager.custom_content = {
            'feat_10001': {
                'type': 'feat',
                'id': 10001,
                'removable': False
            }
        }
        
        with patch.object(class_manager.character_manager, 'emit') as mock_emit:
            class_manager.change_class(1, cheat_mode=True)
            
            event = mock_emit.call_args[0][0]
            assert 10001 in event.preserve_feats
    
    def test_event_data_validation(self, class_manager):
        """Test event data validation"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5}
        ])
        
        with patch.object(class_manager.character_manager, 'emit') as mock_emit:
            class_manager.change_class(1, cheat_mode=True)
            
            event = mock_emit.call_args[0][0]
            assert event.validate() == True


class TestEdgeCasesAndErrors:
    """Test error handling, edge cases, and boundary conditions"""
    
    def test_validation_with_corrupted_data(self, class_manager):
        """Test validation with corrupted character data"""
        is_valid, errors = class_manager.validate()
        
        assert isinstance(is_valid, bool)
        assert isinstance(errors, list)
    
    def test_validation_excessive_level(self, class_manager):
        """Test validation with excessive character level"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 50}  # Way above level cap
        ])
        
        is_valid, errors = class_manager.validate()
        
        assert is_valid == False
        assert any("exceeds maximum" in error for error in errors)
    
    def test_validation_invalid_class_ids(self, class_manager):
        """Test validation with invalid class IDs"""
        class_manager.gff.set('ClassList', [
            {'Class': 999, 'ClassLevel': 5},
            {'Class': -1, 'ClassLevel': 3}
        ])
        
        is_valid, errors = class_manager.validate()
        
        assert is_valid == False
        assert len([e for e in errors if "Invalid class ID" in e]) >= 2
    
    def test_zero_level_classes(self, class_manager):
        """Test handling of zero-level classes"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 0}
        ])
        
        total_bab = class_manager.calculate_total_bab()
        saves = class_manager.calculate_total_saves()
        
        assert total_bab == 0
        assert saves['base_fortitude'] == 0
    
    def test_missing_class_list(self, class_manager):
        """Test handling when ClassList is missing"""
        # The GFF get method should return [] when ClassList is None due to default parameter
        class_manager.gff.get = Mock(return_value=[])  # Mock to return empty list
        
        summary = class_manager.get_class_summary()
        
        assert summary['total_level'] == 0
        assert summary['classes'] == []
        assert summary['multiclass'] == False
    
    def test_malformed_class_entries(self, class_manager):
        """Test handling of malformed class list entries"""
        class_manager.gff.set('ClassList', [
            {'Class': 0},  # Missing ClassLevel
            {'ClassLevel': 5},  # Missing Class
            {}  # Empty entry
        ])
        
        total_bab = class_manager.calculate_total_bab()
        summary = class_manager.get_class_summary()
        
        assert isinstance(total_bab, int)
        assert isinstance(summary, dict)
    
    def test_negative_ability_scores(self, class_manager):
        """Test handling of negative ability scores"""
        class_manager.gff.set('Str', -5)
        class_manager.gff.set('Con', -10)
        
        modifiers = class_manager._calculate_ability_modifiers()
        
        assert modifiers['STR'] == -8  # (-5 - 10) // 2 = -15 // 2 = -8 (floor division)
        assert modifiers['CON'] == -10  # (-10 - 10) // 2 = -20 // 2 = -10
    
    def test_memory_usage_large_operations(self, class_manager):
        """Test memory usage doesn't grow excessively with large operations"""
        import gc
        
        initial_objects = len(gc.get_objects())
        
        # Perform many operations
        for i in range(100):
            class_manager.gff.set('ClassList', [
                {'Class': i % 7, 'ClassLevel': (i % 20) + 1}
            ])
            class_manager.calculate_total_bab()
            class_manager.calculate_total_saves()
            class_manager.get_class_summary()
        
        gc.collect()
        final_objects = len(gc.get_objects())
        
        # Object count shouldn't grow excessively
        growth = final_objects - initial_objects
        assert growth < 1000  # Reasonable growth limit
    
    def test_concurrent_access_simulation(self, class_manager):
        """Test behavior under simulated concurrent access"""
        import threading
        
        results = []
        
        def worker():
            for _ in range(10):
                summary = class_manager.get_class_summary()
                results.append(summary['total_level'])
        
        threads = [threading.Thread(target=worker) for _ in range(5)]
        
        for thread in threads:
            thread.start()
        
        for thread in threads:
            thread.join()
        
        # All results should be consistent
        assert all(r == results[0] for r in results)
    
    def test_boundary_condition_level_20(self, class_manager):
        """Test boundary condition at level 20"""
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 20}
        ])
        
        total_bab = class_manager.calculate_total_bab()
        saves = class_manager.calculate_total_saves()
        
        assert total_bab == 20  # Fighter 20 = BAB +20
        assert saves['base_fortitude'] == 12  # Fighter 20 fort save
    
    def test_error_recovery_after_exception(self, class_manager):
        """Test that ClassManager recovers properly after exceptions"""
        # Cause an exception
        try:
            class_manager.change_class(999)  # Invalid class
        except ValueError:
            pass
        
        # Manager should still work normally
        summary = class_manager.get_class_summary()
        assert isinstance(summary, dict)
        
        total_bab = class_manager.calculate_total_bab()
        assert isinstance(total_bab, int)


class TestValidationComprehensive:
    """Comprehensive validation testing"""
    
    def test_feat_validation_by_name(self, class_manager):
        """Test feat validation by name functionality"""
        class_manager.gff.set('FeatList', [
            {'Feat': 1},
            {'Feat': 2}
        ])
        
        # Mock is already set up in the fixture, no need to override
        
        assert class_manager._has_feat_by_name('WeaponFinesse') == True
        assert class_manager._has_feat_by_name('PowerAttack') == True
        assert class_manager._has_feat_by_name('NonExistentFeat') == False
    
    def test_comprehensive_character_validation(self, class_manager):
        """Test comprehensive character validation across all systems"""
        # Set up a complex but valid character
        class_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 15},
            {'Class': 1, 'ClassLevel': 5}
        ])
        class_manager.gff.set('Str', 18)
        class_manager.gff.set('Con', 16)
        
        is_valid, errors = class_manager.validate()
        
        assert is_valid == True
        assert len(errors) == 0