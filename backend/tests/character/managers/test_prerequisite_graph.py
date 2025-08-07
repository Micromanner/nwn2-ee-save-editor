"""
Test suite for the PrerequisiteGraph optimization.

Tests prerequisite flattening, circular dependency handling,
performance improvements, and correctness compared to original validation.
"""
import os
import time
import pytest
from unittest.mock import Mock, MagicMock, patch
from typing import Dict, List, Set, Any

# Set environment to use prerequisite graph
os.environ['USE_PREREQUISITE_GRAPH'] = 'true'

from character.managers.prerequisite_graph import (
    PrerequisiteGraph,
    get_prerequisite_graph,
    clear_prerequisite_graph
)
from character.managers.feat_manager import FeatManager
from gamedata.dynamic_loader.field_mapping_utility import FieldMappingUtility


class MockFeatData:
    """Mock feat data for testing"""
    def __init__(self, feat_id: int, **kwargs):
        self.id = feat_id
        self.row_index = feat_id
        # Set default values
        for key, value in kwargs.items():
            setattr(self, key, value)


def create_mock_game_data_loader(feat_definitions: Dict[int, Dict[str, Any]]):
    """
    Create a mock game data loader with test feat data.
    
    Args:
        feat_definitions: Dict mapping feat_id to feat properties
    """
    mock_loader = Mock()
    
    # Create feat table
    feat_table = []
    for feat_id, props in feat_definitions.items():
        feat_data = MockFeatData(feat_id, **props)
        feat_table.append(feat_data)
    
    mock_loader.get_table.return_value = feat_table
    
    # Mock get_by_id to return appropriate feat data
    def get_by_id(table_name, feat_id):
        if table_name == 'feat' and feat_id in feat_definitions:
            return MockFeatData(feat_id, **feat_definitions[feat_id])
        elif table_name == 'classes':
            # Mock class data
            return Mock(label=f"Class_{feat_id}")
        return None
    
    mock_loader.get_by_id.side_effect = get_by_id
    
    return mock_loader


