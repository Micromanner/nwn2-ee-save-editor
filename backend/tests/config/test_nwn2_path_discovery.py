"""
Tests for Rust-powered NWN2 path discovery functionality.

This module tests the NWN2PathFinder and NWN2Paths classes that use
Rust extensions for high-performance path discovery with Steam/GOG
categorization and performance profiling.
"""

import pytest
import os
import tempfile
import shutil
from pathlib import Path
from unittest.mock import patch, MagicMock
from typing import List, Dict, Any

# Import the modules under test
from config.nwn2_settings import (
    NWN2PathFinder, 
    NWN2Paths, 
    profile_path_discovery_performance,
    nwn2_paths
)

# Import Rust wrapper for direct testing
try:
    from rust_extensions.python.nwn2_rust_wrapper import (
        discover_nwn2_paths, 
        DiscoveryResult, 
        PathTiming,
        RUST_AVAILABLE
    )
    RUST_EXTENSIONS_AVAILABLE = RUST_AVAILABLE
except ImportError:
    RUST_EXTENSIONS_AVAILABLE = False


class TestRustExtensionsAvailability:
    """Test that Rust extensions are properly available."""
    
    def test_rust_extensions_imported(self):
        """Test that Rust extensions are available and imported."""
        assert RUST_EXTENSIONS_AVAILABLE, "Rust extensions should be available for testing"
    
    def test_discover_nwn2_paths_callable(self):
        """Test that the main discovery function is callable."""
        if not RUST_EXTENSIONS_AVAILABLE:
            pytest.skip("Rust extensions not available")
        
        assert callable(discover_nwn2_paths), "discover_nwn2_paths should be callable"


class TestDiscoveryResult:
    """Test the DiscoveryResult wrapper class."""
    
    @pytest.fixture
    def mock_rust_result(self):
        """Create a mock Rust result object."""
        mock_result = MagicMock()
        mock_result.nwn2_paths = ["/path/to/nwn2", "/path/to/nwn2_steam"]
        mock_result.steam_paths = ["/path/to/nwn2_steam"]
        mock_result.gog_paths = ["/path/to/nwn2_gog"]
        mock_result.total_time_ms = 150
        
        # Mock timing breakdown
        mock_timing = MagicMock()
        mock_timing.operation = "path_discovery"
        mock_timing.duration_ms = 150
        mock_timing.paths_checked = 10
        mock_timing.paths_found = 2
        mock_result.timing_breakdown = [mock_timing]
        
        return mock_result
    
    def test_discovery_result_from_rust_result(self, mock_rust_result):
        """Test creating DiscoveryResult from Rust result."""
        if not RUST_EXTENSIONS_AVAILABLE:
            pytest.skip("Rust extensions not available")
        
        result = DiscoveryResult.from_rust_result(mock_rust_result)
        
        assert result.nwn2_paths == ["/path/to/nwn2", "/path/to/nwn2_steam"]
        assert result.steam_paths == ["/path/to/nwn2_steam"]
        assert result.gog_paths == ["/path/to/nwn2_gog"]
        assert result.total_time_ms == 150
        assert result.total_time_seconds == 0.15
        assert len(result.timing_breakdown) == 1
        assert result.timing_breakdown[0].operation == "path_discovery"
    
    def test_discovery_result_repr(self, mock_rust_result):
        """Test DiscoveryResult string representation."""
        if not RUST_EXTENSIONS_AVAILABLE:
            pytest.skip("Rust extensions not available")
        
        result = DiscoveryResult.from_rust_result(mock_rust_result)
        repr_str = repr(result)
        
        assert "DiscoveryResult" in repr_str
        assert "nwn2=2" in repr_str
        assert "steam=1" in repr_str
        assert "gog=1" in repr_str
        assert "time=150ms" in repr_str


class TestPathTiming:
    """Test the PathTiming wrapper class."""
    
    def test_path_timing_creation(self):
        """Test creating PathTiming object."""
        timing = PathTiming("path_discovery", 150, 10, 2)
        
        assert timing.operation == "path_discovery"
        assert timing.duration_ms == 150
        assert timing.paths_checked == 10
        assert timing.paths_found == 2
    
    def test_path_timing_repr(self):
        """Test PathTiming string representation."""
        timing = PathTiming("path_discovery", 150, 10, 2)
        repr_str = repr(timing)
        
        assert "PathTiming" in repr_str
        assert "op=path_discovery" in repr_str
        assert "time=150ms" in repr_str
        assert "checked=10" in repr_str
        assert "found=2" in repr_str


