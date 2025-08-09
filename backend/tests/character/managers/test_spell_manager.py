"""
Tests for SpellManager - spell slots, spell lists, domain spells, and metamagic
"""

import pytest
from unittest.mock import Mock, MagicMock, patch
from datetime import datetime

from character.managers.spell_manager import SpellManager
from character.events import EventType, ClassChangedEvent, LevelGainedEvent, FeatChangedEvent


@pytest.fixture
def mock_character_manager():
    """Create a mock CharacterManager with necessary attributes"""
    manager = Mock()
    
    # Mock GFF data
    manager.gff = MagicMock()
    manager.gff.get.return_value = []
    manager.gff.set = MagicMock()
    
    # Mock game data loader
    manager.game_data_loader = Mock()
    
    # Mock custom content
    manager.custom_content = {}
    
    # Mock has_feat method
    manager.has_feat = Mock(return_value=False)
    
    # Add event emitter methods
    manager._observers = {}
    manager.on = Mock(side_effect=lambda event_type, handler: manager._observers.setdefault(event_type, []).append(handler))
    manager.emit = Mock()
    
    return manager


@pytest.fixture
def spell_manager(mock_character_manager):
    """Create a SpellManager instance with mock dependencies"""
    return SpellManager(mock_character_manager)


@pytest.fixture
def wizard_class_data():
    """Mock wizard class data"""
    wizard = Mock(spec=['label', 'hit_die', 'skill_point_base', 'attack_bonus_table',
                       'saving_throw_table', 'spell_caster', 'spell_gain_table', 
                       'spell_known_table', 'primary_ability', 'spell_ability'])
    wizard.label = 'Wizard'
    wizard.hit_die = 4
    wizard.skill_point_base = 2
    wizard.attack_bonus_table = 'CLS_ATK_1'
    wizard.saving_throw_table = 'CLS_SAV_WIZ'
    wizard.spell_caster = 1
    wizard.spell_gain_table = 'CLS_SPGN_WIZ'
    wizard.spell_known_table = '****'  # Wizards prepare spells
    wizard.primary_ability = '****'
    wizard.spell_ability = '****'
    return wizard


@pytest.fixture
def sorcerer_class_data():
    """Mock sorcerer class data"""
    sorcerer = Mock()
    sorcerer.label = 'Sorcerer'
    sorcerer.hit_die = 4
    sorcerer.skill_point_base = 2
    sorcerer.spell_caster = 1
    sorcerer.spell_gain_table = 'CLS_SPGN_SORC'
    sorcerer.spell_known_table = 'CLS_SPKN_SORC'  # Sorcerers know spells
    return sorcerer


@pytest.fixture
def cleric_class_data():
    """Mock cleric class data"""
    cleric = Mock(spec=['label', 'hit_die', 'skill_point_base', 'spell_caster', 
                        'spell_gain_table', 'spell_known_table', 'primary_ability', 
                        'spell_ability'])
    cleric.label = 'Cleric'
    cleric.hit_die = 8
    cleric.skill_point_base = 2
    cleric.spell_caster = 1
    cleric.spell_gain_table = 'CLS_SPGN_CLER'
    cleric.spell_known_table = '****'  # Clerics prepare spells
    cleric.primary_ability = '****'
    cleric.spell_ability = '****'
    return cleric


@pytest.fixture
def spell_gain_table():
    """Mock spell gain table (CLS_SPGN_WIZ for level 5)"""
    rows = []
    for level in range(20):
        row = Mock()
        # Wizard spell slots at level 5: 4/3/2/1/0/0/0/0/0/0
        if level == 4:  # Level 5 (0-indexed)
            row.spellslevel0 = 4
            row.spellslevel1 = 3
            row.spellslevel2 = 2
            row.spellslevel3 = 1
            row.spellslevel4 = 0
            row.spellslevel5 = 0
            row.spellslevel6 = 0
            row.spellslevel7 = 0
            row.spellslevel8 = 0
            row.spellslevel9 = 0
        else:
            for i in range(10):
                setattr(row, f'spellslevel{i}', 0)
        rows.append(row)
    return rows


