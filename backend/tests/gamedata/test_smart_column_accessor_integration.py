"""
Integration tests for RuleDetector with ResourceManager.
"""
import pytest
from unittest.mock import Mock, patch
from parsers.resource_manager import ResourceManager
from nwn2_rust import TDAParser
from gamedata.rule_detector import RuleDetector


class TestRuleDetectorIntegration:
    """Test RuleDetector integration with ResourceManager."""
    
    @pytest.fixture
    def mock_resource_manager(self):
        """Create a mock ResourceManager with test data."""
        rm = Mock(spec=ResourceManager)
        
        # Mock spells.2da
        spells_parser = Mock(spec=TDAParser)
        spells_parser.columns = ['ID', 'Label', 'Name', 'IconResRef', 'School']
        spells_parser.get_rows_as_dicts.return_value = [
            {'ID': '0', 'Label': 'SPELL_MAGIC_MISSILE', 'Name': '1234', 'IconResRef': 'spell_mm', 'School': 'V'},
            {'ID': '1', 'Label': 'SPELL_FIREBALL', 'Name': '5678', 'IconResRef': 'spell_fb', 'School': 'V'},
        ]
        
        # Mock classes.2da
        classes_parser = Mock(spec=TDAParser)
        classes_parser.columns = ['ID', 'Label', 'Name', 'HitDie']
        classes_parser.get_rows_as_dicts.return_value = [
            {'ID': '0', 'Label': 'CLASS_TYPE_FIGHTER', 'Name': '111', 'HitDie': '10'},
            {'ID': '1', 'Label': 'CLASS_TYPE_WIZARD', 'Name': '222', 'HitDie': '4'},
        ]
        
        # Mock iprp_spells.2da (for spell property mappings)
        iprp_parser = Mock(spec=TDAParser)
        iprp_parser.columns = ['ID', 'SpellIndex', 'Name']
        iprp_parser.get_rows_as_dicts.return_value = [
            {'ID': '0', 'SpellIndex': '0', 'Name': '1234'},  # Maps to SPELL_MAGIC_MISSILE
            {'ID': '1', 'SpellIndex': '1', 'Name': '5678'},  # Maps to SPELL_FIREBALL
        ]
        
        # Setup get_2da_with_overrides to return appropriate parsers
        def get_2da_side_effect(name):
            if name == 'spells':
                return spells_parser
            elif name == 'classes':
                return classes_parser
            elif name == 'iprp_spells':
                return iprp_parser
            return None
        
        rm.get_2da_with_overrides.side_effect = get_2da_side_effect
        
        # Mock TLK strings
        rm.get_string.side_effect = lambda ref: f"String_{ref}"
        
        return rm
    
    def test_spell_icon_resolution(self, mock_resource_manager):
        """Test that RuleDetector correctly resolves spell icons."""
        accessor = RuleDetector(mock_resource_manager)
        
        # Test direct spell ID to icon
        icon = accessor.get_spell_icon(0)
        assert icon == 'spell_mm'
        
        icon = accessor.get_spell_icon(1)
        assert icon == 'spell_fb'
        
        # Verify it called ResourceManager correctly
        mock_resource_manager.get_2da_with_overrides.assert_any_call('spells')
    
    def test_spell_name_resolution(self, mock_resource_manager):
        """Test spell name resolution through TLK."""
        accessor = RuleDetector(mock_resource_manager)
        
        # Get spell name (should resolve TLK reference)
        name = accessor.get_spell_name(0)
        assert name == 'String_1234'
        
        name = accessor.get_spell_name(1)
        assert name == 'String_5678'
        
        # Verify TLK was called
        mock_resource_manager.get_string.assert_any_call(1234)
        mock_resource_manager.get_string.assert_any_call(5678)
    
    def test_class_resolution(self, mock_resource_manager):
        """Test class data resolution."""
        accessor = RuleDetector(mock_resource_manager)
        
        # Get class name
        name = accessor.get_class_name(0)
        assert name == 'String_111'
        
        name = accessor.get_class_name(1)
        assert name == 'String_222'
        
        # Verify ResourceManager was called
        mock_resource_manager.get_2da_with_overrides.assert_any_call('classes')
    
    def test_property_spell_mapping(self, mock_resource_manager):
        """Test item property spell index mapping."""
        accessor = RuleDetector(mock_resource_manager)
        
        # Test mapping from property spell index to actual spell
        spell_id = accessor.get_property_spell_id(0)
        assert spell_id == 0  # Should map to SPELL_MAGIC_MISSILE
        
        spell_id = accessor.get_property_spell_id(1)
        assert spell_id == 1  # Should map to SPELL_FIREBALL
        
        # Verify it loaded iprp_spells
        mock_resource_manager.get_2da_with_overrides.assert_any_call('iprp_spells')
    
    def test_caching_behavior(self, mock_resource_manager):
        """Test that RuleDetector caches data appropriately."""
        accessor = RuleDetector(mock_resource_manager)
        
        # First call should load from ResourceManager
        icon1 = accessor.get_spell_icon(0)
        assert mock_resource_manager.get_2da_with_overrides.call_count == 1
        
        # Second call should use cache
        icon2 = accessor.get_spell_icon(0)
        assert icon1 == icon2
        # Call count should still be 1 (not increased)
        assert mock_resource_manager.get_2da_with_overrides.call_count == 1
        
        # Different spell should still use cache
        icon3 = accessor.get_spell_icon(1)
        assert mock_resource_manager.get_2da_with_overrides.call_count == 1
    
    def test_module_override_support(self, mock_resource_manager):
        """Test that accessor respects module overrides."""
        accessor = RuleDetector(mock_resource_manager)
        
        # Change the mock to return different data (simulating module override)
        override_parser = Mock(spec=TDAParser)
        override_parser.columns = ['ID', 'Label', 'Name', 'IconResRef', 'School']
        override_parser.get_rows_as_dicts.return_value = [
            {'ID': '0', 'Label': 'SPELL_MAGIC_MISSILE', 'Name': '9999', 'IconResRef': 'mod_spell_mm', 'School': 'V'},
        ]
        
        # Clear any cache and update mock
        accessor._spell_cache = None
        mock_resource_manager.get_2da_with_overrides.side_effect = lambda name: override_parser if name == 'spells' else None
        
        # Should get module-specific icon
        icon = accessor.get_spell_icon(0)
        assert icon == 'mod_spell_mm'
    
    def test_missing_data_handling(self, mock_resource_manager):
        """Test handling of missing data."""
        accessor = RuleDetector(mock_resource_manager)
        
        # Test non-existent spell
        icon = accessor.get_spell_icon(999)
        assert icon is None
        
        name = accessor.get_spell_name(999)
        assert name == 'Unknown Spell'
        
        # Test when 2DA is missing
        mock_resource_manager.get_2da_with_overrides.side_effect = lambda name: None
        accessor._spell_cache = None  # Clear cache
        
        icon = accessor.get_spell_icon(0)
        assert icon is None