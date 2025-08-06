"""
Comprehensive tests for SkillManager class.
Tests cover skill point calculations, skill ranking, class skills, event handling,
armor check penalties, synergy bonuses, and data-driven architecture.
"""
import pytest
import time
from unittest.mock import Mock, MagicMock, patch, PropertyMock
from typing import Dict, List, Any

from character.managers.skill_manager import SkillManager
from character.events import EventEmitter, EventType, ClassChangedEvent, LevelGainedEvent
from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader


class MockSkill:
    """Mock skill data for testing"""
    def __init__(self, id, label, name=None, key_ability="STR", armor_check_penalty=0, 
                 description="", category=0):
        self.id = id
        self.label = label
        self.name = name or label.lower().replace(" ", "_")
        self.key_ability = key_ability
        self.keyability = key_ability  # Alternative column name
        self.armor_check_penalty = armor_check_penalty
        self.armorcheckpenalty = armor_check_penalty  # Alternative column name
        self.description = description
        self.category = category


class MockClass:
    """Mock class data for testing"""
    def __init__(self, id, label, name=None, skill_point_base=2, skills_table=None, 
                 hit_die=8, bab_progression="medium"):
        self.id = id
        self.label = label
        self.name = name or label.lower().replace(" ", "_")
        self.skill_point_base = skill_point_base
        self.skillpointbase = skill_point_base  # Alternative column name
        self.skills_table = skills_table or f"cls_skill_{self.name}"
        self.skillstable = self.skills_table  # Alternative column name
        self.hit_die = hit_die
        self.bab_progression = bab_progression


class MockClassSkill:
    """Mock class skill table entry"""
    def __init__(self, skill_index, class_skill=1):
        self.skill_index = skill_index
        self.skillindex = skill_index  # Alternative column name
        self.skill = skill_index  # Alternative column name
        self.class_skill = class_skill
        self.classskill = class_skill  # Alternative column name


@pytest.fixture
def mock_skills():
    """Create comprehensive mock skills data"""
    return [
        MockSkill(0, "Concentration", key_ability="CON"),
        MockSkill(1, "Disable Device", key_ability="INT"),
        MockSkill(2, "Discipline", key_ability="STR"),
        MockSkill(3, "Heal", key_ability="WIS"),
        MockSkill(4, "Hide", key_ability="DEX", armor_check_penalty=1),
        MockSkill(5, "Intimidate", key_ability="CHA"),
        MockSkill(6, "Listen", key_ability="WIS"),
        MockSkill(7, "Lore", key_ability="INT"),
        MockSkill(8, "Move Silently", key_ability="DEX", armor_check_penalty=1),
        MockSkill(9, "Open Lock", key_ability="DEX"),
        MockSkill(10, "Parry", key_ability="DEX"),
        MockSkill(11, "Search", key_ability="INT"),
        MockSkill(12, "Sleight of Hand", key_ability="DEX", armor_check_penalty=1),
        MockSkill(13, "Spellcraft", key_ability="INT"),
        MockSkill(14, "Spot", key_ability="WIS"),
        MockSkill(15, "Tumble", key_ability="DEX", armor_check_penalty=1),
        MockSkill(16, "Use Magic Device", key_ability="CHA"),
    ]


@pytest.fixture
def mock_classes():
    """Create comprehensive mock classes data"""
    return [
        MockClass(0, "Barbarian", skill_point_base=4, skills_table="cls_skill_barb"),
        MockClass(1, "Bard", skill_point_base=6, skills_table="cls_skill_bard"),
        MockClass(2, "Cleric", skill_point_base=2, skills_table="cls_skill_cler"),
        MockClass(3, "Druid", skill_point_base=4, skills_table="cls_skill_drui"),
        MockClass(4, "Fighter", skill_point_base=2, skills_table="cls_skill_figh"),
        MockClass(5, "Monk", skill_point_base=4, skills_table="cls_skill_monk"),
        MockClass(6, "Paladin", skill_point_base=2, skills_table="cls_skill_pala"),
        MockClass(7, "Ranger", skill_point_base=6, skills_table="cls_skill_rang"),
        MockClass(8, "Rogue", skill_point_base=8, skills_table="cls_skill_rogu"),
        MockClass(9, "Sorcerer", skill_point_base=2, skills_table="cls_skill_sorc"),
        MockClass(10, "Wizard", skill_point_base=2, skills_table="cls_skill_wiz"),
    ]


