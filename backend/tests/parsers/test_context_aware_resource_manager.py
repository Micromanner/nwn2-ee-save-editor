"""
Tests for context-aware ResourceManager functionality
"""
import pytest
import tempfile
from pathlib import Path
from unittest.mock import Mock, patch

from parsers.resource_manager import ResourceManager
from character.services import CharacterImportService


class TestContextAwareResourceManager:
    """Test context-aware module loading in ResourceManager"""
    
    @pytest.fixture
    def temp_nwn2_dir(self):
        """Create temporary NWN2 directory structure"""
        temp_dir = Path(tempfile.mkdtemp())
        
        # Create basic NWN2 structure
        data_dir = temp_dir / "data"
        data_dir.mkdir()
        
        # Create minimal 2da.zip with classes.2da
        import zipfile
        zip_path = data_dir / "2da.zip"
        with zipfile.ZipFile(zip_path, 'w') as zf:
            zf.writestr('classes.2da', '2DA V2.0\n\n\tLABEL\n0\tFighter\n1\tWizard\n')
        
        # Create modules directory
        modules_dir = temp_dir / "Modules"
        modules_dir.mkdir()
        
        # Create a test module
        test_module = modules_dir / "TestModule.mod"
        test_module.write_bytes(b"test module data")
        
        yield temp_dir
        
        # Cleanup
        import shutil
        shutil.rmtree(temp_dir)
    
    @pytest.fixture
    def resource_manager(self, temp_nwn2_dir):
        """Create ResourceManager with temporary directory"""
        return ResourceManager(nwn2_path=str(temp_nwn2_dir), suppress_warnings=True)
    
    def test_scan_override_directories_without_context(self, resource_manager):
        """Test _scan_override_directories works without module context"""
        # Should work without context (existing behavior)
        resource_manager._scan_override_directories()
        
        # Should have empty module overrides since no context provided
        assert resource_manager._module_overrides == {}
        assert resource_manager._current_module is None
    
    def test_scan_override_directories_with_context(self, resource_manager, temp_nwn2_dir):
        """Test _scan_override_directories with module context"""
        # Create a mock module
        module_context = {
            'module_name': 'TestModule',
            'module_path': str(temp_nwn2_dir / "Modules" / "TestModule.mod")
        }
        
        # Mock the set_module method since we don't have a real .mod file
        with patch.object(resource_manager, 'set_module', return_value=True) as mock_set:
            resource_manager._scan_override_directories(module_context)
            
            # Should have called set_module with the correct path
            mock_set.assert_called_once_with(str(temp_nwn2_dir / "Modules" / "TestModule.mod"))
    
    def test_set_context_method(self, resource_manager):
        """Test the new set_context method"""
        module_info = {
            'module_name': 'TestModule',
            'module_path': '/path/to/TestModule.mod',
            'uses_custom_content': True
        }
        
        # Mock _scan_override_directories to verify it's called with correct context
        with patch.object(resource_manager, '_scan_override_directories') as mock_scan:
            resource_manager.set_context(module_info)
            
            # Should call _scan_override_directories with module context
            mock_scan.assert_called_once()
            args = mock_scan.call_args[0]
            if len(args) > 0:
                context = args[0]
                assert context['module_name'] == 'TestModule'
                assert context['module_path'] == '/path/to/TestModule.mod'
    
    def test_set_context_without_module_info(self, resource_manager):
        """Test set_context with incomplete module info"""
        module_info = {
            'module_name': '',  # Empty module name
            'uses_custom_content': False
        }
        
        # Mock _scan_override_directories to verify it's called without context
        with patch.object(resource_manager, '_scan_override_directories') as mock_scan:
            resource_manager.set_context(module_info)
            
            # Should call _scan_override_directories with None context
            mock_scan.assert_called_once_with(None)


class TestCharacterServiceContextIntegration:
    """Test CharacterImportService integration with context-aware ResourceManager"""
    
    def test_currentmodule_txt_detection(self, tmp_path):
        """Test that currentmodule.txt is properly read and module path is stored"""
        # Create save directory with currentmodule.txt
        save_dir = tmp_path / "save"
        save_dir.mkdir()
        
        currentmodule_file = save_dir / "currentmodule.txt"
        currentmodule_file.write_text("TestModule")
        
        # Create character file in save directory
        character_file = save_dir / "PLAYER.bic"
        character_file.write_text("dummy")
        
        # Mock resource manager
        mock_rm = Mock()
        mock_rm.find_module.return_value = '/path/to/TestModule.mod'
        
        # Create character service
        character_service = CharacterImportService(resource_manager=mock_rm)
        
        # Test data
        data = {}
        
        # Mock the campaign detection part to avoid complex dependencies
        with patch.object(character_service, '_detect_campaign_info'):
            character_service._detect_module_info(data, str(character_file))
        
        # Should have detected module from currentmodule.txt
        assert data['_module_info']['module_name'] == 'TestModule'
        assert data['_module_info']['module_path'] == '/path/to/TestModule.mod'
        
        # Should have called find_module on resource manager
        mock_rm.find_module.assert_called_once_with('TestModule')
    
    def test_module_context_not_loaded_globally(self, tmp_path):
        """Test that modules are not loaded globally during detection"""
        # Create save directory with currentmodule.txt
        save_dir = tmp_path / "save"
        save_dir.mkdir()
        
        currentmodule_file = save_dir / "currentmodule.txt"
        currentmodule_file.write_text("TestModule")
        
        character_file = save_dir / "PLAYER.bic"
        character_file.write_text("dummy")
        
        # Mock resource manager
        mock_rm = Mock()
        mock_rm.find_module.return_value = '/path/to/TestModule.mod'
        mock_rm.set_module = Mock()  # This should NOT be called
        
        character_service = CharacterImportService(resource_manager=mock_rm)
        data = {}
        
        # Mock the campaign detection part
        with patch.object(character_service, '_detect_campaign_info'):
            character_service._detect_module_info(data, str(character_file))
        
        # Should NOT have called set_module (new behavior - no global loading)
        mock_rm.set_module.assert_not_called()
        
        # Should have stored module info for later context setting
        assert 'module_path' in data['_module_info']
        assert data['_module_info']['module_path'] == '/path/to/TestModule.mod'


if __name__ == '__main__':
    pytest.main([__file__, '-v'])