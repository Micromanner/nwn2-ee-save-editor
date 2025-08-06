"""
Tests for SecureCodeCache - Code caching system
"""
import pytest
import tempfile
import shutil
from pathlib import Path
import time
import json

from gamedata.dynamic_loader.code_cache import SecureCodeCache


class TestSecureCodeCache:
    """Test secure code caching."""
    
    @pytest.fixture
    def temp_cache_dir(self):
        """Create temporary cache directory."""
        temp_dir = tempfile.mkdtemp()
        yield Path(temp_dir)
        # Cleanup
        shutil.rmtree(temp_dir)
    
    @pytest.fixture
    def cache(self, temp_cache_dir):
        """Create cache instance with temp directory."""
        return SecureCodeCache(temp_cache_dir)
    
    def test_basic_caching(self, cache, temp_cache_dir):
        """Test basic cache save and load."""
        table_name = "test_table"
        code = '''
class TestTableData:
    __slots__ = ('_id', '_name')
    
    def __init__(self, **kwargs):
        self._id = kwargs.get('id')
        self._name = kwargs.get('name')
'''
        
        # Generator function
        def generate():
            return code
        
        # First call should generate
        result = cache.load_or_generate(table_name, None, generate)
        assert result == code
        
        # Check file was created
        cache_files = list(temp_cache_dir.glob("*.py"))
        assert len(cache_files) == 1
        assert cache_files[0].stem == table_name
        
        # Second call should load from cache
        result2 = cache.load_or_generate(table_name, None, generate)
        assert result2 == code
    
    def test_file_hash_validation(self, cache, temp_cache_dir):
        """Test cache invalidation based on file hash."""
        # Create a test file
        test_file = temp_cache_dir / "test.2da"
        test_file.write_text("2DA V2.0\n\nID Name\n0 Test")
        
        table_name = "test"
        code_v1 = "class TestData: pass"
        code_v2 = "class TestDataV2: pass"
        
        call_count = 0
        
        def generate():
            nonlocal call_count
            call_count += 1
            return code_v1 if call_count == 1 else code_v2
        
        # First call
        result1 = cache.load_or_generate(table_name, test_file, generate)
        assert result1 == code_v1
        assert call_count == 1
        
        # Second call - should use cache
        result2 = cache.load_or_generate(table_name, test_file, generate)
        assert result2 == code_v1
        assert call_count == 1  # Generator not called again
        
        # Modify file
        time.sleep(0.01)  # Ensure different mtime
        test_file.write_text("2DA V2.0\n\nID Name Value\n0 Test 100")
        
        # Third call - should regenerate due to file change
        result3 = cache.load_or_generate(table_name, test_file, generate)
        assert result3 == code_v2
        assert call_count == 2
    
    def test_metadata_tracking(self, cache, temp_cache_dir):
        """Test metadata file creation and updates."""
        table_name = "test"
        code = "class TestData: pass"
        
        cache.load_or_generate(table_name, None, lambda: code)
        
        # Check metadata file
        metadata_file = temp_cache_dir / "cache_metadata.json"
        assert metadata_file.exists()
        
        with open(metadata_file) as f:
            metadata = json.load(f)
        
        assert table_name in metadata
        assert metadata[table_name]["table_name"] == table_name
        assert metadata[table_name]["code_size"] == len(code)
        assert "generated_at" in metadata[table_name]
    
    def test_old_cache_cleanup(self, cache, temp_cache_dir):
        """Test cleanup of old cache files."""
        table_name = "test"
        
        # Create multiple cache files for same table
        for i in range(3):
            cache_file = temp_cache_dir / f"{table_name}_{i}.py"
            cache_file.write_text(f"# Version {i}")
            cache.metadata[f"{table_name}_{i}"] = {"table_name": table_name}
        
        # Generate new version
        cache.load_or_generate(table_name, None, lambda: "# New version")
        
        # Old files should be removed
        remaining_files = list(temp_cache_dir.glob(f"{table_name}_*.py"))
        assert len(remaining_files) == 1
        assert remaining_files[0].stem == table_name
    
    def test_clear_cache(self, cache, temp_cache_dir):
        """Test clearing entire cache."""
        # Generate some cached files
        for i in range(5):
            cache.load_or_generate(f"table_{i}", None, lambda: f"class Table{i}Data: pass")
        
        # Verify files exist
        assert len(list(temp_cache_dir.glob("*.py"))) == 5
        assert len(cache.metadata) == 5
        
        # Clear cache
        cache.clear_cache()
        
        # Verify cleanup
        assert len(list(temp_cache_dir.glob("*.py"))) == 0
        assert len(cache.metadata) == 0
        assert cache.metadata_file.exists()  # Metadata file should still exist
    
    def test_cache_stats(self, cache, temp_cache_dir):
        """Test cache statistics."""
        # Generate some files
        for i in range(3):
            code = f"class Table{i}Data: {' ' * (i * 100)}pass"  # Different sizes
            cache.load_or_generate(f"table_{i}", None, lambda c=code: c)
        
        stats = cache.get_cache_stats()
        
        assert stats["file_count"] == 3
        assert stats["metadata_entries"] == 3
        assert stats["total_size_kb"] > 0
        assert stats["cache_dir"] == str(temp_cache_dir)
        assert stats["oldest_entry"] is not None
        assert stats["newest_entry"] is not None
    
    def test_orphaned_file_cleanup(self, cache, temp_cache_dir):
        """Test cleanup of orphaned cache files."""
        # Create orphaned files (not in metadata)
        for i in range(3):
            orphan = temp_cache_dir / f"orphan_{i}.py"
            orphan.write_text("# Orphaned file")
        
        # Create legitimate cached file
        cache.load_or_generate("legitimate", None, lambda: "class LegitimateData: pass")
        
        # Should have 4 files total
        assert len(list(temp_cache_dir.glob("*.py"))) == 4
        
        # Clean orphans
        cache.cleanup_orphaned_files()
        
        # Should only have legitimate file
        remaining = list(temp_cache_dir.glob("*.py"))
        assert len(remaining) == 1
        assert remaining[0].stem == "legitimate"
    
    def test_unicode_handling(self, cache, temp_cache_dir):
        """Test handling of unicode in generated code."""
        table_name = "test_unicode"
        code = '''
class TestData:
    """Class for 测试 table."""
    name = "café"
    symbol = "€"
'''
        
        result = cache.load_or_generate(table_name, None, lambda: code)
        assert result == code
        
        # Verify it can be loaded back
        result2 = cache.load_or_generate(table_name, None, lambda: "should not be called")
        assert result2 == code
    
    def test_concurrent_access(self, cache, temp_cache_dir):
        """Test cache behavior with concurrent access."""
        # This is a simple test - in production you'd want more thorough testing
        table_name = "concurrent_test"
        
        def generate():
            time.sleep(0.01)  # Simulate work
            return "class ConcurrentData: pass"
        
        # Multiple calls in quick succession
        results = []
        for _ in range(3):
            result = cache.load_or_generate(table_name, None, generate)
            results.append(result)
        
        # All should get same result
        assert all(r == results[0] for r in results)
        
        # Should only have one cache file
        cache_files = list(temp_cache_dir.glob(f"{table_name}*.py"))
        assert len(cache_files) == 1
    
    def test_error_handling(self, cache, temp_cache_dir):
        """Test error handling in various scenarios."""
        # Test with generator that raises exception
        def bad_generator():
            raise ValueError("Generation failed")
        
        with pytest.raises(ValueError):
            cache.load_or_generate("bad_table", None, bad_generator)
        
        # Test with read-only cache directory
        cache_file = temp_cache_dir / "readonly.py"
        cache_file.write_text("class ReadOnlyData: pass")
        cache_file.chmod(0o444)  # Read-only
        
        # Should handle gracefully (log warning but not crash)
        try:
            # Attempt to write to read-only file
            result = cache.load_or_generate("readonly", None, lambda: "new content")
            # Should still work, just not update the file
            assert result == "new content"
        finally:
            # Restore permissions for cleanup
            cache_file.chmod(0o644)


