"""
Tests for the Relationship Validator system
"""
import pytest
from unittest.mock import Mock, MagicMock, patch
from pathlib import Path

from gamedata.dynamic_loader.relationship_validator import (
    RelationshipValidator, RelationshipDefinition, RelationshipType, ValidationReport
)
from gamedata.services.rule_detector import RuleDetector


class TestRelationshipDefinition:
    """Test the RelationshipDefinition dataclass."""
    
    def test_relationship_creation(self):
        """Test creating a relationship definition."""
        rel = RelationshipDefinition(
            source_table="feat",
            source_column="PREREQFEAT1",
            target_table="feat",
            relationship_type=RelationshipType.LOOKUP
        )
        
        assert rel.source_table == "feat"
        assert rel.source_column == "PREREQFEAT1"
        assert rel.target_table == "feat"
        assert rel.relationship_type == RelationshipType.LOOKUP
        assert rel.is_nullable is True
        assert rel.validation_errors == []
    
    def test_relationship_hash(self):
        """Test that relationships are hashable for use in sets."""
        rel1 = RelationshipDefinition(
            source_table="classes",
            source_column="FavoredClass",
            target_table="classes",
            relationship_type=RelationshipType.LOOKUP
        )
        
        rel2 = RelationshipDefinition(
            source_table="classes",
            source_column="FavoredClass",
            target_table="classes",
            relationship_type=RelationshipType.LOOKUP
        )
        
        # Same relationships should have same hash
        assert hash(rel1) == hash(rel2)
        
        # Should work in sets
        rel_set = {rel1, rel2}
        assert len(rel_set) == 1
    
    def test_relationship_string_representation(self):
        """Test string representation of relationship."""
        rel = RelationshipDefinition(
            source_table="baseitems",
            source_column="WeaponType",
            target_table="weapontypes",
            relationship_type=RelationshipType.LOOKUP
        )
        
        assert str(rel) == "baseitems.WeaponType -> weapontypes"


class TestValidationReport:
    """Test the ValidationReport dataclass."""
    
    def test_report_creation(self):
        """Test creating a validation report."""
        report = ValidationReport()
        
        assert report.total_relationships == 0
        assert report.valid_relationships == 0
        assert report.broken_references == []
        assert report.missing_tables == set()
        assert report.dependency_order == []
    
    def test_add_broken_reference(self):
        """Test adding broken references to report."""
        report = ValidationReport()
        
        report.add_broken_reference(
            source_table="feat",
            source_column="PREREQFEAT1",
            source_row=10,
            target_table="feat",
            target_id=999
        )
        
        assert len(report.broken_references) == 1
        ref = report.broken_references[0]
        assert ref['source_table'] == "feat"
        assert ref['source_column'] == "PREREQFEAT1"
        assert ref['source_row'] == 10
        assert ref['target_table'] == "feat"
        assert ref['target_id'] == 999
        assert "Row 10" in ref['error']
    
    def test_report_summary(self):
        """Test generating report summary."""
        report = ValidationReport()
        report.total_relationships = 10
        report.valid_relationships = 8
        report.missing_tables.add("custom_table")
        report.add_broken_reference("feat", "PREREQ", 1, "feat", 99)
        
        summary = report.get_summary()
        
        assert "Total relationships: 10" in summary
        assert "Valid relationships: 8" in summary
        assert "Broken references: 1" in summary
        assert "Missing tables: 1" in summary
        assert "custom_table" in summary


