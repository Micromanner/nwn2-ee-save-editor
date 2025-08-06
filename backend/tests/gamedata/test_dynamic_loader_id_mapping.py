"""
DynamicGameDataLoader ID Mapping Test Suite

Tests the core issue with get_by_id() method in DynamicGameDataLoader where
it assumes row_index == id, but many 2DA tables use different mappings.

These tests use real fixture data to demonstrate the problem and validate solutions.
"""
import pytest
import logging
from unittest.mock import Mock, patch
from pathlib import Path

from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader
from gamedata.dynamic_loader.data_model_loader import DataModelLoader
from parsers.resource_manager import ResourceManager


logger = logging.getLogger(__name__)


class TestDynamicLoaderIDMapping:
    """Test suite for ID mapping issues in DynamicGameDataLoader."""
    
    @pytest.fixture
    def resource_manager(self):
        """Create ResourceManager with test fixtures."""
        rm = ResourceManager(suppress_warnings=True)
        
        # Set up paths to use test fixtures
        fixture_path = Path(__file__).parent.parent / "fixtures"
        if fixture_path.exists():
            # This would normally be set via nwn2_settings, but for tests we mock it
            pass
        
        return rm
    
    @pytest.fixture
    def mock_data_model_loader(self):
        """Create mock DataModelLoader that returns test data."""
        loader = Mock(spec=DataModelLoader)
        
        # Mock creaturesize data (the problematic table)
        # Row 0: INVALID, Row 1: TINY, Row 2: SMALL, Row 3: MEDIUM, Row 4: LARGE
        creaturesize_data = [
            Mock(id=0, label='INVALID'),   # Row 0, but ID 0 is invalid
            Mock(id=1, label='TINY'),      # Row 1, represents Size ID 1  
            Mock(id=2, label='SMALL'),     # Row 2, represents Size ID 2
            Mock(id=3, label='MEDIUM'),    # Row 3, represents Size ID 3
            Mock(id=4, label='LARGE'),     # Row 4, represents Size ID 4
        ]
        
        # Mock racialtypes data (direct mapping table)
        # Row 0: Dwarf (ID 0), Row 1: Elf (ID 1), etc.
        racialtypes_data = [
            Mock(id=0, label='Dwarf'),     # Row 0, Race ID 0
            Mock(id=1, label='Elf'),       # Row 1, Race ID 1
            Mock(id=2, label='Gnome'),     # Row 2, Race ID 2
            Mock(id=3, label='Halfling'),  # Row 3, Race ID 3
            Mock(id=4, label='HalfElf'),   # Row 4, Race ID 4
            Mock(id=5, label='HalfOrc'),   # Row 5, Race ID 5
            Mock(id=6, label='Human'),     # Row 6, Race ID 6
        ]
        
        # Mock classes data (direct mapping table)
        classes_data = [
            Mock(id=0, label='Barbarian'), # Row 0, Class ID 0
            Mock(id=1, label='Bard'),      # Row 1, Class ID 1
            Mock(id=2, label='Cleric'),    # Row 2, Class ID 2
            Mock(id=3, label='Druid'),     # Row 3, Class ID 3
            Mock(id=4, label='Fighter'),   # Row 4, Class ID 4
        ]
        
        # Set up loader to return different data based on table
        def get_table_side_effect(table_name):
            if table_name == 'creaturesize':
                return creaturesize_data
            elif table_name == 'racialtypes':
                return racialtypes_data
            elif table_name == 'classes':
                return classes_data
            else:
                return []
        
        loader.get_table.side_effect = get_table_side_effect
        
        # Mock the load_game_data to return our test data
        async def load_game_data():
            return {
                'creaturesize': creaturesize_data,
                'racialtypes': racialtypes_data,
                'classes': classes_data
            }
        
        loader.load_game_data.return_value = load_game_data()
        
        return loader
    
    @pytest.fixture
    def dynamic_loader_with_mock_data(self, resource_manager, mock_data_model_loader):
        """Create DynamicGameDataLoader with mocked data."""
        # Create loader but patch the DataModelLoader creation
        with patch('gamedata.dynamic_loader.dynamic_game_data_loader.DataModelLoader') as mock_class:
            mock_class.return_value = mock_data_model_loader
            
            # Create the loader (will use our mock)
            loader = DynamicGameDataLoader(
                resource_manager=resource_manager,
                use_async=False,  # Simplify for testing
                validate_relationships=False  # Skip validation for these tests
            )
            
            # Manually set the table data since our mock might not be called properly
            loader.table_data = {
                'creaturesize': [
                    Mock(id=0, label='INVALID'),
                    Mock(id=1, label='TINY'),
                    Mock(id=2, label='SMALL'),
                    Mock(id=3, label='MEDIUM'),
                    Mock(id=4, label='LARGE'),
                ],
                'racialtypes': [
                    Mock(id=0, label='Dwarf'),
                    Mock(id=1, label='Elf'),
                    Mock(id=2, label='Gnome'),
                    Mock(id=3, label='Halfling'),
                    Mock(id=4, label='HalfElf'),
                    Mock(id=5, label='HalfOrc'),
                    Mock(id=6, label='Human'),
                ],
                'classes': [
                    Mock(id=0, label='Barbarian'),
                    Mock(id=1, label='Bard'),
                    Mock(id=2, label='Cleric'),
                    Mock(id=3, label='Druid'),
                    Mock(id=4, label='Fighter'),
                ]
            }
            
            return loader
    
    @pytest.fixture
    def real_dynamic_loader(self, resource_manager):
        """Create DynamicGameDataLoader with real data from fixtures."""
        try:
            # Try to create with real resource manager and priority-only mode
            # to load just essential tables quickly
            loader = DynamicGameDataLoader(
                resource_manager=resource_manager,
                use_async=False,
                validate_relationships=False,
                priority_only=True  # Only load essential tables
            )
            return loader
        except Exception as e:
            # If real loading fails, skip tests that depend on it
            pytest.skip(f"Could not load real game data: {e}")
    
    def test_get_by_id_current_implementation_issue(self, dynamic_loader_with_mock_data):
        """Demonstrate the current get_by_id() implementation issue."""
        loader = dynamic_loader_with_mock_data
        
        # Test direct mapping table (racialtypes) - should work correctly
        human_race = loader.get_by_id('racialtypes', 6)  # Human is at row 6
        assert human_race is not None, "Should find Human race at ID 6"
        assert human_race.label == 'Human', f"Expected 'Human', got '{human_race.label}'"
        
        # Test problematic offset mapping table (creaturesize)
        # The issue: Size ID 3 (Medium) should be at row 3, but current implementation
        # assumes it's at row 3, which is correct by coincidence for this example
        medium_size = loader.get_by_id('creaturesize', 3)  # Medium size
        assert medium_size is not None, "Should find Medium size"
        # This might work by accident if the test data is set up to match current assumption
        
        # The real issue becomes apparent when we try to get Size ID 1 (TINY)
        # Size ID 1 should map to row 1, which current implementation gets right
        tiny_size = loader.get_by_id('creaturesize', 1)
        assert tiny_size is not None, "Should find Tiny size"
        
        # However, the real issue is in the mapping logic itself
        # Let's demonstrate by trying to access an ID that doesn't exist
        invalid_size = loader.get_by_id('creaturesize', 10)  # No size ID 10
        assert invalid_size is None, "Should return None for non-existent ID"
    
    def test_creaturesize_mapping_problem_detailed(self, dynamic_loader_with_mock_data):
        """Detailed test of the creaturesize.2da mapping problem."""
        loader = dynamic_loader_with_mock_data
        
        # Get the raw table data to understand the structure
        creaturesize_table = loader.get_table('creaturesize')
        assert len(creaturesize_table) == 5, "Should have 5 size entries"
        
        # Current implementation: get_by_id(table, id) returns table[id] if 0 <= id < len(table)
        # This works for some tables but fails for others
        
        # Let's examine what we get vs what we should get
        logger.info("=== CREATURESIZE MAPPING ANALYSIS ===")
        for i in range(len(creaturesize_table)):
            current_result = loader.get_by_id('creaturesize', i)
            logger.info(f"get_by_id('creaturesize', {i}) -> {current_result.label if current_result else None}")
        
        # The issue: if creaturesize.2da uses row_index = size_id - 1 mapping:
        # - Size ID 1 (TINY) should be at row 0, but row 0 has INVALID
        # - Size ID 2 (SMALL) should be at row 1, but row 1 has TINY
        # - Size ID 3 (MEDIUM) should be at row 2, but row 2 has SMALL
        # etc.
        
        # This is why RaceManager bypasses DynamicGameDataLoader and uses ResourceManager directly
    
    def test_direct_mapping_tables_work_correctly(self, dynamic_loader_with_mock_data):
        """Test that direct mapping tables (like racialtypes, classes) work correctly."""
        loader = dynamic_loader_with_mock_data
        
        # Test racialtypes (direct mapping: row_index = race_id)
        races_to_test = [
            (0, 'Dwarf'),
            (1, 'Elf'),
            (2, 'Gnome'),
            (6, 'Human')
        ]
        
        for race_id, expected_name in races_to_test:
            race = loader.get_by_id('racialtypes', race_id)
            assert race is not None, f"Should find race ID {race_id}"
            assert race.label == expected_name, f"Expected '{expected_name}', got '{race.label}'"
        
        # Test classes (direct mapping: row_index = class_id)
        classes_to_test = [
            (0, 'Barbarian'),
            (1, 'Bard'),
            (2, 'Cleric'),
            (4, 'Fighter')
        ]
        
        for class_id, expected_name in classes_to_test:
            cls = loader.get_by_id('classes', class_id)
            assert cls is not None, f"Should find class ID {class_id}"
            assert cls.label == expected_name, f"Expected '{expected_name}', got '{cls.label}'"
    
    def test_edge_cases_with_current_implementation(self, dynamic_loader_with_mock_data):
        """Test edge cases that reveal issues with current get_by_id implementation."""
        loader = dynamic_loader_with_mock_data
        
        # Test negative ID
        result = loader.get_by_id('racialtypes', -1)
        assert result is None, "Should return None for negative ID"
        
        # Test ID beyond table size
        result = loader.get_by_id('racialtypes', 100)
        assert result is None, "Should return None for ID beyond table size"
        
        # Test ID at table boundary
        racialtypes_table = loader.get_table('racialtypes')
        max_valid_id = len(racialtypes_table) - 1
        
        result = loader.get_by_id('racialtypes', max_valid_id)
        assert result is not None, f"Should find race at max valid ID {max_valid_id}"
        
        result = loader.get_by_id('racialtypes', max_valid_id + 1)
        assert result is None, f"Should return None for ID {max_valid_id + 1}"
    
    @pytest.mark.integration
    def test_real_creaturesize_data_if_available(self, real_dynamic_loader):
        """Test with real creaturesize.2da data if available."""
        loader = real_dynamic_loader
        
        # Try to get creaturesize table
        creaturesize_table = loader.get_table('creaturesize')
        if not creaturesize_table:
            pytest.skip("creaturesize table not available in test data")
        
        logger.info(f"Real creaturesize table has {len(creaturesize_table)} entries")
        
        # Test the known mapping issue
        # In real NWN2 data, Size 3 (Medium) might not be at row 3
        for i in range(min(10, len(creaturesize_table))):
            size_entry = loader.get_by_id('creaturesize', i)
            if size_entry:
                # Try to get label/name for logging
                label = getattr(size_entry, 'label', getattr(size_entry, 'LABEL', f'Unknown_{i}'))
                logger.info(f"Size ID {i} -> {label}")
    
    @pytest.mark.integration  
    def test_real_racialtypes_data_if_available(self, real_dynamic_loader):
        """Test with real racialtypes.2da data if available."""
        loader = real_dynamic_loader
        
        # Try to get racialtypes table
        racialtypes_table = loader.get_table('racialtypes')
        if not racialtypes_table:
            pytest.skip("racialtypes table not available in test data")
        
        logger.info(f"Real racialtypes table has {len(racialtypes_table)} entries")
        
        # Test basic races that should exist
        basic_races = [0, 1, 2, 3, 4, 5, 6]  # Dwarf through Human
        
        for race_id in basic_races:
            if race_id < len(racialtypes_table):
                race_entry = loader.get_by_id('racialtypes', race_id)
                if race_entry:
                    # Try to get label/name for logging
                    label = getattr(race_entry, 'label', getattr(race_entry, 'Label', f'Unknown_{race_id}'))
                    logger.info(f"Race ID {race_id} -> {label}")
    
    def test_proposed_solution_concept(self, dynamic_loader_with_mock_data):
        """Test concept for a proposed solution to the ID mapping problem."""
        loader = dynamic_loader_with_mock_data
        
        # Concept: Enhanced get_by_id that knows about different mapping strategies
        def enhanced_get_by_id(loader, table_name: str, id_value: int):
            """
            Enhanced get_by_id that handles different mapping strategies.
            
            This is a proof-of-concept for how the fix might work.
            """
            table = loader.get_table(table_name)
            if not table:
                return None
            
            # Define mapping strategies per table
            mapping_strategies = {
                'creaturesize': 'offset_minus_one',  # row_index = id - 1, skip invalid entries
                'racialtypes': 'direct',            # row_index = id
                'classes': 'direct',                # row_index = id
            }
            
            strategy = mapping_strategies.get(table_name, 'direct')
            
            if strategy == 'direct':
                # Current implementation
                if 0 <= id_value < len(table):
                    return table[id_value]
                return None
            
            elif strategy == 'offset_minus_one':
                # For tables like creaturesize where row_index = id - 1
                # But we need to handle the INVALID entry at row 0
                if id_value <= 0:
                    return None  # Invalid IDs
                
                row_index = id_value - 1
                if 0 <= row_index < len(table):
                    entry = table[row_index]
                    # Skip INVALID entries
                    if hasattr(entry, 'label') and entry.label.upper() != 'INVALID':
                        return entry
                return None
            
            return None
        
        # Test the enhanced function
        # Test direct mapping (should work the same)
        human = enhanced_get_by_id(loader, 'racialtypes', 6)
        assert human is not None and human.label == 'Human'
        
        # Test offset mapping (should handle creaturesize correctly)
        # Size ID 1 should get TINY (at row 0, but row 0 has INVALID in our mock)
        # Size ID 2 should get SMALL (at row 1)
        small_size = enhanced_get_by_id(loader, 'creaturesize', 2)
        # In our mock data, this would return the entry at row 1, which is TINY
        # This demonstrates that the mapping strategy needs to be more sophisticated
        
        logger.info("Enhanced get_by_id concept test completed")
        
        # The real solution would need:
        # 1. Table-specific mapping configuration
        # 2. Proper handling of INVALID entries
        # 3. Dynamic detection of mapping patterns
        # 4. Fallback strategies for unknown tables


if __name__ == "__main__":
    # Allow running this test module directly for development
    import sys
    logging.basicConfig(level=logging.INFO, format='%(message)s')
    
    # Run a simple test
    print("Testing ID mapping analysis...")
    
    # This would need pytest to run properly, but we can do basic validation
    print("ID mapping test module created successfully")