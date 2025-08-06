"""
Test for Storm Lord expansion loading fix

This test verifies that the expansion classes (particularly Storm Lord at class ID 56)
load correctly from the X2 expansion ZIP file.
"""

import pytest
import os
import sys
import tempfile
import zipfile
from pathlib import Path
from unittest.mock import Mock, patch

from parsers.resource_manager import ResourceManager
from parsers import TDAParser
from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader

# Add backend directory to Python path for imports
backend_dir = Path(__file__).parent.parent.parent
sys.path.insert(0, str(backend_dir))

import django
os.environ.setdefault('DJANGO_SETTINGS_MODULE', 'config.settings')
django.setup()


@pytest.fixture
def mock_expansion_data():
    """Create mock expansion ZIP data with Storm Lord"""
    
    # Base game classes (simplified)
    base_classes_content = """2DA V2.30

   Label     Name  PlayerClass  PlayerChar  Description  PrimaryAbil  AlignRestrict  AlignRstrctType  InvertRestrict  Constant  
0  barbarian  378  1            1           379          0            0              0                0               1         
1  bard       381  1            1           382          5            0              0                0               2         
2  cleric     383  1            1           384          4            0              0                0               3         
""".strip()

    # X1 classes (adds some prestige classes)
    x1_classes_content = """2DA V2.30

   Label     Name  PlayerClass  PlayerChar  Description  PrimaryAbil  AlignRestrict  AlignRstrctType  InvertRestrict  Constant  
0  barbarian  378  1            1           379          0            0              0                0               1         
1  bard       381  1            1           382          5            0              0                0               2         
2  cleric     383  1            1           384          4            0              0                0               3         
55 padding   ****  0            0           ****         ****         ****           ****             ****            ****      
56 padding   ****  0            0           ****         ****         ****           ****             ****            ****      
""".strip()

    # X2 classes (adds Storm Lord at class ID 56)
    x2_classes_content = """2DA V2.30

   Label      Name  PlayerClass  PlayerChar  Description  PrimaryAbil  AlignRestrict  AlignRstrctType  InvertRestrict  Constant  
0  barbarian   378  1            1           379          0            0              0                0               1         
1  bard        381  1            1           382          5            0              0                0               2         
2  cleric      383  1            1           384          4            0              0                0               3         
55 padding    ****  0            0           ****         ****         ****           ****             ****            ****      
56 stormlord  999  1            1           1000         4            0              0                0               CLS_SAVTHR_CLER
""".strip()

    return {
        'base_classes': base_classes_content,
        'x1_classes': x1_classes_content,
        'x2_classes': x2_classes_content
    }


@pytest.fixture
def temp_nwn2_with_expansions(tmp_path, mock_expansion_data):
    """Create temporary NWN2 directory with expansion ZIPs"""
    
    # Create NWN2 directory structure
    nwn2_path = tmp_path / "NWN2"
    data_dir = nwn2_path / "data"
    data_dir.mkdir(parents=True)
    
    # Create base game ZIP
    base_zip_path = data_dir / "2da.zip"
    with zipfile.ZipFile(base_zip_path, 'w') as base_zip:
        base_zip.writestr("2DA/classes.2da", mock_expansion_data['base_classes'])
    
    # Create X1 expansion ZIP
    x1_zip_path = data_dir / "2da_x1.zip"
    with zipfile.ZipFile(x1_zip_path, 'w') as x1_zip:
        x1_zip.writestr("2DA_X1/classes.2da", mock_expansion_data['x1_classes'])
    
    # Create X2 expansion ZIP (contains Storm Lord)
    x2_zip_path = data_dir / "2da_x2.zip"
    with zipfile.ZipFile(x2_zip_path, 'w') as x2_zip:
        x2_zip.writestr("2DA_X2/classes.2da", mock_expansion_data['x2_classes'])
    
    return nwn2_path