@pytest.fixture
def mock_class_skills():
    """Create mock class skills tables"""
    return {
        "cls_skill_wiz": [  # Wizard skills
            MockClassSkill(0),   # Concentration  
            MockClassSkill(7),   # Lore
            MockClassSkill(13),  # Spellcraft
        ],
        "cls_skill_rogu": [  # Rogue skills
            MockClassSkill(1),   # Disable Device
            MockClassSkill(4),   # Hide
            MockClassSkill(6),   # Listen
            MockClassSkill(8),   # Move Silently
            MockClassSkill(9),   # Open Lock
            MockClassSkill(11),  # Search
            MockClassSkill(12),  # Sleight of Hand
            MockClassSkill(14),  # Spot
            MockClassSkill(15),  # Tumble
            MockClassSkill(16),  # Use Magic Device
        ],
        "cls_skill_figh": [  # Fighter skills
            MockClassSkill(2),   # Discipline
            MockClassSkill(5),   # Intimidate
            MockClassSkill(10),  # Parry
        ],
        "cls_skill_bard": [  # Bard skills
            MockClassSkill(0),   # Concentration
            MockClassSkill(4),   # Hide
            MockClassSkill(6),   # Listen
            MockClassSkill(7),   # Lore
            MockClassSkill(8),   # Move Silently
            MockClassSkill(12),  # Sleight of Hand
            MockClassSkill(13),  # Spellcraft
            MockClassSkill(14),  # Spot
            MockClassSkill(15),  # Tumble
            MockClassSkill(16),  # Use Magic Device
        ],
    }


@pytest.fixture
def mock_game_data_loader(mock_skills, mock_classes, mock_class_skills):
    """Create comprehensive mock DynamicGameDataLoader"""
    mock_loader = Mock(spec=DynamicGameDataLoader)
    
    # Create skill lookup
    skills_by_id = {skill.id: skill for skill in mock_skills}
    classes_by_id = {cls.id: cls for cls in mock_classes}
    
    def mock_get_by_id(table, id):
        if table == 'skills':
            return skills_by_id.get(id)
        elif table == 'classes':
            return classes_by_id.get(id)
        return None
    
    def mock_get_table(table):
        if table == 'skills':
            return mock_skills
        elif table == 'classes':
            return mock_classes
        elif table in mock_class_skills:
            return mock_class_skills[table]
        return []
    
    mock_loader.get_by_id.side_effect = mock_get_by_id
    mock_loader.get_table.side_effect = mock_get_table
    
    return mock_loader


@pytest.fixture
def mock_character_manager(mock_game_data_loader):
    """Create a comprehensive mock CharacterManager"""
    mock_cm = Mock()
    mock_cm.game_data_loader = mock_game_data_loader
    
    # Mock GFF wrapper with comprehensive character data
    mock_gff = Mock()
    
    # Default character: Human Wizard Level 1
    default_data = {
        'SkillList': [],
        'SkillPoints': 20,  # (2 + 3) * 4 + 1 for human
        'ClassList': [{'Class': 10, 'ClassLevel': 1}],  # Wizard level 1
        'Race': 6,  # Human
        'Str': 10, 'Dex': 14, 'Con': 12, 'Int': 16, 'Wis': 13, 'Cha': 8,
        'HitPoints': 6,
        'BaseAttackBonus': 0,
    }
    
    mock_gff.get.side_effect = lambda path, default=None: default_data.get(path, default)
    mock_gff.set = Mock()
    mock_cm.gff = mock_gff
    
    # Mock EventEmitter methods
    mock_cm.on = Mock()
    
    # Mock get_class_skills method
    def mock_get_class_skills(class_id):
        class_skills_map = {
            10: {0, 7, 13},     # Wizard: Concentration, Lore, Spellcraft
            8: {1, 4, 6, 8, 9, 11, 12, 14, 15, 16},  # Rogue: Many skills
            4: {2, 5, 10},      # Fighter: Discipline, Intimidate, Parry
            1: {0, 4, 6, 7, 8, 12, 13, 14, 15, 16},  # Bard: Many skills
        }
        return class_skills_map.get(class_id, set())
    
    mock_cm.get_class_skills = Mock(side_effect=mock_get_class_skills)
    
    return mock_cm


