"""Comprehensive tests for SaveGameHandler functionality."""

import os
import tempfile
import zipfile
from io import BytesIO
from pathlib import Path
import shutil
import pytest
from unittest.mock import patch, mock_open, MagicMock
import concurrent.futures
import threading
import time

from services.core.savegame_handler import SaveGameHandler
from nwn2_rust import GffParser, GffWriter


# Fixtures

@pytest.fixture
def base_dir():
    """Get the base directory for the project."""
    return Path(__file__).parent.parent.parent


@pytest.fixture
def sample_savegame_path(base_dir):
    """Path to the sample savegame."""
    return base_dir / 'sample_save' / '000048 - 23-07-2025-13-31'


@pytest.fixture
def temp_dir():
    """Create a temporary directory for test operations."""
    temp_path = tempfile.mkdtemp()
    yield temp_path
    # Cleanup after test
    if os.path.exists(temp_path):
        shutil.rmtree(temp_path)


@pytest.fixture
def temp_savegame(sample_savegame_path, temp_dir):
    """Create a copy of the sample savegame in temp directory."""
    if not sample_savegame_path.exists():
        pytest.skip(f"Sample savegame not found at {sample_savegame_path}")
    
    temp_save_path = os.path.join(temp_dir, 'test_save')
    shutil.copytree(sample_savegame_path, temp_save_path)
    return temp_save_path


@pytest.fixture
def minimal_save_zip(temp_dir):
    """Create a minimal valid save game zip for testing."""
    save_path = os.path.join(temp_dir, 'minimal_save')
    os.makedirs(save_path)
    zip_path = os.path.join(save_path, 'resgff.zip')
    
    # Create minimal valid files
    with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED) as zf:
        # Minimal IFO header (56 bytes)
        ifo_header = b'IFO V3.2' + b'\x00' * 48
        playerlist_data = ifo_header + b'TEST_DATA'
        
        # Minimal BIC header
        bic_header = b'BIC V3.2' + b'\x00' * 48  
        player_bic_data = bic_header + b'CHARACTER_DATA'
        
        # Add files with NWN2 metadata
        info = zipfile.ZipInfo('playerlist.ifo')
        info.date_time = (1980, 0, 0, 0, 0, 0)
        info.create_system = 0
        info.extract_version = 20
        zf.writestr(info, playerlist_data)
        
        info = zipfile.ZipInfo('player.bic')
        info.date_time = (1980, 0, 0, 0, 0, 0)
        info.create_system = 0
        info.extract_version = 20
        zf.writestr(info, player_bic_data)
        
        # Add some companion files
        for companion in ['khelgar', 'neeshka', 'qara']:
            ros_header = b'ROS V3.2' + b'\x00' * 48
            ros_data = ros_header + f'{companion}_DATA'.encode()
            
            info = zipfile.ZipInfo(f'{companion}.ros')
            info.date_time = (1980, 0, 0, 0, 0, 0)
            info.create_system = 0
            info.extract_version = 20
            zf.writestr(info, ros_data)
    
    return save_path


@pytest.fixture
def corrupted_save_zip(temp_dir):
    """Create a corrupted save game zip for error testing."""
    save_path = os.path.join(temp_dir, 'corrupted_save')
    os.makedirs(save_path)
    zip_path = os.path.join(save_path, 'resgff.zip')
    
    # Write corrupted zip data
    with open(zip_path, 'wb') as f:
        f.write(b'This is not a valid ZIP file!')
    
    return save_path


@pytest.fixture
def gff_parser():
    """Create a GFF parser instance."""
    return GFFParser()


# Test Classes