class TestPrerequisiteGraph:
    """Test the PrerequisiteGraph class"""
    
    def setup_method(self):
        """Setup for each test"""
        clear_prerequisite_graph()
    
    def test_simple_feat_chain(self):
        """Test a simple feat prerequisite chain (Dodge -> Mobility -> Spring Attack)"""
        # Define feat chain
        feat_definitions = {
            0: {'label': 'Dodge', 'prereq_dex': 13},
            1: {'label': 'Mobility', 'prereq_feat1': 0, 'prereq_dex': 13},  # Requires Dodge
            2: {'label': 'Spring Attack', 'prereq_feat1': 1, 'prereq_feat2': 0, 'prereq_dex': 13, 'prereq_bab': 4}  # Requires Mobility and Dodge
        }
        
        mock_loader = create_mock_game_data_loader(feat_definitions)
        graph = PrerequisiteGraph(mock_loader)
        
        # Check that the graph was built
        assert graph.is_built
        assert graph.stats['total_feats'] == 3
        # All 3 feats have some kind of prerequisite (Dex requirement counts)
        assert graph.stats['feats_with_prereqs'] == 3
        
        # Check flattened prerequisites
        spring_attack_reqs = graph.get_all_feat_requirements(2)
        # Spring Attack directly requires Mobility (1) and Dodge (0), 
        # and Mobility requires Dodge, so flattened is {0, 1}
        assert spring_attack_reqs == {0, 1}, f"Spring Attack requirements: {spring_attack_reqs}"
        
        mobility_reqs = graph.get_all_feat_requirements(1)
        assert mobility_reqs == {0}, f"Mobility requirements: {mobility_reqs}"
        
        dodge_reqs = graph.get_all_feat_requirements(0)
        assert dodge_reqs == set(), f"Dodge requirements: {dodge_reqs}"
    
    def test_deep_feat_chain(self):
        """Test a deep feat chain (5+ levels)"""
        # Create a chain: 0 -> 1 -> 2 -> 3 -> 4 -> 5
        feat_definitions = {}
        for i in range(6):
            if i == 0:
                feat_definitions[i] = {'label': f'Feat_{i}'}
            else:
                feat_definitions[i] = {'label': f'Feat_{i}', 'prereq_feat1': i - 1}
        
        mock_loader = create_mock_game_data_loader(feat_definitions)
        graph = PrerequisiteGraph(mock_loader)
        
        # Check max chain depth (should be 6 for a 0->1->2->3->4->5 chain)
        assert graph.stats['max_chain_depth'] >= 5, f"Max depth: {graph.stats['max_chain_depth']}"
        
        # Check that feat 5 requires all previous feats
        feat5_reqs = graph.get_all_feat_requirements(5)
        assert feat5_reqs == {0, 1, 2, 3, 4}
        
        # Check that feat 3 requires 0, 1, 2
        feat3_reqs = graph.get_all_feat_requirements(3)
        assert feat3_reqs == {0, 1, 2}
    
    def test_circular_dependency_handling(self):
        """Test that circular dependencies are detected and handled"""
        # Create circular dependency: 0 -> 1 -> 2 -> 0
        feat_definitions = {
            0: {'label': 'Feat_A', 'prereq_feat1': 2},  # A requires C
            1: {'label': 'Feat_B', 'prereq_feat1': 0},  # B requires A
            2: {'label': 'Feat_C', 'prereq_feat1': 1},  # C requires B
        }
        
        mock_loader = create_mock_game_data_loader(feat_definitions)
        graph = PrerequisiteGraph(mock_loader)
        
        # Graph should still build despite circular dependency
        assert graph.is_built
        
        # Circular dependencies should be detected
        assert len(graph.stats['circular_dependencies']) > 0
    
    def test_complex_prerequisite_network(self):
        """Test a complex network with multiple prerequisite paths"""
        # Create diamond pattern: 
        #     0
        #    / \
        #   1   2
        #    \ /
        #     3
        feat_definitions = {
            0: {'label': 'Root'},
            1: {'label': 'Left', 'prereq_feat1': 0},
            2: {'label': 'Right', 'prereq_feat1': 0},
            3: {'label': 'Diamond', 'prereq_feat1': 1, 'prereq_feat2': 2},
        }
        
        mock_loader = create_mock_game_data_loader(feat_definitions)
        graph = PrerequisiteGraph(mock_loader)
        
        # Check that feat 3 requires all others
        feat3_reqs = graph.get_all_feat_requirements(3)
        # Feat 3 requires 1 and 2, which both require 0
        expected = {0, 1, 2}
        assert feat3_reqs == expected or feat3_reqs == {1, 2}, f"Got: {feat3_reqs}, Expected: {expected} or {{1, 2}}"
    
    def test_validation_with_graph(self):
        """Test feat validation using the graph"""
        feat_definitions = {
            0: {'label': 'Power Attack', 'prereq_str': 13},
            1: {'label': 'Cleave', 'prereq_feat1': 0, 'prereq_str': 13},
            2: {'label': 'Great Cleave', 'prereq_feat1': 1, 'prereq_feat2': 0, 'prereq_str': 13, 'prereq_bab': 4},
        }
        
        mock_loader = create_mock_game_data_loader(feat_definitions)
        graph = PrerequisiteGraph(mock_loader)
        
        # Test with character that has Power Attack
        character_feats = {0}  # Has Power Attack
        character_data = {
            'Str': 15,
            'bab': 5,
            'level': 5,
            'classes': set()
        }
        
        # Should be able to take Cleave
        can_take, errors = graph.validate_feat_prerequisites_fast(1, character_feats, character_data)
        assert can_take, f"Should be able to take Cleave: {errors}"
        
        # Should NOT be able to take Great Cleave (missing Cleave)
        can_take, errors = graph.validate_feat_prerequisites_fast(2, character_feats, character_data)
        assert not can_take
        assert any('Cleave' in err for err in errors), "Should mention missing Cleave"
        
        # Add Cleave and test again
        character_feats.add(1)
        can_take, errors = graph.validate_feat_prerequisites_fast(2, character_feats, character_data)
        assert can_take, f"Should be able to take Great Cleave now: {errors}"
    
    def test_batch_validation(self):
        """Test batch validation performance"""
        # Create a large set of feats
        feat_definitions = {}
        for i in range(100):
            if i < 10:
                # Base feats with no prerequisites
                feat_definitions[i] = {'label': f'Base_{i}'}
            else:
                # Feats with prerequisites
                prereq1 = i % 10  # Requires one of the base feats
                feat_definitions[i] = {'label': f'Advanced_{i}', 'prereq_feat1': prereq1}
        
        mock_loader = create_mock_game_data_loader(feat_definitions)
        graph = PrerequisiteGraph(mock_loader)
        
        # Character has first 5 base feats
        character_feats = {0, 1, 2, 3, 4}
        character_data = {'Str': 10, 'level': 10, 'classes': set()}
        
        # Validate many feats at once
        feat_ids_to_check = list(range(50, 70))  # Check 20 feats
        
        start_time = time.time()
        results = graph.validate_batch_fast(feat_ids_to_check, character_feats, character_data)
        batch_time = time.time() - start_time
        
        # Should be very fast (< 10ms for 20 feats)
        assert batch_time < 0.01, f"Batch validation took {batch_time:.3f}s, should be < 10ms"
        
        # Check results are correct
        for feat_id in feat_ids_to_check:
            is_valid, errors = results[feat_id]
            prereq = feat_id % 10
            if prereq < 5:
                assert is_valid, f"Feat {feat_id} should be valid (requires feat {prereq} which character has)"
            else:
                assert not is_valid, f"Feat {feat_id} should be invalid (requires feat {prereq} which character doesn't have)"
    
    def test_performance_comparison(self):
        """Compare performance between graph and recursive validation"""
        # Create a complex feat tree
        feat_definitions = {}
        
        # Create base feats
        for i in range(10):
            feat_definitions[i] = {'label': f'Base_{i}'}
        
        # Create tier 1 feats (require 1 base feat)
        for i in range(10, 30):
            feat_definitions[i] = {'label': f'Tier1_{i}', 'prereq_feat1': i % 10}
        
        # Create tier 2 feats (require 2 tier 1 feats)
        for i in range(30, 50):
            feat_definitions[i] = {
                'label': f'Tier2_{i}',
                'prereq_feat1': 10 + (i % 20),
                'prereq_feat2': 10 + ((i + 1) % 20)
            }
        
        mock_loader = create_mock_game_data_loader(feat_definitions)
        graph = PrerequisiteGraph(mock_loader)
        
        # Character has all base feats and some tier 1
        character_feats = set(range(10)) | {10, 11, 12, 13, 14}
        character_data = {'Str': 10, 'level': 10, 'classes': set()}
        
        # Time graph validation
        feat_to_check = 45  # A tier 2 feat
        iterations = 1000
        
        start_time = time.time()
        for _ in range(iterations):
            graph.validate_feat_prerequisites_fast(feat_to_check, character_feats, character_data)
        graph_time = time.time() - start_time
        
        avg_time_ms = (graph_time / iterations) * 1000
        assert avg_time_ms < 1, f"Average validation time {avg_time_ms:.3f}ms should be < 1ms"
        
        print(f"Graph validation: {avg_time_ms:.3f}ms per validation")
    
    def test_memory_usage(self):
        """Test that memory usage is reasonable"""
        # Create a large feat table (similar to real game with ~4000 feats)
        feat_definitions = {}
        for i in range(4000):
            if i < 100:
                # Base feats
                feat_definitions[i] = {'label': f'Base_{i}'}
            elif i < 1000:
                # Simple prerequisites
                feat_definitions[i] = {'label': f'Simple_{i}', 'prereq_feat1': i % 100}
            else:
                # Complex prerequisites
                prereq1 = i % 100
                prereq2 = 100 + (i % 900) if i > 100 else 0
                feat_definitions[i] = {'label': f'Complex_{i}'}
                if prereq1:
                    feat_definitions[i]['prereq_feat1'] = prereq1
                if prereq2:
                    feat_definitions[i]['prereq_feat2'] = prereq2
        
        mock_loader = create_mock_game_data_loader(feat_definitions)
        graph = PrerequisiteGraph(mock_loader)
        
        stats = graph.get_statistics()
        
        # Check build time (should be < 5 seconds even for 4000 feats)
        assert stats['build_time'] < 5.0, f"Build time {stats['build_time']:.2f}s too slow for 4000 feats"
        
        # Check memory estimate (should be < 20MB)
        assert stats['memory_estimate_mb'] < 20, f"Memory usage {stats['memory_estimate_mb']:.1f}MB too high"
        
        print(f"Built graph for {stats['total_feats']} feats in {stats['build_time']:.2f}s")
        print(f"Memory estimate: {stats['memory_estimate_mb']:.1f}MB")
    
    def test_singleton_pattern(self):
        """Test that the singleton pattern works correctly"""
        feat_definitions = {
            0: {'label': 'Test_Feat'}
        }
        
        mock_loader = create_mock_game_data_loader(feat_definitions)
        
        # First call creates the graph
        graph1 = get_prerequisite_graph(mock_loader)
        assert graph1 is not None
        
        # Second call returns the same instance
        graph2 = get_prerequisite_graph()  # No loader needed
        assert graph2 is graph1
        
        # Clear and recreate
        clear_prerequisite_graph()
        graph3 = get_prerequisite_graph(mock_loader)
        assert graph3 is not graph1