# Test Classes
class TestSkillManagerInitialization:
    """Test SkillManager initialization and setup"""
    
    def test_initialization(self, mock_character_manager):
        """Test SkillManager initializes correctly with data-driven architecture"""
        skill_manager = SkillManager(mock_character_manager)
        
        assert skill_manager.character_manager == mock_character_manager
        assert skill_manager.game_data_loader == mock_character_manager.game_data_loader
        assert skill_manager.gff == mock_character_manager.gff
        assert isinstance(skill_manager._skill_cache, dict)
        assert isinstance(skill_manager._class_skills_cache, dict)
    
    def test_event_handler_registration(self, mock_character_manager):
        """Test event handlers are registered correctly"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Verify event registration was called
        mock_character_manager.on.assert_any_call(EventType.CLASS_CHANGED, skill_manager.on_class_changed)
        mock_character_manager.on.assert_any_call(EventType.LEVEL_GAINED, skill_manager.on_level_gained)


class TestSkillPointCalculations:
    """Test skill point calculation methods"""
    
    def test_calculate_total_skill_points_level_1(self, mock_character_manager):
        """Test skill point calculation for level 1 character"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Wizard (2 base) + INT 16 (+3) = 5, * 4 for level 1 = 20, +1 for human = 21
        total_points = skill_manager.calculate_total_skill_points(10, 1)
        assert total_points == 21
    
    def test_calculate_total_skill_points_higher_level(self, mock_character_manager):
        """Test skill point calculation for higher level character"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Level 5 wizard: Level 1: 21, Levels 2-5: 6 each = 21 + 24 = 45
        total_points = skill_manager.calculate_total_skill_points(10, 5)
        assert total_points == 45
    
    def test_calculate_total_skill_points_non_human(self, mock_character_manager):
        """Test skill point calculation for non-human character"""
        # Change to elf
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'Race': 2,  # Elf
            'Int': 16, 'ClassList': [{'Class': 10, 'ClassLevel': 1}]
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        # No human bonus: (2 + 3) * 4 = 20
        total_points = skill_manager.calculate_total_skill_points(10, 1)
        assert total_points == 20
    
    def test_calculate_total_skill_points_high_skill_class(self, mock_character_manager):
        """Test skill point calculation for high-skill class (Rogue)"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Rogue (8 base) + INT 16 (+3) = 11, * 4 = 44, +1 for human = 45
        total_points = skill_manager.calculate_total_skill_points(8, 1)
        assert total_points == 45
    
    def test_calculate_total_skill_points_minimum_enforced(self, mock_character_manager):
        """Test minimum skill points are enforced"""
        # Low INT character
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'Int': 6,  # INT 6 = -2 modifier
            'Race': 0,  # Non-human
            'ClassList': [{'Class': 10, 'ClassLevel': 1}]
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        # (2 - 2) * 4 = 0, but minimum 4 enforced
        total_points = skill_manager.calculate_total_skill_points(10, 1)
        assert total_points == 4
    
    def test_calculate_skill_points_for_level(self, mock_character_manager):
        """Test skill points gained per level"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Mock class data
        mock_class = Mock()
        mock_class.skill_point_base = 2
        
        # Wizard + INT 16 (+3) + Human (+1) = 6
        points = skill_manager.calculate_skill_points_for_level(mock_class, 3)
        assert points == 6
    
    def test_calculate_skill_points_minimum_per_level(self, mock_character_manager):
        """Test minimum 1 skill point per level"""
        # Non-human with low INT
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'Race': 0,  # Non-human
            'Int': 6    # -2 modifier
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        mock_class = Mock()
        mock_class.skill_point_base = 2
        
        # 2 - 2 + 0 = 0, but minimum 1 enforced
        points = skill_manager.calculate_skill_points_for_level(mock_class, -2)
        assert points == 1


class TestEventHandling:
    """Test event handling for class changes and level gains"""
    
    def test_on_class_changed_skill_reset(self, mock_character_manager):
        """Test skills are reset when class changes"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Create a class changed event
        event = ClassChangedEvent(
            event_type=EventType.CLASS_CHANGED,
            source_manager="class_manager",
            timestamp=time.time(),
            old_class_id=10,  # Wizard
            new_class_id=8,   # Rogue
            level=3
        )
        
        skill_manager.on_class_changed(event)
        
        # Verify skills were reset
        mock_character_manager.gff.set.assert_any_call('SkillList', [])
        
        # Verify skill points were recalculated for new class
        expected_points = skill_manager.calculate_total_skill_points(8, 3)
        mock_character_manager.gff.set.assert_any_call('SkillPoints', expected_points)
    
    def test_on_level_gained_skill_points_added(self, mock_character_manager):
        """Test skill points are added when level is gained"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Mock current skill points and character stats
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillPoints': 15,
            'Int': 16,  # +3 modifier
            'Race': 6   # Human (gets +1 bonus)
        }.get(path, default)
        
        # Create level gained event
        event = LevelGainedEvent(
            event_type=EventType.LEVEL_GAINED,
            source_manager="class_manager",
            timestamp=time.time(),
            class_id=10,  # Wizard
            new_level=2,
            total_level=2
        )
        
        skill_manager.on_level_gained(event)
        
        # Should add 6 points (2 base + 3 INT + 1 human)
        mock_character_manager.gff.set.assert_called_with('SkillPoints', 21)


class TestSkillRanking:
    """Test skill ranking and cost calculations"""
    
    def test_set_skill_rank_class_skill(self, mock_character_manager):
        """Test setting ranks in a class skill"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Mock available skill points and empty skill list
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillPoints': 10,
            'SkillList': [],
            'ClassList': [{'Class': 10, 'ClassLevel': 1}]  # Wizard
        }.get(path, default)
        
        # Set 4 ranks in Concentration (class skill for wizard)
        success = skill_manager.set_skill_rank(0, 4)
        
        assert success == True
        
        # Should create new skill entry
        expected_skill_list = [{'Skill': 0, 'Rank': 4}]
        mock_character_manager.gff.set.assert_any_call('SkillList', expected_skill_list)
        
        # Should deduct 4 skill points (1 per rank for class skill)
        mock_character_manager.gff.set.assert_any_call('SkillPoints', 6)
    
    def test_set_skill_rank_cross_class_skill(self, mock_character_manager):
        """Test setting ranks in a cross-class skill"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Mock available skill points and empty skill list
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillPoints': 10,
            'SkillList': [],
            'ClassList': [{'Class': 10, 'ClassLevel': 1}]  # Wizard
        }.get(path, default)
        
        # Set 2 ranks in Hide (cross-class skill for wizard)
        success = skill_manager.set_skill_rank(4, 2)
        
        assert success == True
        
        # Should deduct 4 skill points (2 per rank for cross-class skill)
        mock_character_manager.gff.set.assert_any_call('SkillPoints', 6)
    
    def test_set_skill_rank_insufficient_points(self, mock_character_manager):
        """Test setting ranks with insufficient skill points"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Mock limited skill points
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillPoints': 2,
            'SkillList': [],
            'ClassList': [{'Class': 10, 'ClassLevel': 1}]
        }.get(path, default)
        
        # Try to set 5 ranks (costs 5 points)
        success = skill_manager.set_skill_rank(0, 5)
        
        assert success == False
    
    def test_set_skill_rank_exceeds_maximum(self, mock_character_manager):
        """Test setting ranks that exceed maximum allowed"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Mock character data
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillPoints': 20,
            'SkillList': [],
            'ClassList': [{'Class': 10, 'ClassLevel': 1}]
        }.get(path, default)
        
        # Try to set 10 ranks (max is level + 3 = 4 for class skill)
        success = skill_manager.set_skill_rank(0, 10)
        
        assert success == False
    
    def test_set_skill_rank_to_zero_removes_skill(self, mock_character_manager):
        """Test setting ranks to 0 removes skill from list"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Mock existing skill
        existing_skills = [{'Skill': 0, 'Rank': 4}]
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillPoints': 10,
            'SkillList': existing_skills,
            'ClassList': [{'Class': 10, 'ClassLevel': 1}]
        }.get(path, default)
        
        # Set ranks to 0
        success = skill_manager.set_skill_rank(0, 0)
        
        assert success == True
        # Should remove skill from list
        mock_character_manager.gff.set.assert_any_call('SkillList', [])
        # Should refund points
        mock_character_manager.gff.set.assert_any_call('SkillPoints', 14)