class TestSaveGameHandlerBasicOperations:
    """Test basic SaveGameHandler operations."""
    
    def test_init_with_directory(self, minimal_save_zip):
        """Test initialization with directory path."""
        handler = SaveGameHandler(minimal_save_zip)
        assert handler.save_dir == minimal_save_zip
        assert handler.zip_path == os.path.join(minimal_save_zip, 'resgff.zip')
    
    def test_init_with_zip_file(self, minimal_save_zip):
        """Test initialization with zip file path."""
        zip_path = os.path.join(minimal_save_zip, 'resgff.zip')
        handler = SaveGameHandler(zip_path)
        assert handler.save_dir == minimal_save_zip
        assert handler.zip_path == zip_path
    
    def test_init_missing_zip(self, temp_dir):
        """Test initialization with missing zip file."""
        nonexistent_path = os.path.join(temp_dir, 'nonexistent')
        os.makedirs(nonexistent_path)
        
        with pytest.raises(FileNotFoundError, match="resgff.zip not found"):
            SaveGameHandler(nonexistent_path)
    
    def test_extract_player_bic(self, temp_savegame, gff_parser):
        """Test extracting player.bic from savegame."""
        handler = SaveGameHandler(temp_savegame)
        
        # Test extraction
        bic_data = handler.extract_player_bic()
        
        # Verify it's valid BIC data
        assert bic_data is not None
        assert len(bic_data) > 56  # At least header size
        
        # Check header
        header = bic_data[:4].decode('ascii')
        assert header == 'BIC '
        
        # Verify it can be parsed
        gff_data = gff_parser.load(BytesIO(bic_data))
        assert gff_data is not None
    
    def test_extract_player_data(self, temp_savegame, gff_parser):
        """Test extracting player data from playerlist.ifo."""
        handler = SaveGameHandler(temp_savegame)
        
        # Get playerlist.ifo data
        playerlist_data = handler.extract_player_data()
        
        assert playerlist_data is not None
        assert len(playerlist_data) > 56  # At least header size
        
        # Check it's valid IFO data
        header = playerlist_data[:4].decode('ascii')
        assert header == 'IFO '
        
        # Parse and check for player data
        playerlist = gff_parser.load(BytesIO(playerlist_data))
        
        # Check for Mod_PlayerList
        mod_list = playerlist.get_field('Mod_PlayerList')
        assert mod_list is not None
        assert len(mod_list.value) > 0
    
    def test_extract_missing_player_bic(self, temp_dir):
        """Test extracting player.bic when it doesn't exist."""
        # Create save without player.bic
        save_path = os.path.join(temp_dir, 'no_player_save')
        os.makedirs(save_path)
        
        zip_path = os.path.join(save_path, 'resgff.zip')
        with zipfile.ZipFile(zip_path, 'w') as zf:
            zf.writestr('module.ifo', b'IFO V3.2' + b'\x00' * 48)
        
        handler = SaveGameHandler(save_path)
        # extract_player_bic returns None when file doesn't exist
        bic_data = handler.extract_player_bic()
        assert bic_data is None
    
    def test_list_files(self, minimal_save_zip):
        """Test listing all files in save game."""
        handler = SaveGameHandler(minimal_save_zip)
        files = handler.list_files()
        
        expected_files = ['playerlist.ifo', 'player.bic', 'khelgar.ros', 'neeshka.ros', 'qara.ros']
        assert set(files) == set(expected_files)
    
    def test_list_companions(self, minimal_save_zip):
        """Test listing companion names."""
        handler = SaveGameHandler(minimal_save_zip)
        companions = handler.list_companions()
        
        expected_companions = ['khelgar', 'neeshka', 'qara']
        assert set(companions) == set(expected_companions)
    
    def test_extract_companion(self, minimal_save_zip):
        """Test extracting companion .ros file."""
        handler = SaveGameHandler(minimal_save_zip)
        
        khelgar_data = handler.extract_companion('khelgar')
        assert khelgar_data is not None
        assert len(khelgar_data) > 56
        
        # Check header
        header = khelgar_data[:4].decode('ascii')
        assert header == 'ROS '
        
    def test_extract_nonexistent_file(self, minimal_save_zip):
        """Test extracting a file that doesn't exist."""
        from services.core.savegame_handler import SaveGameError
        handler = SaveGameHandler(minimal_save_zip)
        
        with pytest.raises(SaveGameError):
            handler.extract_file('nonexistent.file')
    