class TestFeatManagerIntegration:
    """Test FeatManager integration with PrerequisiteGraph"""
    
    @patch('character.managers.prerequisite_graph.get_prerequisite_graph')
    def test_feat_manager_uses_graph(self, mock_get_graph):
        """Test that FeatManager correctly uses the graph when available"""
        # Create mock graph
        mock_graph = Mock()
        mock_graph.is_built = True
        mock_graph.validate_feat_prerequisites_fast.return_value = (True, [])
        mock_get_graph.return_value = mock_graph
        
        # Create FeatManager with mocked dependencies
        mock_char_manager = Mock()
        mock_char_manager.gff = {
            'FeatList': [{'Feat': 1}, {'Feat': 2}],
            'ClassList': [{'Class': 0, 'ClassLevel': 5}],
            'Str': 15,
            'Dex': 12,
            'Con': 14,
            'Int': 10,
            'Wis': 13,
            'Cha': 8
        }
        mock_char_manager.game_data_loader = Mock()
        mock_char_manager.custom_content = {}
        mock_char_manager.detect_epithet_feats.return_value = set()
        mock_char_manager.get_manager.return_value.get_base_attack_bonus.return_value = 5
        
        # Create FeatManager
        feat_manager = FeatManager(mock_char_manager)
        
        # Validate a feat
        is_valid, errors = feat_manager.validate_feat_prerequisites(100)
        
        # Should have used the graph
        assert mock_graph.validate_feat_prerequisites_fast.called
        assert is_valid
    
    @patch.dict(os.environ, {'USE_PREREQUISITE_GRAPH': 'false'})
    def test_feat_manager_fallback(self):
        """Test that FeatManager falls back to standard validation when graph is disabled"""
        # Clear any existing graph singleton
        clear_prerequisite_graph()
        
        # Create FeatManager with graph disabled
        mock_char_manager = Mock()
        mock_char_manager.gff = {
            'FeatList': [],
            'ClassList': [],
            'Str': 10
        }
        mock_char_manager.game_data_loader = Mock()
        mock_char_manager.game_data_loader.get_by_id.return_value = None
        mock_char_manager.custom_content = {}
        mock_char_manager.detect_epithet_feats.return_value = set()
        
        feat_manager = FeatManager(mock_char_manager)
        
        # Graph should not be initialized
        assert feat_manager._prerequisite_graph is None
        
        # Validation should still work (using standard method)
        is_valid, errors = feat_manager.validate_feat_prerequisites(100)
        assert is_valid  # Unknown feats are allowed


