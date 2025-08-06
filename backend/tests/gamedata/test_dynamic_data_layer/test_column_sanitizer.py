"""
Tests for ColumnNameSanitizer - Security and edge case testing
"""
import pytest
from gamedata.dynamic_loader.column_sanitizer import ColumnNameSanitizer


class TestColumnNameSanitizer:
    """Test column name sanitization for security and correctness."""
    
    @pytest.fixture
    def sanitizer(self):
        return ColumnNameSanitizer()
    
    def test_basic_sanitization(self, sanitizer):
        """Test basic column name sanitization."""
        # Normal names should pass through
        assert sanitizer.sanitize("Name") == "Name"
        assert sanitizer.sanitize("HitDie") == "HitDie"
        assert sanitizer.sanitize("spell_level") == "spell_level"
        
        # Numbers at start should be prefixed
        assert sanitizer.sanitize("2DAName") == "col_2DAName"
        assert sanitizer.sanitize("123") == "col_123"
    
    def test_special_characters(self, sanitizer):
        """Test handling of special characters."""
        # Hyphens and spaces
        assert sanitizer.sanitize("MOD-COLUMN") == "MOD_COLUMN"
        assert sanitizer.sanitize("My Column") == "My_Column"
        assert sanitizer.sanitize("Path/To/File") == "Path_To_File"
        
        # Mathematical symbols
        assert sanitizer.sanitize("Value+Bonus") == "ValueplusBonus"
        assert sanitizer.sanitize("A&B") == "AandB"
        assert sanitizer.sanitize("50%") == "col_50pct"  # Starts with digit
        assert sanitizer.sanitize("#1") == "num1"
    
    def test_reserved_words(self, sanitizer):
        """Test handling of Python reserved words."""
        # Keywords
        assert sanitizer.sanitize("class") == "class_"
        assert sanitizer.sanitize("def") == "def_"
        assert sanitizer.sanitize("return") == "return_"
        assert sanitizer.sanitize("import") == "import_"
        
        # Soft keywords
        assert sanitizer.sanitize("match") == "match_"
        assert sanitizer.sanitize("case") == "case_"
    
    def test_empty_and_invalid(self, sanitizer):
        """Test edge cases with empty or invalid input."""
        assert sanitizer.sanitize("") == "col_empty"
        assert sanitizer.sanitize("   ") == "col_invalid"
        assert sanitizer.sanitize("***") == "starstarstar"
        assert sanitizer.sanitize("---") == "col_invalid"
    
    def test_unicode_handling(self, sanitizer):
        """Test handling of unicode characters."""
        assert sanitizer.sanitize("café") == "caf"  # Non-ASCII removed
        assert sanitizer.sanitize("名前") == "col_invalid"  # Non-latin
        assert sanitizer.sanitize("test™") == "test"  # Special symbols
    
    def test_collision_handling(self, sanitizer):
        """Test unique column name generation."""
        columns = ["MOD-COLUMN", "MOD_COLUMN", "MOD COLUMN", "MOD/COLUMN"]
        mapping = sanitizer.sanitize_unique_columns(columns)
        
        # All should map to unique names
        safe_names = list(mapping.values())
        assert len(safe_names) == len(set(safe_names))
        
        # First should get base name
        assert mapping["MOD-COLUMN"] == "MOD_COLUMN"
        # Others should get numbered suffixes
        assert mapping["MOD_COLUMN"] == "MOD_COLUMN_2"
        assert mapping["MOD COLUMN"] == "MOD_COLUMN_3"
        assert mapping["MOD/COLUMN"] == "MOD_COLUMN_4"
    
    def test_table_name_sanitization(self, sanitizer):
        """Test table name to class name conversion."""
        assert sanitizer.sanitize_table_name("classes") == "Classes"
        assert sanitizer.sanitize_table_name("cls_feat_barb") == "Cls_feat_barb"
        assert sanitizer.sanitize_table_name("2da_x1") == "Col_2da_x1"
        assert sanitizer.sanitize_table_name("my-mod-table.2da") == "My_mod_table"
    
    def test_malicious_input(self, sanitizer):
        """Test handling of potentially malicious column names."""
        # Attempted code injection
        assert "__" not in sanitizer.sanitize("__import__")
        # exec() becomes exec (parentheses removed)
        assert sanitizer.sanitize("exec()") == "exec"
        # eval(code) becomes eval_code
        assert sanitizer.sanitize("eval(code)") == "eval_code"
        
        # Path traversal attempts
        safe = sanitizer.sanitize("../../etc/passwd")
        assert ".." not in safe
        assert "/" not in safe
        
        # SQL injection attempts
        safe = sanitizer.sanitize("'; DROP TABLE--")
        assert ";" not in safe
        assert "--" not in safe
    
    def test_long_names(self, sanitizer):
        """Test handling of very long column names."""
        long_name = "A" * 1000
        safe = sanitizer.sanitize(long_name)
        assert len(safe) <= 1000  # Should not explode in length
        assert safe.startswith("A")
    
    def test_batch_validation(self, sanitizer):
        """Test batch validation of names."""
        names = ["valid_name", "class", "123invalid", "good-name", "_private"]
        results = sanitizer.validate_batch(names)
        
        assert results["valid_name"] is True
        assert results["class"] is False  # Reserved word
        assert results["123invalid"] is False  # Starts with number
        assert results["good-name"] is False  # Contains hyphen
        assert results["_private"] is True  # Valid identifier


class TestRealWorldExamples:
    """Test with real examples from NWN2 mods."""
    
    @pytest.fixture
    def sanitizer(self):
        return ColumnNameSanitizer()
    
    def test_kaedrin_prc_columns(self, sanitizer):
        """Test columns from Kaedrin's PrC Pack."""
        # Common PrC column patterns
        columns = [
            "MinLevel",
            "MinLevelClass", 
            "ReqFeat1",
            "ReqFeat2",
            "OrReqFeat0",
            "MinSpellLevel",
            "Arcane/Divine",
            "Base Attack Bonus",
            "Skill: Hide",
            "Skill: Move Silently"
        ]
        
        mapping = sanitizer.sanitize_unique_columns(columns)
        
        # Check key mappings
        assert mapping["MinLevel"] == "MinLevel"
        assert mapping["Arcane/Divine"] == "Arcane_Divine"
        assert mapping["Base Attack Bonus"] == "Base_Attack_Bonus"
        assert mapping["Skill: Hide"] == "Skill_Hide"
    
    def test_spell_fixes_columns(self, sanitizer):
        """Test columns from spell fix mods."""
        columns = [
            "SR",
            "SR%",
            "DC+",
            "CL+",
            "Meta:Still",
            "Meta:Silent",
            "(Spell)Power",
            "Use/Day"
        ]
        
        mapping = sanitizer.sanitize_unique_columns(columns)
        
        # Check conversions
        assert mapping["SR%"] == "SRpct"
        assert mapping["DC+"] == "DCplus"
        assert mapping["CL+"] == "CLplus"
        assert mapping["Meta:Still"] == "Meta_Still"
        assert mapping["(Spell)Power"] == "Spell_Power"  # Parentheses removed
        assert mapping["Use/Day"] == "Use_Day"