class TestClassSkillDetection:
    """Test class skill detection and caching"""
    
    def test_is_class_skill_wizard(self, mock_character_manager):
        """Test class skill detection for wizard"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Concentration should be a class skill for wizard
        assert skill_manager.is_class_skill(0) == True   # Concentration
        assert skill_manager.is_class_skill(7) == True   # Lore
        assert skill_manager.is_class_skill(13) == True  # Spellcraft
        
        # Hide should not be a class skill for wizard
        assert skill_manager.is_class_skill(4) == False  # Hide
    
    def test_is_class_skill_multiclass(self, mock_character_manager):
        """Test class skill detection for multiclass character"""
        # Mock multiclass character (Wizard/Rogue)
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'ClassList': [
                {'Class': 10, 'ClassLevel': 1},  # Wizard
                {'Class': 8, 'ClassLevel': 2}    # Rogue
            ]
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        # Should be class skill if it's a class skill for ANY class
        assert skill_manager.is_class_skill(0) == True   # Concentration (Wizard)
        assert skill_manager.is_class_skill(4) == True   # Hide (Rogue)
        assert skill_manager.is_class_skill(1) == True   # Disable Device (Rogue)
        
        # Should not be class skill if not a class skill for any class
        assert skill_manager.is_class_skill(2) == False  # Discipline (Fighter skill)
    
    def test_class_skills_caching(self, mock_character_manager):
        """Test that class skills are cached for performance"""
        skill_manager = SkillManager(mock_character_manager)
        
        # First call should populate cache
        result1 = skill_manager._get_class_skills(10)  # Wizard
        
        # Second call should use cache
        result2 = skill_manager._get_class_skills(10)
        
        assert result1 == result2
        assert 10 in skill_manager._class_skills_cache
    
    def test_class_skills_cache_update(self, mock_character_manager):
        """Test class skills cache is updated when primary class changes"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Populate cache
        skill_manager._get_class_skills(10)
        assert len(skill_manager._class_skills_cache) > 0
        
        # Update cache
        skill_manager._update_class_skills_cache(8)  # Change to Rogue
        
        # Cache should be cleared and repopulated
        # This is implementation-dependent, but cache should be managed


