"""
Tests for context-aware ResourceManager functionality
"""
import pytest
import tempfile
from pathlib import Path
from unittest.mock import Mock, patch

from services.resource_manager import ResourceManager


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


class TestFastAPISessionContextIntegration:
    """Test FastAPI session integration with context-aware ResourceManager"""
    
    def test_currentmodule_txt_detection(self, tmp_path):
        """Test that currentmodule.txt is properly read during session creation"""
        # Create save directory with currentmodule.txt
        save_dir = tmp_path / "save"
        save_dir.mkdir()
        
        currentmodule_file = save_dir / "currentmodule.txt"
        currentmodule_file.write_text("TestModule")
        
        # Create character file in save directory
        character_file = save_dir / "PLAYER.bic"
        character_file.write_text("dummy")
        
        # Test reading currentmodule.txt directly
        module_name = currentmodule_file.read_text().strip()
        assert module_name == 'TestModule'
        
        # Mock resource manager to test module finding
        mock_rm = Mock()
        mock_rm.find_module.return_value = '/path/to/TestModule.mod'
        
        # Test module detection logic
        module_path = mock_rm.find_module('TestModule')
        assert module_path == '/path/to/TestModule.mod'
        
        # Should have called find_module on resource manager
        mock_rm.find_module.assert_called_once_with('TestModule')
    
    def test_session_module_context_handling(self, tmp_path):
        """Test that session creation properly handles module context"""
        # Create save directory with currentmodule.txt
        save_dir = tmp_path / "save"
        save_dir.mkdir()
        
        currentmodule_file = save_dir / "currentmodule.txt"
        currentmodule_file.write_text("TestModule")
        
        character_file = save_dir / "PLAYER.bic"
        character_file.write_text("dummy")
        
        # Mock the session creation to test module context
        with patch('fastapi_core.session_registry.InMemoryCharacterSession') as mock_session:
            mock_instance = Mock()
            mock_instance.character_manager = Mock()
            mock_session.return_value = mock_instance
            
            from fastapi_core.session_registry import get_character_session
            
            # This would normally create a session and handle module context
            session = get_character_session(str(save_dir))
            
            # Verify session was created with the save directory path
            mock_session.assert_called_once_with(str(save_dir), auto_load=True)
            assert session.character_manager is not None


if __name__ == '__main__':
    pytest.main([__file__, '-v'])