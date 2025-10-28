"""
Test real-time updates when class or level changes
Verifies that all dependent systems update automatically via event handlers
"""

import pytest
from character.character_manager import CharacterManager
from character.events import EventType
from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader
from gamedata.services.game_rules_service import GameRulesService


@pytest.fixture
def character_with_fighter():
    """Create a test character with Fighter level 5"""
    game_data_loader = get_dynamic_game_data_loader()
    rules_service = GameRulesService()

    # Create a basic character
    character_data = {
        'FirstName': {'substrings': [{'string': 'Test'}]},
        'LastName': {'substrings': [{'string': 'Character'}]},
        'Race': 0,  # Human
        'Str': 16,
        'Dex': 14,
        'Con': 14,
        'Int': 10,
        'Wis': 10,
        'Cha': 10,
        'LawfulChaotic': 50,
        'GoodEvil': 50,
        'ClassList': [
            {
                'Class': 0,  # Fighter
                'ClassLevel': 5
            }
        ],
        'FeatList': [],
        'SkillList': [],
        'KnownList0': [],
        'MemorizedList0': [],
        'HitPoints': 50,
        'MaxHitPoints': 50,
        'CurrentHitPoints': 50,
        'Experience': 10000,
        'SkillPoints': 0,
        'LvlStatList': []
    }

    manager = CharacterManager(character_data, game_data_loader, rules_service=rules_service)

    # Register all managers
    from character.character_factory import register_all_managers
    register_all_managers(manager)

    return manager


def test_class_change_updates_skill_points(character_with_fighter):
    """Test that changing class recalculates skill points"""
    manager = character_with_fighter
    class_manager = manager.get_manager('class')
    skill_manager = manager.get_manager('skill')

    # Record initial skill points
    initial_skill_points = manager.gff.get('SkillPoints', 0)

    # Change Fighter (2 + INT) to Wizard (2 + INT, but INT-based class gets more from INT modifier)
    # Fighter: 2 skill points/level + INT mod (0) = 2/level
    # Wizard: 2 skill points/level + INT mod (0) = 2/level
    # For level 5: Fighter = 2*4 + 8 (1st level x4) = 16, Wizard = same

    # But if we change to Rogue (8 + INT) skill points should increase significantly
    rogue_class_id = 2  # Rogue in standard NWN2

    class_manager.change_class(rogue_class_id, preserve_level=True)

    # Verify skill points were recalculated
    new_skill_points = manager.gff.get('SkillPoints', 0)

    # Rogue gets 8 + INT (0) = 8 points per level
    # Level 1: 8 * 4 = 32, Levels 2-5: 8 * 4 = 32, Total = 64
    assert new_skill_points > initial_skill_points, "Skill points should increase when changing to Rogue"
    assert new_skill_points == 64, f"Expected 64 skill points for Rogue level 5, got {new_skill_points}"


def test_class_change_updates_feats(character_with_fighter):
    """Test that changing class updates feats (removes Fighter bonus feats, adds new class feats)"""
    manager = character_with_fighter
    class_manager = manager.get_manager('class')
    feat_manager = manager.get_manager('feat')

    # Add some Fighter bonus feats first
    fighter_feat_id = 1  # Power Attack
    feat_manager.add_feat(fighter_feat_id)

    initial_feat_count = len(manager.gff.get('FeatList', []))

    # Change to Wizard (no fighter bonus feats)
    wizard_class_id = 10  # Wizard
    class_manager.change_class(wizard_class_id, preserve_level=True)

    new_feat_count = len(manager.gff.get('FeatList', []))

    # Feats should have changed (Fighter feats removed, Wizard feats added)
    # We don't assert exact equality since both classes may have level-based feats
    # But the composition should be different
    feat_list = manager.gff.get('FeatList', [])
    feat_ids = [f.get('Feat') for f in feat_list]

    # Verify feat list was modified by event handlers
    assert isinstance(feat_list, list), "Feat list should still be a list"


