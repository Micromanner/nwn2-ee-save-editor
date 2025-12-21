import pytest
import os
import sys
import json
from pathlib import Path
from typing import Dict, List, Optional
from unittest.mock import patch, MagicMock

# Add backend to path
backend_path = Path(__file__).parent.parent.parent  # Go up 3 levels to backend
sys.path.insert(0, str(backend_path))

from services.resource_manager import ResourceManager
from nwn2_rust import GffParser


# Test Data Fixtures
@pytest.fixture
def resource_manager():
    """Provide a ResourceManager instance"""
    return ResourceManager()


@pytest.fixture
def gff_parser():
    """Provide a GFFParser instance"""
    return GFFParser()


@pytest.fixture
def test_campaigns_dir():
    """Path to test campaigns directory"""
    return Path(__file__).parent.parent / "test_module/Remaster Campaign Files/Campaigns"


@pytest.fixture
def path_of_evil_dir(test_campaigns_dir):
    """Path to Path of Evil campaign directory"""
    return test_campaigns_dir / "Path of Evil"


@pytest.fixture
def path_of_evil_cam_file(path_of_evil_dir):
    """Path to Path of Evil campaign.cam file"""
    return path_of_evil_dir / "Campaign.cam"


@pytest.fixture
def mock_cam_data():
    """Mock CAM file data structure"""
    return {
        'DisplayName': {
            'substrings': [{'string': 'Path of Evil'}]
        },
        'Description': {
            'substrings': [{'string': 'Heroes have to start somewhere...'}]
        },
        'StartModule': 'poe_intro',
        'ModNames': [
            {'ModuleName': 'poe_intro'},
            {'ModuleName': 'poe_final'},
            {'ModuleName': 'poe_halruaa'},
            {'ModuleName': 'poe_murann'},
            {'ModuleName': 'poe_muzad'},
            {'ModuleName': 'poe_pros'},
            {'ModuleName': 'poe_qasr'},
            {'ModuleName': 'poe_sidequests'},
            {'ModuleName': 'poe_stronghold'},
            {'ModuleName': 'poe_mpass'},
            {'ModuleName': 'poe_overland_west'},
            {'ModuleName': 'poe_overland_east'}
        ],
        'LvlCap': 30,
        'XPCap': 435000,
        'Cam_PartySize': 4,
        'TlkCustom': 'poe_custom',
        'Author': 'John Doe',
        'StartingXP': 0,
        'AllowXPScaling': 1
    }


class TestCamFileStructure:
    """Test CAM file structure parsing and validation"""
    
    def test_read_campaign_file_directly(self, gff_parser, path_of_evil_cam_file):
        """Test reading a .cam file directly with GFFParser"""
        if not path_of_evil_cam_file.exists():
            pytest.skip("Test campaign file not found")
            
        # Read with GFFParser
        campaign_data = gff_parser.read(str(path_of_evil_cam_file))
        
        # Check essential fields exist
        assert 'DisplayName' in campaign_data
        assert 'Description' in campaign_data
        assert 'StartModule' in campaign_data
        assert 'ModNames' in campaign_data
        assert 'LvlCap' in campaign_data
        
        # Check module list structure
        modules = campaign_data.get('ModNames', [])
        assert isinstance(modules, list)
        assert len(modules) > 0, "Campaign should have at least one module"
        
        # Validate module list entries
        for module in modules:
            assert isinstance(module, dict), "Each module should be a dict"
            assert 'ModuleName' in module, "Each module should have ModuleName field"
            
    def test_cam_file_version_compatibility(self, gff_parser, path_of_evil_cam_file):
        """Test CAM file version handling"""
        if not path_of_evil_cam_file.exists():
            pytest.skip("Test campaign file not found")
            
        gff_parser.read(str(path_of_evil_cam_file))
        
        # Check GFF version
        assert gff_parser.file_version == "V3.2", "CAM files should use GFF V3.2"
        assert gff_parser.file_type == "CAM ", "File type should be CAM with space"
        
    def test_localized_string_extraction(self, resource_manager, mock_cam_data):
        """Test extraction of localized strings from CAM data"""
        mock_parser_instance = MagicMock()
        mock_parser_instance.read.return_value = mock_cam_data

        with patch('services.resource_manager.GFFParser', return_value=mock_parser_instance):
            with patch('pathlib.Path.glob') as mock_glob:
                mock_glob.return_value = [Path("/fake/campaign.cam")]
                campaign_info = resource_manager.find_campaign("/fake/path")

        assert campaign_info['name'] == 'Path of Evil'
        assert 'Heroes have to start somewhere' in campaign_info['description']


