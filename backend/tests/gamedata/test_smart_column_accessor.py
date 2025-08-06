"""
Tests for RuleDetector - Dynamic rule detection system
"""
import pytest
from unittest.mock import Mock, MagicMock
from gamedata.rule_detector import RuleDetector, TwoDA, ResourceManager


class MockTwoDA:
    """Mock implementation of TwoDA protocol for testing"""
    def __init__(self, rows=None, headers=None):
        self.rows = rows or []
        self.headers = headers or []
    
    def get_resource_count(self) -> int:
        return len(self.rows)
    
    def get_row_dict(self, row_id: int):
        if 0 <= row_id < len(self.rows):
            return self.rows[row_id]
        return None
    
    def get_cell(self, row_id: int, column_name: str):
        row = self.get_row_dict(row_id)
        return row.get(column_name) if row else None
    
    def get_rows_as_dicts(self):
        return self.rows
    
    def get_column_headers(self):
        return self.headers


@pytest.fixture
def mock_resource_manager():
    """Create a mock ResourceManager with test data"""
    rm = Mock(spec=ResourceManager)
    
    # feat.2da - standard feat requirements
    feat_headers = ['Label', 'MINSTR', 'MINDEX', 'MININT', 'PREREQFEAT1', 'PREREQFEAT2', 
                    'REQSKILL', 'ReqSkillMinRanks', 'MinLevel', 'AlignRestrict']
    feat_rows = [
        {
            'Label': 'PowerAttack',
            'MINSTR': '13',
            'MINDEX': '****',
            'MININT': '****',
            'PREREQFEAT1': '****',
            'PREREQFEAT2': '****',
            'REQSKILL': '****',
            'ReqSkillMinRanks': '****',
            'MinLevel': '1',
            'AlignRestrict': '****'
        },
        {
            'Label': 'CombatExpertise',
            'MINSTR': '****',
            'MINDEX': '****',
            'MININT': '13',
            'PREREQFEAT1': '****',
            'PREREQFEAT2': '****',
            'REQSKILL': '****',
            'ReqSkillMinRanks': '****',
            'MinLevel': '1',
            'AlignRestrict': '****'
        },
        {
            'Label': 'ImprovedTrip',
            'MINSTR': '****',
            'MINDEX': '****',
            'MININT': '13',
            'PREREQFEAT1': '1',  # Combat Expertise
            'PREREQFEAT2': '****',
            'REQSKILL': '****',
            'ReqSkillMinRanks': '****',
            'MinLevel': '1',
            'AlignRestrict': '****'
        },
        {
            'Label': 'SkillFocus',
            'MINSTR': '****',
            'MINDEX': '****',
            'MININT': '****',
            'PREREQFEAT1': '****',
            'PREREQFEAT2': '****',
            'REQSKILL': '8',  # Concentration
            'ReqSkillMinRanks': '5',
            'MinLevel': '1',
            'AlignRestrict': '****'
        }
    ]
    
    # cls_pres_wm.2da - Weapon Master prerequisites
    cls_pres_headers = ['LABEL', 'ReqType', 'ReqParam1', 'ReqParam2']
    cls_pres_rows = [
        {'LABEL': 'Base_Attack', 'ReqType': 'BAB', 'ReqParam1': '5', 'ReqParam2': '****'},
        {'LABEL': 'Weapon_Focus', 'ReqType': 'FEAT', 'ReqParam1': '100', 'ReqParam2': '****'},
        {'LABEL': 'Intimidate', 'ReqType': 'SKILL', 'ReqParam1': '24', 'ReqParam2': '4'},
    ]
    
    # cls_feat_barb.2da - Barbarian feat progression
    cls_feat_headers = ['FeatLabel', 'FeatIndex', 'GrantedOnLevel', 'List']
    cls_feat_rows = [
        {'FeatLabel': 'Rage', 'FeatIndex': '200', 'GrantedOnLevel': '1', 'List': '3'},
        {'FeatLabel': 'UncannyDodge', 'FeatIndex': '201', 'GrantedOnLevel': '2', 'List': '0'},
        {'FeatLabel': 'ImprovedUncannyDodge', 'FeatIndex': '202', 'GrantedOnLevel': '5', 'List': '0'},
    ]
    
    # spells.2da - spell class availability
    spells_headers = ['Label', 'Name', 'Bard', 'Cleric', 'Druid', 'Paladin', 'Ranger', 
                      'Wiz_Sorc', 'Warlock', 'Spirit_Shaman', 'FeatID']
    spells_rows = [
        {
            'Label': 'Cure_Light_Wounds',
            'Name': '100',
            'Bard': '1',
            'Cleric': '1',
            'Druid': '1',
            'Paladin': '1',
            'Ranger': '2',
            'Wiz_Sorc': '****',
            'Warlock': '****',
            'Spirit_Shaman': '1',
            'FeatID': '****'
        },
        {
            'Label': 'Fireball',
            'Name': '101',
            'Bard': '****',
            'Cleric': '****',
            'Druid': '****',
            'Paladin': '****',
            'Ranger': '****',
            'Wiz_Sorc': '3',
            'Warlock': '****',
            'Spirit_Shaman': '****',
            'FeatID': '****'
        }
    ]
    
    # classes.2da
    classes_headers = ['Label', 'Name', 'SpellCaster', 'SpellGainTable', 'SpellKnownTable', 
                       'PlayerClass', 'AlignRestrict', 'AlignRstrctType']
    classes_rows = [
        {
            'Label': 'Barbarian',
            'Name': '110',
            'SpellCaster': '0',
            'SpellGainTable': '****',
            'SpellKnownTable': '****',
            'PlayerClass': '1',
            'AlignRestrict': '0x01',
            'AlignRstrctType': '0x02'
        },
        {
            'Label': 'Wizard',
            'Name': '111',
            'SpellCaster': '1',
            'SpellGainTable': 'CLS_SPGN_WIZ',
            'SpellKnownTable': '****',
            'PlayerClass': '1',
            'AlignRestrict': '0x00',
            'AlignRstrctType': '0x00'
        }
    ]
    
    # Set up mock returns
    def get_2da_side_effect(name):
        name_lower = name.lower()
        if name_lower == 'feat':
            return MockTwoDA(feat_rows, feat_headers)
        elif name_lower == 'cls_pres_wm':
            return MockTwoDA(cls_pres_rows, cls_pres_headers)
        elif name_lower == 'cls_feat_barb':
            return MockTwoDA(cls_feat_rows, cls_feat_headers)
        elif name_lower == 'spells':
            return MockTwoDA(spells_rows, spells_headers)
        elif name_lower == 'classes':
            return MockTwoDA(classes_rows, classes_headers)
        return None
    
    rm.get_2da_with_overrides.side_effect = get_2da_side_effect
    return rm