class TestNWN2PathFinder:
    """Test the NWN2PathFinder class with Rust implementation."""
    
    @pytest.fixture
    def mock_discovery_result(self):
        """Create a mock discovery result for testing."""
        timing = PathTiming("path_discovery", 100, 5, 2)
        return DiscoveryResult(
            nwn2_paths=["/fake/nwn2", "/fake/steam/nwn2"],
            steam_paths=["/fake/steam/nwn2"],
            gog_paths=["/fake/gog/nwn2"],
            total_time_ms=100,
            timing_breakdown=[timing]
        )
    
    @patch('config.nwn2_settings.discover_nwn2_paths')
    def test_auto_discover_nwn2_paths_default(self, mock_discover, mock_discovery_result):
        """Test auto_discover_nwn2_paths with default search paths."""
        mock_discover.return_value = mock_discovery_result
        
        result = NWN2PathFinder.auto_discover_nwn2_paths()
        
        mock_discover.assert_called_once_with(None)
        expected_paths = [Path("/fake/nwn2"), Path("/fake/steam/nwn2")]
        assert result == expected_paths
    
    @patch('config.nwn2_settings.discover_nwn2_paths')
    def test_auto_discover_nwn2_paths_custom_paths(self, mock_discover, mock_discovery_result):
        """Test auto_discover_nwn2_paths with custom search paths."""
        mock_discover.return_value = mock_discovery_result
        custom_paths = [Path("/custom/path1"), Path("/custom/path2")]
        
        result = NWN2PathFinder.auto_discover_nwn2_paths(custom_paths)
        
        mock_discover.assert_called_once_with(custom_paths)
        expected_paths = [Path("/fake/nwn2"), Path("/fake/steam/nwn2")]
        assert result == expected_paths
    
    @patch('config.nwn2_settings.discover_nwn2_paths')
    def test_find_nwn2_installation_found(self, mock_discover, mock_discovery_result):
        """Test find_nwn2_installation when paths are found."""
        mock_discover.return_value = mock_discovery_result
        
        result = NWN2PathFinder.find_nwn2_installation()
        
        assert result == Path("/fake/nwn2")
    
    @patch('config.nwn2_settings.discover_nwn2_paths')
    def test_find_nwn2_installation_not_found(self, mock_discover):
        """Test find_nwn2_installation when no paths are found."""
        empty_result = DiscoveryResult([], [], [], 50, [])
        mock_discover.return_value = empty_result
        
        result = NWN2PathFinder.find_nwn2_installation()
        
        assert result is None
    
    @patch('config.nwn2_settings.discover_nwn2_paths')
    def test_find_steam_installation(self, mock_discover, mock_discovery_result):
        """Test find_steam_installation."""
        mock_discover.return_value = mock_discovery_result
        
        result = NWN2PathFinder.find_steam_installation()
        
        assert result == Path("/fake/steam/nwn2")
    
    @patch('config.nwn2_settings.discover_nwn2_paths')
    def test_find_gog_installation(self, mock_discover, mock_discovery_result):
        """Test find_gog_installation."""
        mock_discover.return_value = mock_discovery_result
        
        result = NWN2PathFinder.find_gog_installation()
        
        assert result == Path("/fake/gog/nwn2")
    
    @patch('config.nwn2_settings.discover_nwn2_paths')
    def test_get_discovery_timing(self, mock_discover, mock_discovery_result):
        """Test get_discovery_timing returns detailed timing information."""
        mock_discover.return_value = mock_discovery_result
        
        result = NWN2PathFinder.get_discovery_timing()
        
        assert isinstance(result, dict)
        assert result['total_time_ms'] == 100
        assert result['total_time_seconds'] == 0.1
        assert 'timing_breakdown' in result
        assert len(result['timing_breakdown']) == 1
        
        breakdown = result['timing_breakdown'][0]
        assert breakdown['operation'] == 'path_discovery'
        assert breakdown['duration_ms'] == 100
        assert breakdown['paths_checked'] == 5
        assert breakdown['paths_found'] == 2