class TestSaveGameHandlerUpdateOperations:
    """Test update operations on save game files."""
    
    def test_update_single_file(self, temp_savegame):
        """Test updating a single file in the save."""
        handler = SaveGameHandler(temp_savegame)
        
        # Get original file list
        original_files = set(handler.list_files())
        
        # Update player.bic with new content
        new_content = b'BIC V3.2' + b'\x00' * 48 + b'NEW_DATA'
        handler.update_file('player.bic', new_content, backup=False)
        
        # Verify update
        updated_data = handler.extract_player_bic()
        assert updated_data == new_content
        
        # Verify other files unchanged
        assert set(handler.list_files()) == original_files
    
    def test_update_with_backup(self, temp_savegame, temp_dir):
        """Test that backups are created correctly."""
        handler = SaveGameHandler(temp_savegame)
        
        # Update with backup
        dummy_data = handler.extract_player_bic()
        handler.update_file('player.bic', dummy_data, backup=True)
        
        # Check backup was created
        parent_dir = os.path.dirname(temp_savegame)
        backup_dirs = [d for d in os.listdir(parent_dir) 
                      if d.startswith('test_save_backup_')]
        
        assert len(backup_dirs) > 0
        
        # Verify backup has same structure
        backup_path = os.path.join(parent_dir, backup_dirs[0])
        assert os.path.exists(os.path.join(backup_path, 'resgff.zip'))
    
    def test_update_player_complete(self, temp_savegame, gff_parser):
        """Test updating both player.bic and playerlist.ifo."""
        handler = SaveGameHandler(temp_savegame)
        
        # Get original data
        original_bic = handler.extract_player_bic()
        original_playerlist = handler.extract_player_data()
        
        # Parse both files
        bic_element = gff_parser.load(BytesIO(original_bic))
        playerlist_element = gff_parser.load(BytesIO(original_playerlist))
        
        # Get original STR values
        original_bic_str = bic_element.get_field('Str').value
        
        # Find player in playerlist
        mod_list = playerlist_element.get_field('Mod_PlayerList')
        player_struct = mod_list.value[0] if mod_list and mod_list.value else None
        assert player_struct is not None
        
        # Update to new value
        new_str = 99 if original_bic_str != 99 else 98
        
        # Update both structures
        bic_element.get_field('Str').value = new_str
        player_struct.get_field('Str').value = new_str
        
        # Write updated data
        bic_writer = GFFWriter.from_parser(gff_parser)
        playerlist_writer = GFFWriter(file_type='IFO ')
        
        bic_output = BytesIO()
        playerlist_output = BytesIO()
        
        bic_writer.save(bic_output, bic_element)
        playerlist_writer.save(playerlist_output, playerlist_element)
        
        # Update both files
        handler.update_player_complete(
            playerlist_output.getvalue(),
            bic_output.getvalue()
        )
        
        # Verify both files were updated
        new_bic = handler.extract_player_bic()
        new_bic_element = gff_parser.load(BytesIO(new_bic))
        assert new_bic_element.get_field('Str').value == new_str
        
        # Check playerlist.ifo
        new_playerlist = handler.extract_player_data()
        new_pl_element = gff_parser.load(BytesIO(new_playerlist))
        new_mod_list = new_pl_element.get_field('Mod_PlayerList')
        new_player = new_mod_list.value[0] if new_mod_list and new_mod_list.value else None
        assert new_player.get_field('Str').value == new_str
    
    def test_update_companion(self, minimal_save_zip):
        """Test updating companion .ros file."""
        handler = SaveGameHandler(minimal_save_zip)
        
        # Update Khelgar
        new_content = b'ROS V3.2' + b'\x00' * 48 + b'UPDATED_KHELGAR'
        handler.update_companion('khelgar', new_content, backup=False)
        
        # Verify update
        updated_data = handler.extract_companion('khelgar')
        assert updated_data == new_content
        
        # Verify other companions unchanged
        neeshka_data = handler.extract_companion('neeshka')
        assert b'neeshka_DATA' in neeshka_data
    
    def test_update_preserves_zip_format(self, minimal_save_zip):
        """Test that updates preserve NWN2 zip format."""
        handler = SaveGameHandler(minimal_save_zip)
        
        # Update a file
        new_content = b'BIC V3.2' + b'\x00' * 48 + b'FORMAT_TEST'
        handler.update_file('player.bic', new_content, backup=False)
        
        # Check zip format preservation
        with zipfile.ZipFile(handler.zip_path, 'r') as zf:
            for info in zf.filelist:
                assert info.date_time == (1980, 0, 0, 0, 0, 0)
                assert info.create_system == 0
                assert info.extract_version == 20
    


