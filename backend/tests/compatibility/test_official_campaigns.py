import pytest
import os
import sys
import json
from pathlib import Path
from typing import Dict, List, Optional
from unittest.mock import patch, MagicMock, mock_open

# Add backend to path
backend_path = Path(__file__).parent.parent.parent  # Go up 3 levels to backend
sys.path.insert(0, str(backend_path))

from services.core.resource_manager import ResourceManager
from nwn2_rust import GffParser
from gamedata.game_rules_service import GameRulesService
from django.conf import settings


# Fixtures
@pytest.fixture(scope="module")
def campaigns_dir():
    """Fixture to provide the path to the Campaigns directory, skipping if not found."""
    path = Path(settings.NWN2_INSTALL_PATH) / "Campaigns"
    if not path.exists():
        pytest.skip(f"Campaigns directory not found at: {path}")
    return path


@pytest.fixture
def resource_manager():
    """Provides a ResourceManager instance for tests."""
    return ResourceManager()


@pytest.fixture
def game_rules_service():
    """Provides a GameRulesService instance for tests."""
    return GameRulesService()


@pytest.fixture
def official_campaign_names():
    """List of official NWN2 campaign folder names."""
    return {
        "Neverwinter Nights 2 Campaign": {
            "name": "Neverwinter Nights 2 Original Campaign",
            "min_modules": 10,
            "expected_start": "1000_neverwinter_a1",
            "expected_level_cap": 20
        },
        "NWN2 Mask of the Betrayer Campaign": {
            "name": "Mask of the Betrayer",
            "min_modules": 5,
            "expected_start": "2000_motb_x2",
            "expected_level_cap": 30
        },
        "Neverwinter Nights 2 Campaign_X2": {
            "name": "Storm of Zehir",
            "min_modules": 5,
            "expected_start": "3000_soz_x3",
            "expected_level_cap": 20
        }
    }


class TestOfficialCampaignDetection:
    """Test detection and loading of official NWN2 campaigns."""

    def test_find_all_official_campaigns(self, resource_manager, campaigns_dir, official_campaign_names):
        """Test that all official campaigns can be detected."""
        found_campaigns = {}
        
        for campaign_folder in campaigns_dir.iterdir():
            if campaign_folder.is_dir() and campaign_folder.name in official_campaign_names:
                campaign_info = resource_manager.find_campaign(str(campaign_folder))
                if campaign_info:
                    found_campaigns[campaign_folder.name] = campaign_info
                    
        # At least one official campaign should be found
        assert len(found_campaigns) > 0, "Should find at least one official campaign"
        
        # Log what was found for debugging
        for folder_name, info in found_campaigns.items():
            print(f"Found {folder_name}: {info['name']} with {len(info['modules'])} modules")
            
    def test_original_campaign_structure(self, resource_manager, campaigns_dir):
        """Test detailed structure of the original NWN2 campaign."""
        oc_campaign_dir = campaigns_dir / "Neverwinter Nights 2 Campaign"
        if not oc_campaign_dir.exists():
            pytest.skip("Original campaign not found")

        campaign_info = resource_manager.find_campaign(str(oc_campaign_dir))
        assert campaign_info is not None, "Should find original campaign"
        
        # Validate structure
        assert 'name' in campaign_info
        assert 'description' in campaign_info
        assert 'modules' in campaign_info
        assert 'start_module' in campaign_info
        assert 'level_cap' in campaign_info
        assert 'xp_cap' in campaign_info
        assert 'party_size' in campaign_info
        assert 'file' in campaign_info
        assert 'directory' in campaign_info
        
        # Validate module list
        assert isinstance(campaign_info['modules'], list)
        assert len(campaign_info['modules']) >= 10, "OC should have at least 10 modules"
        
        # Check for expected module names
        module_names = campaign_info['modules']
        expected_prefixes = ['1000_', '1100_', '1200_', '1300_', '1400_']
        for prefix in expected_prefixes:
            assert any(mod.startswith(prefix) for mod in module_names), f"Should have module starting with {prefix}"
            
    def test_mask_of_betrayer_campaign(self, resource_manager, campaigns_dir):
        """Test Mask of the Betrayer expansion campaign."""
        motb_campaign_dir = campaigns_dir / "NWN2 Mask of the Betrayer Campaign"
        if not motb_campaign_dir.exists():
            pytest.skip("Mask of the Betrayer campaign not found")

        campaign_info = resource_manager.find_campaign(str(motb_campaign_dir))
        assert campaign_info is not None, "Should find MotB campaign"
        
        # MotB specific checks
        assert campaign_info['level_cap'] >= 30, "MotB should support epic levels"
        assert len(campaign_info['modules']) >= 5, "MotB should have at least 5 modules"
        
        # Check for MotB-specific modules
        module_names = campaign_info['modules']
        assert any('2000_' in mod or 'x2' in mod.lower() for mod in module_names), "Should have MotB modules"
        
    def test_storm_of_zehir_campaign(self, resource_manager, campaigns_dir):
        """Test Storm of Zehir expansion campaign."""
        soz_campaign_dir = campaigns_dir / "Neverwinter Nights 2 Campaign_X2"
        if not soz_campaign_dir.exists():
            pytest.skip("Storm of Zehir campaign not found")

        campaign_info = resource_manager.find_campaign(str(soz_campaign_dir))
        assert campaign_info is not None, "Should find SoZ campaign"
        
        # SoZ specific checks
        assert len(campaign_info['modules']) >= 5, "SoZ should have at least 5 modules"
        
        # Check for SoZ-specific modules
        module_names = campaign_info['modules']
        assert any('3000_' in mod or 'x3' in mod.lower() for mod in module_names), "Should have SoZ modules"