class TestNWN2Paths:
    """Test the NWN2Paths class with Rust-powered discovery."""
    
    @pytest.fixture
    def temp_nwn2_dir(self):
        """Create a temporary NWN2-like directory structure."""
        temp_dir = tempfile.mkdtemp()
        nwn2_dir = Path(temp_dir) / "Neverwinter Nights 2"
        nwn2_dir.mkdir(parents=True)
        
        # Create key NWN2 files/folders to make it look like a valid installation
        (nwn2_dir / "data").mkdir()
        (nwn2_dir / "dialog.tlk").touch()
        (nwn2_dir / "enhanced").mkdir()
        (nwn2_dir / "enhanced" / "data").mkdir()
        
        yield nwn2_dir
        
        shutil.rmtree(temp_dir)
    
    def test_nwn2_paths_initialization(self):
        """Test NWN2Paths initializes correctly."""
        paths = NWN2Paths()
        
        assert hasattr(paths, '_game_folder')
        assert hasattr(paths, '_documents_folder')
        assert hasattr(paths, '_steam_workshop_folder')
    
    def test_game_folder_property(self, temp_nwn2_dir):
        """Test game_folder property."""
        paths = NWN2Paths()
        paths._game_folder = temp_nwn2_dir
        
        assert paths.game_folder == temp_nwn2_dir
    
    def test_data_property(self, temp_nwn2_dir):
        """Test data property."""
        paths = NWN2Paths()
        paths._game_folder = temp_nwn2_dir
        
        expected_data_path = temp_nwn2_dir / 'data'
        assert paths.data == expected_data_path
        assert paths.data.exists()
    
    def test_enhanced_property(self, temp_nwn2_dir):
        """Test enhanced property."""
        paths = NWN2Paths()
        paths._game_folder = temp_nwn2_dir
        
        expected_enhanced_path = temp_nwn2_dir / 'enhanced'
        assert paths.enhanced == expected_enhanced_path
        assert paths.enhanced.exists()
    
    def test_enhanced_data_property(self, temp_nwn2_dir):
        """Test enhanced_data property."""
        paths = NWN2Paths()
        paths._game_folder = temp_nwn2_dir
        
        expected_enhanced_data_path = temp_nwn2_dir / 'enhanced' / 'data'
        assert paths.enhanced_data == expected_enhanced_data_path
        assert paths.enhanced_data.exists()
    
    def test_is_enhanced_edition(self, temp_nwn2_dir):
        """Test is_enhanced_edition property."""
        paths = NWN2Paths()
        paths._game_folder = temp_nwn2_dir
        
        assert paths.is_enhanced_edition is True
    
    def test_is_steam_installation_true(self, temp_nwn2_dir):
        """Test is_steam_installation when path contains Steam."""
        paths = NWN2Paths()
        steam_path = temp_nwn2_dir.parent / "Steam" / "steamapps" / "common" / "NWN2"
        steam_path.mkdir(parents=True)
        paths._game_folder = steam_path
        
        assert paths.is_steam_installation is True
    
    def test_is_steam_installation_false(self, temp_nwn2_dir):
        """Test is_steam_installation when path doesn't contain Steam."""
        paths = NWN2Paths()
        paths._game_folder = temp_nwn2_dir
        
        assert paths.is_steam_installation is False
    
    def test_is_gog_installation_true(self, temp_nwn2_dir):
        """Test is_gog_installation when path contains GOG."""
        paths = NWN2Paths()
        gog_path = temp_nwn2_dir.parent / "GOG Games" / "NWN2"
        gog_path.mkdir(parents=True)
        paths._game_folder = gog_path
        
        assert paths.is_gog_installation is True
    
    def test_is_gog_installation_false(self, temp_nwn2_dir):
        """Test is_gog_installation when path doesn't contain GOG."""
        paths = NWN2Paths()
        paths._game_folder = temp_nwn2_dir
        
        assert paths.is_gog_installation is False
    
    def test_get_all_data_folders(self, temp_nwn2_dir):
        """Test get_all_data_folders includes both regular and enhanced data."""
        paths = NWN2Paths()
        paths._game_folder = temp_nwn2_dir
        
        data_folders = paths.get_all_data_folders()
        
        expected_folders = [
            temp_nwn2_dir / 'data',
            temp_nwn2_dir / 'enhanced' / 'data'
        ]
        assert data_folders == expected_folders
    
    @patch('config.nwn2_settings.NWN2PathFinder.get_discovery_timing')
    def test_get_path_discovery_performance(self, mock_timing):
        """Test get_path_discovery_performance returns timing info."""
        mock_timing.return_value = {
            'total_time_ms': 200,
            'total_time_seconds': 0.2,
            'timing_breakdown': []
        }
        
        paths = NWN2Paths()
        result = paths.get_path_discovery_performance()
        
        assert result['total_time_ms'] == 200
        assert result['total_time_seconds'] == 0.2
        mock_timing.assert_called_once()
    
    @patch('config.nwn2_settings.discover_nwn2_paths')
    def test_discover_all_nwn2_installations(self, mock_discover):
        """Test discover_all_nwn2_installations returns categorized results."""
        timing = PathTiming("test_op", 100, 5, 2)
        mock_result = DiscoveryResult(
            nwn2_paths=["/path1", "/path2"],
            steam_paths=["/steam/path"],
            gog_paths=["/gog/path"],
            total_time_ms=100,
            timing_breakdown=[timing]
        )
        mock_discover.return_value = mock_result
        
        paths = NWN2Paths()
        result = paths.discover_all_nwn2_installations()
        
        assert 'all_installations' in result
        assert 'steam_installations' in result
        assert 'gog_installations' in result
        assert 'discovery_time_ms' in result
        assert 'timing_breakdown' in result
        
        assert result['all_installations'] == [Path("/path1"), Path("/path2")]
        assert result['steam_installations'] == [Path("/steam/path")]
        assert result['gog_installations'] == [Path("/gog/path")]
        assert result['discovery_time_ms'] == 100
    
    def test_get_all_paths_info_enhanced(self, temp_nwn2_dir):
        """Test get_all_paths_info includes enhanced Steam/GOG info."""
        paths = NWN2Paths()
        paths._game_folder = temp_nwn2_dir
        
        with patch.object(paths, 'get_path_discovery_performance') as mock_perf:
            mock_perf.return_value = {'test': 'data'}
            
            info = paths.get_all_paths_info()
            
            assert 'game_folder' in info
            game_info = info['game_folder']
            assert 'is_steam' in game_info
            assert 'is_gog' in game_info
            assert 'is_enhanced_edition' in game_info
            assert 'path_discovery_performance' in info
            assert info['path_discovery_performance'] == {'test': 'data'}