class TestSecurityFeatures:
    """Test security aspects of code caching."""
    
    @pytest.fixture
    def cache(self, tmp_path):
        return SecureCodeCache(tmp_path)
    
    def test_no_code_execution(self, cache, tmp_path):
        """Verify that cache only stores strings, never executes code."""
        malicious_code = '''
import os
os.system("echo 'This should never execute'")

class EvilData:
    pass
'''
        
        # Store malicious code
        cache.load_or_generate("evil", None, lambda: malicious_code)
        
        # Verify it's stored as plain text
        cache_file = tmp_path / "evil.py"
        assert cache_file.exists()
        content = cache_file.read_text()
        assert content == malicious_code
        
        # Loading from cache should not execute
        loaded = cache.load_or_generate("evil", None, lambda: "should not generate")
        assert loaded == malicious_code
        # If code was executed, we'd see the echo output
    
    def test_path_traversal_prevention(self, cache, tmp_path):
        """Test that cache files stay within cache directory."""
        # Try various path traversal attempts
        bad_names = [
            "../../../etc/passwd",
            "..\\..\\windows\\system32",
            "/etc/passwd",
            "C:\\Windows\\System32\\config",
        ]
        
        for bad_name in bad_names:
            cache.load_or_generate(bad_name, None, lambda: "class SafeData: pass")
        
        # All files should be in cache directory
        all_files = list(tmp_path.rglob("*.py"))
        assert all(tmp_path in f.parents for f in all_files)
        
        # No files should be created outside cache dir
        parent = tmp_path.parent
        assert not list(parent.glob("*.py"))