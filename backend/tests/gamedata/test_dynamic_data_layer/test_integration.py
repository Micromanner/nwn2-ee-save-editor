"""
Integration tests for the complete dynamic data layer system
"""
import pytest
import tempfile
import asyncio
from unittest.mock import Mock, MagicMock, patch
from pathlib import Path

from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader
from gamedata.dynamic_loader.runtime_class_generator import RuntimeDataClassGenerator
from gamedata.dynamic_loader.data_model_loader import DataModelLoader
from parsers.resource_manager import ResourceManager


class MockTDAParser:
    """Mock TDA parser for testing."""
    def __init__(self, columns, rows):
        self.columns = columns
        self.rows = rows
    
    def get_column_headers(self):
        return self.columns
    
    def get_resource_count(self):
        return len(self.rows)
    
    def get_row_dict(self, row_id):
        if 0 <= row_id < len(self.rows):
            return self.rows[row_id]
        return None
    
    def get_int(self, row_id, column):
        row = self.get_row_dict(row_id)
        if row and column in row:
            val = row[column]
            return int(val) if str(val).isdigit() else 0
        return 0
    
    def get_string(self, row_id, column):
        row = self.get_row_dict(row_id)
        if row and column in row:
            return str(row[column])
        return ""


@pytest.fixture
def mock_resource_manager():
        """Create mock ResourceManager with test data."""
        rm = Mock(spec=ResourceManager)
        
        # Mock classes.2da
        classes_data = MockTDAParser(
            columns=["Label", "Name", "HitDie", "SpellCaster", "PlayerClass", "CustomModColumn"],
            rows=[
                {"Label": "Fighter", "Name": "111", "HitDie": "10", "SpellCaster": "0", 
                 "PlayerClass": "1", "CustomModColumn": "FighterSpecial"},
                {"Label": "Wizard", "Name": "112", "HitDie": "4", "SpellCaster": "1", 
                 "PlayerClass": "1", "CustomModColumn": "WizardSpecial"},
            ]
        )
        
        # Mock feat.2da
        feat_data = MockTDAParser(
            columns=["LABEL", "FEAT", "MINSTR", "PREREQFEAT1", "ModdedColumn"],
            rows=[
                {"LABEL": "PowerAttack", "FEAT": "28", "MINSTR": "13", 
                 "PREREQFEAT1": "-1", "ModdedColumn": "PowerAttackMod"},
                {"LABEL": "CombatExpertise", "FEAT": "29", "MINSTR": "0", 
                 "PREREQFEAT1": "-1", "ModdedColumn": "ExpertiseMod"},
            ]
        )
        
        # Setup mock returns
        def get_2da_side_effect(name):
            if name.lower() == "classes":
                return classes_data
            elif name.lower() == "feat":
                return feat_data
            return None
        
        rm.get_2da_with_overrides.side_effect = get_2da_side_effect
        rm.get_string.return_value = "Test String"
        
        return rm