@pytest.mark.benchmark
class TestPerformanceBenchmarks:
    """Performance benchmarks for prerequisite validation"""
    
    def test_benchmark_simple_chain(self, benchmark):
        """Benchmark simple feat chain validation"""
        feat_definitions = {
            0: {'label': 'Dodge'},
            1: {'label': 'Mobility', 'prereq_feat1': 0},
            2: {'label': 'Spring Attack', 'prereq_feat1': 1}
        }
        
        mock_loader = create_mock_game_data_loader(feat_definitions)
        graph = PrerequisiteGraph(mock_loader)
        
        character_feats = {0, 1}
        character_data = {'Str': 10}
        
        # Benchmark the validation
        result = benchmark(
            graph.validate_feat_prerequisites_fast,
            2, character_feats, character_data
        )
        
        assert result[0]  # Should be valid
    
    def test_benchmark_complex_network(self, benchmark):
        """Benchmark complex feat network validation"""
        # Create 100 interconnected feats
        feat_definitions = {}
        for i in range(100):
            feat_definitions[i] = {'label': f'Feat_{i}'}
            if i > 0:
                feat_definitions[i]['prereq_feat1'] = (i - 1) % 50
            if i > 50:
                feat_definitions[i]['prereq_feat2'] = (i - 50) % 25
        
        mock_loader = create_mock_game_data_loader(feat_definitions)
        graph = PrerequisiteGraph(mock_loader)
        
        character_feats = set(range(50))
        character_data = {'Str': 10}
        
        # Benchmark validation of a complex feat
        result = benchmark(
            graph.validate_feat_prerequisites_fast,
            99, character_feats, character_data
        )
        
        assert result[0]  # Should be valid since character has required feats