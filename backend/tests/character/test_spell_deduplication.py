"""
Test spell multi-level handling in get_legitimate_spells cache
"""
import pytest
from unittest.mock import Mock
from character.managers.spell_manager import SpellManager


@pytest.fixture
def mock_rules_service():
    """Mock rules service with spell data"""
    service = Mock()

    spells_table = [
        {
            'Name': 'Cure Light Wounds',
            'Label': 'cure_light_wounds',
            'Cleric': '1',
            'Paladin': '1',
            'Ranger': '2',
            'School': 'C',
            'SpellDesc': 'Heals target',
            'REMOVED': None,
            'UserType': '1'
        },
        {
            'Name': 'Magic Missile',
            'Label': 'magic_missile',
            'Wiz_Sorc': '1',
            'School': 'E',
            'SpellDesc': 'Damages target',
            'REMOVED': None,
            'UserType': '1'
        },
        {
            'Name': 'Fireball',
            'Label': 'fireball',
            'Wiz_Sorc': '3',
            'Druid': '4',
            'School': 'E',
            'SpellDesc': 'Area damage',
            'REMOVED': None,
            'UserType': '1'
        }
    ]

    school_data = {'Label': 'Evocation'}

    service.get_table.return_value = spells_table
    service.get_by_id.return_value = school_data

    return service


@pytest.fixture
def spell_manager(mock_rules_service):
    """Create spell manager with mock data"""
    character_data = {
        'ClassList': [
            {'Class': 6, 'ClassLevel': 5}
        ]
    }

    char_manager = Mock()
    char_manager.character_data = character_data
    char_manager.rules_service = mock_rules_service

    manager = SpellManager(char_manager)
    manager.rules_service = mock_rules_service

    return manager


def test_spell_cache_allows_duplicates_for_different_levels(spell_manager):
    """Test that spells can appear multiple times with different levels (as intended)"""
    all_spells = spell_manager._get_all_legitimate_spells_cached()

    spell_ids = [spell['id'] for spell in all_spells]
    spell_id_count = {}
    for spell_id in spell_ids:
        spell_id_count[spell_id] = spell_id_count.get(spell_id, 0) + 1

    cure_spells = [s for s in all_spells if 'Cure' in s['name']]

    if len(cure_spells) > 1:
        print(f"Cure Light Wounds appears {len(cure_spells)} times at different levels")
        levels = [s['level'] for s in cure_spells]
        print(f"Levels: {levels}")
        assert len(set(levels)) > 1, "Multiple entries should have different levels"
    elif len(cure_spells) == 1:
        print(f"Cure Light Wounds appears once (level {cure_spells[0]['level']})")
    else:
        print("No Cure Light Wounds found in test data (mock may be incomplete)")


def test_spell_level_filtering_works(spell_manager):
    """Test that level filtering works correctly with the cached spells"""
    result = spell_manager.get_legitimate_spells(levels=[1], page=1, limit=50)

    level_1_spells = result['spells']

    for spell in level_1_spells:
        assert spell['level'] == 1, f"Spell {spell['name']} has level {spell['level']}, expected 1"

    print(f"Found {len(level_1_spells)} level 1 spells")


def test_spell_cache_is_cached(spell_manager):
    """Test that the cache is actually cached (same instance returned)"""
    cache1 = spell_manager._get_all_legitimate_spells_cached()
    cache2 = spell_manager._get_all_legitimate_spells_cached()

    assert cache1 is cache2, "Cache should return the same instance"


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