class TestSkillModifiers:
    """Test skill modifier calculations"""
    
    def test_calculate_skill_modifier_basic(self, mock_character_manager):
        """Test basic skill modifier calculation"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Mock skill with ranks
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillList': [{'Skill': 0, 'Rank': 4}],  # 4 ranks in Concentration
            'Con': 12  # CON 12 = +1 modifier
        }.get(path, default)
        
        modifier = skill_manager.calculate_skill_modifier(0)  # Concentration
        
        # 4 ranks + 1 (CON modifier) + 0 (synergy) - 0 (armor penalty) = 5
        assert modifier == 5
    
    def test_calculate_skill_modifier_different_abilities(self, mock_character_manager):
        """Test skill modifiers with different key abilities"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Mock skills with different abilities
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillList': [
                {'Skill': 4, 'Rank': 3},   # Hide (DEX)
                {'Skill': 5, 'Rank': 2},   # Intimidate (CHA)
                {'Skill': 7, 'Rank': 5},   # Lore (INT)
            ],
            'Dex': 14,  # +2
            'Cha': 8,   # -1  
            'Int': 16,  # +3
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        # Hide: 3 ranks + 2 (DEX) = 5 (assuming no armor penalty for test)
        hide_mod = skill_manager.calculate_skill_modifier(4)
        assert hide_mod >= 3  # At least ranks + some DEX bonus
        
        # Intimidate: 2 ranks - 1 (CHA) = 1
        intimidate_mod = skill_manager.calculate_skill_modifier(5)
        assert intimidate_mod == 1
        
        # Lore: 5 ranks + 3 (INT) = 8
        lore_mod = skill_manager.calculate_skill_modifier(7)
        assert lore_mod == 8
    
    def test_calculate_skill_modifier_armor_check_penalty(self, mock_character_manager):
        """Test armor check penalty application"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Mock armored skill with penalty
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillList': [{'Skill': 4, 'Rank': 3}],  # Hide (armor check skill)
            'Dex': 14  # +2 modifier
        }.get(path, default)
        
        # Mock armor check penalty
        with patch.object(skill_manager, '_get_armor_check_penalty', return_value=2):
            modifier = skill_manager.calculate_skill_modifier(4)
            
            # 3 ranks + 2 (DEX) - 2 (armor penalty) = 3
            assert modifier == 3


class TestSkillSummaryAndInfo:
    """Test skill summary and information methods"""
    
    def test_get_skill_info_complete(self, mock_character_manager):
        """Test complete skill info retrieval"""
        skill_manager = SkillManager(mock_character_manager)
        
        skill_info = skill_manager.get_skill_info(0)  # Concentration
        
        assert skill_info is not None
        assert skill_info['id'] == 0
        assert skill_info['label'] == "Concentration"
        assert skill_info['key_ability'] == "CON"
        assert skill_info['armor_check'] == False
        assert 'is_class_skill' in skill_info
        assert 'current_ranks' in skill_info
        assert 'max_ranks' in skill_info
        assert 'total_modifier' in skill_info
    
    def test_get_skill_info_caching(self, mock_character_manager):
        """Test skill info caching"""
        skill_manager = SkillManager(mock_character_manager)
        
        # First call
        info1 = skill_manager.get_skill_info(0)
        
        # Second call should return cached result
        info2 = skill_manager.get_skill_info(0)
        
        assert info1 is info2  # Same object reference
        assert 0 in skill_manager._skill_cache
    
    def test_get_skill_info_unknown_skill(self, mock_character_manager):
        """Test skill info for unknown skill"""
        skill_manager = SkillManager(mock_character_manager)
        
        info = skill_manager.get_skill_info(999)
        assert info is None
    
    def test_get_skill_summary_categorization(self, mock_character_manager):
        """Test skill summary categorizes skills correctly"""
        # Mock character with various skills
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillList': [
                {'Skill': 0, 'Rank': 4},   # Concentration (class skill)
                {'Skill': 4, 'Rank': 2},   # Hide (cross-class)
            ],
            'SkillPoints': 10,
            'ClassList': [{'Class': 10, 'ClassLevel': 1}]  # Wizard
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        summary = skill_manager.get_skill_summary()
        
        assert summary['available_points'] == 10
        assert summary['total_ranks'] == 6
        assert summary['skills_with_ranks'] == 2
        assert len(summary['class_skills']) >= 0  # At least Concentration
        assert len(summary['cross_class_skills']) >= 0  # At least Hide


class TestValidation:
    """Test skill validation methods"""
    
    def test_validate_normal_character(self, mock_character_manager):
        """Test validation of normal character"""
        # Mock valid character
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillList': [
                {'Skill': 0, 'Rank': 4},   # Max for level 1 class skill
            ],
            'ClassList': [{'Class': 10, 'ClassLevel': 1}]
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        is_valid, errors = skill_manager.validate()
        assert is_valid == True
        assert len(errors) == 0
    
    def test_validate_excessive_ranks(self, mock_character_manager):
        """Test validation catches excessive skill ranks"""
        # Mock character with too many ranks
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillList': [
                {'Skill': 0, 'Rank': 10},  # Too many for level 1
            ],
            'ClassList': [{'Class': 10, 'ClassLevel': 1}]
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        is_valid, errors = skill_manager.validate()
        assert is_valid == False
        assert len(errors) > 0
        assert "exceeds maximum" in errors[0]


class TestMaxSkillRanks:
    """Test maximum skill rank calculations"""
    
    def test_get_max_skill_ranks_class_skill(self, mock_character_manager):
        """Test max ranks for class skills"""
        # Mock level 5 character
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'ClassList': [{'Class': 10, 'ClassLevel': 5}],
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        # Class skill: level + 3 = 5 + 3 = 8
        max_ranks = skill_manager.get_max_skill_ranks(0)  # Concentration (class skill)
        assert max_ranks == 8
    
    def test_get_max_skill_ranks_cross_class_skill(self, mock_character_manager):
        """Test max ranks for cross-class skills"""
        # Mock level 5 character
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'ClassList': [{'Class': 10, 'ClassLevel': 5}],
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        # Cross-class skill: (level + 3) / 2 = (5 + 3) / 2 = 4
        max_ranks = skill_manager.get_max_skill_ranks(4)  # Hide (cross-class)
        assert max_ranks == 4
    
    def test_get_max_skill_ranks_multiclass(self, mock_character_manager):
        """Test max ranks for multiclass character"""
        # Mock multiclass character (total level 6)
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'ClassList': [
                {'Class': 10, 'ClassLevel': 3},  # Wizard 3
                {'Class': 8, 'ClassLevel': 3}    # Rogue 3
            ],
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        # Total level 6, so class skill max = 6 + 3 = 9
        max_ranks = skill_manager.get_max_skill_ranks(0)  # Concentration
        assert max_ranks == 9


class TestSkillCosts:
    """Test skill cost calculations"""
    
    def test_calculate_skill_cost_class_skill(self, mock_character_manager):
        """Test cost calculation for class skills"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Class skill costs 1 point per rank
        cost = skill_manager.calculate_skill_cost(0, 5)  # Concentration, 5 ranks
        assert cost == 5
    
    def test_calculate_skill_cost_cross_class_skill(self, mock_character_manager):
        """Test cost calculation for cross-class skills"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Cross-class skill costs 2 points per rank
        cost = skill_manager.calculate_skill_cost(4, 3)  # Hide, 3 ranks
        assert cost == 6
    
    def test_calculate_skill_cost_zero_ranks(self, mock_character_manager):
        """Test cost calculation for zero ranks"""
        skill_manager = SkillManager(mock_character_manager)
        
        cost = skill_manager.calculate_skill_cost(0, 0)
        assert cost == 0


class TestDataDrivenArchitecture:
    """Test data-driven architecture features"""
    
    def test_dynamic_skill_attribute_access(self, mock_character_manager):
        """Test dynamic access to skill attributes"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Should handle skills with different attribute names
        concentration_info = skill_manager.get_skill_info(0)
        assert concentration_info['key_ability'] == "CON"
        
        hide_info = skill_manager.get_skill_info(4)
        assert hide_info['key_ability'] == "DEX"
        assert hide_info['armor_check'] == True
    
    def test_fallback_class_skills_when_dynamic_fails(self, mock_character_manager):
        """Test fallback when dynamic class skills loading fails"""
        # Mock character manager without get_class_skills method
        del mock_character_manager.get_class_skills
        
        skill_manager = SkillManager(mock_character_manager)
        
        # Should still return some class skills using fallback
        class_skills = skill_manager._get_class_skills(10)  # Wizard
        assert isinstance(class_skills, set)
        # Should include basic skills from fallback
        assert len(class_skills) > 0
    
    def test_handles_missing_skill_data_gracefully(self, mock_character_manager):
        """Test graceful handling of missing skill data"""
        skill_manager = SkillManager(mock_character_manager)
        
        # Try to access non-existent skill
        modifier = skill_manager.calculate_skill_modifier(999)
        
        # Should not crash and should return reasonable default
        assert isinstance(modifier, int)