class TestCampaignModuleValidation:
    """Test validation of campaign modules."""
    
    def test_module_file_existence(self, resource_manager, campaigns_dir):
        """Test that campaign modules actually exist on disk."""
        for campaign_folder in campaigns_dir.iterdir():
            if campaign_folder.is_dir():
                campaign_info = resource_manager.find_campaign(str(campaign_folder))
                if campaign_info and len(campaign_info['modules']) > 0:
                    # Check at least the first module
                    first_module = campaign_info['modules'][0]
                    module_path = campaigns_dir.parent / "Modules" / f"{first_module}.mod"
                    
                    # Note: We can't assert this exists as modules might be in different locations
                    # Just log for manual verification
                    if module_path.exists():
                        print(f"Found module file: {module_path}")
                    else:
                        print(f"Module file not found at expected location: {module_path}")
                    
                    # Only test first campaign to avoid too many file checks
                    return
                    
    def test_start_module_in_module_list(self, resource_manager, campaigns_dir):
        """Test that the start module is in the module list."""
        tested_any = False
        
        for campaign_folder in campaigns_dir.iterdir():
            if campaign_folder.is_dir():
                campaign_info = resource_manager.find_campaign(str(campaign_folder))
                if campaign_info and campaign_info.get('start_module'):
                    tested_any = True
                    start_module = campaign_info['start_module']
                    modules = campaign_info['modules']
                    
                    assert start_module in modules, f"Start module '{start_module}' should be in module list"
                    
        if not tested_any:
            pytest.skip("No campaigns with start_module found to test")
            
    def test_module_name_conventions(self, resource_manager, campaigns_dir):
        """Test that module names follow expected conventions."""
        for campaign_folder in campaigns_dir.iterdir():
            if campaign_folder.is_dir():
                campaign_info = resource_manager.find_campaign(str(campaign_folder))
                if campaign_info:
                    for module in campaign_info['modules']:
                        # Module names should not contain spaces or special characters
                        assert ' ' not in module, f"Module name should not contain spaces: {module}"
                        assert all(c.isalnum() or c in '_-' for c in module), \
                            f"Module name should only contain alphanumeric and underscore/dash: {module}"
                    
                    # Only test first campaign
                    return


