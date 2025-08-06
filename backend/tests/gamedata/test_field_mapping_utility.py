"""
Tests for Field Mapping Utility - verifies 2DA field name compatibility
"""
import pytest
from unittest.mock import Mock

from gamedata.dynamic_loader.field_mapping_utility import FieldMappingUtility, field_mapper


class TestFieldMappingUtility:
    """Test the field mapping utility functionality"""
    
    def test_get_field_value_primary_field(self):
        """Test getting value from primary field name"""
        mock_data = Mock()
        mock_data.label = "Test Value"
        
        mapper = FieldMappingUtility()
        result = mapper.get_field_value(mock_data, 'label')
        
        assert result == "Test Value"
    
    def test_get_field_value_alternative_field(self):
        """Test getting value from alternative field name"""
        # Use a simple object instead of Mock to avoid auto-creation issues
        class MockData:
            def __init__(self):
                self.name = "Alternative Value"
        
        mock_data = MockData()
        
        mapper = FieldMappingUtility()
        result = mapper.get_field_value(mock_data, 'label')  # Should find 'name'
        
        assert result == "Alternative Value"
    
    def test_get_field_value_case_insensitive(self):
        """Test case-insensitive field matching"""
        class MockData:
            def __init__(self):
                self.LABEL = "Upper Case Value"
        
        mock_data = MockData()
        
        mapper = FieldMappingUtility()
        result = mapper.get_field_value(mock_data, 'label')
        
        assert result == "Upper Case Value"
    
    def test_get_field_value_default(self):
        """Test default value when field not found"""
        class MockData:
            pass  # No attributes
        
        mock_data = MockData()
        
        mapper = FieldMappingUtility()
        result = mapper.get_field_value(mock_data, 'label', 'default_value')
        
        assert result == 'default_value'
    
    def test_get_field_value_ignores_empty_string(self):
        """Test that empty strings are treated as missing values"""
        class MockData:
            def __init__(self):
                self.label = ""
                self.name = "Valid Value"
        
        mock_data = MockData()
        
        mapper = FieldMappingUtility()
        result = mapper.get_field_value(mock_data, 'label')
        
        assert result == "Valid Value"
    
    def test_get_field_value_ignores_asterisks(self):
        """Test that '****' values are treated as missing"""
        class MockData:
            def __init__(self):
                self.label = "****"
                self.name = "Valid Value"
        
        mock_data = MockData()
        
        mapper = FieldMappingUtility()
        result = mapper.get_field_value(mock_data, 'label')
        
        assert result == "Valid Value"


class TestRacialAbilityModifiers:
    """Test racial ability modifier field mapping"""
    
    def test_get_ability_modifiers_standard_fields(self):
        """Test with standard field names"""
        mock_race = Mock()
        mock_race.str_adjust = 2
        mock_race.dex_adjust = -1
        mock_race.con_adjust = 0
        mock_race.int_adjust = 1
        mock_race.wis_adjust = 0
        mock_race.cha_adjust = -2
        
        mapper = FieldMappingUtility()
        result = mapper.get_ability_modifiers(mock_race)
        
        expected = {
            'Str': 2, 'Dex': -1, 'Con': 0,
            'Int': 1, 'Wis': 0, 'Cha': -2
        }
        assert result == expected
    
    def test_get_ability_modifiers_alternative_fields(self):
        """Test with alternative field naming conventions"""
        class MockRace:
            def __init__(self):
                self.StrAdjust = 1
                self.DexAdjust = 2
                self.strength_adjust = 3  # Should not override StrAdjust
        
        mock_race = MockRace()
        
        mapper = FieldMappingUtility()
        result = mapper.get_ability_modifiers(mock_race)
        
        assert result['Str'] == 1  # Uses StrAdjust
        assert result['Dex'] == 2  # Uses DexAdjust
        assert result['Con'] == 0  # Default value
    
    def test_get_ability_modifiers_mixed_case(self):
        """Test with mixed case field names"""
        class MockRace:
            def __init__(self):
                self.STRAdjust = 2
                self.dex_adjust = 1
        
        mock_race = MockRace()
        
        mapper = FieldMappingUtility()
        result = mapper.get_ability_modifiers(mock_race)
        
        assert result['Str'] == 2
        assert result['Dex'] == 1