@pytest.fixture
def accessor(mock_resource_manager):
    """Create RuleDetector instance with mock ResourceManager"""
    return RuleDetector(mock_resource_manager)


class TestColumnAnalysis:
    """Test column purpose detection"""
    
    def test_analyze_feat_columns(self, accessor):
        """Test that feat.2da columns are correctly identified"""
        accessor._analyze_and_cache_columns('feat')
        
        cache = accessor._cache['feat']
        assert cache['map']['MINSTR'] == 'min_str'
        assert cache['map']['MINDEX'] == 'min_dex'
        assert cache['map']['MININT'] == 'min_int'
        assert cache['map']['PREREQFEAT1'] == 'prereq_feat'
        assert cache['map']['MinLevel'] == 'min_level'
        assert cache['map']['AlignRestrict'] == 'alignment_restrict'
    
    def test_analyze_class_feat_columns(self, accessor):
        """Test that cls_feat_*.2da columns are correctly identified"""
        accessor._analyze_and_cache_columns('cls_feat_barb')
        
        cache = accessor._cache['cls_feat_barb']
        assert cache['map']['FeatLabel'] == 'feat_label'
        assert cache['map']['FeatIndex'] == 'feat_index'
        assert cache['map']['GrantedOnLevel'] == 'granted_level'
        assert cache['type'] == 'class_feat_progression'
    
    def test_table_type_detection(self, accessor):
        """Test table type pattern matching"""
        assert accessor._get_table_type_no_cache('cls_feat_barb') == 'class_feat_progression'
        assert accessor._get_table_type_no_cache('cls_skill_wizard') == 'class_skill_list'
        assert accessor._get_table_type_no_cache('cls_savthr_monk') == 'class_saves'
        assert accessor._get_table_type_no_cache('cls_pres_wm') == 'class_prerequisites'
        assert accessor._get_table_type_no_cache('feat') is None