class TestResourceManagerCampaignLoading:
    """Test ResourceManager campaign loading functionality"""
    
    def test_find_campaign_success(self, resource_manager, path_of_evil_dir):
        """Test finding campaign information from directory"""
        if not path_of_evil_dir.exists():
            pytest.skip("Test campaign directory not found")
            
        campaign_info = resource_manager.find_campaign(str(path_of_evil_dir))
        
        assert campaign_info is not None, "Should find campaign"
        assert campaign_info['name'] == 'Path of Evil'
        assert "Heroes have to start somewhere" in campaign_info['description']
        assert campaign_info['start_module'] == 'poe_intro'
        assert len(campaign_info['modules']) == 12
        assert campaign_info['level_cap'] == 30
        
    def test_campaign_module_list_order(self, resource_manager, path_of_evil_dir):
        """Test that module list maintains proper order"""
        if not path_of_evil_dir.exists():
            pytest.skip("Test campaign directory not found")
            
        campaign_info = resource_manager.find_campaign(str(path_of_evil_dir))
        
        # Check module names and order
        expected_modules = [
            'poe_intro', 'poe_final', 'poe_halruaa', 'poe_murann',
            'poe_muzad', 'poe_pros', 'poe_qasr', 'poe_sidequests',
            'poe_stronghold', 'poe_mpass', 'poe_overland_west', 'poe_overland_east'
        ]
        
        assert campaign_info['modules'] == expected_modules
        
    def test_campaign_additional_fields(self, resource_manager, path_of_evil_dir):
        """Test extraction of additional campaign fields"""
        if not path_of_evil_dir.exists():
            pytest.skip("Test campaign directory not found")
            
        campaign_info = resource_manager.find_campaign(str(path_of_evil_dir))
        
        # Check additional fields
        assert 'xp_cap' in campaign_info
        assert 'party_size' in campaign_info
        assert campaign_info['party_size'] >= 1 and campaign_info['party_size'] <= 6
        
    def test_campaign_not_found(self, resource_manager):
        """Test behavior when no campaign file exists"""
        fake_dir = Path(__file__).parent.parent / "nonexistent_campaign"
        
        campaign_info = resource_manager.find_campaign(str(fake_dir))
        assert campaign_info is None, "Should return None for non-existent campaign"
        
    def test_find_campaign_in_subdirectories(self, resource_manager, test_campaigns_dir):
        """Test finding campaign files in subdirectories"""
        if not test_campaigns_dir.exists():
            pytest.skip("Campaigns root directory not found")
            
        found_campaigns = []
        for campaign_folder in test_campaigns_dir.iterdir():
            if campaign_folder.is_dir():
                campaign_info = resource_manager.find_campaign(str(campaign_folder))
                if campaign_info:
                    found_campaigns.append(campaign_info['name'])
                    
        assert 'Path of Evil' in found_campaigns, "Should find Path of Evil campaign"
        
    def test_campaign_with_nested_cam_file(self, resource_manager):
        """Test finding .cam file in nested directory structure"""
        with patch('pathlib.Path.glob') as mock_glob:
            # Mock finding cam file in subdirectory
            mock_cam_path = MagicMock()
            mock_cam_path.exists.return_value = True
            mock_glob.side_effect = [[], [mock_cam_path]]  # First glob fails, second succeeds

            mock_parser_instance = MagicMock()
            mock_parser_instance.read.return_value = {'DisplayName': 'Test', 'ModNames': []}

            with patch('services.resource_manager.GFFParser', return_value=mock_parser_instance):
                campaign_info = resource_manager.find_campaign("/fake/path")
                assert campaign_info is not None


class TestCamFileErrorHandling:
    """Test error handling for malformed/corrupted CAM files"""
    
    def test_corrupted_cam_file(self, resource_manager):
        """Test handling of corrupted CAM file"""
        with patch.object(GFFParser, 'read') as mock_read:
            mock_read.side_effect = ValueError("Invalid GFF header")
            
            campaign_info = resource_manager.find_campaign("/fake/path")
            assert campaign_info is None, "Should return None for corrupted file"
            
    def test_cam_file_missing_required_fields(self, resource_manager):
        """Test handling CAM file missing required fields"""
        mock_parser_instance = MagicMock()
        mock_parser_instance.read.return_value = {
            'Description': 'Test',
            'ModNames': []
        }

        with patch('services.resource_manager.GFFParser', return_value=mock_parser_instance):
            with patch('pathlib.Path.glob') as mock_glob:
                mock_glob.return_value = [Path("/fake/campaign.cam")]
                campaign_info = resource_manager.find_campaign("/fake/path")

                # Should still return something but with defaults
                assert campaign_info is not None
                assert campaign_info['name'] == 'Unknown Campaign'
                
    def test_malformed_module_list(self, resource_manager):
        """Test handling malformed module list in CAM file"""
        mock_parser_instance = MagicMock()
        mock_parser_instance.read.return_value = {
            'DisplayName': 'Test Campaign',
            'ModNames': [
                {'ModuleName': 'module1'},
                'invalid_entry',  # String instead of dict
                {'WrongKey': 'module3'},  # Missing ModuleName
                {'ModuleName': 'module4'}
            ]
        }

        with patch('services.resource_manager.GFFParser', return_value=mock_parser_instance):
            with patch('pathlib.Path.glob') as mock_glob:
                mock_glob.return_value = [Path("/fake/campaign.cam")]
                campaign_info = resource_manager.find_campaign("/fake/path")

                # Should only include valid modules
                assert len(campaign_info['modules']) == 2
                assert campaign_info['modules'] == ['module1', 'module4']
                
    def test_permission_error_handling(self, resource_manager):
        """Test handling permission errors when reading CAM file"""
        with patch('pathlib.Path.glob') as mock_glob:
            mock_path = MagicMock()
            mock_glob.return_value = [mock_path]
            
            with patch.object(GFFParser, 'read') as mock_read:
                mock_read.side_effect = PermissionError("Access denied")
                
                campaign_info = resource_manager.find_campaign("/fake/path")
                assert campaign_info is None, "Should return None on permission error"