class TestSaveGameHandlerErrorHandling:
    """Test error handling and edge cases."""
    
    def test_corrupted_zip_handling(self, corrupted_save_zip):
        """Test handling of corrupted zip files."""
        from services.core.savegame_handler import SaveGameError
        handler = SaveGameHandler(corrupted_save_zip)
        
        with pytest.raises(SaveGameError):
            handler.list_files()
    
    def test_missing_file_extraction(self, minimal_save_zip):
        """Test extracting non-existent files."""
        from services.core.savegame_handler import SaveGameError
        handler = SaveGameHandler(minimal_save_zip)
        
        with pytest.raises(SaveGameError):
            handler.extract_file('nonexistent.txt')
    
    def test_update_nonexistent_file_handling(self, minimal_save_zip):
        """Test updating a file that doesn't exist in the zip."""
        handler = SaveGameHandler(minimal_save_zip)
        
        # This should add the new file
        new_content = b'NEW FILE CONTENT'
        handler.update_file('newfile.txt', new_content, backup=False)
        
        # Verify it was added
        assert 'newfile.txt' in handler.list_files()
        assert handler.extract_file('newfile.txt') == new_content
    
    @patch('os.path.exists')
    @patch('os.unlink')
    def test_update_cleanup_on_error(self, mock_unlink, mock_exists, minimal_save_zip):
        """Test that temp files are cleaned up on error."""
        from services.core.savegame_handler import SaveGameError
        handler = SaveGameHandler(minimal_save_zip)
        mock_exists.return_value = True
        
        # Force an error during update
        with patch('shutil.move', side_effect=OSError("Move failed")):
            with pytest.raises(SaveGameError):
                handler.update_file('player.bic', b'data', backup=False)
        
        # Verify cleanup was attempted
        assert mock_unlink.called
    
    def test_empty_zip_handling(self, temp_dir):
        """Test handling of empty zip files."""
        save_path = os.path.join(temp_dir, 'empty_save')
        os.makedirs(save_path)
        
        # Create empty zip
        zip_path = os.path.join(save_path, 'resgff.zip')
        with zipfile.ZipFile(zip_path, 'w'):
            pass
        
        handler = SaveGameHandler(save_path)
        assert handler.list_files() == []
        assert handler.list_companions() == []
        # extract_player_bic returns None when file doesn't exist
        assert handler.extract_player_bic() is None
    
    @patch('zipfile.ZipFile')
    def test_permission_error_handling(self, mock_zipfile, minimal_save_zip):
        """Test handling of permission errors."""
        from services.core.savegame_handler import SaveGameError
        mock_zipfile.side_effect = PermissionError("No permission")
        
        handler = SaveGameHandler(minimal_save_zip)
        
        with pytest.raises(SaveGameError):
            handler.list_files()
    


class TestSaveGameHandlerExtractAndRepack:
    """Test extract_for_editing and repack_from_directory methods."""
    
    def test_extract_for_editing(self, minimal_save_zip, temp_dir):
        """Test extracting all files for editing."""
        handler = SaveGameHandler(minimal_save_zip)
        
        # Extract to specific directory
        extract_dir = os.path.join(temp_dir, 'extracted')
        extracted = handler.extract_for_editing(extract_dir)
        
        # Verify all files extracted
        expected_files = handler.list_files()
        assert len(extracted) == len(expected_files)
        
        for filename in expected_files:
            assert filename in extracted
            assert os.path.exists(extracted[filename])
            
            # Verify content matches
            with open(extracted[filename], 'rb') as f:
                extracted_content = f.read()
            original_content = handler.extract_file(filename)
            assert extracted_content == original_content
    
    def test_extract_for_editing_auto_temp(self, minimal_save_zip):
        """Test extracting with automatic temp directory."""
        handler = SaveGameHandler(minimal_save_zip)
        
        extracted = handler.extract_for_editing()
        
        # Verify temp directory created
        first_path = list(extracted.values())[0]
        temp_dir = os.path.dirname(first_path)
        assert temp_dir.startswith(tempfile.gettempdir())
        assert 'nwn2_save_' in temp_dir
    
    def test_repack_from_directory(self, minimal_save_zip, temp_dir):
        """Test repacking files from directory."""
        handler = SaveGameHandler(minimal_save_zip)
        
        # Extract files
        extract_dir = os.path.join(temp_dir, 'to_repack')
        extracted = handler.extract_for_editing(extract_dir)
        
        # Modify a file
        player_bic_path = extracted['player.bic']
        with open(player_bic_path, 'wb') as f:
            f.write(b'BIC V3.2' + b'\x00' * 48 + b'REPACKED_DATA')
        
        # Repack
        handler.repack_from_directory(extract_dir, backup=False)
        
        # Verify changes
        new_data = handler.extract_player_bic()
        assert b'REPACKED_DATA' in new_data
        
        # Verify file order preserved
        original_files = handler.list_files()
        with zipfile.ZipFile(handler.zip_path, 'r') as zf:
            repacked_files = [info.filename for info in zf.filelist]
        assert repacked_files == original_files
    
    def test_repack_preserves_format(self, minimal_save_zip, temp_dir):
        """Test that repacking preserves NWN2 format."""
        handler = SaveGameHandler(minimal_save_zip)
        
        # Extract and repack without changes
        extract_dir = os.path.join(temp_dir, 'format_test')
        handler.extract_for_editing(extract_dir)
        handler.repack_from_directory(extract_dir, backup=False)
        
        # Check format preserved
        with zipfile.ZipFile(handler.zip_path, 'r') as zf:
            for info in zf.filelist:
                assert info.date_time == (1980, 0, 0, 0, 0, 0)
                assert info.create_system == 0
                assert info.extract_version == 20
    
    def test_all_files_preserved_after_update(self, temp_savegame, gff_parser):
        """Test that all files are preserved after update."""
        handler = SaveGameHandler(temp_savegame)
        
        # Get original files
        original_files = set(handler.list_files())
        
        # Do an update
        bic_data = handler.extract_player_bic()
        playerlist_data = handler.extract_player_data()
        
        # Just update with same data
        handler.update_player_complete(playerlist_data, bic_data)
        
        # Check all files are still present
        new_files = set(handler.list_files())
        assert original_files == new_files
    