@pytest.fixture
def spell_known_table():
    """Mock spell known table (CLS_SPKN_SORC for level 5)"""
    rows = []
    for level in range(20):
        row = Mock()
        # Sorcerer spells known at level 5: 6/4/2/1/0/0/0/0/0/0
        if level == 4:  # Level 5 (0-indexed)
            row.spellslevel0 = 6
            row.spellslevel1 = 4
            row.spellslevel2 = 2
            row.spellslevel3 = 1
            row.spellslevel4 = 0
            row.spellslevel5 = 0
            row.spellslevel6 = 0
            row.spellslevel7 = 0
            row.spellslevel8 = 0
            row.spellslevel9 = 0
        else:
            for i in range(10):
                setattr(row, f'spellslevel{i}', 0)
        rows.append(row)
    return rows


@pytest.fixture
def domain_data():
    """Mock domain data (Good domain)"""
    domain = Mock()
    domain.label = 'Good'
    domain.level_1 = 421  # Protection from Evil
    domain.level_2 = 422  # Aid
    domain.level_3 = 423  # Magic Circle vs Evil
    domain.level_4 = 424  # Holy Smite
    domain.level_5 = 425  # Dispel Evil
    domain.level_6 = 426  # Blade Barrier
    domain.level_7 = 427  # Holy Word
    domain.level_8 = 428  # Holy Aura
    domain.level_9 = 429  # Summon Celestial
    return domain


class TestSpellManagerBasics:
    """Test basic SpellManager functionality"""
    
    def test_initialization(self, spell_manager, mock_character_manager):
        """Test SpellManager initializes correctly"""
        assert spell_manager.character_manager == mock_character_manager
        assert spell_manager.game_data_loader == mock_character_manager.game_data_loader
        assert spell_manager.gff == mock_character_manager.gff
        
        # Check event handlers were registered
        assert mock_character_manager.on.call_count >= 4  # CLASS_CHANGED, LEVEL_GAINED, FEAT_ADDED, FEAT_REMOVED
    
    def test_is_spellcaster_true(self, spell_manager, wizard_class_data):
        """Test _is_spellcaster returns True for casters"""
        assert spell_manager._is_spellcaster(wizard_class_data) is True
    
    def test_is_spellcaster_false(self, spell_manager):
        """Test _is_spellcaster returns False for non-casters"""
        fighter = Mock(spec=['spell_caster', 'spell_gain_table'])
        fighter.spell_caster = 0
        fighter.spell_gain_table = '****'
        assert spell_manager._is_spellcaster(fighter) is False
    
    def test_get_casting_ability(self, spell_manager):
        """Test _get_casting_ability returns correct ability"""
        # Test with class data
        wizard = Mock(spec=['label', 'primary_ability', 'spell_ability'])
        wizard.label = 'Wizard'
        wizard.primary_ability = '****'
        wizard.spell_ability = '****'
        assert spell_manager._get_casting_ability(wizard) == 'Int'
        
        sorcerer = Mock(spec=['label', 'primary_ability', 'spell_ability'])
        sorcerer.label = 'Sorcerer'
        sorcerer.primary_ability = '****'
        sorcerer.spell_ability = '****'
        assert spell_manager._get_casting_ability(sorcerer) == 'Cha'
        
        cleric = Mock(spec=['label', 'primary_ability', 'spell_ability'])
        cleric.label = 'Cleric'
        cleric.primary_ability = '****'
        cleric.spell_ability = '****'
        assert spell_manager._get_casting_ability(cleric) == 'Wis'
    
    def test_is_divine_caster(self, spell_manager):
        """Test _is_divine_caster detection"""
        cleric = Mock()
        cleric.label = 'Cleric'
        assert spell_manager._is_divine_caster(cleric) is True
        
        wizard = Mock()
        wizard.label = 'Wizard'
        assert spell_manager._is_divine_caster(wizard) is False
    
    def test_is_prepared_caster(self, spell_manager):
        """Test _is_prepared_caster detection"""
        wizard = Mock()
        wizard.label = 'Wizard'
        assert spell_manager._is_prepared_caster(wizard) is True
        
        sorcerer = Mock()
        sorcerer.label = 'Sorcerer'
        assert spell_manager._is_prepared_caster(sorcerer) is False


