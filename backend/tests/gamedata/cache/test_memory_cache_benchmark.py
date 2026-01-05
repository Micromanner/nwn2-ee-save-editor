#!/usr/bin/env python
"""
Performance benchmarks for in-memory 2DA cache using pytest-benchmark.
"""
import os
import sys
from pathlib import Path

# Add backend to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

import django
os.environ.setdefault('DJANGO_SETTINGS_MODULE', 'config.settings')
django.setup()

import pytest
from services.core.resource_manager import ResourceManager
from django.conf import settings


@pytest.fixture
def rm_no_cache(tmp_path):
    """ResourceManager with cache disabled."""
    settings.NWN2_MEMORY_CACHE = False
    settings.NWN2_PRELOAD_2DA = False
    rm = ResourceManager(cache_dir=str(tmp_path))
    yield rm
    rm.close()


@pytest.fixture
def rm_with_cache(tmp_path):
    """ResourceManager with cache enabled but no preload."""
    settings.NWN2_MEMORY_CACHE = True
    settings.NWN2_PRELOAD_2DA = False
    rm = ResourceManager(cache_dir=str(tmp_path))
    # Warm up the cache
    for name in ['classes', 'races', 'feat', 'spells', 'skills']:
        rm.get_2da(name)
    yield rm
    rm.close()


@pytest.fixture
def rm_with_preload(tmp_path):
    """ResourceManager with cache and preload enabled."""
    settings.NWN2_MEMORY_CACHE = True
    settings.NWN2_PRELOAD_2DA = True
    rm = ResourceManager(cache_dir=str(tmp_path))
    yield rm
    rm.close()


class TestCacheBenchmarks:
    """Benchmark tests for 2DA cache performance."""
    
    def test_single_access_no_cache(self, benchmark, rm_no_cache):
        """Benchmark single 2DA access without cache."""
        result = benchmark(rm_no_cache.get_2da, 'classes')
        assert result is not None
    
    def test_single_access_with_cache(self, benchmark, rm_with_cache):
        """Benchmark single 2DA access with warm cache."""
        result = benchmark(rm_with_cache.get_2da, 'classes')
        assert result is not None
    
    def test_multiple_access_no_cache(self, benchmark, rm_no_cache):
        """Benchmark accessing multiple different 2DAs without cache."""
        def access_multiple():
            rm_no_cache.get_2da('classes')
            rm_no_cache.get_2da('races')
            rm_no_cache.get_2da('feat')
            rm_no_cache.get_2da('spells')
            rm_no_cache.get_2da('skills')
        
        benchmark(access_multiple)
    
    def test_multiple_access_with_cache(self, benchmark, rm_with_cache):
        """Benchmark accessing multiple different 2DAs with warm cache."""
        def access_multiple():
            rm_with_cache.get_2da('classes')
            rm_with_cache.get_2da('races')
            rm_with_cache.get_2da('feat')
            rm_with_cache.get_2da('spells')
            rm_with_cache.get_2da('skills')
        
        benchmark(access_multiple)
    
    def test_override_chain_no_cache(self, benchmark, rm_no_cache):
        """Benchmark override chain resolution without cache."""
        result = benchmark(rm_no_cache.get_2da_with_overrides, 'classes')
        assert result is not None
    
    def test_override_chain_with_cache(self, benchmark, rm_with_cache):
        """Benchmark override chain resolution with cache."""
        # Prime the cache
        rm_with_cache.get_2da_with_overrides('classes')
        
        result = benchmark(rm_with_cache.get_2da_with_overrides, 'classes')
        assert result is not None


class TestPreloadBenchmarks:
    """Benchmark tests for preload performance."""
    
    def test_preload_time(self, benchmark, tmp_path):
        """Benchmark the time to preload all 2DAs."""
        def create_and_preload():
            settings.NWN2_MEMORY_CACHE = True
            settings.NWN2_PRELOAD_2DA = True
            rm = ResourceManager(cache_dir=str(tmp_path))
            rm.close()
            return rm
        
        result = benchmark(create_and_preload)
        # Check that preloading actually happened
        assert result.get_cached_count() > 50
    
    def test_memory_usage_after_preload(self, rm_with_preload):
        """Measure memory usage after preloading."""
        stats = rm_with_preload.get_cache_stats()
        
        print(f"\nMemory usage statistics:")
        print(f"  Cached items: {stats['cached_items']}")
        print(f"  Memory usage: {stats['current_size_mb']:.2f} MB")
        print(f"  Cache enabled: {stats['enabled']}")
        print(f"  Preload enabled: {stats['preload_enabled']}")
        
        # Verify it's within expected bounds
        assert stats['current_size_mb'] < 50  # Should be under 50MB
        assert stats['cached_items'] > 50     # Should have many items


class TestCacheEfficiency:
    """Test cache efficiency metrics."""
    
    def test_cache_hit_rate(self, rm_with_cache):
        """Test that cache is actually being used."""
        # Access same files multiple times
        files = ['classes', 'races', 'feat', 'spells', 'skills']
        
        # First access (cache miss)
        for name in files:
            rm_with_cache.get_2da(name)
        
        # Multiple accesses (should be cache hits)
        import time
        hit_times = []
        for _ in range(100):
            for name in files:
                start = time.perf_counter()
                rm_with_cache.get_2da(name)
                hit_times.append(time.perf_counter() - start)
        
        avg_hit_time = sum(hit_times) / len(hit_times)
        print(f"\nAverage cache hit time: {avg_hit_time * 1000:.4f} ms")
        
        # Cache hits should be very fast
        assert avg_hit_time < 0.0001  # Less than 0.1ms


def test_real_world_scenario(benchmark, tmp_path):
    """Benchmark a real-world scenario with mixed operations."""
    def scenario():
        settings.NWN2_MEMORY_CACHE = True
        settings.NWN2_PRELOAD_2DA = False
        rm = ResourceManager(cache_dir=str(tmp_path))
        
        # Simulate typical usage pattern
        # 1. Load character-related data
        rm.get_2da('classes')
        rm.get_2da('races')
        rm.get_2da('appearance')
        
        # 2. Load feat/spell data
        rm.get_2da('feat')
        rm.get_2da('spells')
        rm.get_2da('iprp_feats')
        
        # 3. Load item-related data
        rm.get_2da('baseitems')
        rm.get_2da('armor')
        rm.get_2da('weaponsounds')
        
        # 4. Access some data again (cache hits)
        rm.get_2da('classes')
        rm.get_2da('feat')
        rm.get_2da('spells')
        
        rm.close()
    
    benchmark(scenario)


if __name__ == '__main__':
    # Run with: pytest tests/test_memory_cache_benchmark.py -v --benchmark-only
    pytest.main([__file__, '-v', '--benchmark-only'])