class TestRacialSaves:
    """Test racial saving throw field mapping"""
    
    def test_get_racial_saves_standard_fields(self):
        """Test with standard field names"""
        class MockRace:
            def __init__(self):
                self.fort_save = 2
                self.ref_save = 1
                self.will_save = 0
        
        mock_race = MockRace()
        
        mapper = FieldMappingUtility()
        result = mapper.get_racial_saves(mock_race)
        
        expected = {'fortitude': 2, 'reflex': 1, 'will': 0}
        assert result == expected
    
    def test_get_racial_saves_alternative_fields(self):
        """Test with alternative field naming conventions"""
        class MockRace:
            def __init__(self):
                self.FortSave = 1
                self.RefSave = 2
                self.WillSave = 1
        
        mock_race = MockRace()
        
        mapper = FieldMappingUtility()
        result = mapper.get_racial_saves(mock_race)
        
        expected = {'fortitude': 1, 'reflex': 2, 'will': 1}
        assert result == expected
    
    def test_get_racial_saves_missing_fields(self):
        """Test default values when fields are missing"""
        class MockRace:
            pass  # No save fields defined
        
        mock_race = MockRace()
        
        mapper = FieldMappingUtility()
        result = mapper.get_racial_saves(mock_race)
        
        expected = {'fortitude': 0, 'reflex': 0, 'will': 0}
        assert result == expected


class TestFeatPrerequisites:
    """Test feat prerequisite field mapping"""
    
    def test_get_feat_prerequisites_ability_scores(self):
        """Test ability score prerequisites"""
        mock_feat = Mock()
        mock_feat.prereq_str = 15
        mock_feat.prereq_dex = 13
        mock_feat.prereq_con = 0
        mock_feat.prereq_int = 0
        mock_feat.prereq_wis = 0
        mock_feat.prereq_cha = 0
        
        mapper = FieldMappingUtility()
        result = mapper.get_feat_prerequisites(mock_feat)
        
        assert result['abilities']['Str'] == 15
        assert result['abilities']['Dex'] == 13
        assert result['abilities']['Con'] == 0
    
    def test_get_feat_prerequisites_alternative_fields(self):
        """Test with alternative prerequisite field names"""
        class MockFeat:
            def __init__(self):
                self.PreReqStr = 16
                self.MinDex = 14
        
        mock_feat = MockFeat()
        
        mapper = FieldMappingUtility()
        result = mapper.get_feat_prerequisites(mock_feat)
        
        assert result['abilities']['Str'] == 16
        assert result['abilities']['Dex'] == 14
    
    def test_get_feat_prerequisites_feat_requirements(self):
        """Test feat prerequisite extraction"""
        mock_feat = Mock()
        mock_feat.prereq_feat1 = 5
        mock_feat.prereq_feat2 = 10
        
        mapper = FieldMappingUtility()
        result = mapper.get_feat_prerequisites(mock_feat)
        
        assert 5 in result['feats']
        assert 10 in result['feats']
        assert len(result['feats']) == 2
    
    def test_get_feat_prerequisites_class_level(self):
        """Test class and level prerequisites"""
        mock_feat = Mock()
        mock_feat.required_class = 3  # Cleric
        mock_feat.min_level = 5
        mock_feat.prereq_bab = 3
        
        mapper = FieldMappingUtility()
        result = mapper.get_feat_prerequisites(mock_feat)
        
        assert result['class'] == 3
        assert result['level'] == 5
        assert result['bab'] == 3


class TestRacialFeats:
    """Test racial feat extraction"""
    
    def test_get_racial_feats_individual_fields(self):
        """Test extracting feats from individual Feat0, Feat1 fields"""
        mock_race = Mock()
        mock_race.Feat0 = 5
        mock_race.Feat1 = 10
        mock_race.Feat2 = 0  # Should be ignored
        
        mapper = FieldMappingUtility()
        result = mapper.get_racial_feats(mock_race)
        
        assert 5 in result
        assert 10 in result
        assert 0 not in result
        assert len(result) == 2
    
    def test_get_racial_feats_array_format(self):
        """Test extracting feats from array format"""
        mock_race = Mock()
        mock_race.racial_feats = [15, 20, 25]
        
        mapper = FieldMappingUtility()
        result = mapper.get_racial_feats(mock_race)
        
        assert result == [15, 20, 25]
    
    def test_get_racial_feats_string_format(self):
        """Test extracting feats from comma-separated string"""
        mock_race = Mock()
        mock_race.racial_feats = "12,18,24"
        
        mapper = FieldMappingUtility()
        result = mapper.get_racial_feats(mock_race)
        
        assert 12 in result
        assert 18 in result
        assert 24 in result
        assert len(result) == 3
    
    def test_get_racial_feats_no_duplicates(self):
        """Test that duplicate feats are not included"""
        mock_race = Mock()
        mock_race.Feat0 = 5
        mock_race.racial_feats = [5, 10]  # 5 is duplicate
        
        mapper = FieldMappingUtility()
        result = mapper.get_racial_feats(mock_race)
        
        # Should only include 5 once
        assert result.count(5) == 1
        assert 10 in result