class TestSpellSlotCalculation:
    """Test spell slot calculation"""
    
    def test_calculate_spell_slots_wizard(self, spell_manager, mock_character_manager, 
                                         wizard_class_data, spell_gain_table):
        """Test spell slot calculation for a level 5 wizard"""
        # Setup character data
        mock_character_manager.gff.get.side_effect = lambda key, default=None: {
            'ClassList': [{'Class': 10, 'ClassLevel': 5}],
            'Int': 18,  # +4 modifier
            'Wis': 10,
            'Cha': 10
        }.get(key, default)
        
        # Setup game data
        mock_character_manager.game_data_loader.get_by_id.return_value = wizard_class_data
        mock_character_manager.game_data_loader.get_table.return_value = spell_gain_table
        
        # Calculate slots
        slots = spell_manager.calculate_spell_slots()
        
        assert 10 in slots  # Class ID 10
        wizard_slots = slots[10]
        
        # Base slots: 4/3/2/1
        # Bonus slots from 18 INT (+4 modifier):
        # Level 1: 1 + (4-1)//4 = 1 bonus slot
        # Level 2: 1 + (4-2)//4 = 1 bonus slot  
        # Level 3: 1 + (4-3)//4 = 1 bonus slot
        # Level 4: can cast (modifier >= spell level) so 1 + (4-4)//4 = 1 bonus slot
        assert wizard_slots[0] == 4  # No bonus for cantrips
        assert wizard_slots[1] == 4  # 3 + 1 bonus
        assert wizard_slots[2] == 3  # 2 + 1 bonus
        assert wizard_slots[3] == 2  # 1 + 1 bonus
    
    def test_calculate_spell_slots_cleric_with_domains(self, spell_manager, mock_character_manager,
                                                      cleric_class_data, spell_gain_table):
        """Test spell slot calculation for cleric with domains"""
        # Setup character data with domains
        mock_character_manager.gff.get.side_effect = lambda key, default=None: {
            'ClassList': [{'Class': 2, 'ClassLevel': 5, 'Domain1': 5, 'Domain2': 10}],
            'Wis': 16,  # +3 modifier
            'Int': 10,
            'Cha': 10
        }.get(key, default)
        
        # Setup game data
        mock_character_manager.game_data_loader.get_by_id.return_value = cleric_class_data
        mock_character_manager.game_data_loader.get_table.return_value = spell_gain_table
        
        # Calculate slots
        slots = spell_manager.calculate_spell_slots()
        
        cleric_slots = slots[2]
        
        # Clerics get +1 domain slot per spell level (1-9)
        # WIS 16 = +3 modifier
        # Level 1: 3 base + 1 bonus (modifier >= spell level) + 1 domain = 5
        # Level 2: 2 base + 1 bonus + 1 domain = 4
        # Level 3: 1 base + 1 bonus + 1 domain = 3
        assert cleric_slots[0] == 4  # No domain slot for cantrips
        assert cleric_slots[1] == 5  # 3 base + 1 bonus + 1 domain
        assert cleric_slots[2] == 4  # 2 base + 1 bonus + 1 domain
        assert cleric_slots[3] == 3  # 1 base + 1 bonus + 1 domain
    
    def test_calculate_bonus_spell_slots(self, spell_manager, mock_character_manager, wizard_class_data):
        """Test bonus spell slot calculation from ability scores"""
        mock_character_manager.gff.get.return_value = 20  # +5 modifier
        
        # Spell level 1: (5 - 1 + 1) / 4 + 1 = 2 bonus slots
        assert spell_manager._calculate_bonus_spell_slots(wizard_class_data, 1) == 2
        
        # Spell level 3: (5 - 3 + 1) / 4 + 1 = 1 bonus slot
        assert spell_manager._calculate_bonus_spell_slots(wizard_class_data, 3) == 1
        
        # Spell level 5: modifier equals spell level, still get 1 bonus
        assert spell_manager._calculate_bonus_spell_slots(wizard_class_data, 5) == 1
        
        # Spell level 6: modifier < spell level, no bonus
        assert spell_manager._calculate_bonus_spell_slots(wizard_class_data, 6) == 0


