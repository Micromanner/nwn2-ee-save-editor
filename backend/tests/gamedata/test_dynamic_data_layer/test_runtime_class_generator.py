"""
Tests for RuntimeDataClassGenerator - Dynamic class generation
"""
import pytest
from gamedata.dynamic_loader.runtime_class_generator import RuntimeDataClassGenerator


class MockTDAParser:
    """Mock TDA parser for testing."""
    def __init__(self, columns):
        self.columns = columns
    
    def get_column_headers(self):
        return self.columns


class TestRuntimeDataClassGenerator:
    """Test runtime class generation."""
    
    @pytest.fixture
    def generator(self):
        return RuntimeDataClassGenerator()
    
    def test_basic_class_generation(self, generator):
        """Test generating a basic class."""
        columns = ["ID", "Name", "Value", "Description"]
        generated_class = generator.generate_class_from_2da("test_table", columns)
        
        # Check class was generated
        assert generated_class is not None
        assert generated_class.__name__ == "Test_tableData"
        
        # Check slots
        assert hasattr(generated_class, '__slots__')
        assert '_ID' in generated_class.__slots__
        assert '_Name' in generated_class.__slots__
        
        # Check column mapping
        assert hasattr(generated_class, '_column_mapping')
        assert generated_class._column_mapping['ID'] == 'ID'
        assert generated_class._column_mapping['Name'] == 'Name'
    
    def test_instance_creation(self, generator):
        """Test creating instances of generated classes."""
        columns = ["ID", "Label", "Name", "Value"]
        generated_class = generator.generate_class_from_2da("items", columns)
        
        # Create instance
        instance = generated_class(
            ID=1,
            Label="SWORD_001",
            Name="Longsword",
            Value=100
        )
        
        # Check attribute access
        assert instance.ID == 1
        assert instance.Label == "SWORD_001"
        assert instance.Name == "Longsword"
        assert instance.Value == 100
    
    def test_problematic_columns(self, generator):
        """Test handling of problematic column names."""
        columns = ["class", "def", "2DARef", "My-Column", "Value%"]
        generated_class = generator.generate_class_from_2da("test", columns)
        
        # Create instance
        instance = generated_class(
            **{
                "class": "Fighter",
                "def": 10,
                "2DARef": "classes",
                "My-Column": "test",
                "Value%": 50
            }
        )
        
        # Check sanitized access
        assert instance.class_ == "Fighter"
        assert instance.def_ == 10
        assert instance.col_2DARef == "classes"
        assert instance.My_Column == "test"
        assert instance.Valuepct == 50
    
    def test_repr_method(self, generator):
        """Test __repr__ method generation."""
        columns = ["ID", "Name", "Type"]
        generated_class = generator.generate_class_from_2da("test", columns)
        
        instance = generated_class(ID=42, Name="Test Item", Type="Weapon")
        repr_str = repr(instance)
        
        # Should use first column (ID) as primary key
        assert "TestData" in repr_str
        assert "ID=42" in repr_str
    
    def test_to_dict_conversion(self, generator):
        """Test converting instances back to dictionaries."""
        columns = ["ID", "Label", "Min-Level", "Max Level"]
        generated_class = generator.generate_class_from_2da("requirements", columns)
        
        instance = generated_class(
            ID=1,
            Label="REQ_001",
            **{"Min-Level": 5, "Max Level": 10}
        )
        
        # Test with safe names
        safe_dict = instance.to_dict(use_original_names=False)
        assert safe_dict["ID"] == 1
        assert safe_dict["Label"] == "REQ_001"
        assert safe_dict["Min_Level"] == 5
        assert safe_dict["Max_Level"] == 10
        
        # Test with original names
        orig_dict = instance.to_dict(use_original_names=True)
        assert orig_dict["ID"] == 1
        assert orig_dict["Label"] == "REQ_001"
        assert orig_dict["Min-Level"] == 5
        assert orig_dict["Max Level"] == 10
    
    def test_missing_attributes(self, generator):
        """Test error handling for missing attributes."""
        columns = ["ID", "Name"]
        generated_class = generator.generate_class_from_2da("test", columns)
        
        instance = generated_class(ID=1, Name="Test")
        
        # Should raise AttributeError with helpful message
        with pytest.raises(AttributeError) as exc_info:
            _ = instance.NonExistent
        
        error_msg = str(exc_info.value)
        assert "NonExistent" in error_msg
        assert "Available:" in error_msg
        assert "ID" in error_msg
        assert "Name" in error_msg
    
    def test_class_caching(self, generator):
        """Test that classes are cached and reused."""
        columns = ["ID", "Name"]
        
        # Generate class twice
        class1 = generator.generate_class_from_2da("test", columns)
        class2 = generator.generate_class_from_2da("test", columns)
        
        # Should be the same class
        assert class1 is class2
    
    def test_table_name_collisions(self, generator):
        """Test handling of table name collisions."""
        # Generate multiple classes that would have same name
        class1 = generator.generate_class_from_2da("test", ["ID"])
        class2 = generator.generate_class_from_2da("Test", ["ID"])  # Different case
        class3 = generator.generate_class_from_2da("TEST", ["ID"])  # Different case
        
        # All should have unique class names
        names = {class1.__name__, class2.__name__, class3.__name__}
        assert len(names) == 3
    
    def test_code_generation_from_tda(self, generator):
        """Test generating code from TDA parser."""
        mock_tda = MockTDAParser(["ID", "Label", "Name", "Value"])
        code = generator.generate_code_for_table("items", mock_tda)
        
        # Check generated code
        assert "class ItemsData:" in code
        assert "__slots__" in code
        assert "_column_mapping" in code
        assert "def __init__" in code
        assert "def __getattr__" in code
        assert "def __repr__" in code
        assert "def to_dict" in code
    
    def test_setattr_method(self, generator):
        """Test __setattr__ functionality."""
        columns = ["ID", "Name", "Value"]
        generated_class = generator.generate_class_from_2da("test", columns)
        
        instance = generated_class(ID=1, Name="Original", Value=100)
        
        # Test setting with normal names
        instance.Name = "Modified"
        instance.Value = 200
        
        assert instance.Name == "Modified"
        assert instance.Value == 200
        
        # Test setting invalid attribute
        with pytest.raises(AttributeError):
            instance.NonExistent = "value"
    
    def test_empty_columns(self, generator):
        """Test handling of empty column list."""
        generated_class = generator.generate_class_from_2da("empty", [])
        instance = generated_class()
        
        # Should still work, just with no attributes
        assert generated_class.__name__ == "EmptyData"
        assert len(generated_class.__slots__) == 0
        assert repr(instance) == "<EmptyData>"


