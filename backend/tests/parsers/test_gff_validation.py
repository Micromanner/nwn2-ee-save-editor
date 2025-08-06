"""
Comprehensive tests for GFF validation and error recovery
"""
import pytest
import io
import struct
import tempfile
from pathlib import Path

from parsers.gff import GFFParser, GFFWriter, GFFElement, GFFFieldType, LocalizedString, LocalizedSubstring
from parsers.gff_validator import (
    GFFValidator, GFFRecovery, ValidationLevel, ValidationSeverity,
    ValidationIssue, FieldSchema, StructSchema
)


class TestGFFValidator:
    """Test GFF validation functionality"""
    
    @pytest.fixture
    def validator(self):
        """Create a GFF validator instance"""
        return GFFValidator(ValidationLevel.NORMAL)
    
    @pytest.fixture
    def strict_validator(self):
        """Create a strict GFF validator instance"""
        return GFFValidator(ValidationLevel.STRICT)
    
    @pytest.fixture
    def lenient_validator(self):
        """Create a lenient GFF validator instance"""
        return GFFValidator(ValidationLevel.LENIENT)
    
    @pytest.fixture
    def valid_bic_element(self):
        """Create a valid BIC structure"""
        # Create ClassList
        class_fields = [
            GFFElement(GFFFieldType.INT, 0, "Class", 0),  # Fighter
            GFFElement(GFFFieldType.SHORT, 0, "ClassLevel", 5)
        ]
        class_struct = GFFElement(GFFFieldType.STRUCT, 2, "", class_fields)
        
        # Create FeatList
        feat_fields = [
            GFFElement(GFFFieldType.WORD, 0, "Feat", 1),  # Some feat
        ]
        feat_struct = GFFElement(GFFFieldType.STRUCT, 1, "", feat_fields)
        
        # Create SkillList (28 skills)
        skill_structs = []
        for i in range(28):
            skill_fields = [GFFElement(GFFFieldType.BYTE, 0, "Rank", 0)]
            skill_structs.append(GFFElement(GFFFieldType.STRUCT, 0, "", skill_fields))
        
        # Create main BIC structure
        fields = [
            # Names
            GFFElement(GFFFieldType.LOCSTRING, 0, "FirstName", 
                      LocalizedString(-1, [LocalizedSubstring("Test Character", 0, 0)])),
            GFFElement(GFFFieldType.LOCSTRING, 0, "LastName",
                      LocalizedString(-1, [LocalizedSubstring("Lastname", 0, 0)])),
            
            # Abilities
            GFFElement(GFFFieldType.BYTE, 0, "Str", 16),
            GFFElement(GFFFieldType.BYTE, 0, "Dex", 14),
            GFFElement(GFFFieldType.BYTE, 0, "Con", 12),
            GFFElement(GFFFieldType.BYTE, 0, "Int", 10),
            GFFElement(GFFFieldType.BYTE, 0, "Wis", 13),
            GFFElement(GFFFieldType.BYTE, 0, "Cha", 8),
            
            # Level and XP
            GFFElement(GFFFieldType.BYTE, 0, "HitDice", 5),
            GFFElement(GFFFieldType.DWORD, 0, "Experience", 10000),
            
            # Hit Points
            GFFElement(GFFFieldType.SHORT, 0, "MaxHitPoints", 45),
            GFFElement(GFFFieldType.SHORT, 0, "CurrentHitPoints", 38),
            GFFElement(GFFFieldType.SHORT, 0, "PregameCurrent", 38),
            
            # Alignment
            GFFElement(GFFFieldType.BYTE, 0, "LawfulChaotic", 50),
            GFFElement(GFFFieldType.BYTE, 0, "GoodEvil", 50),
            
            # Race and appearance
            GFFElement(GFFFieldType.BYTE, 0, "Race", 0),  # Human
            GFFElement(GFFFieldType.WORD, 0, "Appearance_Type", 1),
            GFFElement(GFFFieldType.BYTE, 0, "Gender", 0),
            
            # Saves
            GFFElement(GFFFieldType.BYTE, 0, "fortbonus", 4),
            GFFElement(GFFFieldType.BYTE, 0, "refbonus", 1),
            GFFElement(GFFFieldType.BYTE, 0, "willbonus", 1),
            
            # Lists
            GFFElement(GFFFieldType.LIST, 0, "ClassList", [class_struct]),
            GFFElement(GFFFieldType.LIST, 0, "FeatList", [feat_struct]),
            GFFElement(GFFFieldType.LIST, 0, "SkillList", skill_structs),
        ]
        
        return GFFElement(GFFFieldType.STRUCT, 0, "", fields)
    
    def test_validate_valid_bic(self, validator, valid_bic_element):
        """Test validation of a valid BIC structure"""
        issues = validator.validate_element(valid_bic_element, "BIC")
        assert len(issues) == 0
    
    def test_validate_missing_required_field(self, validator):
        """Test validation with missing required field"""
        # Create BIC without FirstName
        fields = [
            GFFElement(GFFFieldType.BYTE, 0, "Str", 16),
            GFFElement(GFFFieldType.BYTE, 0, "Dex", 14),
            GFFElement(GFFFieldType.BYTE, 0, "Con", 12),
            GFFElement(GFFFieldType.BYTE, 0, "Int", 10),
            GFFElement(GFFFieldType.BYTE, 0, "Wis", 13),
            GFFElement(GFFFieldType.BYTE, 0, "Cha", 8),
            GFFElement(GFFFieldType.BYTE, 0, "HitDice", 5),
            GFFElement(GFFFieldType.DWORD, 0, "Experience", 10000),
            GFFElement(GFFFieldType.SHORT, 0, "MaxHitPoints", 45),
            GFFElement(GFFFieldType.SHORT, 0, "CurrentHitPoints", 38),
            GFFElement(GFFFieldType.SHORT, 0, "PregameCurrent", 38),
            GFFElement(GFFFieldType.BYTE, 0, "LawfulChaotic", 50),
            GFFElement(GFFFieldType.BYTE, 0, "GoodEvil", 50),
            GFFElement(GFFFieldType.BYTE, 0, "Race", 0),
            GFFElement(GFFFieldType.WORD, 0, "Appearance_Type", 1),
            GFFElement(GFFFieldType.BYTE, 0, "Gender", 0),
            GFFElement(GFFFieldType.BYTE, 0, "fortbonus", 4),
            GFFElement(GFFFieldType.BYTE, 0, "refbonus", 1),
            GFFElement(GFFFieldType.BYTE, 0, "willbonus", 1),
            GFFElement(GFFFieldType.LIST, 0, "ClassList", []),
            GFFElement(GFFFieldType.LIST, 0, "SkillList", []),
        ]
        element = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        issues = validator.validate_element(element, "BIC")
        
        # Should have error for missing FirstName
        assert len(issues) > 0
        firstname_issues = [i for i in issues if "FirstName" in i.field_path]
        assert len(firstname_issues) == 1
        assert firstname_issues[0].severity == ValidationSeverity.ERROR
        assert "Required field" in firstname_issues[0].message
    
    def test_validate_field_type_mismatch(self, validator):
        """Test validation with wrong field type"""
        fields = [
            # FirstName should be LOCSTRING but we provide STRING
            GFFElement(GFFFieldType.STRING, 0, "FirstName", "Test"),
            GFFElement(GFFFieldType.BYTE, 0, "Str", 16),
            GFFElement(GFFFieldType.BYTE, 0, "Dex", 14),
            GFFElement(GFFFieldType.BYTE, 0, "Con", 12),
            GFFElement(GFFFieldType.BYTE, 0, "Int", 10),
            GFFElement(GFFFieldType.BYTE, 0, "Wis", 13),
            GFFElement(GFFFieldType.BYTE, 0, "Cha", 8),
            GFFElement(GFFFieldType.BYTE, 0, "HitDice", 5),
            GFFElement(GFFFieldType.DWORD, 0, "Experience", 10000),
            GFFElement(GFFFieldType.SHORT, 0, "MaxHitPoints", 45),
            GFFElement(GFFFieldType.SHORT, 0, "CurrentHitPoints", 38),
            GFFElement(GFFFieldType.SHORT, 0, "PregameCurrent", 38),
            GFFElement(GFFFieldType.BYTE, 0, "LawfulChaotic", 50),
            GFFElement(GFFFieldType.BYTE, 0, "GoodEvil", 50),
            GFFElement(GFFFieldType.BYTE, 0, "Race", 0),
            GFFElement(GFFFieldType.WORD, 0, "Appearance_Type", 1),
            GFFElement(GFFFieldType.BYTE, 0, "Gender", 0),
            GFFElement(GFFFieldType.BYTE, 0, "fortbonus", 4),
            GFFElement(GFFFieldType.BYTE, 0, "refbonus", 1),
            GFFElement(GFFFieldType.BYTE, 0, "willbonus", 1),
            GFFElement(GFFFieldType.LIST, 0, "ClassList", []),
            GFFElement(GFFFieldType.LIST, 0, "SkillList", []),
        ]
        element = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        issues = validator.validate_element(element, "BIC")
        
        # Should have type mismatch error
        type_issues = [i for i in issues if "Field type mismatch" in i.message]
        assert len(type_issues) == 1
        assert type_issues[0].field_path == "FirstName"
        assert type_issues[0].expected_value == "LOCSTRING"
        assert type_issues[0].actual_value == "STRING"
    
    def test_validate_numeric_range(self, validator):
        """Test validation of numeric values out of range"""
        fields = [
            GFFElement(GFFFieldType.LOCSTRING, 0, "FirstName",
                      LocalizedString(-1, [LocalizedSubstring("Test", 0, 0)])),
            # Ability score too low
            GFFElement(GFFFieldType.BYTE, 0, "Str", 2),  # Min is 3
            # Ability score too high
            GFFElement(GFFFieldType.BYTE, 0, "Dex", 51),  # Max is 50
            GFFElement(GFFFieldType.BYTE, 0, "Con", 12),
            GFFElement(GFFFieldType.BYTE, 0, "Int", 10),
            GFFElement(GFFFieldType.BYTE, 0, "Wis", 13),
            GFFElement(GFFFieldType.BYTE, 0, "Cha", 8),
            # Level too high
            GFFElement(GFFFieldType.BYTE, 0, "HitDice", 31),  # Max is 30
            GFFElement(GFFFieldType.DWORD, 0, "Experience", 10000),
            # Hit points can't be negative
            GFFElement(GFFFieldType.SHORT, 0, "MaxHitPoints", 0),  # Min is 1
            GFFElement(GFFFieldType.SHORT, 0, "CurrentHitPoints", -5),  # Min is 0
            GFFElement(GFFFieldType.SHORT, 0, "PregameCurrent", 38),
            # Alignment out of range
            GFFElement(GFFFieldType.BYTE, 0, "LawfulChaotic", 101),  # Max is 100
            GFFElement(GFFFieldType.BYTE, 0, "GoodEvil", 50),
            GFFElement(GFFFieldType.BYTE, 0, "Race", 0),
            GFFElement(GFFFieldType.WORD, 0, "Appearance_Type", 1),
            GFFElement(GFFFieldType.BYTE, 0, "Gender", 3),  # Max is 1
            GFFElement(GFFFieldType.BYTE, 0, "fortbonus", 4),
            GFFElement(GFFFieldType.BYTE, 0, "refbonus", 1),
            GFFElement(GFFFieldType.BYTE, 0, "willbonus", 1),
            GFFElement(GFFFieldType.LIST, 0, "ClassList", []),
            GFFElement(GFFFieldType.LIST, 0, "SkillList", []),
        ]
        element = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        issues = validator.validate_element(element, "BIC")
        
        # Check we got range errors for each out-of-range value
        range_issues = [i for i in issues if "below minimum" in i.message or "above maximum" in i.message]
        assert len(range_issues) >= 6  # At least 6 range errors
        
        # Check specific errors
        str_issues = [i for i in issues if i.field_path == "Str"]
        assert len(str_issues) == 1
        assert "below minimum" in str_issues[0].message
        
        dex_issues = [i for i in issues if i.field_path == "Dex"]
        assert len(dex_issues) == 1
        assert "above maximum" in dex_issues[0].message
    
    def test_validate_list_items(self, validator, valid_bic_element):
        """Test validation of list item structures"""
        # Modify ClassList to have invalid class level
        class_fields = [
            GFFElement(GFFFieldType.INT, 0, "Class", 0),
            GFFElement(GFFFieldType.SHORT, 0, "ClassLevel", 31)  # Too high
        ]
        invalid_class = GFFElement(GFFFieldType.STRUCT, 2, "", class_fields)
        
        # Replace ClassList
        for field in valid_bic_element.value:
            if field.label == "ClassList":
                field.value = [invalid_class]
                break
        
        issues = validator.validate_element(valid_bic_element, "BIC")
        
        # Should have error for class level
        class_issues = [i for i in issues if "ClassList[0].ClassLevel" in i.field_path]
        assert len(class_issues) == 1
        assert "above maximum" in class_issues[0].message
    
    def test_validate_unknown_file_type(self, validator):
        """Test validation with unknown file type"""
        element = GFFElement(GFFFieldType.STRUCT, 0, "", [])
        
        # Strict validator should warn about unknown file type
        strict_validator = GFFValidator(ValidationLevel.STRICT)
        issues = strict_validator.validate_element(element, "UNKNOWN")
        
        assert len(issues) == 1
        assert issues[0].severity == ValidationSeverity.WARNING
        assert "No validation schema defined" in issues[0].message
    
    def test_validate_lenient_mode(self, lenient_validator):
        """Test lenient validation mode"""
        # Create BIC missing many required fields
        fields = [
            GFFElement(GFFFieldType.BYTE, 0, "Str", 16),
            GFFElement(GFFFieldType.BYTE, 0, "Dex", 14),
            # Missing most other required fields
        ]
        element = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        issues = lenient_validator.validate_element(element, "BIC")
        
        # In lenient mode, missing required fields should be warnings
        required_issues = [i for i in issues if "Required field" in i.message]
        assert len(required_issues) > 0
        assert all(i.severity == ValidationSeverity.WARNING for i in required_issues)
    
    def test_validate_ifo_file(self, validator):
        """Test validation of IFO file structure"""
        fields = [
            GFFElement(GFFFieldType.LOCSTRING, 0, "Mod_Name",
                      LocalizedString(-1, [LocalizedSubstring("Test Module", 0, 0)])),
            GFFElement(GFFFieldType.STRING, 0, "Mod_Tag", "test_module"),
            GFFElement(GFFFieldType.RESREF, 0, "Mod_Entry_Area", "testarea"),
            GFFElement(GFFFieldType.FLOAT, 0, "Mod_Entry_X", 10.5),
            GFFElement(GFFFieldType.FLOAT, 0, "Mod_Entry_Y", 20.5),
            GFFElement(GFFFieldType.FLOAT, 0, "Mod_Entry_Z", 0.0),
        ]
        element = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        issues = validator.validate_element(element, "IFO")
        assert len(issues) == 0
    
    def test_custom_validator(self):
        """Test custom field validator"""
        def check_even_number(field, path, issues):
            if field.value % 2 != 0:
                issues.append(ValidationIssue(
                    ValidationSeverity.ERROR,
                    path,
                    "Value must be even",
                    actual_value=field.value
                ))
        
        # Create custom schema
        schema = StructSchema()
        schema.add_field(FieldSchema("TestField", GFFFieldType.INT, 
                                   custom_validator=check_even_number))
        
        validator = GFFValidator()
        validator.schemas["TEST"] = schema
        
        # Test with odd number
        fields = [GFFElement(GFFFieldType.INT, 0, "TestField", 5)]
        element = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        issues = validator.validate_element(element, "TEST")
        assert len(issues) == 1
        assert "must be even" in issues[0].message
        
        # Test with even number
        fields = [GFFElement(GFFFieldType.INT, 0, "TestField", 6)]
        element = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        issues = validator.validate_element(element, "TEST")
        assert len(issues) == 0