class TestSpellListManagement:
    """Test spell list management"""
    
    def test_get_known_spells(self, spell_manager, mock_character_manager):
        """Test getting known spells for a class"""
        # Setup known spell lists
        mock_character_manager.gff.get.side_effect = lambda key, default=None: {
            'KnownList0': [
                {'Spell': 0, 'SpellClass': 10},  # Acid Splash
                {'Spell': 3, 'SpellClass': 10},  # Daze
                {'Spell': 9, 'SpellClass': 10},  # Light
            ],
            'KnownList1': [
                {'Spell': 10, 'SpellClass': 10},  # Magic Missile
                {'Spell': 14, 'SpellClass': 10},  # Sleep
                {'Spell': 16, 'SpellClass': 11},  # Different class
            ],
            'KnownList2': []
        }.get(key, default)
        
        known = spell_manager.get_known_spells(10)  # Class 10
        
        assert 0 in known
        assert len(known[0]) == 3
        assert 0 in known[0]
        assert 3 in known[0]
        
        assert 1 in known
        assert len(known[1]) == 2  # Excludes spell from class 11
        assert 10 in known[1]
        
        assert 2 not in known  # Empty list
    
    def test_get_memorized_spells(self, spell_manager, mock_character_manager):
        """Test getting memorized/prepared spells"""
        mock_character_manager.gff.get.side_effect = lambda key, default=None: {
            'MemorizedList0': [
                {'Spell': 0, 'Ready': 1, 'SpellMetaMagicN2': 0, 'SpellClass': 10},
                {'Spell': 0, 'Ready': 0, 'SpellMetaMagicN2': 0, 'SpellClass': 10},  # Used
            ],
            'MemorizedList1': [
                {'Spell': 10, 'Ready': 1, 'SpellMetaMagicN2': 1, 'SpellClass': 10},  # Empowered
            ]
        }.get(key, default)
        
        memorized = spell_manager.get_memorized_spells(10)
        
        assert len(memorized[0]) == 2
        assert memorized[0][0]['spell_id'] == 0
        assert memorized[0][0]['ready'] == 1
        assert memorized[0][1]['ready'] == 0
        
        assert len(memorized[1]) == 1
        assert memorized[1][0]['metamagic'] == 1
    
    def test_add_known_spell(self, spell_manager, mock_character_manager):
        """Test adding a spell to known list"""
        mock_character_manager.gff.get.return_value = []
        
        # Add spell
        result = spell_manager.add_known_spell(10, 1, 10)  # Magic Missile
        
        assert result is True
        mock_character_manager.gff.set.assert_called_with(
            'KnownList1',
            [{'Spell': 10, 'SpellClass': 10}]
        )
    
    def test_add_known_spell_already_known(self, spell_manager, mock_character_manager):
        """Test adding already known spell returns False"""
        mock_character_manager.gff.get.return_value = [
            {'Spell': 10, 'SpellClass': 10}
        ]
        
        result = spell_manager.add_known_spell(10, 1, 10)
        
        assert result is False
        mock_character_manager.gff.set.assert_not_called()
    
    def test_prepare_spell(self, spell_manager, mock_character_manager):
        """Test preparing a spell"""
        # Setup spell slots
        mock_character_manager.gff.get.side_effect = lambda key, default=None: {
            'ClassList': [{'Class': 10, 'ClassLevel': 5}],
            'Int': 18,
            'MemorizedList1': []
        }.get(key, default)
        
        # Mock calculate_spell_slots to return available slots
        spell_manager.calculate_spell_slots = Mock(return_value={10: {1: 4}})
        
        # Prepare spell
        result = spell_manager.prepare_spell(10, 1, 10)  # Magic Missile
        
        assert result is True
        expected_spell = {
            'Spell': 10,
            'Ready': 1,
            'SpellMetaMagicN2': 0,
            'SpellClass': 10,
            'SpellDomain': 0
        }
        mock_character_manager.gff.set.assert_called_with('MemorizedList1', [expected_spell])
    
    def test_prepare_spell_no_slots(self, spell_manager, mock_character_manager):
        """Test preparing spell with no available slots"""
        # Setup with full memorized list
        full_list = [{'Spell': i, 'SpellClass': 10} for i in range(4)]
        mock_character_manager.gff.get.return_value = full_list
        
        spell_manager.calculate_spell_slots = Mock(return_value={10: {1: 4}})
        spell_manager.get_memorized_spells = Mock(return_value={1: full_list})
        
        result = spell_manager.prepare_spell(10, 1, 15)
        
        assert result is False
    
    def test_clear_memorized_spells(self, spell_manager, mock_character_manager):
        """Test clearing memorized spells"""
        # Setup mixed class spells
        mock_character_manager.gff.get.return_value = [
            {'Spell': 10, 'SpellClass': 10},
            {'Spell': 11, 'SpellClass': 11},
            {'Spell': 12, 'SpellClass': 10}
        ]
        
        # Clear specific level
        spell_manager.clear_memorized_spells(10, 1)
        
        # Should only keep spells from other classes
        mock_character_manager.gff.set.assert_called_with(
            'MemorizedList1',
            [{'Spell': 11, 'SpellClass': 11}]
        )