class TestClassProperties:
    """Test class property extraction"""
    
    def test_get_class_properties_basic(self):
        """Test basic class property extraction"""
        class MockClass:
            def __init__(self):
                self.label = "Fighter"
                self.hit_die = 10
                self.skill_point_base = 2
                self.spell_caster = 0
                self.player_class = 1
                # Add missing attributes to avoid Mock auto-creation
                self.attack_bonus_table = ""
                self.saving_throw_table = ""
                self.align_restrict = "0x00"
                self.prereq_table = ""
        
        mock_class = MockClass()
        
        mapper = FieldMappingUtility()
        result = mapper.get_class_properties(mock_class)
        
        assert result['label'] == "Fighter"
        assert result['hit_die'] == 10
        assert result['skill_points'] == 2
        assert result['spell_caster'] is False
        assert result['player_class'] is True
    
    def test_get_class_properties_alternative_fields(self):
        """Test with alternative field names"""
        class MockClass:
            def __init__(self):
                self.name = "Wizard"  # Alternative to label
                self.HitDie = 4  # Alternative case
                self.SkillPointBase = 2
        
        mock_class = MockClass()
        
        mapper = FieldMappingUtility()
        result = mapper.get_class_properties(mock_class)
        
        assert result['label'] == "Wizard"
        assert result['hit_die'] == 4
        assert result['skill_points'] == 2


class TestFieldValidation:
    """Test field validation functionality"""
    
    def test_validate_field_access(self):
        """Test field access validation"""
        class MockData:
            def __init__(self):
                self.label = "Test"
                self.str_adjust = 2
                # No 'missing_field'
        
        mock_data = MockData()
        
        mapper = FieldMappingUtility()
        result = mapper.validate_field_access(mock_data, ['label', 'str_adjust', 'missing_field'])
        
        assert result['label'] is True
        assert result['str_adjust'] is True
        assert result['missing_field'] is False


class TestGlobalInstance:
    """Test the global field_mapper instance"""
    
    def test_global_instance_exists(self):
        """Test that global field_mapper instance is available"""
        assert field_mapper is not None
        assert isinstance(field_mapper, FieldMappingUtility)
    
    def test_global_instance_functionality(self):
        """Test that global instance functions correctly"""
        mock_data = Mock()
        mock_data.label = "Global Test"
        
        result = field_mapper.get_field_value(mock_data, 'label')
        assert result == "Global Test"