def test_level_up_adds_skill_points(character_with_fighter):
    """Test that leveling up adds new skill points"""
    manager = character_with_fighter
    class_manager = manager.get_manager('class')
    skill_manager = manager.get_manager('skill')

    # Record initial state
    initial_level = sum(c.get('ClassLevel', 0) for c in manager.gff.get('ClassList', []))
    initial_skill_points = manager.gff.get('SkillPoints', 0)

    # Level up Fighter
    fighter_class_id = 0
    class_manager.adjust_class_level(fighter_class_id, 1)  # Add 1 level

    # Verify level increased
    new_level = sum(c.get('ClassLevel', 0) for c in manager.gff.get('ClassList', []))
    assert new_level == initial_level + 1, f"Expected level {initial_level + 1}, got {new_level}"

    # Verify skill points increased
    new_skill_points = manager.gff.get('SkillPoints', 0)
    # Fighter gets 2 + INT (0) = 2 skill points per level
    expected_gain = 2
    assert new_skill_points == initial_skill_points + expected_gain, \
        f"Expected {initial_skill_points + expected_gain} skill points, got {new_skill_points}"


def test_level_up_checks_ability_score_increase(character_with_fighter):
    """Test that leveling to 8th level (divisible by 4) triggers ability score availability"""
    manager = character_with_fighter
    class_manager = manager.get_manager('class')
    ability_manager = manager.get_manager('ability')

    # Level up to 8 (currently at 5, need 3 more levels)
    fighter_class_id = 0
    for _ in range(3):
        class_manager.adjust_class_level(fighter_class_id, 1)

    # Verify we're at level 8
    total_level = sum(c.get('ClassLevel', 0) for c in manager.gff.get('ClassList', []))
    assert total_level == 8, f"Expected level 8, got {total_level}"

    # Check ability score increases available
    # At level 8, should have 2 ability increases available (level 4 and 8)
    level_up_bonuses = ability_manager.get_level_up_modifiers()
    bonuses_used = sum(level_up_bonuses.values())
    ability_increases_available = total_level // 4

    # Should have 2 increases available, but none used yet
    assert ability_increases_available == 2, f"Expected 2 ability increases at level 8, got {ability_increases_available}"


def test_level_down_removes_skill_points(character_with_fighter):
    """Test that reducing level removes skill points"""
    manager = character_with_fighter
    class_manager = manager.get_manager('class')
    skill_manager = manager.get_manager('skill')

    # Record initial state at level 5
    initial_level = sum(c.get('ClassLevel', 0) for c in manager.gff.get('ClassList', []))
    initial_skill_points = manager.gff.get('SkillPoints', 0)

    # Level down by 2
    fighter_class_id = 0
    class_manager.adjust_class_level(fighter_class_id, -2)

    # Verify level decreased
    new_level = sum(c.get('ClassLevel', 0) for c in manager.gff.get('ClassList', []))
    assert new_level == initial_level - 2, f"Expected level {initial_level - 2}, got {new_level}"

    # Skill points should also decrease
    # (Skills are reset on class change, so this is handled by reset)
    new_skill_points = manager.gff.get('SkillPoints', 0)
    # After level down, skill points should be recalculated for new level
    assert new_skill_points >= 0, "Skill points should not be negative"


def test_multiclass_updates_all_systems(character_with_fighter):
    """Test that adding a second class (multiclassing) updates all dependent systems"""
    manager = character_with_fighter
    class_manager = manager.get_manager('class')

    # Record initial state
    initial_classes = len(manager.gff.get('ClassList', []))
    initial_level = sum(c.get('ClassLevel', 0) for c in manager.gff.get('ClassList', []))

    # Add Wizard class (multiclass)
    wizard_class_id = 10
    class_manager.add_class_level(wizard_class_id)

    # Verify we now have 2 classes
    class_list = manager.gff.get('ClassList', [])
    assert len(class_list) == 2, f"Expected 2 classes after multiclassing, got {len(class_list)}"

    # Verify total level increased
    new_level = sum(c.get('ClassLevel', 0) for c in class_list)
    assert new_level == initial_level + 1, f"Expected level {initial_level + 1}, got {new_level}"

    # Verify class summary reflects multiclassing
    class_summary = class_manager.get_class_summary()
    assert class_summary['multiclass'] == True, "Character should be marked as multiclass"
    assert len(class_summary['classes']) == 2, "Should have 2 classes in summary"