def test_storm_lord_expansion_loading(temp_nwn2_with_expansions):
    """Test that Storm Lord loads correctly from X2 expansion"""
    
    # Create ResourceManager with temporary NWN2 path
    with patch('config.nwn2_settings.nwn2_paths') as mock_paths:
        mock_paths.game_folder = temp_nwn2_with_expansions
        mock_paths.data = temp_nwn2_with_expansions / "data"
        mock_paths.is_enhanced_edition = False
        mock_paths.enhanced_data = None
        
        rm = ResourceManager(nwn2_path=str(temp_nwn2_with_expansions))
        
        # Test 1: Verify classes.2da is found and loaded from X2 (Storm of Zehir)
        assert 'classes.2da' in rm._2da_locations, "classes.2da should be found in ZIP locations"
        
        zip_path, internal_path = rm._2da_locations['classes.2da']
        assert "2da_x2.zip" in zip_path, f"classes.2da should be loaded from X2 expansion, but was loaded from {zip_path}"
        assert internal_path == "2DA_X2/classes.2da", f"Internal path should be 2DA_X2/classes.2da, got {internal_path}"
        
        # Test 2: Load the classes 2DA and verify Storm Lord exists
        classes_parser = rm.get_2da('classes')
        assert classes_parser is not None, "classes.2da should be loadable"
        
        # Verify we have enough rows to contain Storm Lord
        row_count = classes_parser.get_resource_count()
        assert row_count > 56, f"classes.2da should have more than 56 rows, got {row_count}"
        
        # Test 3: Verify Storm Lord exists at class ID 56
        storm_lord_label = classes_parser.get_string(56, 'Label')
        assert storm_lord_label == 'stormlord', f"Class ID 56 should be 'stormlord', got '{storm_lord_label}'"
        
        # Test 4: Verify Storm Lord has correct data
        storm_lord_name_ref = classes_parser.get_int(56, 'Name')
        assert storm_lord_name_ref == 999, f"Storm Lord Name should be 999, got {storm_lord_name_ref}"
        
        storm_lord_constant = classes_parser.get_string(56, 'Constant')
        assert storm_lord_constant == 'CLS_SAVTHR_CLER', f"Storm Lord Constant should be 'CLS_SAVTHR_CLER', got '{storm_lord_constant}'"


def test_expansion_override_chain(temp_nwn2_with_expansions):
    """Test that the override chain works correctly (base → X1 → X2)"""
    
    with patch('config.nwn2_settings.nwn2_paths') as mock_paths:
        mock_paths.game_folder = temp_nwn2_with_expansions
        mock_paths.data = temp_nwn2_with_expansions / "data"
        mock_paths.is_enhanced_edition = False
        mock_paths.enhanced_data = None
        
        rm = ResourceManager(nwn2_path=str(temp_nwn2_with_expansions))
        
        # Load classes from each ZIP individually to verify override chain
        base_zip = temp_nwn2_with_expansions / "data" / "2da.zip"
        x1_zip = temp_nwn2_with_expansions / "data" / "2da_x1.zip"
        x2_zip = temp_nwn2_with_expansions / "data" / "2da_x2.zip"
        
        # Verify base game doesn't have Storm Lord
        with zipfile.ZipFile(base_zip, 'r') as zf:
            base_content = zf.read("2DA/classes.2da").decode('utf-8')
            assert 'stormlord' not in base_content.lower(), "Base game should not contain Storm Lord"
        
        # Verify X1 doesn't have Storm Lord (only padding)
        with zipfile.ZipFile(x1_zip, 'r') as zf:
            x1_content = zf.read("2DA_X1/classes.2da").decode('utf-8')
            assert 'stormlord' not in x1_content.lower(), "X1 should not contain Storm Lord"
            assert 'padding' in x1_content.lower(), "X1 should contain padding entries"
        
        # Verify X2 has Storm Lord
        with zipfile.ZipFile(x2_zip, 'r') as zf:
            x2_content = zf.read("2DA_X2/classes.2da").decode('utf-8')
            assert 'stormlord' in x2_content.lower(), "X2 should contain Storm Lord"
        
        # Final test: ResourceManager should load X2 version (with Storm Lord)
        classes_parser = rm.get_2da('classes')
        storm_lord_label = classes_parser.get_string(56, 'Label')
        assert storm_lord_label == 'stormlord', "ResourceManager should load Storm Lord from X2"


def test_dynamic_game_data_loader_storm_lord(temp_nwn2_with_expansions):
    """Test that DynamicGameDataLoader correctly loads Storm Lord"""
    
    with patch('config.nwn2_settings.nwn2_paths') as mock_paths:
        mock_paths.game_folder = temp_nwn2_with_expansions
        mock_paths.data = temp_nwn2_with_expansions / "data"
        mock_paths.is_enhanced_edition = False
        mock_paths.enhanced_data = None
        
        # Create ResourceManager
        rm = ResourceManager(nwn2_path=str(temp_nwn2_with_expansions))
        
        # Create DynamicGameDataLoader
        loader = DynamicGameDataLoader(resource_manager=rm, use_async=False)
        
        # Test that classes table contains Storm Lord
        classes_table = loader.get_table('classes')
        assert classes_table is not None, "Classes table should be available"
        assert len(classes_table) > 56, f"Classes table should have more than 56 entries, got {len(classes_table)}"
        
        # Test Storm Lord at class ID 56
        storm_lord_class = classes_table[56]
        assert hasattr(storm_lord_class, 'label'), "Class should have label attribute"
        assert storm_lord_class.label == 'stormlord', f"Class ID 56 should be Storm Lord, got '{storm_lord_class.label}'"
        
        # Verify it's not "padding"
        assert storm_lord_class.label != 'padding', "Storm Lord should not be 'padding'"
        assert getattr(storm_lord_class, 'name', None) != '****', "Storm Lord should have valid name reference"


if __name__ == '__main__':
    # Run tests directly
    pytest.main([__file__, '-v'])