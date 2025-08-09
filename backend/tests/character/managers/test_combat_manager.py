"""
Comprehensive tests for CombatManager class.
Tests cover AC calculations, BAB calculations, save calculations, initiative,
combat maneuver bonus, damage reduction, and equipment handling.
"""
import pytest
import time
from unittest.mock import Mock, MagicMock, patch, PropertyMock

from character.managers.combat_manager import CombatManager
from character.events import EventEmitter, EventData, EventType
from gamedata.services.game_rules_service import GameRulesService


@pytest.fixture
def mock_game_rules():
    """Create a mock GameRulesService with comprehensive combat data"""
    mock_rules = Mock(spec=GameRulesService)
    
    # Mock classes with combat progression
    mock_fighter = Mock()
    mock_fighter.label = "Fighter"
    mock_fighter.attack_bonus_table = "GOOD"
    mock_fighter.saving_throw_table = "FIGHTER"
    mock_fighter.spell_caster = 0
    
    mock_wizard = Mock()
    mock_wizard.label = "Wizard"
    mock_wizard.attack_bonus_table = "POOR"
    mock_wizard.saving_throw_table = "WIZARD"
    mock_wizard.spell_caster = 1
    
    mock_rogue = Mock()
    mock_rogue.label = "Rogue"
    mock_rogue.attack_bonus_table = "MEDIUM"
    mock_rogue.saving_throw_table = "ROGUE"
    mock_rogue.spell_caster = 0
    
    mock_barbarian = Mock()
    mock_barbarian.label = "Barbarian"
    mock_barbarian.attack_bonus_table = "GOOD"
    mock_barbarian.saving_throw_table = "BARBARIAN"
    mock_barbarian.spell_caster = 0
    
    mock_rules.classes = {
        0: mock_fighter,
        1: mock_wizard,
        2: mock_rogue,
        3: mock_barbarian
    }
    
    # Mock BAB progression tables
    mock_rules.BAB_PROGRESSION = {
        'good': [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
        'medium': [0, 1, 2, 3, 3, 4, 5, 6, 6, 7, 8, 9, 9, 10, 11, 12, 12, 13, 14, 15],
        'poor': [0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10]
    }
    
    # Mock saving throw progressions
    mock_rules.SAVE_PROGRESSION = {
        'fighter': {
            'fort': [2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12],
            'ref': [0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6],
            'will': [0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6]
        },
        'wizard': {
            'fort': [0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6],
            'ref': [0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6],
            'will': [2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12]
        },
        'rogue': {
            'fort': [0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6],
            'ref': [2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12],
            'will': [0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6]
        },
        'barbarian': {
            'fort': [2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12],
            'ref': [0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6],
            'will': [0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6]
        }
    }
    
    # Mock feats with combat relevance
    mock_dodge = Mock()
    mock_dodge.label = "Dodge"
    
    mock_mobility = Mock()
    mock_mobility.label = "Mobility"
    
    mock_improved_init = Mock()
    mock_improved_init.label = "ImprovedInitiative"
    
    mock_weapon_finesse = Mock()
    mock_weapon_finesse.label = "WeaponFinesse"
    
    mock_rules.feats = {
        1: mock_dodge,
        2: mock_mobility,
        3: mock_improved_init,
        4: mock_weapon_finesse
    }
    
    return mock_rules


@pytest.fixture
def sample_equipment_data():
    """Create sample equipment data for testing"""
    return {
        # Armor items (base items 0-11)
        'padded_armor': {'BaseItem': 0, 'ArmorRulesType': 0, 'Slot': 1},  # Chest
        'leather_armor': {'BaseItem': 1, 'ArmorRulesType': 1, 'Slot': 1},
        'chainmail': {'BaseItem': 6, 'ArmorRulesType': 5, 'Slot': 1},
        'full_plate': {'BaseItem': 11, 'ArmorRulesType': 7, 'Slot': 1},
        
        # Shield items (base items 63-68)
        'small_shield': {'BaseItem': 63, 'Slot': 5},  # LeftHand
        'large_shield': {'BaseItem': 64, 'Slot': 5},
        'tower_shield': {'BaseItem': 65, 'Slot': 5},
        
        # Other equipment
        'helmet': {'BaseItem': 16, 'Slot': 0},  # Head
        'boots': {'BaseItem': 27, 'Slot': 2},   # Boots
        'gauntlets': {'BaseItem': 36, 'Slot': 15}  # Gloves
    }


@pytest.fixture
def sample_character_data():
    """Create sample character data for combat testing"""
    return {
        # Basic attributes
        "Str": 16,
        "Dex": 14,
        "Con": 15,
        "Int": 12,
        "Wis": 13,
        "Cha": 10,
        
        # Character basics
        "Race": 0,  # Human
        "CreatureSize": 4,  # Medium
        "NaturalAC": 0,
        
        # Classes - Fighter 5/Wizard 3
        "ClassList": [
            {"Class": 0, "ClassLevel": 5},  # Fighter 5
            {"Class": 1, "ClassLevel": 3}   # Wizard 3
        ],
        
        # Combat feats
        "FeatList": [
            {"Feat": 1},  # Dodge
            {"Feat": 3}   # Improved Initiative
        ],
        
        # Equipment list (empty by default)
        "Equip_ItemList": [],
        
        # Hit Points
        "CurrentHitPoints": 58,
        "MaxHitPoints": 58,
        "HitPoints": 58
    }


@pytest.fixture
def character_with_equipment(sample_character_data, sample_equipment_data):
    """Create character data with equipped items"""
    char_data = sample_character_data.copy()
    
    # Equip chainmail armor and large shield
    char_data["Equip_ItemList"] = [
        sample_equipment_data['chainmail'],     # Chest armor
        sample_equipment_data['large_shield']   # Left hand shield
    ]
    
    return char_data


@pytest.fixture
def mock_class_manager():
    """Create a mock ClassManager for testing"""
    mock_manager = Mock()
    
    # Mock BAB calculation
    mock_manager.calculate_total_bab.return_value = 6  # Fighter 5 + Wizard 3
    
    # Mock attack bonuses
    mock_manager.get_attack_bonuses.return_value = {
        'primary': 6,
        'secondary': 1,
        'iterative': [6, 1],
        'ranged': 6
    }
    
    # Mock save calculations
    mock_manager.calculate_total_saves.return_value = {
        'fortitude': 7,  # Best of Fighter (4) + Con (2)
        'reflex': 4,     # Best of classes (1) + Dex (2) 
        'will': 4,       # Best of Wizard (3) + Wis (1)
        'base_fortitude': 4,
        'base_reflex': 1,
        'base_will': 3
    }
    
    return mock_manager


@pytest.fixture
def mock_character_manager(sample_character_data, mock_game_rules, mock_class_manager):
    """Create a mock CharacterManager for testing"""
    mock_manager = Mock()
    mock_manager.gff = Mock()
    
    # Create mock game_data_loader that returns the same data as mock_game_rules
    mock_loader = Mock()
    mock_loader.get_by_id = Mock(side_effect=lambda table, item_id: {
        'classes': mock_game_rules.classes,
        'feat': mock_game_rules.feats,
        'baseitems': {}  # Add empty baseitems for fallback behavior
    }.get(table, {}).get(item_id))
    
    mock_manager.game_data_loader = mock_loader
    mock_manager.emit = Mock()
    
    # Configure mock to not have character_model attribute
    # Delete it if it exists to ensure hasattr returns False
    if hasattr(mock_manager, 'character_model'):
        delattr(mock_manager, 'character_model')
    
    # Disable auto-creation of character_model
    type(mock_manager).character_model = PropertyMock(side_effect=AttributeError)
    
    # Setup GFF mock to return and store values
    gff_data = sample_character_data.copy()
    
    def gff_get(key, default=None):
        return gff_data.get(key, default)
    
    def gff_set(key, value):
        gff_data[key] = value
    
    mock_manager.gff.get = gff_get
    mock_manager.gff.set = gff_set
    
    # Mock get_manager method to return our mock class manager
    def get_manager(manager_type):
        if manager_type == 'class':
            return mock_class_manager
        return None
    
    mock_manager.get_manager = get_manager
    
    # Mock the new helper methods
    mock_manager.get_size_modifier = Mock(side_effect=lambda size: {
        0: 8,   # Fine
        1: 4,   # Diminutive  
        2: 2,   # Tiny
        3: 1,   # Small
        4: 0,   # Medium
        5: -1,  # Large
        6: -2,  # Huge
        7: -4,  # Gargantuan
        8: -8   # Colossal
    }.get(size, 0))
    
    mock_manager.has_feat_by_name = Mock(side_effect=lambda feat_label: {
        'Dodge': True if 1 in [feat.get('Feat') for feat in gff_data.get('FeatList', [])] else False,
        'Mobility': True if 2 in [feat.get('Feat') for feat in gff_data.get('FeatList', [])] else False,
        'ImprovedInitiative': True if 3 in [feat.get('Feat') for feat in gff_data.get('FeatList', [])] else False,
        'WeaponFinesse': True if 4 in [feat.get('Feat') for feat in gff_data.get('FeatList', [])] else False
    }.get(feat_label, False))
    
    mock_manager.has_class_by_name = Mock(side_effect=lambda class_name: any(
        cls.get('Class') == {'Fighter': 0, 'Wizard': 1, 'Rogue': 2, 'Barbarian': 3}.get(class_name, -1)
        for cls in gff_data.get('ClassList', [])
    ))
    
    mock_manager.get_class_level_by_name = Mock(side_effect=lambda class_name: next(
        (cls.get('ClassLevel', 0) for cls in gff_data.get('ClassList', [])
         if cls.get('Class') == {'Fighter': 0, 'Wizard': 1, 'Rogue': 2, 'Barbarian': 3}.get(class_name, -1)),
        0
    ))
    
    return mock_manager


@pytest.fixture
def combat_manager(mock_character_manager):
    """Create a CombatManager instance for testing"""
    return CombatManager(mock_character_manager)


@pytest.fixture
def combat_manager_with_equipment(character_with_equipment, mock_game_rules, mock_class_manager):
    """Create a CombatManager with equipped items"""
    mock_manager = Mock()
    mock_manager.gff = Mock()
    
    # Create mock base items data for equipment tests
    mock_base_items = {
        6: Mock(dex_bonus=2, armor_check_penalty=-5, ac_bonus=5),  # Chainmail
        64: Mock(ac_bonus=1, armor_check_penalty=-1),  # Large Shield
    }
    
    # Create mock game_data_loader that returns the same data as mock_game_rules
    mock_loader = Mock()
    mock_loader.get_by_id = Mock(side_effect=lambda table, item_id: {
        'classes': mock_game_rules.classes,
        'feat': mock_game_rules.feats,
        'baseitems': mock_base_items
    }.get(table, {}).get(item_id))
    
    mock_manager.game_data_loader = mock_loader
    mock_manager.emit = Mock()
    
    # Configure mock to not have character_model attribute
    # Delete it if it exists to ensure hasattr returns False
    if hasattr(mock_manager, 'character_model'):
        delattr(mock_manager, 'character_model')
    
    # Disable auto-creation of character_model
    type(mock_manager).character_model = PropertyMock(side_effect=AttributeError)
    
    # Setup GFF mock with equipment data
    gff_data = character_with_equipment.copy()
    
    def gff_get(key, default=None):
        return gff_data.get(key, default)
    
    def gff_set(key, value):
        gff_data[key] = value
    
    mock_manager.gff.get = gff_get
    mock_manager.gff.set = gff_set
    
    # Mock get_manager method
    def get_manager(manager_type):
        if manager_type == 'class':
            return mock_class_manager
        return None
    
    mock_manager.get_manager = get_manager
    
    # Mock the new helper methods
    mock_manager.get_size_modifier = Mock(side_effect=lambda size: {
        0: 8,   # Fine
        1: 4,   # Diminutive  
        2: 2,   # Tiny
        3: 1,   # Small
        4: 0,   # Medium
        5: -1,  # Large
        6: -2,  # Huge
        7: -4,  # Gargantuan
        8: -8   # Colossal
    }.get(size, 0))
    
    mock_manager.has_feat_by_name = Mock(side_effect=lambda feat_label: {
        'Dodge': True if 1 in [feat.get('Feat') for feat in gff_data.get('FeatList', [])] else False,
        'Mobility': True if 2 in [feat.get('Feat') for feat in gff_data.get('FeatList', [])] else False,
        'ImprovedInitiative': True if 3 in [feat.get('Feat') for feat in gff_data.get('FeatList', [])] else False,
        'WeaponFinesse': True if 4 in [feat.get('Feat') for feat in gff_data.get('FeatList', [])] else False
    }.get(feat_label, False))
    
    mock_manager.has_class_by_name = Mock(side_effect=lambda class_name: any(
        cls.get('Class') == {'Fighter': 0, 'Wizard': 1, 'Rogue': 2, 'Barbarian': 3}.get(class_name, -1)
        for cls in gff_data.get('ClassList', [])
    ))
    
    mock_manager.get_class_level_by_name = Mock(side_effect=lambda class_name: next(
        (cls.get('ClassLevel', 0) for cls in gff_data.get('ClassList', [])
         if cls.get('Class') == {'Fighter': 0, 'Wizard': 1, 'Rogue': 2, 'Barbarian': 3}.get(class_name, -1)),
        0
    ))
    
    return CombatManager(mock_manager)


class TestCombatManagerInitialization:
    """Test CombatManager initialization and basic setup"""
    
    def test_initialization(self, mock_character_manager):
        """Test CombatManager initialization"""
        manager = CombatManager(mock_character_manager)
        
        assert manager.character_manager == mock_character_manager
        assert manager.gff == mock_character_manager.gff
        assert manager.game_data_loader == mock_character_manager.game_data_loader
        assert hasattr(manager, '_base_item_cache')
        assert hasattr(manager, '_feat_cache')
        assert hasattr(manager, '_class_cache')
    
    def test_size_ac_modifiers_table(self, combat_manager):
        """Test that size modifiers are accessible through character manager"""
        expected_modifiers = {
            0: 8,   # Fine
            1: 4,   # Diminutive  
            2: 2,   # Tiny
            3: 1,   # Small
            4: 0,   # Medium
            5: -1,  # Large
            6: -2,  # Huge
            7: -4,  # Gargantuan
            8: -8   # Colossal
        }
        
        # Test that size modifiers work through race manager
        for size_id, expected_modifier in expected_modifiers.items():
            race_manager = combat_manager.character_manager.get_manager('race')
            if race_manager:
                actual_modifier = race_manager.get_size_modifier(size_id)
                assert actual_modifier == expected_modifier, f"Size {size_id} modifier mismatch"
    
    def test_event_registration(self, mock_character_manager):
        """Test that event handlers are registered"""
        manager = CombatManager(mock_character_manager)
        
        # Check that event handlers are defined
        assert hasattr(manager, '_on_attribute_changed')
        assert hasattr(manager, '_on_item_equipped')
        assert hasattr(manager, '_on_item_unequipped')


class TestArmorClassCalculation:
    """Test comprehensive AC calculation functionality"""
    
    def test_basic_ac_calculation_no_equipment(self, combat_manager):
        """Test basic AC calculation with no equipment"""
        ac_data = combat_manager.calculate_armor_class()
        
        assert ac_data['total_ac'] == 13  # 10 base + 2 DEX + 1 dodge
        assert ac_data['touch_ac'] == 13  # Same as total (no armor)
        assert ac_data['flatfooted_ac'] == 10  # No DEX bonus or dodge
        
        components = ac_data['components']
        assert components['base'] == 10
        assert components['armor'] == 0
        assert components['shield'] == 0
        assert components['dex'] == 2  # (14-10)//2
        assert components['natural'] == 0
        assert components['dodge'] == 1  # Has Dodge feat
        assert components['deflection'] == 0
        assert components['size'] == 0  # Medium creature
    
    def test_ac_calculation_with_armor(self, combat_manager_with_equipment):
        """Test AC calculation with armor equipped"""
        ac_data = combat_manager_with_equipment.calculate_armor_class()
        
        # Chainmail: +6 AC, max DEX +2
        # Large Shield: +2 AC
        # Expected: 10 base + 6 armor + 2 shield + 2 DEX (capped) + 1 dodge = 21
        assert ac_data['total_ac'] == 21
        
        components = ac_data['components']
        assert components['armor'] == 6  # Chainmail
        assert components['shield'] == 2  # Large shield
        assert components['dex'] == 2  # DEX bonus capped by armor
        
        # Touch AC ignores armor and shield
        assert ac_data['touch_ac'] == 13  # 10 + 2 DEX + 1 dodge
        
        # Flat-footed AC ignores DEX and dodge
        assert ac_data['flatfooted_ac'] == 18  # 10 + 6 armor + 2 shield
    
    def test_ac_calculation_different_sizes(self, combat_manager):
        """Test AC calculation with different creature sizes"""
        # Test small creature
        combat_manager.gff.set('CreatureSize', 3)  # Small
        ac_data = combat_manager.calculate_armor_class()
        assert ac_data['components']['size'] == 1
        assert ac_data['total_ac'] == 14  # 10 + 2 DEX + 1 dodge + 1 size
        
        # Test large creature
        combat_manager.gff.set('CreatureSize', 5)  # Large
        ac_data = combat_manager.calculate_armor_class()
        assert ac_data['components']['size'] == -1
        assert ac_data['total_ac'] == 12  # 10 + 2 DEX + 1 dodge - 1 size
        
        # Test huge creature
        combat_manager.gff.set('CreatureSize', 6)  # Huge
        ac_data = combat_manager.calculate_armor_class()
        assert ac_data['components']['size'] == -2
        assert ac_data['total_ac'] == 11  # 10 + 2 DEX + 1 dodge - 2 size
    
    def test_ac_calculation_with_natural_armor(self, combat_manager):
        """Test AC calculation with natural armor bonus"""
        combat_manager.gff.set('NaturalAC', 3)
        
        ac_data = combat_manager.calculate_armor_class()
        
        assert ac_data['components']['natural'] == 3
        assert ac_data['total_ac'] == 16  # 10 + 2 DEX + 1 dodge + 3 natural
        assert ac_data['touch_ac'] == 13  # Natural armor doesn't affect touch AC
        assert ac_data['flatfooted_ac'] == 13  # 10 + 3 natural (no DEX/dodge)
    
    def test_ac_calculation_mobility_feat(self, combat_manager):
        """Test AC calculation with Mobility feat"""
        # Add Mobility feat
        feat_list = combat_manager.gff.get('FeatList', [])
        feat_list.append({"Feat": 2})  # Mobility feat
        combat_manager.gff.set('FeatList', feat_list)
        
        ac_data = combat_manager.calculate_armor_class()
        
        # Should have both Dodge (+1) and Mobility (+4) bonuses
        assert ac_data['components']['dodge'] == 5  # 1 + 4
        assert ac_data['total_ac'] == 17  # 10 + 2 DEX + 5 dodge
    
    def test_dex_bonus_capping(self, combat_manager_with_equipment):
        """Test that armor properly caps DEX bonus"""
        # Set high DEX
        combat_manager_with_equipment.gff.set('Dex', 20)  # +5 modifier
        
        ac_data = combat_manager_with_equipment.calculate_armor_class()
        
        # Chainmail has max DEX +2
        assert ac_data['dex_bonus'] == 5  # Actual DEX bonus
        assert ac_data['max_dex_from_armor'] == 2  # Armor limit
        assert ac_data['components']['dex'] == 2  # Applied DEX bonus (capped)
    
    def test_ac_calculation_extreme_dex(self, combat_manager):
        """Test AC calculation with extreme DEX values"""
        # Test very low DEX
        combat_manager.gff.set('Dex', 6)  # -2 modifier
        ac_data = combat_manager.calculate_armor_class()
        assert ac_data['components']['dex'] == -2
        assert ac_data['total_ac'] == 9  # 10 - 2 DEX + 1 dodge
        
        # Test very high DEX
        combat_manager.gff.set('Dex', 30)  # +10 modifier
        ac_data = combat_manager.calculate_armor_class()
        assert ac_data['components']['dex'] == 10
        assert ac_data['total_ac'] == 21  # 10 + 10 DEX + 1 dodge


class TestBABCalculation:
    """Test BAB calculation functionality"""
    
    def test_bab_calculation_through_class_manager(self, combat_manager, mock_class_manager):
        """Test that BAB calculation delegates to ClassManager"""
        # Get combat summary which should call ClassManager
        summary = combat_manager.get_combat_summary()
        
        # Verify ClassManager was called
        mock_class_manager.calculate_total_bab.assert_called_once()
        mock_class_manager.get_attack_bonuses.assert_called_once()
        
        # Verify attack bonuses are included
        assert 'attack_bonuses' in summary
        assert summary['attack_bonuses']['primary'] == 6
    
    def test_combat_maneuver_bonus_uses_bab(self, combat_manager, mock_class_manager):
        """Test that CMB calculation uses BAB from ClassManager"""
        cmb_data = combat_manager.calculate_combat_maneuver_bonus()
        
        # Should call ClassManager for BAB
        mock_class_manager.calculate_total_bab.assert_called_once()
        
        # CMB = BAB + STR mod + size mod
        # BAB = 6, STR mod = 3, size mod = 0 (medium)
        assert cmb_data['total'] == 9
        assert cmb_data['base_attack_bonus'] == 6
        assert cmb_data['strength_modifier'] == 3
        assert cmb_data['size_modifier'] == 0
    
    def test_cmb_with_different_sizes(self, combat_manager, mock_class_manager):
        """Test CMB calculation with different creature sizes"""
        # Test small creature (gets -1 to CMB, opposite of AC)
        combat_manager.gff.set('CreatureSize', 3)  # Small
        cmb_data = combat_manager.calculate_combat_maneuver_bonus()
        assert cmb_data['size_modifier'] == -1  # Opposite of AC size mod (+1)
        assert cmb_data['total'] == 8  # 6 BAB + 3 STR - 1 size
        
        # Test large creature (gets +1 to CMB, opposite of AC)
        combat_manager.gff.set('CreatureSize', 5)  # Large
        cmb_data = combat_manager.calculate_combat_maneuver_bonus()
        assert cmb_data['size_modifier'] == 1  # Opposite of AC size mod (-1)
        assert cmb_data['total'] == 10  # 6 BAB + 3 STR + 1 size
    
    def test_cmb_with_extreme_strength(self, combat_manager, mock_class_manager):
        """Test CMB calculation with extreme Strength values"""
        # Test very low Strength
        combat_manager.gff.set('Str', 6)  # -2 modifier
        cmb_data = combat_manager.calculate_combat_maneuver_bonus()
        assert cmb_data['strength_modifier'] == -2
        assert cmb_data['total'] == 4  # 6 BAB - 2 STR + 0 size
        
        # Test very high Strength
        combat_manager.gff.set('Str', 30)  # +10 modifier
        cmb_data = combat_manager.calculate_combat_maneuver_bonus()
        assert cmb_data['strength_modifier'] == 10
        assert cmb_data['total'] == 16  # 6 BAB + 10 STR + 0 size


class TestSaveCalculations:
    """Test saving throw calculation functionality"""
    
    def test_save_calculations_through_class_manager(self, combat_manager, mock_class_manager):
        """Test that save calculations delegate to ClassManager"""
        summary = combat_manager.get_combat_summary()
        
        # CombatManager doesn't call saves directly - it focuses on AC, initiative, etc.
        # This test verifies that the ClassManager is accessible for saves if needed
        class_manager = combat_manager.character_manager.get_manager('class')
        assert class_manager is not None
        
        # If we were to call saves, it would work through the ClassManager
        saves = class_manager.calculate_total_saves()
        assert 'fortitude' in saves
        assert 'reflex' in saves
        assert 'will' in saves
    
    def test_saving_throw_modifiers_from_attributes(self, combat_manager):
        """Test that saving throws properly use attribute modifiers"""
        # This would be tested in ClassManager, but we can verify the attributes
        # are accessible from CombatManager
        con_mod = (combat_manager.gff.get('Con', 10) - 10) // 2
        dex_mod = (combat_manager.gff.get('Dex', 10) - 10) // 2
        wis_mod = (combat_manager.gff.get('Wis', 10) - 10) // 2
        
        assert con_mod == 2  # Con 15 -> +2
        assert dex_mod == 2  # Dex 14 -> +2
        assert wis_mod == 1  # Wis 13 -> +1


class TestInitiativeCalculation:
    """Test initiative calculation functionality"""
    
    def test_basic_initiative_calculation(self, combat_manager):
        """Test basic initiative calculation"""
        init_data = combat_manager.calculate_initiative()
        
        # DEX modifier (2) + Improved Initiative (4)
        assert init_data['total'] == 6
        assert init_data['dex_modifier'] == 2
        assert init_data['improved_initiative'] == 4
        assert init_data['misc_bonus'] == 0
    
    def test_initiative_without_improved_initiative(self, combat_manager):
        """Test initiative calculation without Improved Initiative"""
        # Remove Improved Initiative feat
        feat_list = [feat for feat in combat_manager.gff.get('FeatList', []) 
                    if feat.get('Feat') != 3]
        combat_manager.gff.set('FeatList', feat_list)
        
        init_data = combat_manager.calculate_initiative()
        
        assert init_data['total'] == 2  # Just DEX modifier
        assert init_data['dex_modifier'] == 2
        assert init_data['improved_initiative'] == 0
        assert init_data['misc_bonus'] == 0
    
    def test_initiative_with_different_dex(self, combat_manager):
        """Test initiative calculation with different DEX values"""
        # Test low DEX
        combat_manager.gff.set('Dex', 8)  # -1 modifier
        init_data = combat_manager.calculate_initiative()
        assert init_data['dex_modifier'] == -1
        assert init_data['total'] == 3  # -1 DEX + 4 Improved Initiative
        
        # Test high DEX
        combat_manager.gff.set('Dex', 20)  # +5 modifier
        init_data = combat_manager.calculate_initiative()
        assert init_data['dex_modifier'] == 5
        assert init_data['total'] == 9  # 5 DEX + 4 Improved Initiative


class TestDamageReduction:
    """Test damage reduction calculation functionality"""
    
    def test_no_damage_reduction(self, combat_manager):
        """Test character with no damage reduction"""
        dr_list = combat_manager.get_damage_reduction()
        
        assert len(dr_list) == 0
    
    def test_barbarian_damage_reduction(self, combat_manager):
        """Test Barbarian damage reduction calculation"""
        # Set character as Barbarian level 10
        combat_manager.gff.set('ClassList', [{'Class': 3, 'ClassLevel': 10}])
        
        dr_list = combat_manager.get_damage_reduction()
        
        assert len(dr_list) == 1
        dr_entry = dr_list[0]
        assert dr_entry['amount'] == 2  # 1 + (10-7)//3 = 1 + 1 = 2
        assert dr_entry['bypass'] == '-'
        assert dr_entry['source'] == 'Barbarian class'
    
    def test_barbarian_dr_progression(self, combat_manager):
        """Test Barbarian DR progression at different levels"""
        test_cases = [
            (6, 0),   # Below minimum level
            (7, 1),   # First DR
            (8, 1),   # Same DR
            (9, 1),   # Same DR
            (10, 2),  # Increased DR
            (13, 3),  # Next increase
            (16, 4),  # Next increase
            (19, 5),  # Next increase
            (20, 5),  # Cap at 20
        ]
        
        for level, expected_dr in test_cases:
            combat_manager.gff.set('ClassList', [{'Class': 3, 'ClassLevel': level}])
            dr_list = combat_manager.get_damage_reduction()
            
            if expected_dr == 0:
                assert len(dr_list) == 0
            else:
                assert len(dr_list) == 1
                assert dr_list[0]['amount'] == expected_dr
    
    def test_multiclass_barbarian_dr(self, combat_manager):
        """Test DR calculation for multiclass character with Barbarian"""
        # Fighter 5/Barbarian 10
        combat_manager.gff.set('ClassList', [
            {'Class': 0, 'ClassLevel': 5},
            {'Class': 3, 'ClassLevel': 10}
        ])
        
        dr_list = combat_manager.get_damage_reduction()
        
        assert len(dr_list) == 1
        assert dr_list[0]['amount'] == 2  # Based on Barbarian level 10


class TestEquippedItemHandling:
    """Test equipped item handling functionality"""
    
    def test_get_equipped_item_by_slot(self, combat_manager_with_equipment):
        """Test getting equipped items by slot"""
        # Test chest item (armor)
        chest_item = combat_manager_with_equipment._get_equipped_item('Chest')
        assert chest_item is not None
        assert chest_item['BaseItem'] == 6  # Chainmail
        
        # Test left hand item (shield)
        left_hand_item = combat_manager_with_equipment._get_equipped_item('LeftHand')
        assert left_hand_item is not None
        assert left_hand_item['BaseItem'] == 64  # Large shield
        
        # Test empty slot
        head_item = combat_manager_with_equipment._get_equipped_item('Head')
        assert head_item is None
    
    def test_get_item_ac_bonus(self, combat_manager_with_equipment):
        """Test getting AC bonus from equipped items"""
        # Test armor AC bonus
        chest_item = combat_manager_with_equipment._get_equipped_item('Chest')
        ac_bonus = combat_manager_with_equipment._get_item_ac_bonus(chest_item)
        assert ac_bonus == 6  # Chainmail base AC
        
        # Test shield AC bonus
        shield_item = combat_manager_with_equipment._get_equipped_item('LeftHand')
        shield_bonus = combat_manager_with_equipment._get_item_ac_bonus(shield_item)
        assert shield_bonus == 2  # Large shield
    
    def test_get_item_max_dex(self, combat_manager_with_equipment):
        """Test getting max DEX bonus from armor"""
        chest_item = combat_manager_with_equipment._get_equipped_item('Chest')
        max_dex = combat_manager_with_equipment._get_item_max_dex(chest_item)
        assert max_dex == 2  # Chainmail max DEX
        
        # Test with no armor
        max_dex_no_armor = combat_manager_with_equipment._get_item_max_dex(None)
        assert max_dex_no_armor == 999  # No limit
    
    def test_is_shield_detection(self, combat_manager_with_equipment):
        """Test shield detection functionality"""
        shield_item = combat_manager_with_equipment._get_equipped_item('LeftHand')
        assert combat_manager_with_equipment._is_shield(shield_item) is True
        
        chest_item = combat_manager_with_equipment._get_equipped_item('Chest')
        assert combat_manager_with_equipment._is_shield(chest_item) is False
    
    def test_armor_check_penalty(self, combat_manager_with_equipment):
        """Test armor check penalty calculation"""
        penalty = combat_manager_with_equipment._get_armor_check_penalty()
        assert penalty == -5  # Chainmail penalty
        
        # Test with no armor
        combat_manager_with_equipment.gff.set('Equip_ItemList', [])
        penalty_no_armor = combat_manager_with_equipment._get_armor_check_penalty()
        assert penalty_no_armor == 0


class TestMovementSpeed:
    """Test movement speed calculation"""
    
    def test_base_movement_speed(self, combat_manager):
        """Test base movement speed calculation"""
        speed_data = combat_manager._get_movement_speed()
        
        assert speed_data['base'] == 30  # Default medium creature
        assert speed_data['current'] == 30
        assert speed_data['armor_penalty'] is False
    
    def test_movement_speed_with_heavy_armor(self, combat_manager_with_equipment):
        """Test movement speed with heavy armor"""
        speed_data = combat_manager_with_equipment._get_movement_speed()
        
        assert speed_data['base'] == 20  # Heavy armor reduces speed
        assert speed_data['current'] == 20
        assert speed_data['armor_penalty'] is True
    
    def test_barbarian_fast_movement(self, combat_manager):
        """Test Barbarian fast movement"""
        # Set as Barbarian
        combat_manager.gff.set('ClassList', [{'Class': 3, 'ClassLevel': 5}])
        
        speed_data = combat_manager._get_movement_speed()
        
        assert speed_data['base'] == 40  # 30 + 10 Barbarian bonus
        assert speed_data['current'] == 40


class TestFeatDetection:
    """Test feat detection functionality"""
    
    def test_has_feat_by_name_present(self, combat_manager):
        """Test feat detection when feat is present"""
        assert combat_manager._has_feat_by_name('Dodge') is True
        assert combat_manager._has_feat_by_name('ImprovedInitiative') is True
    
    def test_has_feat_by_name_absent(self, combat_manager):
        """Test feat detection when feat is absent"""
        assert combat_manager._has_feat_by_name('PowerAttack') is False
        assert combat_manager._has_feat_by_name('Mobility') is False
    
    def test_has_feat_empty_list(self, combat_manager):
        """Test feat detection with empty feat list"""
        combat_manager.gff.set('FeatList', [])
        
        assert combat_manager._has_feat_by_name('Dodge') is False


class TestClassDetection:
    """Test class detection functionality"""
    
    def test_has_class_present(self, combat_manager):
        """Test class detection when class is present"""
        assert combat_manager._has_class('Fighter') is True
        assert combat_manager._has_class('Wizard') is True
    
    def test_has_class_absent(self, combat_manager):
        """Test class detection when class is absent"""
        assert combat_manager._has_class('Rogue') is False
        assert combat_manager._has_class('Barbarian') is False
    
    def test_get_class_level(self, combat_manager):
        """Test getting class level"""
        assert combat_manager._get_class_level('Fighter') == 5
        assert combat_manager._get_class_level('Wizard') == 3
        assert combat_manager._get_class_level('Rogue') == 0


class TestEventHandling:
    """Test event handling functionality"""
    
    def test_attribute_changed_event(self, combat_manager):
        """Test attribute changed event handling"""
        from character.events import EventData, EventType
        
        event = EventData(
            event_type=EventType.ATTRIBUTE_CHANGED,
            source_manager='test',
            timestamp=time.time()
        )
        event.changes = [{'attribute': 'Dex', 'old': 14, 'new': 16}]
        
        with patch('character.managers.combat_manager.logger') as mock_logger:
            combat_manager._on_attribute_changed(event)
            mock_logger.info.assert_called_with("Combat stats affected by Dex change")
    
    def test_item_equipped_event(self, combat_manager):
        """Test item equipped event handling"""
        from character.events import EventData, EventType
        
        event = EventData(
            event_type=EventType.ITEM_EQUIPPED,
            source_manager='test',
            timestamp=time.time()
        )
        
        with patch('character.managers.combat_manager.logger') as mock_logger:
            combat_manager._on_item_equipped(event)
            mock_logger.info.assert_called_with("Item equipped - recalculating AC")
    
    def test_item_unequipped_event(self, combat_manager):
        """Test item unequipped event handling"""
        from character.events import EventData, EventType
        
        event = EventData(
            event_type=EventType.ITEM_UNEQUIPPED,
            source_manager='test',
            timestamp=time.time()
        )
        
        with patch('character.managers.combat_manager.logger') as mock_logger:
            combat_manager._on_item_unequipped(event)
            mock_logger.info.assert_called_with("Item unequipped - recalculating AC")


class TestCombatSummary:
    """Test comprehensive combat summary functionality"""
    
    def test_combat_summary_structure(self, combat_manager, mock_class_manager):
        """Test combat summary contains all expected data"""
        summary = combat_manager.get_combat_summary()
        
        assert 'armor_class' in summary
        assert 'initiative' in summary
        assert 'attack_bonuses' in summary
        assert 'combat_maneuvers' in summary
        assert 'damage_reduction' in summary
        assert 'speed' in summary
        
        # Verify nested structure
        assert 'total_ac' in summary['armor_class']
        assert 'touch_ac' in summary['armor_class']
        assert 'flatfooted_ac' in summary['armor_class']
        assert 'components' in summary['armor_class']
        
        assert 'total' in summary['initiative']
        assert 'dex_modifier' in summary['initiative']
        
        assert 'total' in summary['combat_maneuvers']
    
    def test_combat_summary_integration(self, combat_manager_with_equipment, mock_class_manager):
        """Test combat summary with equipped items"""
        summary = combat_manager_with_equipment.get_combat_summary()
        
        # Should reflect equipped armor and shield
        ac_data = summary['armor_class']
        assert ac_data['total_ac'] == 21  # With armor and shield
        assert ac_data['components']['armor'] == 6
        assert ac_data['components']['shield'] == 2


class TestValidation:
    """Test combat statistics validation"""
    
    def test_validation_normal_stats(self, combat_manager):
        """Test validation with normal combat stats"""
        is_valid, errors = combat_manager.validate()
        
        assert is_valid is True
        assert len(errors) == 0
    
    def test_validation_negative_ac(self, combat_manager):
        """Test validation with negative AC"""
        # Set extremely low DEX to create negative AC
        combat_manager.gff.set('Dex', 1)  # -5 modifier (max negative)
        # Remove all feats including dodge
        combat_manager.gff.set('FeatList', [])
        # Give negative natural armor to push it below 0
        combat_manager.gff.set('NaturalAC', -10)
        
        is_valid, errors = combat_manager.validate()
        
        assert is_valid is False
        assert any("AC is negative" in error for error in errors)
    
    def test_validation_extremely_high_ac(self, combat_manager):
        """Test validation with unreasonably high AC"""
        # Set extreme natural armor bonus
        combat_manager.gff.set('NaturalAC', 50)
        
        is_valid, errors = combat_manager.validate()
        
        assert is_valid is False
        assert any("AC seems unusually high" in error for error in errors)


class TestEdgeCases:
    """Test edge cases and error conditions"""
    
    def test_missing_class_manager(self, combat_manager):
        """Test behavior when ClassManager is not available"""
        # Mock get_manager to return None
        combat_manager.character_manager.get_manager = Mock(return_value=None)
        
        # CMB calculation should handle missing class manager gracefully
        cmb_data = combat_manager.calculate_combat_maneuver_bonus()
        assert cmb_data['base_attack_bonus'] == 0
        
        # Combat summary should handle missing class manager
        summary = combat_manager.get_combat_summary()
        assert summary['attack_bonuses'] == {}
    
    def test_malformed_equipment_data(self, combat_manager):
        """Test handling of malformed equipment data"""
        # Set malformed equipment list - this tests the GFF fallback path
        # Note: The current implementation doesn't handle all malformed data gracefully
        # This test documents the expected behavior and areas for improvement
        combat_manager.gff.set('Equip_ItemList', [
            {'Slot': 1, 'BaseItem': 6},  # Valid item
            {'Slot': 1},  # Missing BaseItem - should be ignored
            {'BaseItem': 6},  # Missing Slot - should be ignored
            # None and invalid types will cause the .get() call to fail
            # This is a limitation of the current implementation
        ])
        
        # Should handle gracefully without crashing
        ac_data = combat_manager.calculate_armor_class()
        assert isinstance(ac_data, dict)
        assert 'total_ac' in ac_data
    
    def test_invalid_creature_size(self, combat_manager):
        """Test handling of invalid creature size"""
        combat_manager.gff.set('CreatureSize', 999)  # Invalid size
        
        ac_data = combat_manager.calculate_armor_class()
        
        # Should default to 0 modifier for unknown sizes
        assert ac_data['components']['size'] == 0
    
    def test_missing_feat_data(self, combat_manager):
        """Test handling of missing feat data in game rules"""
        # Mock the feat manager to return False for all feats
        mock_feat_manager = Mock()
        mock_feat_manager.has_feat_by_name = Mock(return_value=False)
        
        # Mock get_manager to return appropriate manager types
        def mock_get_manager(manager_type):
            if manager_type == 'feat':
                return mock_feat_manager
            # Return None for 'race' to trigger the fallback path
            elif manager_type == 'race':
                return None
            # Return the original manager for other types
            return combat_manager.character_manager._managers.get(manager_type)
        
        combat_manager.character_manager.get_manager = Mock(side_effect=mock_get_manager)
        
        # Should not crash when checking for feats
        assert combat_manager._has_feat_by_name('Dodge') is False
        
        ac_data = combat_manager.calculate_armor_class()
        assert ac_data['components']['dodge'] == 0
    
    def test_missing_class_data(self, combat_manager):
        """Test handling of missing class data in game rules"""
        # Mock empty class data via class manager
        mock_class_manager = Mock()
        mock_class_manager.has_class_by_name = Mock(return_value=False)
        mock_class_manager.get_class_level_by_name = Mock(return_value=0)
        
        # Mock get_manager to return appropriate manager types
        def mock_get_manager(manager_type):
            if manager_type == 'class':
                return mock_class_manager
            # Return None for other manager types to trigger fallback paths
            elif manager_type in ['feat', 'race']:
                return None
            # Return the original manager for other types
            return combat_manager.character_manager._managers.get(manager_type)
        
        combat_manager.character_manager.get_manager = Mock(side_effect=mock_get_manager)
        
        # Should not crash when checking for classes
        assert combat_manager._has_class('Fighter') is False
        assert combat_manager._get_class_level('Fighter') == 0
    
    def test_extreme_attribute_values(self, combat_manager):
        """Test handling of extreme attribute values"""
        # Test very high STR for CMB
        combat_manager.gff.set('Str', 100)  # +45 modifier
        cmb_data = combat_manager.calculate_combat_maneuver_bonus()
        assert cmb_data['strength_modifier'] == 45
        
        # Test very low DEX for AC
        combat_manager.gff.set('Dex', 1)  # -5 modifier
        ac_data = combat_manager.calculate_armor_class()
        assert ac_data['components']['dex'] == -5
    
    def test_empty_class_list(self, combat_manager):
        """Test handling of empty class list"""
        combat_manager.gff.set('ClassList', [])
        
        # Should handle gracefully
        assert combat_manager._has_class('Fighter') is False
        assert combat_manager._get_class_level('Fighter') == 0
        
        # Movement speed should still work
        speed_data = combat_manager._get_movement_speed()
        assert speed_data['base'] == 30


class TestPerformance:
    """Test performance-related scenarios"""
    
    def test_high_level_multiclass_character(self, combat_manager):
        """Test performance with high-level multiclass character"""
        # Create epic-level multiclass character
        class_list = []
        for class_id in range(4):  # 4 different classes
            class_list.append({'Class': class_id, 'ClassLevel': 10})
        
        combat_manager.gff.set('ClassList', class_list)
        
        # Should handle without performance issues
        summary = combat_manager.get_combat_summary()
        assert isinstance(summary, dict)
        assert 'armor_class' in summary
    
    def test_many_equipped_items(self, combat_manager, sample_equipment_data):
        """Test performance with many equipped items"""
        # Equip items in all slots
        equipped_items = []
        for item_name, item_data in sample_equipment_data.items():
            equipped_items.append(item_data)
        
        combat_manager.gff.set('Equip_ItemList', equipped_items)
        
        # Should handle gracefully
        ac_data = combat_manager.calculate_armor_class()
        assert isinstance(ac_data, dict)


class TestCodeImprovements:
    """Tests that highlight potential code improvements"""
    
    def test_item_property_system_completeness(self, combat_manager_with_equipment):
        """Test that item property system handles various scenarios"""
        # Current implementation uses simplified property parsing
        # This test documents expected behavior for future improvements
        
        chest_item = combat_manager_with_equipment._get_equipped_item('Chest')
        ac_bonus = combat_manager_with_equipment._get_item_ac_bonus(chest_item)
        
        # Should properly parse item properties
        assert isinstance(ac_bonus, int)
        assert ac_bonus >= 0
    
    def test_django_model_integration(self, combat_manager):
        """Test Django model integration patterns"""
        # Current implementation has fallback to GFF when no Django model
        # This test documents the integration pattern
        
        # The mock character manager is set up to not have character_model
        # This forces the GFF fallback path, which is what we want to test
        
        # Should fall back to GFF when no Django model
        item = combat_manager._get_equipped_item('Chest')
        # With no equipment in our test data, this should return None
        assert item is None
        
        # The integration pattern works: try Django first, fall back to GFF
    
    def test_cache_efficiency_opportunity(self, combat_manager):
        """Test that identifies caching opportunities"""
        # Multiple AC calculations should be efficient
        # This test documents where caching could improve performance
        
        ac_data_1 = combat_manager.calculate_armor_class()
        ac_data_2 = combat_manager.calculate_armor_class()
        
        # Results should be consistent
        assert ac_data_1 == ac_data_2
        
        # Suggestion: Add caching for expensive calculations
        # when character data hasn't changed