class TestSaveGameHandlerConcurrency:
    """Test concurrent access scenarios."""
    
    def test_concurrent_reads(self, minimal_save_zip):
        """Test multiple concurrent read operations."""
        handler = SaveGameHandler(minimal_save_zip)
        
        def read_file(filename):
            return handler.extract_file(filename)
        
        # Perform concurrent reads
        with concurrent.futures.ThreadPoolExecutor(max_workers=5) as executor:
            futures = [
                executor.submit(read_file, 'player.bic'),
                executor.submit(read_file, 'playerlist.ifo'),
                executor.submit(read_file, 'khelgar.ros'),
                executor.submit(read_file, 'neeshka.ros'),
                executor.submit(read_file, 'qara.ros')
            ]
            
            results = [f.result() for f in futures]
            
        # Verify all reads succeeded
        assert all(result is not None for result in results)
    
    def test_read_during_update(self, temp_savegame):
        """Test reading while update is in progress."""
        handler = SaveGameHandler(temp_savegame)
        
        original_data = handler.extract_player_bic()
        update_complete = threading.Event()
        read_exception = None
        
        def update_file():
            time.sleep(0.1)  # Simulate slow update
            handler.update_file('player.bic', b'UPDATED', backup=False)
            update_complete.set()
        
        def read_file():
            nonlocal read_exception
            try:
                # Try to read during update
                time.sleep(0.05)  # Let update start
                handler.extract_player_bic()
            except Exception as e:
                read_exception = e
        
        # Start update and read concurrently
        update_thread = threading.Thread(target=update_file)
        read_thread = threading.Thread(target=read_file)
        
        update_thread.start()
        read_thread.start()
        
        update_thread.join()
        read_thread.join()
        
        # Both operations should complete without errors
        assert update_complete.is_set()
        assert read_exception is None


class TestSaveGameHandlerMemoryUsage:
    """Test memory usage with large files."""
    
    def test_large_file_handling(self, temp_dir):
        """Test handling of large save files."""
        # Create a save with a large file
        save_path = os.path.join(temp_dir, 'large_save')
        os.makedirs(save_path)
        
        # Create 10MB of data
        large_data = b'X' * (10 * 1024 * 1024)
        
        zip_path = os.path.join(save_path, 'resgff.zip')
        with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED) as zf:
            # Add standard files
            zf.writestr('player.bic', b'BIC V3.2' + b'\x00' * 48 + large_data)
            zf.writestr('playerlist.ifo', b'IFO V3.2' + b'\x00' * 48)
        
        handler = SaveGameHandler(save_path)
        
        # Should handle large files without issues
        data = handler.extract_player_bic()
        assert len(data) > 10 * 1024 * 1024
        
        # Update should also work
        new_large_data = b'Y' * (10 * 1024 * 1024)
        handler.update_file('player.bic', b'BIC V3.2' + b'\x00' * 48 + new_large_data, backup=False)
        
        # Verify update
        updated_data = handler.extract_player_bic()
        assert b'Y' * 1000 in updated_data
    