class TestPerformanceProfiling:
    """Test performance profiling functionality."""
    
    @patch('config.nwn2_settings.profile_path_discovery')
    def test_profile_path_discovery_performance(self, mock_profile):
        """Test profile_path_discovery_performance function."""
        mock_profile.return_value = {
            'mean_seconds': 0.1,
            'min_seconds': 0.05,
            'max_seconds': 0.15,
            'iterations': 10
        }
        
        result = profile_path_discovery_performance(10)
        
        mock_profile.assert_called_once_with(10)
        assert result['mean_seconds'] == 0.1
        assert result['iterations'] == 10


class TestIntegration:
    """Integration tests with real Rust functionality (if available)."""
    
    @pytest.mark.skipif(not RUST_EXTENSIONS_AVAILABLE, reason="Rust extensions not available")
    def test_real_rust_path_discovery(self):
        """Test actual Rust path discovery (may not find real paths in test environment)."""
        result = discover_nwn2_paths()
        
        assert isinstance(result, DiscoveryResult)
        assert isinstance(result.nwn2_paths, list)
        assert isinstance(result.steam_paths, list)
        assert isinstance(result.gog_paths, list)
        assert isinstance(result.total_time_ms, int)
        assert isinstance(result.timing_breakdown, list)
        assert result.total_time_ms >= 0
    
    @pytest.mark.skipif(not RUST_EXTENSIONS_AVAILABLE, reason="Rust extensions not available")
    def test_nwn2_path_finder_integration(self):
        """Test NWN2PathFinder integration with real Rust implementation."""
        # This may not find actual NWN2 installations in test environment
        result = NWN2PathFinder.auto_discover_nwn2_paths()
        
        assert isinstance(result, list)
        # All results should be Path objects
        for path in result:
            assert isinstance(path, Path)
    
    @pytest.mark.skipif(not RUST_EXTENSIONS_AVAILABLE, reason="Rust extensions not available")
    def test_performance_timing_integration(self):
        """Test performance timing integration."""
        timing_info = NWN2PathFinder.get_discovery_timing()
        
        assert isinstance(timing_info, dict)
        assert 'total_time_ms' in timing_info
        assert 'total_time_seconds' in timing_info
        assert 'timing_breakdown' in timing_info
        assert timing_info['total_time_ms'] >= 0
        assert timing_info['total_time_seconds'] >= 0
        assert isinstance(timing_info['timing_breakdown'], list)


class TestErrorHandling:
    """Test error handling scenarios."""
    
    @patch('config.nwn2_settings.discover_nwn2_paths')
    def test_rust_exception_handling(self, mock_discover):
        """Test handling of Rust exceptions."""
        mock_discover.side_effect = RuntimeError("Rust error")
        
        with pytest.raises(RuntimeError, match="Rust error"):
            NWN2PathFinder.auto_discover_nwn2_paths()
    
    def test_paths_info_performance_error_handling(self):
        """Test error handling in get_all_paths_info performance section."""
        paths = NWN2Paths()
        paths._game_folder = Path("/fake/path")
        
        with patch.object(paths, 'get_path_discovery_performance') as mock_perf:
            mock_perf.side_effect = Exception("Performance error")
            
            info = paths.get_all_paths_info()
            
            assert 'path_discovery_performance' in info
            assert 'error' in info['path_discovery_performance']
            assert 'Performance error' in info['path_discovery_performance']['error']


if __name__ == '__main__':
    pytest.main([__file__, '-v'])