class TestRequirementExtraction:
    """Test requirement detection and extraction"""
    
    def test_simple_attribute_requirements(self, accessor):
        """Test extraction of attribute requirements from feat.2da"""
        row = {
            'Label': 'PowerAttack',
            'MINSTR': '13',
            'MINDEX': '****',
            'MININT': '****',
            'MinLevel': '1'
        }
        
        reqs = accessor.get_requirements('feat', row)
        assert reqs['min_str'] == 13
        assert reqs['min_level'] == 1
        assert 'min_dex' not in reqs  # Should skip **** values
        assert 'min_int' not in reqs
    
    def test_feat_prerequisites(self, accessor):
        """Test extraction of feat prerequisites"""
        row = {
            'Label': 'ImprovedTrip',
            'PREREQFEAT1': '1',
            'PREREQFEAT2': '****',
            'MININT': '13'
        }
        
        reqs = accessor.get_requirements('feat', row)
        assert reqs['min_int'] == 13
        assert 'prereq_feats' in reqs
        assert reqs['prereq_feats']['all_of'] == [1]
    
    def test_skill_requirements(self, accessor):
        """Test extraction of skill requirements from feat.2da"""
        row = {
            'Label': 'SkillFocus',
            'REQSKILL': '8',
            'ReqSkillMinRanks': '5',
            'REQSKILL2': '****',
            'ReqSkillMinRanks2': '****'
        }
        
        reqs = accessor.get_requirements('feat', row)
        assert 'required_skills' in reqs
        assert len(reqs['required_skills']) == 1
        assert reqs['required_skills'][0] == {'id': 8, 'ranks': 5}
    
    def test_cls_pres_requirements(self, accessor):
        """Test extraction from cls_pres_*.2da format"""
        # Test BAB requirement
        row = {'LABEL': 'Base_Attack', 'ReqType': 'BAB', 'ReqParam1': '5', 'ReqParam2': '****'}
        reqs = accessor.get_requirements('cls_pres_wm', row)
        assert reqs == {}  # BAB not handled in current implementation
        
        # Test FEAT requirement
        row = {'LABEL': 'Weapon_Focus', 'ReqType': 'FEAT', 'ReqParam1': '100', 'ReqParam2': '****'}
        reqs = accessor.get_requirements('cls_pres_wm', row)
        assert 'prereq_feats' in reqs
        assert reqs['prereq_feats']['all_of'] == [100]
        
        # Test SKILL requirement
        row = {'LABEL': 'Intimidate', 'ReqType': 'SKILL', 'ReqParam1': '24', 'ReqParam2': '4'}
        reqs = accessor.get_requirements('cls_pres_wm', row)
        assert 'required_skills' in reqs
        assert reqs['required_skills'][0] == {'id': 24, 'ranks': 4}
    
    def test_null_value_handling(self, accessor):
        """Test that **** values are properly ignored"""
        row = {
            'MINSTR': '****',
            'MINDEX': '****',
            'PREREQFEAT1': '****',
            'MinLevel': '****'
        }
        
        reqs = accessor.get_requirements('feat', row)
        assert reqs == {}  # Should have no requirements
    
    def test_invalid_value_handling(self, accessor, caplog):
        """Test handling of invalid values with logging"""
        row = {
            'MINSTR': 'invalid',
            'MINDEX': '13.5',
            'MinLevel': 'abc'
        }
        
        reqs = accessor.get_requirements('feat', row)
        assert reqs == {}  # Should skip invalid values
        
        # Check that warnings were logged
        assert 'Could not parse value' in caplog.text


