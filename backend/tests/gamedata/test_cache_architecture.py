"""
Tests for cache architecture to ensure no conflicts between disk and memory caching.
"""
import pytest
import tempfile
from pathlib import Path
from unittest.mock import Mock, patch, MagicMock
import zipfile
import io

from services.core.resource_manager import ResourceManager
from nwn2_rust import TDAParser
from gamedata.enhanced_icon_cache import EnhancedIconCache
from services.core.safe_cache import SafeCache


class TestCacheArchitecture:
    """Test the cache architecture to ensure consistency."""
    
    @pytest.fixture
    def temp_cache_dir(self):
        """Create a temporary cache directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            yield Path(tmpdir)
    
    @pytest.fixture
    def mock_2da_data(self):
        """Create mock 2DA data."""
        parser = TDAParser()
        parser.columns = ['ID', 'Name', 'IconResRef']
        parser.resources = [
            ['0', 'Fire', 'spell_fire'],
            ['1', 'Ice', 'spell_ice'],
            ['2', 'Lightning', 'spell_lightning']
        ]
        return parser
    
    @pytest.fixture
    def resource_manager_with_cache(self, temp_cache_dir, mock_2da_data):
        """Create a ResourceManager with pre-populated cache."""
        # Create cached .msgpack files
        for name in ['spells.2da', 'classes.2da', 'feats.2da']:
            cache_file = temp_cache_dir / name
            SafeCache.save(cache_file, mock_2da_data)
        
        # Mock the ZIP file locations
        with patch.object(ResourceManager, '_scan_zip_files'):
            rm = ResourceManager(cache_dir=str(temp_cache_dir))
            rm._2da_locations = {
                'spells.2da': ('test.zip', 'spells.2da'),
                'classes.2da': ('test.zip', 'classes.2da'),
                'feats.2da': ('test.zip', 'feats.2da')
            }
            # Mock ZIP files
            rm._zip_files = {'test.zip': MagicMock()}
            return rm
    
    def test_memory_cache_preloading(self, resource_manager_with_cache):
        """Test that .msgpack files are preloaded into memory on startup."""
        rm = resource_manager_with_cache
        
        # Enable memory cache and preload
        rm._memory_cache_enabled = True
        rm._preload_on_init = True
        
        # Run preload
        rm._preload_all_base_2das()
        
        # Verify files are in memory cache
        assert 'spells.2da' in rm._2da_cache
        assert 'classes.2da' in rm._2da_cache
        # feats.2da might not load due to smart preloading priority
        
        # Verify they're not compressed (since they're small)
        assert not rm._2da_compressed.get('spells.2da', False)
    
    def test_get_2da_uses_memory_cache_only(self, resource_manager_with_cache):
        """Test that get_2da() uses memory cache and doesn't check disk cache."""
        rm = resource_manager_with_cache
        rm._memory_cache_enabled = True
        
        # Preload data
        rm._preload_all_base_2das()
        
        # Mock the disk cache method to ensure it's not called
        with patch.object(rm, '_load_from_disk_cache') as mock_disk_cache:
            # Get 2DA from memory
            result = rm.get_2da('spells')
            
            # Verify disk cache was NOT called
            mock_disk_cache.assert_not_called()
            
            # Verify we got the data
            assert result is not None
            assert result.columns == ['ID', 'Name', 'IconResRef']
    
    def test_contextual_cache_keys(self, resource_manager_with_cache):
        """Test that cache keys include module context."""
        rm = resource_manager_with_cache
        rm._memory_cache_enabled = True
        
        # Set a module
        rm._current_module = 'TestModule.mod'
        
        # Build cache key
        key1 = rm._build_cache_key('spells.2da')
        assert key1 == 'TestModule.mod:spells.2da'
        
        # Without module
        rm._current_module = None
        key2 = rm._build_cache_key('spells.2da')
        assert key2 == 'spells.2da'
    
    def test_override_chain_consistency(self, resource_manager_with_cache, temp_cache_dir):
        """Test that override chain works correctly with memory cache."""
        rm = resource_manager_with_cache
        rm._memory_cache_enabled = True
        
        # Create different versions of spells.2da
        base_parser = TDAParser()
        base_parser.columns = ['ID', 'Name']
        base_parser.resources = [['0', 'BaseSpell']]
        
        override_parser = TDAParser()
        override_parser.columns = ['ID', 'Name']
        override_parser.resources = [['0', 'OverrideSpell']]
        
        hak_parser = TDAParser()
        hak_parser.columns = ['ID', 'Name']
        hak_parser.resources = [['0', 'HakSpell']]
        
        module_parser = TDAParser()
        module_parser.columns = ['ID', 'Name']
        module_parser.resources = [['0', 'ModuleSpell']]
        
        # Set up override chain
        rm._2da_cache['spells.2da'] = base_parser
        rm._override_dir_overrides['spells.2da'] = override_parser
        rm._hak_overrides = [{'spells.2da': hak_parser}]
        rm._module_overrides['spells.2da'] = module_parser
        
        # Test with module context
        rm._current_module = 'TestModule.mod'
        result = rm.get_2da_with_overrides('spells')
        
        # Should get module version (highest priority)
        assert result.resources[0][1] == 'ModuleSpell'
        
        # Clear module override and cache
        rm._module_overrides.clear()
        # Clear the cache entry for this module context
        cache_key = rm._build_cache_key('spells.2da')
        if cache_key in rm._2da_cache:
            del rm._2da_cache[cache_key]
        
        result = rm.get_2da_with_overrides('spells')
        
        # Should get HAK version
        assert result.resources[0][1] == 'HakSpell'
        
        # Clear HAK override and cache
        rm._hak_overrides.clear()
        cache_key = rm._build_cache_key('spells.2da')
        if cache_key in rm._2da_cache:
            del rm._2da_cache[cache_key]
        
        result = rm.get_2da_with_overrides('spells')
        
        # Should get override directory version
        assert result.resources[0][1] == 'OverrideSpell'
    
    def test_icon_cache_uses_resource_manager(self):
        """Test that icon cache uses ResourceManager for 2DA data."""
        # Create mock resource manager
        mock_rm = Mock(spec=ResourceManager)
        mock_parser = Mock()
        mock_parser.columns = ['ID', 'Name', 'IconResRef']
        mock_parser.get_rows_as_dicts.return_value = [
            {'ID': '0', 'Name': 'Fire', 'IconResRef': 'spell_fire'},
            {'ID': '1', 'Name': 'Ice', 'IconResRef': 'spell_ice'}
        ]
        mock_rm.get_2da_with_overrides.return_value = mock_parser
        
        # Create icon cache with mock resource manager
        icon_cache = EnhancedIconCache(resource_manager=mock_rm)
        
        # Get spell icons
        icons = icon_cache._get_spell_icons(mock_rm)
        
        # Verify ResourceManager was called correctly
        mock_rm.get_2da_with_overrides.assert_called_once_with('spells')
        
        # Verify icons were extracted
        assert 'spell_fire' in icons
        assert 'spell_ice' in icons
    
    def test_cache_memory_limit(self, resource_manager_with_cache):
        """Test that cache respects memory limits."""
        rm = resource_manager_with_cache
        rm._memory_cache_enabled = True
        rm._cache_max_mb = 0.001  # Very small limit to trigger eviction
        
        # Create large parser
        large_parser = TDAParser()
        large_parser.columns = ['ID', 'Data']
        large_parser.resources = [['0', 'X' * 10000] for _ in range(100)]
        
        # Add to cache
        rm._add_to_cache('large1.2da', large_parser)
        rm._add_to_cache('large2.2da', large_parser)
        rm._add_to_cache('large3.2da', large_parser)
        
        # Cache should have evicted some items
        assert len(rm._2da_cache) < 3
    
    def test_module_change_cache_invalidation(self, resource_manager_with_cache):
        """Test that changing modules properly updates cache keys."""
        rm = resource_manager_with_cache
        rm._memory_cache_enabled = True
        
        # Load base game spells
        rm._current_module = None
        spells1 = rm.get_2da_with_overrides('spells')
        
        # Change to module
        rm._current_module = 'Module1.mod'
        cache_key1 = rm._build_cache_key('spells.2da')
        
        # Change to different module
        rm._current_module = 'Module2.mod'
        cache_key2 = rm._build_cache_key('spells.2da')
        
        # Keys should be different
        assert cache_key1 != cache_key2
        assert 'Module1.mod' in cache_key1
        assert 'Module2.mod' in cache_key2


@pytest.mark.integration
class TestCacheIntegration:
    """Integration tests for the complete cache system."""
    
    @pytest.fixture
    def temp_cache_dir(self):
        """Create a temporary cache directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            yield Path(tmpdir)
    
    def test_full_system_no_duplication(self, temp_cache_dir):
        """Test that the full system doesn't duplicate data in memory."""
        # This would be an integration test with real files
        # For now, we'll use mocks to verify the architecture
        
        with patch('services.resource_manager.nwn2_paths') as mock_paths:
            mock_paths.game_folder = temp_cache_dir
            mock_paths.user_override = temp_cache_dir / 'override'
            mock_paths.user_override.mkdir(exist_ok=True)
            
            # Create resource manager
            rm = ResourceManager(cache_dir=str(temp_cache_dir))
            
            # Create icon cache with resource manager
            icon_cache = EnhancedIconCache(resource_manager=rm)
            
            # Verify they share the same data source
            assert icon_cache._resource_manager is rm