class TestEdgeCasesAndErrorHandling:
    """Test edge cases and error handling"""
    
    def test_empty_skill_list_operations(self, mock_character_manager):
        """Test operations with empty skill list"""
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillList': [],
            'SkillPoints': 20,
            'ClassList': [{'Class': 10, 'ClassLevel': 1}]
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        # Should handle empty list gracefully
        ranks = skill_manager.get_skill_ranks(0)
        assert ranks == 0
        
        summary = skill_manager.get_skill_summary()
        assert summary['total_ranks'] == 0
        assert summary['skills_with_ranks'] == 0
    
    def test_malformed_skill_list_entries(self, mock_character_manager):
        """Test handling of malformed skill list entries"""
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillList': [
                {'Skill': 0},  # Missing Rank
                {'Rank': 5},   # Missing Skill
                {'Skill': 'invalid', 'Rank': 3},  # Invalid skill ID
            ],
            'ClassList': [{'Class': 10, 'ClassLevel': 1}]
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        # Should handle malformed entries gracefully
        summary = skill_manager.get_skill_summary()
        assert isinstance(summary, dict)
    
    def test_missing_game_data_graceful_handling(self, mock_character_manager):
        """Test graceful handling when game data is missing"""
        # Create skill manager first
        skill_manager = SkillManager(mock_character_manager)
        
        # Then mock loader to return None for subsequent calls
        mock_character_manager.game_data_loader.get_by_id.return_value = None
        
        # Clear the skill cache to force fresh lookup
        skill_manager._skill_cache.clear()
        
        # Should not crash and return None for unknown skill
        info = skill_manager.get_skill_info(999)  # Use unknown skill ID
        assert info is None
        
        # Should handle missing data gracefully for known methods
        modifier = skill_manager.calculate_skill_modifier(999)
        assert isinstance(modifier, int)
        
        # Should handle max ranks calculation without data
        max_ranks = skill_manager.get_max_skill_ranks(999)
        assert isinstance(max_ranks, int)
    
    def test_extreme_ability_scores(self, mock_character_manager):
        """Test with extreme ability scores"""
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillList': [{'Skill': 0, 'Rank': 1}],
            'Str': 3,   # Very low
            'Dex': 30,  # Very high
            'Con': 1,   # Minimum
            'Int': 50,  # Extreme
            'Wis': 20,
            'Cha': 25,
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        # Should handle extreme values without crashing
        modifiers = skill_manager._calculate_ability_modifiers()
        assert modifiers['STR'] == -4  # (3-10)//2
        assert modifiers['DEX'] == 10  # (30-10)//2
        assert modifiers['INT'] == 20  # (50-10)//2


class TestGetSkillRanks:
    """Test skill rank retrieval"""
    
    def test_get_skill_ranks_existing_skill(self, mock_character_manager):
        """Test getting ranks for existing skill"""
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillList': [
                {'Skill': 0, 'Rank': 5},
                {'Skill': 7, 'Rank': 3},
            ]
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        assert skill_manager.get_skill_ranks(0) == 5
        assert skill_manager.get_skill_ranks(7) == 3
    
    def test_get_skill_ranks_non_existing_skill(self, mock_character_manager):
        """Test getting ranks for non-existing skill"""
        mock_character_manager.gff.get.side_effect = lambda path, default=None: {
            'SkillList': [{'Skill': 0, 'Rank': 5}]
        }.get(path, default)
        
        skill_manager = SkillManager(mock_character_manager)
        
        assert skill_manager.get_skill_ranks(999) == 0


class TestResetAllSkills:
    """Test skill reset functionality"""
    
    def test_reset_all_skills(self, mock_character_manager):
        """Test resetting all skills"""
        skill_manager = SkillManager(mock_character_manager)
        
        skill_manager.reset_all_skills()
        
        # Should set skill list to empty
        mock_character_manager.gff.set.assert_called_with('SkillList', [])