class TestGFFErrorRecovery:
    """Test GFF error recovery functionality"""
    
    @pytest.fixture
    def recovery_parser(self):
        """Create a parser with error recovery enabled"""
        return GFFParser(error_recovery=True)
    
    def create_corrupted_header(self, corruption_type="version"):
        """Create a corrupted GFF header"""
        header = bytearray(56)
        
        if corruption_type == "version":
            header[0:4] = b'TEST'
            header[4:8] = b'V2.0'  # Wrong version
        elif corruption_type == "size":
            # Return header that's too short
            return header[:40]
        elif corruption_type == "counts":
            header[0:4] = b'TEST'
            header[4:8] = b'V3.2'
            # Set invalid counts
            struct.pack_into('<I', header, 8, 56)    # struct offset
            struct.pack_into('<I', header, 12, 999999)  # struct count way too high
        
        return header
    
    def test_recovery_wrong_version(self, recovery_parser):
        """Test recovery from wrong version"""
        header = self.create_corrupted_header("version")
        # Add minimal struct data
        struct_data = struct.pack('<III', 0, 0, 0)
        
        stream = io.BytesIO(header + struct_data)
        
        # Should not raise with error recovery
        result = recovery_parser.load(stream)
        errors = recovery_parser.get_recovery_errors()
        
        # Should have logged version error but continued
        assert any("version" in err.lower() for err in errors)
    
    def test_recovery_truncated_file(self, recovery_parser):
        """Test recovery from truncated file"""
        # Create valid header but no data
        header = bytearray(56)
        header[0:4] = b'BIC '
        header[4:8] = b'V3.2'
        
        # Set counts indicating data that doesn't exist
        struct.pack_into('<I', header, 8, 56)    # struct offset
        struct.pack_into('<I', header, 12, 1)    # struct count
        struct.pack_into('<I', header, 16, 68)   # field offset
        struct.pack_into('<I', header, 20, 5)    # field count
        
        stream = io.BytesIO(header)  # No actual struct/field data
        
        result = recovery_parser.load(stream)
        errors = recovery_parser.get_recovery_errors()
        
        # Should have errors about missing data
        assert len(errors) > 0
    
    def test_recovery_invalid_field_indices(self, recovery_parser):
        """Test recovery from invalid field indices"""
        # Create a minimal valid GFF structure
        header = bytearray(56)
        header[0:4] = b'TEST'
        header[4:8] = b'V3.2'
        
        # Offsets
        struct_offset = 56
        field_offset = struct_offset + 12  # 1 struct
        label_offset = field_offset + 12   # 1 field
        
        struct.pack_into('<I', header, 8, struct_offset)
        struct.pack_into('<I', header, 12, 1)  # struct count
        struct.pack_into('<I', header, 16, field_offset)
        struct.pack_into('<I', header, 20, 1)  # field count
        struct.pack_into('<I', header, 24, label_offset)
        struct.pack_into('<I', header, 28, 1)  # label count
        
        # Create struct with invalid field index
        struct_data = struct.pack('<III', 0, 999, 1)  # field index 999 doesn't exist
        
        # Create field
        field_data = struct.pack('<III', GFFFieldType.INT, 0, 42)
        
        # Create label
        label_data = b'TestLabel'.ljust(16, b'\x00')
        
        file_data = header + struct_data + field_data + label_data
        stream = io.BytesIO(file_data)
        
        result = recovery_parser.load(stream)
        errors = recovery_parser.get_recovery_errors()
        
        # Should have error about invalid field index
        assert any("field index" in err.lower() for err in errors)
    
    def test_recovery_corrupted_string(self, recovery_parser):
        """Test recovery from corrupted string field"""
        # This would require a more complex setup with field data buffer
        # For now, just verify the parser accepts error_recovery parameter
        assert recovery_parser.error_recovery == True
        assert recovery_parser.recovery_errors == []


