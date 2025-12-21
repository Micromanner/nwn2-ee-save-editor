"""
Test the batch read functionality of SaveGameHandler
"""
import pytest
import os
from pathlib import Path
from services.savegame_handler import SaveGameHandler


@pytest.fixture
def sample_save_path():
    """Get path to sample save"""
    return Path(__file__).parent.parent / 'sample_save' / '000000 - 23-07-2025-13-06'


def test_batch_read_returns_same_data(sample_save_path):
    """Verify batch read returns the same data as individual reads"""
    handler = SaveGameHandler(sample_save_path)
    
    # Get data using old method (individual reads)
    old_data = {}
    old_data['playerlist.ifo'] = handler.extract_file('playerlist.ifo')
    try:
        old_data['player.bic'] = handler.extract_file('player.bic')
    except:
        pass
    
    for filename in handler.list_files():
        if filename.endswith('.ros'):
            try:
                old_data[filename] = handler.extract_file(filename)
            except:
                pass
    
    # Get data using new batch method
    new_data = handler.batch_read_character_files()
    
    # Compare results
    assert len(old_data) == len(new_data), f"Different number of files: old={len(old_data)}, new={len(new_data)}"
    
    for filename in old_data:
        assert filename in new_data, f"File {filename} missing from batch read"
        assert old_data[filename] == new_data[filename], f"Data mismatch for {filename}"


def test_batch_read_context_manager(sample_save_path):
    """Test the batch_read_context context manager"""
    handler = SaveGameHandler(sample_save_path)
    
    files_read = {}
    with handler.batch_read_context() as reader:
        # Read playerlist.ifo
        player_data = reader.read('playerlist.ifo')
        assert player_data is not None
        files_read['playerlist.ifo'] = player_data
        
        # Read player.bic (might not exist)
        bic_data = reader.read('player.bic')
        if bic_data:
            files_read['player.bic'] = bic_data
        
        # List and read .ros files
        for filename in reader.list_files():
            if filename.endswith('.ros'):
                ros_data = reader.read(filename)
                if ros_data:
                    files_read[filename] = ros_data
    
    # Verify we got files
    assert len(files_read) > 0
    assert 'playerlist.ifo' in files_read


def test_batch_read_handles_missing_playerlist(tmp_path):
    """Test that batch_read_character_files raises error if playerlist.ifo missing"""
    # Create empty ZIP for testing
    import zipfile
    zip_path = tmp_path / 'resgff.zip'
    with zipfile.ZipFile(zip_path, 'w') as zf:
        # Add a dummy file that's not playerlist.ifo
        zf.writestr('dummy.txt', b'dummy content')
    
    handler = SaveGameHandler(tmp_path)
    
    # Should raise SaveGameError
    from services.savegame_handler import SaveGameError
    with pytest.raises(SaveGameError, match="playerlist.ifo"):
        handler.batch_read_character_files()


def test_batch_reader_returns_none_for_missing_files(sample_save_path):
    """Test that BatchReader.read() returns None for missing files"""
    handler = SaveGameHandler(sample_save_path)
    
    with handler.batch_read_context() as reader:
        # Try to read a non-existent file
        result = reader.read('nonexistent.file')
        assert result is None
        
        # But playerlist.ifo should exist
        result = reader.read('playerlist.ifo')
        assert result is not None