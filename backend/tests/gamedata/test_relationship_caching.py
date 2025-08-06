"""
Tests for relationship caching functionality
"""
import pytest
import json
import tempfile
from pathlib import Path
from datetime import datetime
from unittest.mock import Mock, patch

from gamedata.dynamic_loader.code_cache import SecureCodeCache
from gamedata.dynamic_loader.relationship_validator import (
    RelationshipDefinition, RelationshipType, ValidationReport
)


class TestRelationshipCaching:
    """Test caching of relationship data."""
    
    @pytest.fixture
    def temp_cache_dir(self):
        """Create a temporary directory for cache testing."""
        with tempfile.TemporaryDirectory() as tmpdir:
            yield Path(tmpdir)
    
    @pytest.fixture
    def cache(self, temp_cache_dir):
        """Create a SecureCodeCache instance with temp directory."""
        return SecureCodeCache(temp_cache_dir)
    
    @pytest.fixture
    def sample_relationships(self):
        """Create sample relationships for testing."""
        return {
            RelationshipDefinition(
                source_table="feat",
                source_column="PREREQFEAT1",
                target_table="feat",
                relationship_type=RelationshipType.LOOKUP
            ),
            RelationshipDefinition(
                source_table="classes",
                source_column="FeatsTable",
                target_table="dynamic",
                relationship_type=RelationshipType.TABLE_REFERENCE
            ),
            RelationshipDefinition(
                source_table="racialtypes",
                source_column="FavoredClass",
                target_table="classes",
                relationship_type=RelationshipType.LOOKUP
            )
        }
    
    @pytest.fixture
    def sample_validation_report(self):
        """Create a sample validation report."""
        report = ValidationReport()
        report.total_relationships = 3
        report.valid_relationships = 2
        report.broken_references = [
            {
                'source_table': 'feat',
                'source_column': 'PREREQ',
                'source_row': 10,
                'target_table': 'feat',
                'target_id': 999,
                'error': 'Invalid reference'
            }
        ]
        report.missing_tables = {'custom_table'}
        report.dependency_order = ['classes', 'feat', 'racialtypes']
        return report
    
    def test_save_relationships(self, cache, sample_relationships, sample_validation_report):
        """Test saving relationships to cache."""
        cache.save_relationships(sample_relationships, sample_validation_report)
        
        # Check that file was created
        relationships_file = cache.cache_dir / "relationships.json"
        assert relationships_file.exists()
        
        # Load and verify content
        with open(relationships_file, 'r') as f:
            data = json.load(f)
        
        assert 'relationships' in data
        assert 'validation_report' in data
        assert 'generated_at' in data
        
        # Check relationships
        assert len(data['relationships']) == 3
        
        # Check that all relationship types are preserved
        rel_types = {r['relationship_type'] for r in data['relationships']}
        assert 'lookup' in rel_types
        assert 'table_reference' in rel_types
        
        # Check validation report
        report_data = data['validation_report']
        assert report_data['total_relationships'] == 3
        assert report_data['valid_relationships'] == 2
        assert len(report_data['broken_references']) == 1
        assert 'custom_table' in report_data['missing_tables']
        assert report_data['dependency_order'] == ['classes', 'feat', 'racialtypes']
    
    def test_load_relationships(self, cache, sample_relationships, sample_validation_report):
        """Test loading relationships from cache."""
        # First save
        cache.save_relationships(sample_relationships, sample_validation_report)
        
        # Then load
        loaded_data = cache.load_relationships()
        
        assert loaded_data is not None
        assert 'relationships' in loaded_data
        assert 'validation_report' in loaded_data
        
        # Verify loaded data matches saved data
        assert len(loaded_data['relationships']) == 3
        assert loaded_data['validation_report']['total_relationships'] == 3
    
    def test_load_missing_relationships(self, cache):
        """Test loading when no cached relationships exist."""
        loaded_data = cache.load_relationships()
        assert loaded_data is None
    
    def test_load_corrupted_relationships(self, cache):
        """Test loading when cache file is corrupted."""
        # Create corrupted file
        relationships_file = cache.cache_dir / "relationships.json"
        relationships_file.write_text("invalid json {]}")
        
        # Should handle gracefully
        loaded_data = cache.load_relationships()
        assert loaded_data is None
    
    def test_relationships_hash(self, cache):
        """Test generating hash for table structure."""
        # Create mock table data
        class MockInstance:
            def get_safe_columns(self):
                return ['id', 'name', 'value']
        
        table_data = {
            'table1': [MockInstance(), MockInstance()],
            'table2': [MockInstance()],
            'table3': []
        }
        
        hash1 = cache.get_relationships_hash(table_data)
        assert hash1 != ""
        assert len(hash1) == 64  # SHA256 hex string
        
        # Same data should produce same hash
        hash2 = cache.get_relationships_hash(table_data)
        assert hash1 == hash2
        
        # Different data should produce different hash
        table_data['table4'] = [MockInstance()]
        hash3 = cache.get_relationships_hash(table_data)
        assert hash3 != hash1
    
    def test_save_with_table_structure_hash(self, cache, sample_relationships, sample_validation_report):
        """Test that table structure hash is saved with relationships."""
        # Create mock table data
        class MockInstance:
            def get_safe_columns(self):
                return ['id', 'name']
        
        table_data = {
            'feat': [MockInstance()],
            'classes': [MockInstance()]
        }
        
        # Save relationships
        cache.save_relationships(sample_relationships, sample_validation_report)
        
        # Manually add hash (simulating what DataModelLoader does)
        relationships_file = cache.cache_dir / "relationships.json"
        with open(relationships_file, 'r+') as f:
            data = json.load(f)
            data['table_structure_hash'] = cache.get_relationships_hash(table_data)
            f.seek(0)
            json.dump(data, f, indent=2)
            f.truncate()
        
        # Load and verify hash is present
        loaded_data = cache.load_relationships()
        assert 'table_structure_hash' in loaded_data
        assert loaded_data['table_structure_hash'] == cache.get_relationships_hash(table_data)
    
    def test_broken_references_limit(self, cache, sample_relationships):
        """Test that broken references are limited to prevent huge cache files."""
        # Create report with many broken references
        report = ValidationReport()
        report.total_relationships = 1
        report.valid_relationships = 0
        
        # Add 200 broken references
        for i in range(200):
            report.add_broken_reference(
                source_table='test',
                source_column='col',
                source_row=i,
                target_table='target',
                target_id=i
            )
        
        # Save
        cache.save_relationships(sample_relationships, report)
        
        # Load and check that only first 100 are saved
        loaded_data = cache.load_relationships()
        assert len(loaded_data['validation_report']['broken_references']) == 100
    
    def test_cache_performance(self, cache, sample_relationships, sample_validation_report):
        """Test that caching improves performance."""
        import time
        
        # Time saving
        start_time = time.time()
        cache.save_relationships(sample_relationships, sample_validation_report)
        save_time = time.time() - start_time
        
        # Time loading
        start_time = time.time()
        loaded_data = cache.load_relationships()
        load_time = time.time() - start_time
        
        # Loading should be fast
        assert load_time < 0.1  # Should load in under 100ms
        assert loaded_data is not None
    
    def test_cache_with_special_characters(self, cache):
        """Test caching with special characters in table/column names."""
        # Create relationships with special characters
        relationships = {
            RelationshipDefinition(
                source_table="table-with-dash",
                source_column="Column With Spaces",
                target_table="table_with_underscore",
                relationship_type=RelationshipType.LOOKUP
            )
        }
        
        report = ValidationReport()
        report.total_relationships = 1
        report.valid_relationships = 1
        
        # Should handle special characters gracefully
        cache.save_relationships(relationships, report)
        loaded_data = cache.load_relationships()
        
        assert loaded_data is not None
        assert len(loaded_data['relationships']) == 1
        rel = loaded_data['relationships'][0]
        assert rel['source_table'] == "table-with-dash"
        assert rel['source_column'] == "Column With Spaces"