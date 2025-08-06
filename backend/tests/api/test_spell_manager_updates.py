"""
Test the updated SpellManager methods
"""
import pytest
from unittest.mock import Mock, MagicMock
from character.managers.spell_manager import SpellManager
from character.character_manager import CharacterManager


class TestSpellManagerUpdates:
    """Test the new public methods in SpellManager"""
    
    @pytest.fixture
    def mock_character_manager(self):
        """Create a mock character manager with game data"""
        manager = Mock(spec=CharacterManager)
        
        # Mock game data
        manager.game_data = Mock()
        
        # Mock spell data
        spell_0 = Mock()
        spell_0.Wiz_Sorc = 0  # Cantrip for wizard/sorcerer
        spell_0.Cleric = None
        spell_0.Bard = 1
        
        spell_100 = Mock()
        spell_100.Wiz_Sorc = 1  # Level 1 spell
        spell_100.Cleric = 1
        spell_100.Bard = None
        
        manager.game_data.spells = {
            0: spell_0,
            100: spell_100
        }
        
        # Mock class data
        wizard_class = Mock()
        wizard_class.label = "Wizard"
        wizard_class.spelltablecolumn = "Wiz_Sorc"
        wizard_class.memorizespells = 1  # Prepares spells
        wizard_class.spellcaster = 1  # Full progression
        
        sorcerer_class = Mock()
        sorcerer_class.label = "Sorcerer"
        sorcerer_class.spelltablecolumn = "Wiz_Sorc"
        sorcerer_class.memorizespells = 0  # Spontaneous
        sorcerer_class.spellknowntable = "cls_spkn_sorc"
        sorcerer_class.spellcaster = 1
        
        cleric_class = Mock()
        cleric_class.label = "Cleric"
        cleric_class.spelltablecolumn = "Cleric"
        cleric_class.memorizespells = 1
        cleric_class.spellcaster = 1
        
        paladin_class = Mock()
        paladin_class.label = "Paladin"
        paladin_class.spelltablecolumn = "Paladin"
        paladin_class.memorizespells = 1
        paladin_class.spellcaster = 2  # Level - 3 progression
        
        manager.game_data.classes = {
            10: wizard_class,   # Wizard
            9: sorcerer_class,  # Sorcerer
            2: cleric_class,    # Cleric
            6: paladin_class    # Paladin
        }
        
        # Mock GFF wrapper
        manager.gff = Mock()
        manager.gff.get = MagicMock(return_value=[])
        manager.gff.set = MagicMock()
        
        return manager
    
    def test_get_spell_level_for_class(self, mock_character_manager):
        """Test getting spell level for different classes"""
        spell_manager = SpellManager(mock_character_manager)
        
        # Test wizard cantrip
        assert spell_manager.get_spell_level_for_class(0, 10) == 0
        
        # Test wizard level 1 spell
        assert spell_manager.get_spell_level_for_class(100, 10) == 1
        
        # Test cleric level 1 spell
        assert spell_manager.get_spell_level_for_class(100, 2) == 1
        
        # Test spell not available for class
        assert spell_manager.get_spell_level_for_class(0, 2) is None
        
        # Test invalid spell
        assert spell_manager.get_spell_level_for_class(999, 10) is None
        
        # Test invalid class
        assert spell_manager.get_spell_level_for_class(100, 999) is None
    
    def test_is_spellcaster(self, mock_character_manager):
        """Test checking if a class can cast spells"""
        spell_manager = SpellManager(mock_character_manager)
        
        # Add spell progression indicator
        mock_character_manager.game_data.classes[10].spellgaintable = "cls_spgn_wiz"
        mock_character_manager.game_data.classes[2].spellgaintable0 = "cls_spgn_cler"
        
        assert spell_manager.is_spellcaster(10) is True  # Wizard
        assert spell_manager.is_spellcaster(2) is True   # Cleric
        assert spell_manager.is_spellcaster(999) is False  # Invalid class
    
    def test_is_prepared_caster(self, mock_character_manager):
        """Test checking if a class prepares spells"""
        spell_manager = SpellManager(mock_character_manager)
        
        assert spell_manager.is_prepared_caster(10) is True   # Wizard prepares
        assert spell_manager.is_prepared_caster(9) is False   # Sorcerer is spontaneous
        assert spell_manager.is_prepared_caster(2) is True    # Cleric prepares
        assert spell_manager.is_prepared_caster(999) is False  # Invalid class
    
    def test_get_caster_level(self, mock_character_manager):
        """Test calculating caster level"""
        spell_manager = SpellManager(mock_character_manager)
        
        # Mock class list
        mock_character_manager.gff.get.return_value = [
            {"Class": 10, "ClassLevel": 5},   # Level 5 wizard
            {"Class": 6, "ClassLevel": 8}     # Level 8 paladin
        ]
        
        # Wizard has full progression
        assert spell_manager.get_caster_level(0) == 5
        
        # Paladin has level - 3 progression
        assert spell_manager.get_caster_level(1) == 5  # 8 - 3 = 5
        
        # Invalid index
        assert spell_manager.get_caster_level(99) == 0
    
    def test_add_and_remove_known_spell(self, mock_character_manager):
        """Test adding and removing known spells"""
        spell_manager = SpellManager(mock_character_manager)
        
        # Add a spell
        assert spell_manager.add_known_spell(10, 1, 100) is True
        
        # Verify it was added
        mock_character_manager.gff.set.assert_called_with(
            'KnownList1', 
            [{'Spell': 100, 'SpellClass': 10}]
        )
        
        # Mock the spell as already known
        mock_character_manager.gff.get.return_value = [
            {'Spell': 100, 'SpellClass': 10}
        ]
        
        # Try to add again - should return False
        assert spell_manager.add_known_spell(10, 1, 100) is False
        
        # Remove the spell
        assert spell_manager.remove_known_spell(10, 1, 100) is True
        
        # Verify it was removed
        mock_character_manager.gff.set.assert_called_with('KnownList1', [])
        
        # Try to remove again - should return False
        mock_character_manager.gff.get.return_value = []
        assert spell_manager.remove_known_spell(10, 1, 100) is False
    
    def test_get_memorized_spells(self, mock_character_manager):
        """Test getting memorized spells"""
        spell_manager = SpellManager(mock_character_manager)
        
        # Mock memorized spells
        mock_character_manager.gff.get.side_effect = lambda key, default: {
            'MemorizedList0': [
                {'Spell': 0, 'SpellClass': 10, 'SpellMetaMagic': 0, 'Ready': True}
            ],
            'MemorizedList1': [
                {'Spell': 100, 'SpellClass': 10, 'SpellMetaMagic': 1, 'Ready': False}
            ]
        }.get(key, default)
        
        memorized = spell_manager.get_memorized_spells()
        
        assert len(memorized) == 2
        assert memorized[0]['level'] == 0
        assert memorized[0]['spell_id'] == 0
        assert memorized[0]['ready'] is True
        assert memorized[1]['level'] == 1
        assert memorized[1]['metamagic'] == 1