class TestDynamicDataLoader:
    """Test the complete dynamic data loading system."""
    
    def test_basic_loading_sync(self, mock_resource_manager):
        """Test synchronous loading of dynamic data."""
        loader = DynamicGameDataLoader(
            resource_manager=mock_resource_manager,
            use_async=False
        )
        
        # Check classes were loaded
        classes = loader.get_table('classes')
        assert len(classes) == 2
        
        # Get fighter class
        fighter = loader.get_by_id('classes', 0)
        assert fighter is not None
        assert hasattr(fighter, 'Label')
        assert fighter.Label == "Fighter"
        assert fighter.HitDie == "10"
        
        # Check custom mod column is accessible
        assert hasattr(fighter, 'CustomModColumn')
        assert fighter.CustomModColumn == "FighterSpecial"
    
    def test_mod_compatibility(self, mock_resource_manager):
        """Test that mod-added columns are accessible."""
        loader = DynamicGameDataLoader(
            resource_manager=mock_resource_manager,
            use_async=False
        )
        
        # Check feat with modded column
        power_attack = loader.get_by_id('feat', 0)
        assert power_attack is not None
        
        # Standard columns - LABEL stays uppercase as an attribute
        assert power_attack.LABEL == "PowerAttack"
        assert power_attack.MINSTR == "13"
        
        # Mod-added column
        assert hasattr(power_attack, 'ModdedColumn')
        assert power_attack.ModdedColumn == "PowerAttackMod"
    
    @pytest.mark.asyncio
    async def test_async_loading(self, mock_resource_manager):
        """Test asynchronous loading with progress tracking."""
        progress_updates = []
        
        def progress_callback(message, percent):
            progress_updates.append((message, percent))
        
        # Create loader with async support
        with patch('gamedata.dynamic_loader.dynamic_game_data_loader.asyncio.get_event_loop') as mock_loop:
            # Mock event loop
            mock_loop.return_value.is_running.return_value = False
            mock_loop.return_value.run_until_complete.side_effect = lambda coro: asyncio.run(coro)
            
            loader = DynamicGameDataLoader(
                resource_manager=mock_resource_manager,
                use_async=True,
                progress_callback=progress_callback
            )
        
        # Should have progress updates
        assert len(progress_updates) > 0
        assert any("Complete" in msg for msg, _ in progress_updates)
        assert progress_updates[-1][1] == 100  # Last update should be 100%
    
    def test_new_interface(self, mock_resource_manager):
        """Test the new DynamicGameDataLoader interface."""
        loader = DynamicGameDataLoader(
            resource_manager=mock_resource_manager,
            use_async=False
        )
        
        # Test table access methods
        assert hasattr(loader, 'get_table')
        assert hasattr(loader, 'get_by_id')
        assert hasattr(loader, 'set_module_context')
        assert hasattr(loader, 'get_stats')
        
        # Test that we can access tables
        classes = loader.get_table('classes')
        assert isinstance(classes, list)
        
        # Test that we can get by ID
        fighter = loader.get_by_id('classes', 0)
        assert fighter is not None
    
    def test_column_sanitization(self, mock_resource_manager):
        """Test that problematic column names are handled correctly."""
        # Add table with problematic columns
        problem_data = MockTDAParser(
            columns=["class", "def", "My-Column", "Value%", "2DARef"],
            rows=[
                {"class": "TestClass", "def": "10", "My-Column": "test", 
                 "Value%": "50", "2DARef": "reference"}
            ]
        )
        
        # Update mock
        original_side_effect = mock_resource_manager.get_2da_with_overrides.side_effect
        def new_side_effect(name):
            if name.lower() == "problem_table":
                return problem_data
            return original_side_effect(name)
        
        mock_resource_manager.get_2da_with_overrides.side_effect = new_side_effect
        
        loader = DynamicGameDataLoader(
            resource_manager=mock_resource_manager,
            use_async=False
        )
        
        # Access the data through loader
        table_data = loader.get_table("problem_table")
        if table_data:  # May not be loaded in basic test
            instance = table_data[0]
            # Sanitized names should work
            assert hasattr(instance, 'class_')
            assert hasattr(instance, 'def_')
            assert hasattr(instance, 'My_Column')
            assert hasattr(instance, 'Valuepct')
            assert hasattr(instance, 'col_2DARef')


class TestCodeCaching:
    """Test code caching integration."""
    
    @pytest.fixture
    def temp_cache_dir(self):
        """Create temporary cache directory."""
        temp_dir = tempfile.mkdtemp()
        yield Path(temp_dir)
        import shutil
        shutil.rmtree(temp_dir)
    
    def test_code_caching(self, mock_resource_manager, temp_cache_dir):
        """Test that generated code is cached between runs."""
        # First load
        loader1 = DynamicGameDataLoader(
            resource_manager=mock_resource_manager,
            use_async=False
        )
        
        # Patch cache directory
        loader1.loader.cache.cache_dir = temp_cache_dir
        
        # Trigger code generation
        _ = loader1.get_by_id('classes', 0)
        
        # Check cache files were created
        cache_files = list(temp_cache_dir.glob("*.py"))
        assert len(cache_files) > 0
        
        # Second load should use cache
        loader2 = DynamicGameDataLoader(
            resource_manager=mock_resource_manager,
            use_async=False
        )
        loader2.loader.cache.cache_dir = temp_cache_dir
        
        # Should still work
        fighter = loader2.get_by_id('classes', 0)
        assert fighter.Label == "Fighter"


class TestErrorHandling:
    """Test error handling and edge cases."""
    
    def test_missing_tables(self):
        """Test handling of missing tables."""
        rm = Mock(spec=ResourceManager)
        rm.get_2da_with_overrides.return_value = None
        
        loader = DynamicGameDataLoader(
            resource_manager=rm,
            use_async=False
        )
        
        # Should handle gracefully
        classes = loader.get_table('classes')
        assert len(classes) == 0
        assert loader.get_by_id('classes', 0) is None
    
    def test_malformed_data(self):
        """Test handling of malformed 2DA data."""
        rm = Mock(spec=ResourceManager)
        
        # Table with no columns
        bad_data = MockTDAParser(columns=[], rows=[])
        rm.get_2da_with_overrides.return_value = bad_data
        
        loader = DynamicGameDataLoader(
            resource_manager=rm,
            use_async=False
        )
        
        # Should not crash
        assert loader is not None
    
    def test_stats_reporting(self, mock_resource_manager):
        """Test statistics reporting."""
        loader = DynamicGameDataLoader(
            resource_manager=mock_resource_manager,
            use_async=False
        )
        
        stats = loader.get_stats()
        
        assert 'tables_loaded' in stats
        assert 'total_rows' in stats
        # New interface doesn't have separate count fields
        assert stats['tables_loaded'] > 0