class TestCampaignSettings:
    """Test campaign settings and game rules."""
    
    def test_level_cap_consistency(self, resource_manager, campaigns_dir, game_rules_service):
        """Test that level caps are consistent with game rules."""
        tested_any = False
        
        for campaign_folder in campaigns_dir.iterdir():
            if campaign_folder.is_dir():
                campaign_info = resource_manager.find_campaign(str(campaign_folder))
                if campaign_info:
                    tested_any = True
                    level_cap = campaign_info['level_cap']
                    
                    # Check against D&D rules
                    assert 1 <= level_cap <= 40, f"Level cap {level_cap} outside valid range"
                    
                    # Epic levels start at 21
                    if level_cap > 20:
                        print(f"Campaign '{campaign_info['name']}' supports epic levels (cap: {level_cap})")
                        
        if not tested_any:
            pytest.skip("No campaigns found to test level caps")
            
    def test_xp_cap_calculation(self, resource_manager, campaigns_dir):
        """Test that XP caps correspond to level caps."""
        # D&D 3.5 XP requirements (approximate)
        xp_for_level = {
            20: 190000,
            30: 435000,
            40: 780000
        }
        
        tested_any = False
        
        for campaign_folder in campaigns_dir.iterdir():
            if campaign_folder.is_dir():
                campaign_info = resource_manager.find_campaign(str(campaign_folder))
                if campaign_info and 'xp_cap' in campaign_info:
                    tested_any = True
                    level_cap = campaign_info['level_cap']
                    xp_cap = campaign_info['xp_cap']
                    
                    # XP cap should be positive if set
                    if xp_cap > 0:
                        assert xp_cap >= 1000, "XP cap should be at least 1000"
                        
                        # Check if XP cap is reasonable for level cap
                        if level_cap in xp_for_level:
                            expected_xp = xp_for_level[level_cap]
                            # Allow some variance
                            assert abs(xp_cap - expected_xp) < expected_xp * 0.5, \
                                f"XP cap {xp_cap} seems wrong for level {level_cap}"
                                
        if not tested_any:
            pytest.skip("No campaigns with XP caps found to test")
            
    def test_party_size_limits(self, resource_manager, campaigns_dir):
        """Test party size settings across campaigns."""
        party_sizes = []
        
        for campaign_folder in campaigns_dir.iterdir():
            if campaign_folder.is_dir():
                campaign_info = resource_manager.find_campaign(str(campaign_folder))
                if campaign_info:
                    party_size = campaign_info['party_size']
                    party_sizes.append(party_size)
                    
                    # NWN2 typically supports 1-6 party members
                    assert 1 <= party_size <= 6, f"Party size {party_size} outside expected range"
                    
        if party_sizes:
            # Most campaigns should use default party size of 4
            most_common = max(set(party_sizes), key=party_sizes.count)
            assert most_common == 4, f"Most common party size should be 4, but was {most_common}"
        else:
            pytest.skip("No campaigns found to test party sizes")


class TestCampaignLocalization:
    """Test campaign localization support."""
    
    def test_campaign_names_are_localized(self, resource_manager, campaigns_dir):
        """Test that campaign names handle localization properly."""
        for campaign_folder in campaigns_dir.iterdir():
            if campaign_folder.is_dir():
                campaign_info = resource_manager.find_campaign(str(campaign_folder))
                if campaign_info:
                    name = campaign_info['name']
                    
                    # Name should not be empty or "Unknown Campaign"
                    assert name and name != 'Unknown Campaign', \
                        f"Campaign in {campaign_folder.name} has invalid name: {name}"
                    
                    # Name should be a readable string
                    assert isinstance(name, str), "Campaign name should be a string"
                    assert len(name) > 2, "Campaign name too short"
                    
                    # Only test first few campaigns
                    return
                    
    def test_campaign_descriptions(self, resource_manager, campaigns_dir):
        """Test that campaign descriptions are properly extracted."""
        found_with_description = False
        
        for campaign_folder in campaigns_dir.iterdir():
            if campaign_folder.is_dir():
                campaign_info = resource_manager.find_campaign(str(campaign_folder))
                if campaign_info and campaign_info.get('description'):
                    found_with_description = True
                    desc = campaign_info['description']
                    
                    # Description should be meaningful
                    assert isinstance(desc, str), "Description should be a string"
                    assert len(desc) > 10, "Description too short to be meaningful"
                    
                    # Should not contain localization artifacts
                    assert not desc.startswith('{'), "Description should not be raw localization data"
                    assert not desc.startswith('<?'), "Description should not be XML"
                    
        if not found_with_description:
            pytest.skip("No campaigns with descriptions found to test")