class TestRelationshipValidator:
    """Test the RelationshipValidator class."""
    
    @pytest.fixture
    def mock_rule_detector(self):
        """Create a mock rule detector."""
        detector = Mock(spec=RuleDetector)
        detector.get_column_purpose = Mock(return_value=None)
        return detector
    
    @pytest.fixture
    def sample_table_data(self):
        """Create sample table data for testing."""
        # Mock data instances
        class MockInstance:
            def __init__(self, **kwargs):
                self._data = kwargs
                self._safe_columns = list(kwargs.keys())
            
            def get_safe_columns(self):
                return self._safe_columns
            
            def __getattr__(self, name):
                return self._data.get(name)
        
        # Create sample data
        classes_data = [
            MockInstance(id=0, label="Fighter", FeatsTable="cls_feat_fight"),
            MockInstance(id=1, label="Wizard", FeatsTable="cls_feat_wiz"),
        ]
        
        feat_data = [
            MockInstance(id=0, label="Power Attack", PREREQFEAT1="****"),
            MockInstance(id=1, label="Cleave", PREREQFEAT1=0),
            MockInstance(id=2, label="Great Cleave", PREREQFEAT1=1),
        ]
        
        races_data = [
            MockInstance(id=0, label="Human", FavoredClass=-1),
            MockInstance(id=1, label="Elf", FavoredClass=1),  # Wizard
        ]
        
        return {
            'classes': classes_data,
            'feat': feat_data,
            'racialtypes': races_data,
            'cls_feat_fight': [],
            'cls_feat_wiz': []
        }
    
    def test_validator_initialization(self, mock_rule_detector):
        """Test initializing the validator."""
        validator = RelationshipValidator(mock_rule_detector)
        
        assert validator.rule_detector == mock_rule_detector
        assert len(validator.relationships) == 0
        assert len(validator.table_data) == 0
    
    def test_detect_relationships(self, mock_rule_detector, sample_table_data):
        """Test detecting relationships in table data."""
        validator = RelationshipValidator(mock_rule_detector)
        
        relationships = validator.detect_relationships(sample_table_data)
        
        # Should detect several relationships
        assert len(relationships) > 0
        
        # Check for specific expected relationships
        rel_strings = {str(r) for r in relationships}
        
        # Classes -> table references
        assert any("classes.FeatsTable" in s for s in rel_strings)
        
        # Feat prerequisites
        assert any("feat.PREREQFEAT1 -> feat" in s for s in rel_strings)
        
        # Race favored class
        assert any("racialtypes.FavoredClass -> classes" in s for s in rel_strings)
    
    def test_validate_relationships(self, mock_rule_detector, sample_table_data):
        """Test validating detected relationships."""
        validator = RelationshipValidator(mock_rule_detector)
        validator.detect_relationships(sample_table_data)
        
        report = validator.validate_relationships()
        
        assert report.total_relationships > 0
        assert report.valid_relationships > 0
        
        # Should have dependency order
        assert len(report.dependency_order) > 0
        
        # Classes should come before tables that reference them
        if 'classes' in report.dependency_order and 'racialtypes' in report.dependency_order:
            assert report.dependency_order.index('classes') < report.dependency_order.index('racialtypes')
    
    def test_detect_broken_references(self, mock_rule_detector):
        """Test detecting broken references."""
        # Create data with broken reference
        class MockInstance:
            def __init__(self, **kwargs):
                self._data = kwargs
                self._safe_columns = list(kwargs.keys())
            
            def get_safe_columns(self):
                return self._safe_columns
            
            def __getattr__(self, name):
                return self._data.get(name)
        
        table_data = {
            'feat': [
                MockInstance(id=0, label="Test Feat", PREREQFEAT1=999)  # Invalid reference
            ]
        }
        
        validator = RelationshipValidator(mock_rule_detector)
        validator.detect_relationships(table_data)
        
        report = validator.validate_relationships()
        
        # Should have broken references
        assert len(report.broken_references) > 0
        assert any(ref['target_id'] == 999 for ref in report.broken_references)
    
    def test_table_dependencies(self, mock_rule_detector, sample_table_data):
        """Test getting table dependencies."""
        validator = RelationshipValidator(mock_rule_detector)
        validator.detect_relationships(sample_table_data)
        
        # Get dependencies for racialtypes
        deps = validator.get_table_dependencies('racialtypes')
        assert 'classes' in deps
        
        # Get dependents of classes
        dependents = validator.get_table_dependents('classes')
        assert 'racialtypes' in dependents
    
    def test_generate_dot_graph(self, mock_rule_detector, sample_table_data):
        """Test generating Graphviz DOT output."""
        validator = RelationshipValidator(mock_rule_detector)
        validator.detect_relationships(sample_table_data)
        
        dot_output = validator.generate_dot_graph()
        
        # Should be valid DOT format
        assert dot_output.startswith("digraph TableRelationships {")
        assert dot_output.endswith("}")
        
        # Should contain nodes
        assert '"classes"' in dot_output
        assert '"feat"' in dot_output
        
        # Should contain edges
        assert '->' in dot_output
        assert 'label=' in dot_output
    
    def test_null_value_handling(self, mock_rule_detector):
        """Test that null values (****) are properly ignored."""
        class MockInstance:
            def __init__(self, **kwargs):
                self._data = kwargs
                self._safe_columns = list(kwargs.keys())
            
            def get_safe_columns(self):
                return self._safe_columns
            
            def __getattr__(self, name):
                return self._data.get(name)
        
        table_data = {
            'feat': [
                MockInstance(id=0, label="Test", PREREQFEAT1="****"),
                MockInstance(id=1, label="Test2", PREREQFEAT1=None),
            ]
        }
        
        validator = RelationshipValidator(mock_rule_detector)
        validator.detect_relationships(table_data)
        
        report = validator.validate_relationships()
        
        # Null values should not create broken references
        assert len(report.broken_references) == 0
    
    def test_rule_detector_integration(self):
        """Test integration with actual RuleDetector patterns."""
        # Create a real RuleDetector mock with proper patterns
        detector = Mock(spec=RuleDetector)
        
        def mock_get_column_purpose(table_name, column_name):
            if column_name == "FeatsTable":
                return "feats_table"
            elif column_name == "SPELLID":
                return "spell_id"
            elif column_name == "FavoredClass":
                return "favored_class"
            return None
        
        detector.get_column_purpose = Mock(side_effect=mock_get_column_purpose)
        
        # Create test data
        class MockInstance:
            def __init__(self, **kwargs):
                self._data = kwargs
                self._safe_columns = list(kwargs.keys())
            
            def get_safe_columns(self):
                return self._safe_columns
            
            def __getattr__(self, name):
                return self._data.get(name)
        
        table_data = {
            'classes': [MockInstance(id=0, FeatsTable="cls_feat_fight")],
            'feat': [MockInstance(id=0, SPELLID=100)],
            'racialtypes': [MockInstance(id=0, FavoredClass=0)],
            'spells': [MockInstance(id=100, label="Magic Missile")],
        }
        
        validator = RelationshipValidator(detector)
        relationships = validator.detect_relationships(table_data)
        
        # Should use rule detector patterns
        rel_strings = {str(r) for r in relationships}
        assert any("feat.SPELLID -> spells" in s for s in rel_strings)