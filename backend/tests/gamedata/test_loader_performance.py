"""
Performance profiling tests for data model loader
"""
import pytest
import asyncio
import cProfile
import pstats
import time
from pathlib import Path
from io import StringIO

from parsers.resource_manager import ResourceManager
from gamedata.dynamic_loader.data_model_loader import DataModelLoader
from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader
from gamedata.services.game_rules_service import GameRulesService


class PerformanceProfiler:
    """Helper class to profile different loading steps"""
    
    def __init__(self):
        self.timings = {}
        self.profiler = None
        
    def start_profiling(self, name: str):
        """Start profiling a section"""
        self.profiler = cProfile.Profile()
        self.profiler.enable()
        self.timings[name] = {'start': time.perf_counter()}
        
    def stop_profiling(self, name: str):
        """Stop profiling and record results"""
        if self.profiler:
            self.profiler.disable()
            end_time = time.perf_counter()
            self.timings[name]['end'] = end_time
            self.timings[name]['duration'] = end_time - self.timings[name]['start']
            
            # Capture profile stats
            s = StringIO()
            ps = pstats.Stats(self.profiler, stream=s)
            ps.sort_stats('cumulative')
            ps.print_stats(20)  # Top 20 functions
            
            self.timings[name]['profile_stats'] = s.getvalue()
            self.profiler = None
            
    def print_results(self):
        """Print timing results"""
        print("\n=== PERFORMANCE PROFILING RESULTS ===")
        for name, data in self.timings.items():
            print(f"\n{name}: {data['duration']:.3f}s")
            print("Top functions:")
            print(data['profile_stats'])


@pytest.mark.performance
def test_loader_performance_breakdown():
    """Profile each step of the loading process"""
    profiler = PerformanceProfiler()
    
    try:
        # Step 1: ResourceManager creation
        profiler.start_profiling("ResourceManager Creation")
        rm = ResourceManager(suppress_warnings=True)
        profiler.stop_profiling("ResourceManager Creation")
        
        # Step 2: DataModelLoader creation
        profiler.start_profiling("DataModelLoader Creation") 
        loader = DataModelLoader(rm, validate_relationships=False, priority_only=True)
        profiler.stop_profiling("DataModelLoader Creation")
        
        # Step 3: Scan 2DA files
        profiler.start_profiling("Scan 2DA Files")
        tables = asyncio.run(loader._scan_2da_files())
        profiler.stop_profiling("Scan 2DA Files")
        
        # Step 4: Sort tables
        profiler.start_profiling("Sort Tables")
        tables = loader._sort_tables_by_priority(tables)
        profiler.stop_profiling("Sort Tables")
        
        # Step 5: Generate classes
        profiler.start_profiling("Generate Classes")
        asyncio.run(loader._generate_classes(tables))
        profiler.stop_profiling("Generate Classes")
        
        # Step 6: Load table data (the suspected bottleneck)
        profiler.start_profiling("Load Table Data")
        asyncio.run(loader._load_table_data(tables))
        profiler.stop_profiling("Load Table Data")
        
        # Step 7: Finalize data
        profiler.start_profiling("Finalize Data")
        asyncio.run(loader._finalize_data())
        profiler.stop_profiling("Finalize Data")
        
        profiler.print_results()
        
        # Report totals
        total_time = sum(data['duration'] for data in profiler.timings.values())
        print(f"\nTOTAL TIME: {total_time:.3f}s")
        print(f"LOADED TABLES: {len(loader.table_data)}")
        total_rows = sum(len(instances) for instances in loader.table_data.values())
        print(f"TOTAL ROWS: {total_rows}")
        
        # Assert performance targets
        assert total_time < 5.0, f"Loading took {total_time:.3f}s, target is < 5.0s"
        
    finally:
        profiler.print_results()


@pytest.mark.performance
def test_object_creation_overhead():
    """Profile just the object creation part"""
    profiler = PerformanceProfiler()
    
    # Setup
    rm = ResourceManager(suppress_warnings=True)
    loader = DataModelLoader(rm, validate_relationships=False, priority_only=True)
    
    # Get one table for focused testing
    tables = asyncio.run(loader._scan_2da_files())
    asyncio.run(loader._generate_classes(tables))
    
    # Find the largest table for testing
    largest_table = max(tables, key=lambda t: t.get('row_count', 0))
    table_name = largest_table['name']
    table_data = largest_table['data']
    data_class = loader.generated_classes[table_name]
    
    print(f"\nTesting object creation for table: {table_name}")
    print(f"Row count: {largest_table.get('row_count', 0)}")
    
    # Profile raw data extraction
    profiler.start_profiling("Extract Row Data")
    row_count = table_data.get_resource_count() if hasattr(table_data, 'get_resource_count') else 0
    row_data_list = []
    for row_id in range(row_count):
        if hasattr(table_data, 'get_row_dict'):
            row_dict = table_data.get_row_dict(row_id)
            if row_dict:
                row_data_list.append(row_dict)
    profiler.stop_profiling("Extract Row Data")
    
    # Profile object creation
    profiler.start_profiling("Create Objects")
    instances = [data_class(_resource_manager=rm, **row_dict) for row_dict in row_data_list]
    profiler.stop_profiling("Create Objects")
    
    profiler.print_results()
    
    # Calculate per-object time
    create_time = profiler.timings["Create Objects"]["duration"]
    per_object_ms = (create_time / len(instances)) * 1000
    print(f"\nPer-object creation time: {per_object_ms:.3f}ms")
    print(f"Created {len(instances)} objects in {create_time:.3f}s")


@pytest.mark.performance  
def test_string_resolution_overhead():
    """Profile string resolution specifically"""
    profiler = PerformanceProfiler()
    
    # Setup minimal test
    rm = ResourceManager(suppress_warnings=True)
    
    # Test string resolution performance
    profiler.start_profiling("String Resolution Test")
    
    # Simulate resolving common string IDs
    test_ids = [1, 2, 3, 16777216, 16777217, 16777218] * 1000
    resolved_strings = []
    
    for string_id in test_ids:
        resolved = rm.get_string(string_id)
        resolved_strings.append(resolved)
        
    profiler.stop_profiling("String Resolution Test")
    
    profiler.print_results()
    
    resolve_time = profiler.timings["String Resolution Test"]["duration"]
    per_resolve_us = (resolve_time / len(test_ids)) * 1000000
    print(f"\nPer-string resolution time: {per_resolve_us:.1f}μs")
    print(f"Resolved {len(test_ids)} strings in {resolve_time:.3f}s")


@pytest.mark.performance
def test_current_vs_optimized():
    """Compare current implementation vs our optimizations"""
    
    # Test current implementation
    start_time = time.perf_counter()
    rm = ResourceManager(suppress_warnings=True)
    loader = DynamicGameDataLoader(rm, use_async=False, priority_only=True)
    current_time = time.perf_counter() - start_time
    
    print(f"\nCurrent implementation: {current_time:.3f}s")
    print(f"Tables loaded: {len(loader.table_data)}")
    total_rows = sum(len(instances) for instances in loader.table_data.values())
    print(f"Total rows: {total_rows}")
    
    # Performance assertions
    print(f"\nPerformance target: < 0.5s")
    print(f"Current performance: {current_time:.3f}s")
    print(f"Improvement needed: {((current_time - 0.5) / current_time * 100):.1f}%")


if __name__ == "__main__":
    # Run the profiling tests
    test_loader_performance_breakdown()
    test_object_creation_overhead() 
    test_string_resolution_overhead()
    test_current_vs_optimized()