class TestCampaignValidation:
    """Test campaign data validation"""
    
    def test_validate_level_cap_range(self, resource_manager, mock_cam_data):
        """Test level cap validation"""
        test_cases = [
            (0, 0),      # Zero stored as-is (no validation currently)
            (1, 1),      # Minimum valid
            (20, 20),    # Default
            (30, 30),    # Common max
            (40, 40),    # Maximum valid
            (50, 50),    # Above common max but stored as-is
            (-5, -5),    # Negative stored as-is (no validation currently)
        ]

        for input_cap, expected_cap in test_cases:
            mock_cam_data['LvlCap'] = input_cap

            mock_parser_instance = MagicMock()
            mock_parser_instance.read.return_value = mock_cam_data

            with patch('services.resource_manager.GFFParser', return_value=mock_parser_instance):
                with patch('pathlib.Path.glob') as mock_glob:
                    mock_glob.return_value = [Path("/fake/campaign.cam")]
                    campaign_info = resource_manager.find_campaign("/fake/path")

            assert campaign_info['level_cap'] == expected_cap
                
    def test_validate_party_size(self, resource_manager, mock_cam_data):
        """Test party size validation"""
        test_cases = [
            (0, 0),      # Zero stored as-is
            (1, 1),      # Minimum valid
            (4, 4),      # Default
            (6, 6),      # Maximum valid
            (10, 10),    # Above typical max but stored as-is
            (-1, -1),    # Negative stored as-is (validation happens elsewhere)
        ]

        for input_size, expected_size in test_cases:
            mock_cam_data['Cam_PartySize'] = input_size

            mock_parser_instance = MagicMock()
            mock_parser_instance.read.return_value = mock_cam_data

            with patch('services.resource_manager.GFFParser', return_value=mock_parser_instance):
                with patch('pathlib.Path.glob') as mock_glob:
                    mock_glob.return_value = [Path("/fake/campaign.cam")]
                    campaign_info = resource_manager.find_campaign("/fake/path")

            assert campaign_info['party_size'] == expected_size


class TestCampaignPerformance:
    """Test performance aspects of campaign loading"""
    
    def test_large_module_list_performance(self, resource_manager):
        """Test performance with campaigns having many modules"""
        # Create mock data with many modules
        large_module_list = [
            {'ModuleName': f'module_{i}'} for i in range(100)
        ]

        mock_cam_data = {
            'DisplayName': 'Large Campaign',
            'ModNames': large_module_list,
            'StartModule': 'module_0'
        }

        mock_parser_instance = MagicMock()
        mock_parser_instance.read.return_value = mock_cam_data

        with patch('services.resource_manager.GFFParser', return_value=mock_parser_instance):
            with patch('pathlib.Path.glob') as mock_glob:
                mock_glob.return_value = [Path("/fake/campaign.cam")]

                import time
                start_time = time.time()
                campaign_info = resource_manager.find_campaign("/fake/path")
                elapsed_time = time.time() - start_time

                assert len(campaign_info['modules']) == 100
                assert elapsed_time < 0.1, "Should process large module list quickly"
                
    def test_multiple_cam_files_in_directory(self, resource_manager):
        """Test behavior when multiple .cam files exist"""
        with patch('pathlib.Path.glob') as mock_glob:
            # Multiple cam files found
            mock_paths = [
                Path("/fake/campaign1.cam"),
                Path("/fake/campaign2.cam"),
                Path("/fake/campaign3.cam")
            ]
            mock_glob.return_value = mock_paths

            mock_parser_instance = MagicMock()
            mock_parser_instance.read.return_value = {
                'DisplayName': 'First Campaign',
                'ModNames': []
            }

            with patch('services.resource_manager.GFFParser', return_value=mock_parser_instance):
                campaign_info = resource_manager.find_campaign("/fake/path")

                # Should use first cam file found
                assert campaign_info is not None
                assert campaign_info['name'] == 'First Campaign'
                assert campaign_info['file'] == str(mock_paths[0])