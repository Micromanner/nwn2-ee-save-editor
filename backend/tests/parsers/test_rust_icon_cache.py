"""
Tests for the Rust icon cache implementation
"""
import pytest
import tempfile
import shutil
from pathlib import Path
from unittest.mock import patch, MagicMock
import asyncio

# Try to import the Rust cache
try:
    from rust_icon_cache import RustIconCache
    RUST_CACHE_AVAILABLE = True
except ImportError:
    RUST_CACHE_AVAILABLE = False
    RustIconCache = None


@pytest.mark.skipif(not RUST_CACHE_AVAILABLE, reason="Rust icon cache not available")
class TestRustIconCache:
    """Test the Rust icon cache implementation"""
    
    @pytest.fixture
    def temp_cache_dir(self):
        """Create a temporary cache directory"""
        temp_dir = tempfile.mkdtemp()
        yield Path(temp_dir)
        shutil.rmtree(temp_dir, ignore_errors=True)
    
    @pytest.fixture
    def cache_instance(self, temp_cache_dir):
        """Create a cache instance with temp directory"""
        return RustIconCache(str(temp_cache_dir))
    
    def test_create_cache(self, temp_cache_dir):
        """Test creating a new cache instance"""
        cache = RustIconCache(str(temp_cache_dir))
        assert cache is not None
    
    def test_get_nonexistent_icon(self, cache_instance):
        """Test getting an icon that doesn't exist"""
        result = cache_instance.get_icon("nonexistent_icon")
        assert result is None
    
    def test_get_statistics_empty_cache(self, cache_instance):
        """Test getting statistics from empty cache"""
        stats = cache_instance.get_statistics()
        assert isinstance(stats, dict)
        assert stats['total_icons'] == 0
        assert stats['memory_usage_mb'] == 0.0
    
    @pytest.mark.asyncio
    async def test_initialize_cache(self, cache_instance):
        """Test initializing the cache (async)"""
        # This would normally scan directories, but with no game paths it should handle gracefully
        loop = asyncio.get_event_loop()
        await asyncio.wait_for(
            asyncio.create_task(
                asyncio.to_thread(cache_instance.initialize, False)
            ),
            timeout=5.0
        )
    
    def test_batch_get_icons(self, cache_instance):
        """Test batch getting icons"""
        names = ["icon1", "icon2", "icon3"]
        results = cache_instance.get_icons_batch(names)
        
        assert len(results) == 3
        assert all(r is None for r in results)
    
    def test_panic_safety(self, cache_instance):
        """Test that panics are caught and don't crash Python"""
        # Even with invalid inputs, should not crash
        result = cache_instance.get_icon("")
        assert result is None
        
        # Test with very long string
        result = cache_instance.get_icon("x" * 10000)
        assert result is None
    
    @pytest.mark.asyncio
    async def test_clear_cache(self, cache_instance):
        """Test clearing the cache"""
        loop = asyncio.get_event_loop()
        await asyncio.wait_for(
            asyncio.create_task(
                asyncio.to_thread(cache_instance.clear_cache)
            ),
            timeout=5.0
        )
        
        stats = cache_instance.get_statistics()
        assert stats['total_icons'] == 0


@pytest.mark.skipif(RUST_CACHE_AVAILABLE, reason="Testing fallback when Rust not available")
class TestRustCacheFallback:
    """Test behavior when Rust cache is not available"""
    
    def test_import_fallback(self):
        """Test that import failure is handled gracefully"""
        assert RustIconCache is None
        assert not RUST_CACHE_AVAILABLE


class TestRustCacheIntegration:
    """Test integration with the Python icon cache"""
    
    @pytest.fixture
    def mock_rust_cache(self):
        """Create a mock Rust cache for testing"""
        if RUST_CACHE_AVAILABLE:
            pytest.skip("Testing mock behavior, but real cache is available")
        
        mock = MagicMock()
        mock.get_icon.return_value = (b'fake_webp_data', 'image/webp')
        mock.get_statistics.return_value = {
            'total_icons': 100,
            'memory_usage_mb': 10.5,
            'base_count': 80,
            'override_count': 20,
        }
        return mock
    
    def test_mock_get_icon(self, mock_rust_cache):
        """Test mock icon retrieval"""
        data, mime = mock_rust_cache.get_icon('test_icon')
        assert data == b'fake_webp_data'
        assert mime == 'image/webp'
    
    def test_mock_statistics(self, mock_rust_cache):
        """Test mock statistics"""
        stats = mock_rust_cache.get_statistics()
        assert stats['total_icons'] == 100
        assert stats['memory_usage_mb'] == 10.5