class TestDomainSpells:
    """Test domain spell functionality"""
    
    def test_get_domain_spells(self, spell_manager, mock_character_manager, domain_data):
        """Test getting domain spells for a cleric"""
        # Setup cleric with domains
        mock_character_manager.gff.get.return_value = [
            {'Class': 2, 'ClassLevel': 9, 'Domain1': 5, 'Domain2': 10}
        ]
        
        # Mock domain data
        mock_character_manager.game_data_loader.get_by_id.side_effect = lambda table, id: {
            ('domains', 5): domain_data,
            ('domains', 10): None  # Second domain not found
        }.get((table, id))
        
        # Add domain spell attributes to mock
        for i in range(1, 10):
            setattr(domain_data, f'level_{i}', 420 + i)
            
        domain_spells = spell_manager.get_domain_spells(2)
        
        assert 1 in domain_spells
        assert 421 in domain_spells[1]
        assert 5 in domain_spells
        assert 425 in domain_spells[5]


class TestMetamagic:
    """Test metamagic functionality"""
    
    def test_get_metamagic_feats(self, spell_manager, mock_character_manager):
        """Test getting character's metamagic feats"""
        # Mock feat manager to return True for some metamagic
        mock_feat_manager = Mock()
        mock_feat_manager.has_feat.side_effect = lambda feat_id: feat_id in [11, 12, 25]
        mock_character_manager.get_manager.return_value = mock_feat_manager
        
        metamagic = spell_manager.get_metamagic_feats()
        
        assert 11 in metamagic  # Empower
        assert 12 in metamagic  # Extend
        assert 25 in metamagic  # Maximize
        assert 26 not in metamagic  # Quicken
    
    def test_calculate_metamagic_cost(self, spell_manager):
        """Test calculating spell level adjustment for metamagic"""
        # Empower only
        assert spell_manager.calculate_metamagic_cost(0x01) == 2
        
        # Empower + Maximize
        assert spell_manager.calculate_metamagic_cost(0x05) == 5  # 2 + 3
        
        # Silent + Still
        assert spell_manager.calculate_metamagic_cost(0x30) == 2  # 1 + 1
    
    def test_prepare_spell_with_metamagic(self, spell_manager, mock_character_manager):
        """Test preparing a spell with metamagic"""
        mock_character_manager.gff.get.return_value = []
        spell_manager.calculate_spell_slots = Mock(return_value={10: {3: 1}})  # Slot at level 3
        
        # Prepare empowered level 1 spell (requires level 3 slot)
        result = spell_manager.prepare_spell(10, 3, 10, metamagic=0x01)
        
        assert result is True
        call_args = mock_character_manager.gff.set.call_args[0]
        assert call_args[0] == 'MemorizedList3'
        assert call_args[1][0]['SpellMetaMagicN2'] == 0x01