class TestCaching:
    """Test caching functionality"""
    
    def test_column_analysis_cached(self, accessor):
        """Test that column analysis is cached after first use"""
        # First call should analyze
        accessor._analyze_and_cache_columns('feat')
        assert 'feat' in accessor._cache
        
        # Modify cache to test it's being used
        accessor._cache['feat']['map']['TEST'] = 'test_purpose'
        
        # Second call should use cache
        accessor._analyze_and_cache_columns('feat')
        assert accessor._cache['feat']['map']['TEST'] == 'test_purpose'
    
    def test_missing_table_handling(self, accessor):
        """Test handling of non-existent tables"""
        accessor._analyze_and_cache_columns('nonexistent')
        
        cache = accessor._cache['nonexistent']
        assert cache['map'] == {}
        assert cache['type'] is None


class TestEdgeCases:
    """Test edge cases and error conditions"""
    
    def test_empty_row(self, accessor):
        """Test handling of empty row data"""
        reqs = accessor.get_requirements('feat', {})
        assert reqs == {}
    
    def test_none_values(self, accessor):
        """Test handling of None values"""
        row = {
            'MINSTR': None,
            'PREREQFEAT1': None,
            'MinLevel': None
        }
        
        reqs = accessor.get_requirements('feat', row)
        assert reqs == {}
    
    def test_mixed_case_columns(self, accessor):
        """Test that pattern matching is case-insensitive"""
        # Create mock table with mixed case columns
        headers = ['minstr', 'MinDex', 'MININT', 'prereqFeat1']
        rows = [{'minstr': '13', 'MinDex': '14', 'MININT': '15', 'prereqFeat1': '100'}]
        
        mock_2da = MockTwoDA(rows, headers)
        accessor.rm.get_2da_with_overrides.return_value = mock_2da
        
        accessor._analyze_and_cache_columns('test_mixed')
        cache = accessor._cache['test_mixed']
        
        # All should be detected despite case differences
        assert 'minstr' in cache['map']
        assert 'MinDex' in cache['map']  
        assert 'MININT' in cache['map']
        assert 'prereqFeat1' in cache['map']


class TestRealWorldScenarios:
    """Test real-world usage scenarios"""
    
    def test_paladin_requirements(self, accessor):
        """Test checking Paladin class requirements (Lawful Good only)"""
        # Paladin row from classes.2da
        row = {
            'Label': 'Paladin',
            'AlignRestrict': '0x01',
            'AlignRstrctType': '0x01'
        }
        
        reqs = accessor.get_requirements('classes', row)
        assert 'alignment_restrict' in reqs
        assert reqs['alignment_restrict'] == 1  # 0x01
    
    def test_weapon_master_full_requirements(self, accessor):
        """Test complete Weapon Master prestige class requirements"""
        # Get all requirements from cls_pres_wm
        all_reqs = {}
        
        for row in accessor.rm.get_2da_with_overrides('cls_pres_wm').get_rows_as_dicts():
            reqs = accessor.get_requirements('cls_pres_wm', row)
            # Merge requirements
            for key, value in reqs.items():
                if key == 'prereq_feats':
                    all_reqs.setdefault('prereq_feats', {'all_of': []})
                    all_reqs['prereq_feats']['all_of'].extend(value.get('all_of', []))
                elif key == 'required_skills':
                    all_reqs.setdefault('required_skills', []).extend(value)
                else:
                    all_reqs[key] = value
        
        # Should have feat and skill requirements
        assert 'prereq_feats' in all_reqs
        assert 100 in all_reqs['prereq_feats']['all_of']  # Weapon Focus
        assert 'required_skills' in all_reqs
        assert any(s['id'] == 24 and s['ranks'] == 4 for s in all_reqs['required_skills'])  # Intimidate 4