class TestRealWorldTables:
    """Test with real NWN2 table structures."""
    
    @pytest.fixture
    def generator(self):
        return RuntimeDataClassGenerator()
    
    def test_classes_2da(self, generator):
        """Test with classes.2da structure."""
        columns = [
            "Label", "Name", "Plural", "Lower", "Description",
            "Icon", "HitDie", "AttackBonusTable", "FeatsTable",
            "SavingThrowTable", "SkillsTable", "SkillPointBase",
            "SpellCaster", "SpellAbil", "SpellGainTable",
            "SpellKnownTable", "PlayerClass", "MaxLevel"
        ]
        
        generated_class = generator.generate_class_from_2da("classes", columns)
        
        # Create a fighter instance
        fighter = generated_class(
            Label="Fighter",
            Name=111,  # String ref
            HitDie=10,
            AttackBonusTable="CLS_ATK_1",
            SkillPointBase=2,
            SpellCaster=0,
            PlayerClass=1
        )
        
        assert fighter.Label == "Fighter"
        assert fighter.HitDie == 10
        assert fighter.SpellCaster == 0
    
    def test_feat_2da(self, generator):
        """Test with feat.2da structure."""
        columns = [
            "LABEL", "FEAT", "DESCRIPTION", "ICON",
            "MINSTR", "MINDEX", "MINCON", "MININT", "MINWIS", "MINCHA",
            "MINSPELLLVL", "PREREQFEAT1", "PREREQFEAT2",
            "GAINMULTIPLE", "EFFECTSSTACK", "ALLCLASSESCANUSE",
            "CATEGORY", "SPELLID", "SUCCESSOR", "USESMAPFEAT"
        ]
        
        generated_class = generator.generate_class_from_2da("feat", columns)
        
        # Create a power attack instance
        power_attack = generated_class(
            LABEL="PowerAttack",
            FEAT=28,
            MINSTR=13,
            PREREQFEAT1=-1,
            PREREQFEAT2=-1,
            ALLCLASSESCANUSE=1
        )
        
        assert power_attack.LABEL == "PowerAttack"
        assert power_attack.MINSTR == 13
        assert power_attack.ALLCLASSESCANUSE == 1