class TestExpandedFieldMapping:
    """Test expanded field mapping functionality for NWN2 PascalCase fields"""
    
    def test_pascalcase_table_references(self):
        """Test PascalCase table reference field mapping"""
        class MockClass:
            def __init__(self):
                self.AttackBonusTable = "CLS_ATK_1"
                self.SavingThrowTable = "CLS_SAVTHR_BARB"
                self.HitDie = "12"
        
        mock_class = MockClass()
        mapper = FieldMappingUtility()
        
        # Test that PascalCase is found first
        attack_table = mapper.get_field_value(mock_class, 'attack_bonus_table')
        save_table = mapper.get_field_value(mock_class, 'saving_throw_table')
        hit_die = mapper.get_field_value(mock_class, 'hit_die')
        
        assert attack_table == "CLS_ATK_1"
        assert save_table == "CLS_SAVTHR_BARB"
        assert hit_die == "12"
    
    def test_hex_string_conversion(self):
        """Test hex string to int conversion"""
        mapper = FieldMappingUtility()
        
        # Test hex conversion
        assert mapper._safe_hex_int("0x02", 0) == 2
        assert mapper._safe_hex_int("0X0F", 0) == 15
        assert mapper._safe_hex_int("0x00", 1) == 0
        assert mapper._safe_hex_int("invalid", 5) == 5
        assert mapper._safe_hex_int("", 3) == 3
        assert mapper._safe_hex_int("****", 7) == 7
    
    def test_safe_int_with_strings(self):
        """Test safe int conversion with NWN2 string values"""
        mapper = FieldMappingUtility()
        
        # Test string number conversion
        assert mapper._safe_int("12", 0) == 12
        assert mapper._safe_int("-3", 0) == -3
        assert mapper._safe_int("0x10", 0) == 16  # Should handle hex
        assert mapper._safe_int("****", 5) == 5
        assert mapper._safe_int("", 7) == 7
        assert mapper._safe_int("invalid", 2) == 2
    
    def test_safe_bool_with_nwn2_values(self):
        """Test boolean conversion with NWN2 string representations"""
        mapper = FieldMappingUtility()
        
        # Test various boolean representations
        assert mapper._safe_bool("1", False) is True
        assert mapper._safe_bool("0", True) is False
        assert mapper._safe_bool("true", False) is True
        assert mapper._safe_bool("false", True) is False
        assert mapper._safe_bool("yes", False) is True
        assert mapper._safe_bool("no", True) is False
        assert mapper._safe_bool("****", True) is True  # Default
        assert mapper._safe_bool("", False) is False  # Default
    
    def test_prerequisite_table_parsing(self):
        """Test prerequisite table name parsing"""
        mapper = FieldMappingUtility()
        
        # Test valid prerequisite table
        result = mapper.parse_prerequisite_table("CLS_PRES_SHADOW")
        assert result['table_type'] == 'class_prerequisites'
        assert result['identifier'] == 'SHADOW'
        assert result['raw_table'] == 'CLS_PRES_SHADOW'
        
        # Test invalid format
        result = mapper.parse_prerequisite_table("INVALID_FORMAT")
        assert 'table_type' not in result
        assert result['raw_table'] == 'INVALID_FORMAT'
        
        # Test empty/null values
        result = mapper.parse_prerequisite_table("")
        assert result == {}
        
        result = mapper.parse_prerequisite_table("****")
        assert result == {}
    
    def test_get_alignment_restriction(self):
        """Test alignment restriction extraction with hex parsing"""
        class MockClass:
            def __init__(self):
                self.AlignRestrict = "0x02"
        
        mock_class = MockClass()
        mapper = FieldMappingUtility()
        
        result = mapper.get_alignment_restriction(mock_class)
        assert result == 2
    
    def test_get_class_table_values(self):
        """Test comprehensive class table value extraction"""
        class MockClass:
            def __init__(self):
                self.AttackBonusTable = "CLS_ATK_FIGHTER"
                self.SavingThrowTable = "CLS_SAVTHR_FIGHTER"
                self.HitDie = "10"
                self.SkillPointBase = "2"
        
        mock_class = MockClass()
        mapper = FieldMappingUtility()
        
        result = mapper.get_class_table_values(mock_class)
        
        assert result['attack_bonus_table'] == "CLS_ATK_FIGHTER"
        assert result['saving_throw_table'] == "CLS_SAVTHR_FIGHTER"
        assert result['hit_die'] == 10
        assert result['skill_points'] == 2
        assert 'bab' in result
        assert 'fort_save' in result
    
    def test_robust_field_value_with_conversion(self):
        """Test robust field value extraction with type conversion"""
        class MockData:
            def __init__(self):
                self.HitDie = "12"
                self.AlignRestrict = "0x0F"
                self.SpellCaster = "1"
                self.Name = "Fighter"
        
        mock_data = MockData()
        mapper = FieldMappingUtility()
        
        # Test int conversion
        hit_die = mapper.get_robust_field_value(mock_data, ['hit_die'], 'int', 8)
        assert hit_die == 12
        
        # Test hex conversion
        align = mapper.get_robust_field_value(mock_data, ['align_restrict'], 'hex', 0)
        assert align == 15
        
        # Test bool conversion
        caster = mapper.get_robust_field_value(mock_data, ['spell_caster'], 'bool', False)
        assert caster is True
        
        # Test string conversion
        name = mapper.get_robust_field_value(mock_data, ['name'], 'str', '')
        assert name == "Fighter"
    
    def test_bulk_field_extraction(self):
        """Test bulk field extraction with comprehensive mapping"""
        class MockClass:
            def __init__(self):
                self.AttackBonusTable = "CLS_ATK_1"
                self.HitDie = "10"
                self.SpellCaster = "0"
                self.AlignRestrict = "0x02"
        
        mock_class = MockClass()
        mapper = FieldMappingUtility()
        
        field_mapping = {
            'attack_table': {
                'patterns': ['attack_bonus_table'],
                'type': 'str',
                'default': ''
            },
            'hit_die': {
                'patterns': ['hit_die'],
                'type': 'int',
                'default': 8
            },
            'is_caster': {
                'patterns': ['spell_caster'],
                'type': 'bool',
                'default': False
            },
            'alignment': {
                'patterns': ['align_restrict'],
                'type': 'hex',
                'default': 0
            }
        }
        
        result = mapper.bulk_field_extraction(mock_class, field_mapping)
        
        assert result['attack_table'] == "CLS_ATK_1"
        assert result['hit_die'] == 10
        assert result['is_caster'] is False
        assert result['alignment'] == 2