class TestGFFRecoveryAnalysis:
    """Test GFF file corruption analysis"""
    
    def test_analyze_truncated_file(self, tmp_path):
        """Test analysis of truncated file"""
        # Create truncated file
        test_file = tmp_path / "truncated.bic"
        test_file.write_bytes(b'BIC V3.2' + b'\x00' * 20)  # Only 28 bytes
        
        issues = GFFRecovery.analyze_corruption(test_file)
        
        assert len(issues) > 0
        assert any("too small" in issue.message for issue in issues)
    
    def test_analyze_invalid_header(self, tmp_path):
        """Test analysis of file with invalid header"""
        # Create file with bad file type
        test_file = tmp_path / "invalid.gff"
        header = bytearray(56)
        header[0:4] = b'\xFF\xFF\xFF\xFF'  # Invalid characters
        header[4:8] = b'V3.2'
        test_file.write_bytes(header)
        
        issues = GFFRecovery.analyze_corruption(test_file)
        
        assert any("Invalid file type" in issue.message for issue in issues)
    
    def test_analyze_overlapping_sections(self, tmp_path):
        """Test analysis of file with overlapping data sections"""
        test_file = tmp_path / "overlap.gff"
        header = bytearray(56)
        header[0:4] = b'TEST'
        header[4:8] = b'V3.2'
        
        # Create overlapping sections
        struct.pack_into('<I', header, 8, 100)   # struct offset
        struct.pack_into('<I', header, 12, 10)   # struct count (120 bytes)
        struct.pack_into('<I', header, 16, 150)  # field offset (overlaps!)
        struct.pack_into('<I', header, 20, 5)    # field count
        
        # Add enough data to make file valid size
        test_file.write_bytes(header + b'\x00' * 200)
        
        issues = GFFRecovery.analyze_corruption(test_file)
        
        assert any("overlap" in issue.message.lower() for issue in issues)