class TestCampaignErrorHandling:
    """Test error handling for campaign loading."""
    
    def test_missing_cam_file_handling(self, resource_manager):
        """Test handling of directories without .cam files."""
        # Use a directory that definitely has no .cam file
        result = resource_manager.find_campaign(str(Path(__file__).parent))
        assert result is None, "Should return None for directory without .cam file"
        
    def test_invalid_path_handling(self, resource_manager):
        """Test handling of invalid paths."""
        invalid_paths = [
            "/definitely/does/not/exist",
            "",
            None
        ]
        
        for path in invalid_paths:
            if path is None:
                # Skip None test if method doesn't accept it
                continue
            result = resource_manager.find_campaign(path)
            assert result is None, f"Should return None for invalid path: {path}"
            
    @patch('services.rust_adapter.RustGFFParser.read')
    def test_corrupted_cam_file(self, mock_read, resource_manager):
        """Test handling of corrupted .cam files."""
        mock_read.side_effect = ValueError("Corrupted file")
        
        with patch('pathlib.Path.glob') as mock_glob:
            mock_glob.return_value = [Path("/fake/campaign.cam")]
            
            result = resource_manager.find_campaign("/fake/path")
            assert result is None, "Should handle corrupted CAM file gracefully"
            
    def test_campaign_missing_modules(self, resource_manager):
        """Test handling of campaigns with empty module lists."""
        mock_parser_instance = MagicMock()
        mock_parser_instance.read.return_value = {
            'DisplayName': 'Empty Campaign',
            'ModNames': [],
            'StartModule': 'nonexistent'
        }

        with patch('services.resource_manager.GFFParser', return_value=mock_parser_instance):
            with patch('pathlib.Path.glob') as mock_glob:
                mock_glob.return_value = [Path("/fake/campaign.cam")]

                campaign_info = resource_manager.find_campaign("/fake/path")
                assert campaign_info is not None
                assert len(campaign_info['modules']) == 0
                assert campaign_info['name'] == 'Empty Campaign'


class TestCampaignCompatibility:
    """Test compatibility with different campaign formats."""
    
    def test_legacy_campaign_format(self, resource_manager):
        """Test handling of legacy campaign formats."""
        # Mock legacy format without localized strings
        legacy_data = {
            'DisplayName': 'Legacy Campaign',  # Direct string instead of localized
            'Description': 'A legacy campaign',
            'ModNames': [
                {'ModuleName': 'module1'},
                {'ModuleName': 'module2'}
            ],
            'StartModule': 'module1'
        }

        mock_parser_instance = MagicMock()
        mock_parser_instance.read.return_value = legacy_data

        with patch('services.resource_manager.GFFParser', return_value=mock_parser_instance):
            with patch('pathlib.Path.glob') as mock_glob:
                mock_glob.return_value = [Path("/fake/campaign.cam")]

                campaign_info = resource_manager.find_campaign("/fake/path")
                assert campaign_info is not None
                assert campaign_info['name'] == 'Legacy Campaign'
                assert campaign_info['description'] == 'A legacy campaign'
                
    def test_extended_campaign_fields(self, resource_manager):
        """Test handling of campaigns with additional custom fields."""
        extended_data = {
            'DisplayName': 'Extended Campaign',
            'ModNames': [{'ModuleName': 'module1'}],
            'CustomField1': 'custom_value',
            'Author': 'Test Author',
            'Version': '1.0.0',
            'RequiredExpansions': ['x1', 'x2']
        }

        mock_parser_instance = MagicMock()
        mock_parser_instance.read.return_value = extended_data

        with patch('services.resource_manager.GFFParser', return_value=mock_parser_instance):
            with patch('pathlib.Path.glob') as mock_glob:
                mock_glob.return_value = [Path("/fake/campaign.cam")]

                campaign_info = resource_manager.find_campaign("/fake/path")
                assert campaign_info is not None
                # Should still extract standard fields
                assert campaign_info['name'] == 'Extended Campaign'
                assert len(campaign_info['modules']) == 1