class TestEventHandling:
    """Test event handling"""
    
    def test_on_class_changed_to_caster(self, spell_manager, mock_character_manager, wizard_class_data):
        """Test handling class change to a spellcaster"""
        # Mock as spellcaster
        mock_character_manager.game_data_loader.get_by_id.return_value = wizard_class_data
        
        # Create event
        event = ClassChangedEvent(
            event_type=EventType.CLASS_CHANGED,
            source_manager='class',
            timestamp=datetime.now().timestamp(),
            old_class_id=1,
            new_class_id=10,
            level=5,
            preserve_feats=[]
        )
        
        # Spy on methods
        spell_manager._update_spell_slots = Mock()
        spell_manager._update_known_spells = Mock()
        spell_manager.emit = Mock()
        
        # Handle event
        spell_manager.on_class_changed(event)
        
        spell_manager._update_spell_slots.assert_called_once()
        spell_manager._update_known_spells.assert_called_with(10, 5)
        spell_manager.emit.assert_called_once()
    
    def test_on_class_changed_to_non_caster(self, spell_manager, mock_character_manager):
        """Test handling class change to non-spellcaster"""
        # Mock as non-spellcaster
        fighter = Mock(spec=['spell_caster', 'spell_gain_table', 'spell_known_table'])
        fighter.spell_caster = 0
        fighter.spell_gain_table = '****'
        fighter.spell_known_table = '****'
        mock_character_manager.game_data_loader.get_by_id.return_value = fighter
        
        event = ClassChangedEvent(
            event_type=EventType.CLASS_CHANGED,
            source_manager='class',
            timestamp=datetime.now().timestamp(),
            old_class_id=10,
            new_class_id=1,
            level=5,
            preserve_feats=[]
        )
        
        spell_manager._clear_all_spell_lists = Mock()
        
        spell_manager.on_class_changed(event)
        
        spell_manager._clear_all_spell_lists.assert_called_once()
    
    def test_on_level_gained(self, spell_manager, mock_character_manager, wizard_class_data):
        """Test handling level gain for spellcaster"""
        mock_character_manager.game_data_loader.get_by_id.return_value = wizard_class_data
        
        event = LevelGainedEvent(
            event_type=EventType.LEVEL_GAINED,
            source_manager='class',
            timestamp=datetime.now().timestamp(),
            class_id=10,
            new_level=6,
            total_level=6
        )
        
        spell_manager._update_spell_slots = Mock()
        spell_manager._handle_level_up_spells = Mock()
        spell_manager.emit = Mock()
        
        spell_manager.on_level_gained(event)
        
        spell_manager._update_spell_slots.assert_called_once()
        spell_manager._handle_level_up_spells.assert_called_with(10, 6)
        spell_manager.emit.assert_called_once()
    
    def test_on_feat_added_metamagic(self, spell_manager, mock_character_manager):
        """Test handling metamagic feat addition"""
        event = FeatChangedEvent(
            event_type=EventType.FEAT_ADDED,
            source_manager='feat',
            timestamp=datetime.now().timestamp(),
            feat_id=11,  # Empower Spell
            action='added',
            source='manual'
        )
        
        spell_manager.emit = Mock()
        
        spell_manager.on_feat_added(event)
        
        spell_manager.emit.assert_called_with(EventType.SPELLS_CHANGED, {
            'source': 'metamagic_added',
            'feat_id': 11
        })