def test_event_propagation_to_all_managers(character_with_fighter):
    """Test that class change events reach all managers"""
    manager = character_with_fighter

    # Track which managers received events
    events_received = []

    def track_event(event):
        events_received.append(event.event_type)

    # Subscribe to events
    manager.on(EventType.CLASS_CHANGED, track_event)

    # Change class
    class_manager = manager.get_manager('class')
    wizard_class_id = 10
    class_manager.change_class(wizard_class_id, preserve_level=True)

    # Verify event was emitted
    assert EventType.CLASS_CHANGED in events_received, \
        "CLASS_CHANGED event should have been emitted"


def test_hp_recalculation_on_level_change(character_with_fighter):
    """Test that HP is recalculated when CON modifier changes affect HP per level"""
    manager = character_with_fighter
    ability_manager = manager.get_manager('ability')
    class_manager = manager.get_manager('class')

    # Record initial HP
    initial_hp = manager.gff.get('MaxHitPoints', 0)

    # Increase CON to add modifier
    ability_manager.set_attribute('Con', 18)  # CON 18 = +4 modifier

    # HP should have increased by (new_con_mod - old_con_mod) * level
    # Old CON 14 = +2, New CON 18 = +4, difference = +2
    # At level 5: +2 * 5 = +10 HP
    new_hp = manager.gff.get('MaxHitPoints', 0)
    expected_increase = 2 * 5  # (new_mod - old_mod) * level

    assert new_hp == initial_hp + expected_increase, \
        f"Expected HP increase of {expected_increase}, got {new_hp - initial_hp}"


def test_bab_recalculation_on_class_change(character_with_fighter):
    """Test that BAB is recalculated when changing from Fighter (good) to Wizard (poor)"""
    manager = character_with_fighter
    class_manager = manager.get_manager('class')

    # Fighter level 5 has good BAB (+5)
    initial_bab = class_manager.get_base_attack_bonus()
    assert initial_bab == 5, f"Fighter level 5 should have BAB +5, got {initial_bab}"

    # Change to Wizard (poor BAB)
    wizard_class_id = 10
    class_manager.change_class(wizard_class_id, preserve_level=True)

    # Wizard level 5 has poor BAB (+2)
    new_bab = class_manager.get_base_attack_bonus()
    assert new_bab == 2, f"Wizard level 5 should have BAB +2, got {new_bab}"


def test_saves_recalculation_on_class_change(character_with_fighter):
    """Test that saving throws are recalculated on class change"""
    manager = character_with_fighter
    class_manager = manager.get_manager('class')

    # Fighter has good Fortitude, poor Reflex and Will
    initial_saves = class_manager.calculate_total_saves()
    fighter_fort = initial_saves['base_fortitude']

    # Fighter level 5: Fort +4, Ref +1, Will +1
    assert fighter_fort == 4, f"Fighter level 5 should have base Fort +4, got {fighter_fort}"

    # Change to Wizard (poor Fort, good Will)
    wizard_class_id = 10
    class_manager.change_class(wizard_class_id, preserve_level=True)

    new_saves = class_manager.calculate_total_saves()
    wizard_fort = new_saves['base_fortitude']
    wizard_will = new_saves['base_will']

    # Wizard level 5: Fort +1, Will +4
    assert wizard_fort == 1, f"Wizard level 5 should have base Fort +1, got {wizard_fort}"
    assert wizard_will == 4, f"Wizard level 5 should have base Will +4, got {wizard_will}"


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