class TestSaveGameHandlerIntegration:
    """Integration tests with real save modifications."""
    
    def test_character_manager_integration(self, temp_savegame, gff_parser):
        """Test integration with CharacterManager."""
        from character.character_manager import CharacterManager
        from character.managers import AttributeManager

        handler = SaveGameHandler(temp_savegame)

        # Extract and parse player.bic - returns plain dict with __struct_id__ metadata
        bic_data = handler.extract_player_bic()
        char_data = gff_parser.load(BytesIO(bic_data))

        # Create CharacterManager with plain dict
        manager = CharacterManager(char_data)
        manager.register_manager('attribute', AttributeManager)

        # Modify attributes via manager
        attr_manager = manager.get_manager('attribute')
        original_str = char_data.get('Str', 10)
        new_str = 20 if original_str != 20 else 18
        attr_manager.set_attribute('Str', new_str)

        # Save back - need to update both player.bic and playerlist.ifo
        playerlist_data = handler.extract_player_data()
        playerlist_dict = gff_parser.load(BytesIO(playerlist_data))

        # Find and update player in playerlist
        mod_list = playerlist_dict.get('Mod_PlayerList', [])
        if mod_list:
            mod_list[0]['Str'] = new_str

        # Write both files - writer handles __struct_id__ metadata
        bic_writer = GFFWriter.from_parser(gff_parser)
        playerlist_writer = GFFWriter(file_type='IFO ')

        bic_output = BytesIO()
        playerlist_output = BytesIO()

        bic_writer.save(bic_output, manager.gff.raw_data)
        playerlist_writer.save(playerlist_output, playerlist_dict)

        # Update both files
        handler.update_player_complete(
            playerlist_output.getvalue(),
            bic_output.getvalue()
        )

        # Verify change persisted
        new_bic = handler.extract_player_bic()
        new_char_data = gff_parser.load(BytesIO(new_bic))
        assert new_char_data.get('Str') == new_str
    
    def test_full_save_edit_workflow(self, temp_savegame, gff_parser):
        """Test complete workflow: extract, edit multiple files, repack."""
        handler = SaveGameHandler(temp_savegame)
        
        # Extract all files
        extracted = handler.extract_for_editing()
        
        # Modify player.bic
        with open(extracted['player.bic'], 'rb') as f:
            bic_data = f.read()
        
        bic_element = gff_parser.load(BytesIO(bic_data))
        if bic_element.get_field('Str'):
            bic_element.get_field('Str').value = 25
        
        # Write back
        bic_writer = GFFWriter.from_parser(gff_parser)
        bic_output = BytesIO()
        bic_writer.save(bic_output, bic_element)
        
        with open(extracted['player.bic'], 'wb') as f:
            f.write(bic_output.getvalue())
        
        # Repack everything
        extract_dir = os.path.dirname(extracted['player.bic'])
        handler.repack_from_directory(extract_dir, backup=True)
        
        # Verify changes
        new_bic = handler.extract_player_bic()
        new_element = gff_parser.load(BytesIO(new_bic))
        if new_element.get_field('Str'):
            assert new_element.get_field('Str').value == 25


class TestSaveGameHandlerValidation:
    """Test data validation scenarios."""
    
    def test_invalid_gff_data_handling(self, minimal_save_zip):
        """Test handling of invalid GFF data during update."""
        handler = SaveGameHandler(minimal_save_zip)
        
        # Try to update with invalid GFF data
        invalid_data = b'INVALID GFF DATA THAT DOESNT MATCH FORMAT'
        
        # Should accept the data (handler doesn't validate)
        handler.update_file('test.gff', invalid_data, backup=False)
        
        # But extraction should return the invalid data as-is
        extracted = handler.extract_file('test.gff')
        assert extracted == invalid_data
    
    def test_special_characters_in_filenames(self, temp_dir):
        """Test handling files with special characters."""
        save_path = os.path.join(temp_dir, 'special_chars')
        os.makedirs(save_path)
        
        zip_path = os.path.join(save_path, 'resgff.zip')
        with zipfile.ZipFile(zip_path, 'w') as zf:
            # Add files with special characters
            files = [
                'player.bic',
                'test file.txt',  # space
                'data[1].sav',    # brackets
                'info@v2.dat'     # special char
            ]
            
            for filename in files:
                zf.writestr(filename, f'Content of {filename}'.encode())
        
        handler = SaveGameHandler(save_path)
        
        # Should handle all files
        assert set(handler.list_files()) == set(files)
        
        # Should extract correctly
        for filename in files:
            data = handler.extract_file(filename)
            assert data == f'Content of {filename}'.encode()