class TestSpellResistance:
    """Test spell resistance calculation"""
    
    def test_get_spell_resistance(self, spell_manager, mock_character_manager):
        """Test calculating total spell resistance"""
        mock_character_manager.gff.get.return_value = 10  # Base SR
        
        sr = spell_manager.get_spell_resistance()
        
        assert sr == 10  # Base only for now
        # TODO: Add feat and item SR when those systems are implemented


class TestValidation:
    """Test spell validation"""
    
    def test_validate_valid_configuration(self, spell_manager, mock_character_manager, wizard_class_data):
        """Test validation passes for valid spell configuration"""
        # Setup valid configuration
        mock_character_manager.gff.get.side_effect = lambda key, default=None: {
            'ClassList': [{'Class': 10, 'ClassLevel': 5}]
        }.get(key, default)
        
        mock_character_manager.game_data_loader.get_by_id.return_value = wizard_class_data
        
        # Mock methods to return valid data
        spell_manager.calculate_spell_slots = Mock(return_value={10: {1: 4}})
        spell_manager.get_memorized_spells = Mock(return_value={1: [{'spell_id': 10}]})
        
        is_valid, errors = spell_manager.validate()
        
        assert is_valid is True
        assert len(errors) == 0
    
    def test_validate_too_many_memorized(self, spell_manager, mock_character_manager, wizard_class_data):
        """Test validation fails when too many spells memorized"""
        mock_character_manager.gff.get.return_value = [{'Class': 10, 'ClassLevel': 5}]
        mock_character_manager.game_data_loader.get_by_id.return_value = wizard_class_data
        
        # 2 slots but 3 spells memorized
        spell_manager.calculate_spell_slots = Mock(return_value={10: {1: 2}})
        spell_manager.get_memorized_spells = Mock(return_value={
            1: [{'spell_id': 10}, {'spell_id': 11}, {'spell_id': 12}]
        })
        
        is_valid, errors = spell_manager.validate()
        
        assert is_valid is False
        assert len(errors) == 1
        assert "3 spells memorized" in errors[0]
        assert "only 2 slots" in errors[0]


class TestSpellSummary:
    """Test spell summary generation"""
    
    def test_get_spell_summary(self, spell_manager, mock_character_manager, wizard_class_data):
        """Test generating spell summary"""
        mock_character_manager.gff.get.side_effect = lambda key, default=None: {
            'SpellResistance': 5,
            'ClassList': [{'Class': 10, 'ClassLevel': 5}]
        }.get(key, default)
        
        mock_character_manager.game_data_loader.get_by_id.return_value = wizard_class_data
        
        # Mock feat manager
        mock_feat_manager = Mock()
        mock_feat_manager.has_feat.side_effect = lambda feat_id: feat_id == 11  # Has Empower
        mock_character_manager.get_manager.return_value = mock_feat_manager
        
        # Mock spell slots
        spell_manager.calculate_spell_slots = Mock(return_value={
            10: {0: 4, 1: 3, 2: 2}
        })
        
        summary = spell_manager.get_spell_summary()
        
        assert summary['spell_resistance'] == 5
        assert len(summary['caster_classes']) == 1
        assert summary['caster_classes'][0]['id'] == 10
        assert summary['caster_classes'][0]['name'] == 'Wizard'
        assert summary['caster_classes'][0]['total_slots'] == 9  # 4+3+2
        assert summary['caster_classes'][0]['max_spell_level'] == 2
        assert summary['total_spell_levels'] == 9
        
        # Check metamagic
        assert len(summary['metamagic_feats']) == 1
        assert summary['metamagic_feats'][0]['id'] == 11
        assert summary['metamagic_feats'][0]['level_cost'] == 2