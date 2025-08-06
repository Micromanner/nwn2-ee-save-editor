"""Tests for savegame API views."""

import os
import tempfile
import shutil
import time
import zipfile
from pathlib import Path
from unittest.mock import patch, MagicMock, Mock
from io import BytesIO
import glob


import pytest
from django.contrib.auth.models import User
from rest_framework.test import APIClient
from rest_framework import status

from character.models import Character
from parsers.savegame_handler import SaveGameHandler
from parsers.gff import GFFParser, GFFWriter, GFFElement, GFFFieldType
from api.savegame_views import update_savegame_character


@pytest.fixture
def api_client():
    """Create API client for tests."""
    return APIClient()


@pytest.fixture
def test_user(db):
    """Create test user."""
    return User.objects.create_user('testuser', 'test@example.com', 'password')


@pytest.fixture
def other_user(db):
    """Create another test user for permission testing."""
    return User.objects.create_user('otheruser', 'other@example.com', 'password')


@pytest.fixture
def authenticated_client(api_client, test_user):
    """Create authenticated API client."""
    api_client.force_authenticate(user=test_user)
    return api_client


@pytest.fixture
def sample_savegame_path():
    """Get path to sample savegame."""
    base_dir = Path(__file__).parent.parent.parent.parent  # Go up to backend dir
    return base_dir / 'sample_save' / '000048 - 23-07-2025-13-31'


@pytest.fixture
def temp_savegame_dir(sample_savegame_path):
    """Create temporary copy of savegame for testing."""
    temp_dir = tempfile.mkdtemp()
    
    if sample_savegame_path.exists():
        temp_save = os.path.join(temp_dir, 'test_save')
        shutil.copytree(sample_savegame_path, temp_save)
        yield temp_save
    else:
        # Create minimal test savegame structure
        test_save = os.path.join(temp_dir, 'test_save')
        os.makedirs(test_save)
        
        # Create minimal resgff.zip
        zip_path = os.path.join(test_save, 'resgff.zip')
        with zipfile.ZipFile(zip_path, 'w') as zf:
            # Add playerlist.ifo
            parser = GFFParser()
            writer = GFFWriter(file_type='IFO ')
            
            playerlist = GFFElement(GFFFieldType.STRUCT, 0, "", [])
            player_struct = GFFElement(GFFFieldType.STRUCT, 0, "", [
                GFFElement(GFFFieldType.BYTE, 0, "Str", 10),
                GFFElement(GFFFieldType.BYTE, 0, "Dex", 12),
                GFFElement(GFFFieldType.BYTE, 0, "Con", 14),
                GFFElement(GFFFieldType.BYTE, 0, "Int", 16),
                GFFElement(GFFFieldType.BYTE, 0, "Wis", 18),
                GFFElement(GFFFieldType.BYTE, 0, "Cha", 20),
                GFFElement(GFFFieldType.SHORT, 0, "HitPoints", 10),
                GFFElement(GFFFieldType.SHORT, 0, "CurrentHitPoints", 10),
                GFFElement(GFFFieldType.SHORT, 0, "MaxHitPoints", 10),
                GFFElement(GFFFieldType.DWORD, 0, "Experience", 0),
                GFFElement(GFFFieldType.STRING, 0, "FirstName", "Test"),
                GFFElement(GFFFieldType.STRING, 0, "LastName", "Character")
            ])
            mod_list = GFFElement(GFFFieldType.LIST, 0, "Mod_PlayerList", [player_struct])
            playerlist.fields.append(mod_list)
            
            output = BytesIO()
            writer.save(output, playerlist)
            zf.writestr('playerlist.ifo', output.getvalue())
            
            # Add player.bic with same fields
            bic_writer = GFFWriter(file_type='BIC ')
            bic_output = BytesIO()
            bic_writer.save(bic_output, player_struct)
            zf.writestr('player.bic', bic_output.getvalue())
            
            # Add a companion
            companion_writer = GFFWriter(file_type='ROS ')
            companion_struct = GFFElement(GFFFieldType.STRUCT, 0, "", [
                GFFElement(GFFFieldType.STRING, 0, "FirstName", "Companion"),
                GFFElement(GFFFieldType.STRING, 0, "LastName", "Test"),
                GFFElement(GFFFieldType.BYTE, 0, "Str", 15)
            ])
            companion_output = BytesIO()
            companion_writer.save(companion_output, companion_struct)
            zf.writestr('companion1.ros', companion_output.getvalue())
        
        yield test_save
    
    # Cleanup
    shutil.rmtree(temp_dir)


@pytest.fixture
def savegame_character(db, test_user, temp_savegame_dir):
    """Create a savegame character."""
    return Character.objects.create(
        owner=test_user,
        file_path=temp_savegame_dir,
        file_name='test_save',
        first_name='Test',
        last_name='Character',
        is_savegame=True,
        character_level=1,
        strength=10
    )


@pytest.fixture
def regular_character(db, test_user):
    """Create a regular (non-savegame) character."""
    return Character.objects.create(
        owner=test_user,
        file_path='/some/path/character.bic',
        file_name='character.bic',
        first_name='Regular',
        last_name='Character',
        is_savegame=False,
        character_level=1,
        strength=10
    )


@pytest.fixture
def corrupted_savegame_dir():
    """Create a corrupted savegame for error testing."""
    temp_dir = tempfile.mkdtemp()
    test_save = os.path.join(temp_dir, 'corrupted_save')
    os.makedirs(test_save)
    
    # Create corrupted resgff.zip
    zip_path = os.path.join(test_save, 'resgff.zip')
    with open(zip_path, 'wb') as f:
        f.write(b'This is not a valid zip file')
    
    yield test_save
    shutil.rmtree(temp_dir)


class TestSavegameImport:
    """Test savegame import functionality."""
    
    def test_import_savegame_success(self, authenticated_client, temp_savegame_dir):
        """Test successful import of a character from savegame."""
        save_path = str(temp_savegame_dir).replace('/mnt/c/', 'C:\\').replace('/', '\\')
        
        response = authenticated_client.post('/api/savegames/import/', {
            'save_path': save_path
        }, format='json')
        
        assert response.status_code == status.HTTP_201_CREATED
        
        data = response.json()
        assert 'id' in data
        assert 'first_name' in data
        assert data.get('is_savegame', False) is True
        
        # Verify character in database
        character = Character.objects.get(id=data['id'])
        assert character.is_savegame is True
        assert character.file_path == str(temp_savegame_dir)
    
    def test_import_savegame_missing_path(self, authenticated_client):
        """Test import fails when save_path is missing."""
        response = authenticated_client.post('/api/savegames/import/', {}, format='json')
        
        assert response.status_code == status.HTTP_400_BAD_REQUEST
        assert 'save_path is required' in response.json()['error']
    
    def test_import_savegame_invalid_directory(self, authenticated_client):
        """Test import fails when directory doesn't exist."""
        response = authenticated_client.post('/api/savegames/import/', {
            'save_path': '/nonexistent/path'
        }, format='json')
        
        assert response.status_code == status.HTTP_404_NOT_FOUND
        assert 'Save directory not found' in response.json()['error']
    
    def test_import_savegame_missing_resgff(self, authenticated_client):
        """Test import fails when resgff.zip is missing."""
        temp_dir = tempfile.mkdtemp()
        try:
            response = authenticated_client.post('/api/savegames/import/', {
                'save_path': temp_dir
            }, format='json')
            
            assert response.status_code == status.HTTP_404_NOT_FOUND
            assert 'resgff.zip not found' in response.json()['error']
        finally:
            shutil.rmtree(temp_dir)
    
    def test_import_savegame_windows_path_conversion(self, authenticated_client, temp_savegame_dir):
        """Test Windows path is correctly converted to WSL path."""
        # Test with Windows-style path
        windows_path = 'C:\\Users\\test\\saves\\test_save'
        wsl_path = '/mnt/c/Users/test/saves/test_save'
        
        with patch('os.path.isdir') as mock_isdir:
            mock_isdir.return_value = False
            
            response = authenticated_client.post('/api/savegames/import/', {
                'save_path': windows_path
            }, format='json')
            
            # Check that the path was converted correctly
            mock_isdir.assert_called_with(wsl_path)
    
    def test_import_savegame_corrupted_file(self, authenticated_client, corrupted_savegame_dir):
        """Test import fails gracefully with corrupted savegame."""
        response = authenticated_client.post('/api/savegames/import/', {
            'save_path': corrupted_savegame_dir
        }, format='json')
        
        assert response.status_code == status.HTTP_500_INTERNAL_SERVER_ERROR
        assert 'error' in response.json()
    
    def test_import_savegame_unauthenticated(self, api_client, temp_savegame_dir):
        """Test import requires authentication."""
        response = api_client.post('/api/savegames/import/', {
            'save_path': temp_savegame_dir
        }, format='json')
        
        assert response.status_code == status.HTTP_401_UNAUTHORIZED


class TestListSavegameCompanions:
    """Test listing companions in savegame."""
    
    def test_list_companions_success(self, authenticated_client, savegame_character):
        """Test successfully listing companions in savegame."""
        response = authenticated_client.get(f'/api/savegames/{savegame_character.id}/companions/')
        
        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert 'companions' in data
        assert 'count' in data
        assert isinstance(data['companions'], list)
        assert data['count'] >= 0
    
    def test_list_companions_non_savegame(self, authenticated_client, regular_character):
        """Test listing companions fails for non-savegame character."""
        response = authenticated_client.get(f'/api/savegames/{regular_character.id}/companions/')
        
        assert response.status_code == status.HTTP_400_BAD_REQUEST
        assert 'not from a save game' in response.json()['error']
    
    def test_list_companions_character_not_found(self, authenticated_client):
        """Test listing companions fails when character doesn't exist."""
        response = authenticated_client.get('/api/savegames/999999/companions/')
        
        assert response.status_code == status.HTTP_404_NOT_FOUND
        assert 'Character not found' in response.json()['error']
    
    def test_list_companions_other_users_character(self, authenticated_client, other_user, temp_savegame_dir):
        """Test user cannot list companions of another user's character."""
        # Create character owned by other user
        other_character = Character.objects.create(
            owner=other_user,
            file_path=temp_savegame_dir,
            file_name='other_save',
            first_name='Other',
            last_name='Character',
            is_savegame=True,
            character_level=1
        )
        
        response = authenticated_client.get(f'/api/savegames/{other_character.id}/companions/')
        
        # Should either return 404 or 403 depending on API design
        assert response.status_code in [status.HTTP_403_FORBIDDEN, status.HTTP_404_NOT_FOUND]
    
    @patch('parsers.savegame_handler.SaveGameHandler.list_companions')
    def test_list_companions_handler_error(self, mock_list_companions, authenticated_client, savegame_character):
        """Test error handling when SaveGameHandler fails."""
        mock_list_companions.side_effect = Exception("Failed to read save file")
        
        response = authenticated_client.get(f'/api/savegames/{savegame_character.id}/companions/')
        
        assert response.status_code == status.HTTP_500_INTERNAL_SERVER_ERROR
        assert 'Failed to read save file' in response.json()['error']


class TestUpdateSavegameCharacter:
    """Test updating character data in savegame."""
    
    def test_update_attributes_success(self, authenticated_client, savegame_character, temp_savegame_dir):
        """Test successfully updating character attributes."""
        new_str = 25
        response = authenticated_client.post(f'/api/savegames/{savegame_character.id}/update/', {
            'updates': {
                'attributes': {
                    'Str': new_str
                }
            }
        }, format='json')
        
        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert data['success'] is True
        assert data['backup_created'] is True
        assert 'changes' in data
        
        # Verify both files were updated
        handler = SaveGameHandler(temp_savegame_dir)
        parser = GFFParser()
        
        new_bic = handler.extract_player_bic()
        new_bic_element = parser.load(BytesIO(new_bic))
        assert new_bic_element.get_field('Str').value == new_str
        
        new_playerlist = handler.extract_player_data()
        new_pl_element = parser.load(BytesIO(new_playerlist))
        new_mod_list = new_pl_element.get_field('Mod_PlayerList')
        assert new_mod_list.value[0].get_field('Str').value == new_str
    
    def test_update_multiple_attributes(self, authenticated_client, savegame_character):
        """Test updating multiple attributes at once."""
        updates = {
            'Str': 18,
            'Dex': 14,
            'Con': 16,
            'Int': 12,
            'Wis': 10,
            'Cha': 8
        }
        
        response = authenticated_client.post(f'/api/savegames/{savegame_character.id}/update/', {
            'updates': {
                'attributes': updates
            }
        }, format='json')
        
        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert len(data['changes']) == 6
    
    def test_update_without_player_bic(self, authenticated_client, test_user):
        """Test update fails when player.bic is missing."""
        temp_dir = tempfile.mkdtemp()
        try:
            test_save = os.path.join(temp_dir, 'no_bic_save')
            os.makedirs(test_save)
            
            # Create minimal resgff.zip without player.bic
            zip_path = os.path.join(test_save, 'resgff.zip')
            with zipfile.ZipFile(zip_path, 'w') as zf:
                # Add only playerlist.ifo
                parser = GFFParser()
                writer = GFFWriter(file_type='IFO ')
                
                playerlist = GFFElement(GFFFieldType.STRUCT, 0, "", [])
                player_struct = GFFElement(GFFFieldType.STRUCT, 0, "", [
                    GFFElement(GFFFieldType.BYTE, 0, "Str", 10)
                ])
                mod_list = GFFElement(GFFFieldType.LIST, 0, "Mod_PlayerList", [player_struct])
                playerlist.fields.append(mod_list)
                
                output = BytesIO()
                writer.save(output, playerlist)
                zf.writestr('playerlist.ifo', output.getvalue())
            
            # Create character
            character = Character.objects.create(
                owner=test_user,
                file_path=test_save,
                file_name='no_bic_save',
                first_name='Test',
                last_name='NoBic',
                is_savegame=True,
                character_level=1
            )
            
            response = authenticated_client.post(f'/api/savegames/{character.id}/update/', {
                'updates': {
                    'attributes': {'Str': 20}
                }
            }, format='json')
            
            assert response.status_code == status.HTTP_400_BAD_REQUEST
            assert 'player.bic' in response.json()['error']
        finally:
            shutil.rmtree(temp_dir)
    
    def test_update_non_savegame_character(self, authenticated_client, regular_character):
        """Test update fails for non-savegame character."""
        response = authenticated_client.post(f'/api/savegames/{regular_character.id}/update/', {
            'updates': {'attributes': {'Str': 20}}
        }, format='json')
        
        assert response.status_code == status.HTTP_400_BAD_REQUEST
        assert 'not from a save game' in response.json()['error']
    
    def test_update_no_updates_provided(self, authenticated_client, savegame_character):
        """Test update fails when no updates are provided."""
        response = authenticated_client.post(f'/api/savegames/{savegame_character.id}/update/', {}, format='json')
        
        assert response.status_code == status.HTTP_400_BAD_REQUEST
        assert 'No updates provided' in response.json()['error']
    
    def test_update_invalid_attribute(self, authenticated_client, savegame_character):
        """Test update with invalid attribute name."""
        response = authenticated_client.post(f'/api/savegames/{savegame_character.id}/update/', {
            'updates': {
                'attributes': {
                    'InvalidAttr': 20
                }
            }
        }, format='json')
        
        # Should still succeed but not update invalid attributes
        assert response.status_code == status.HTTP_200_OK
        data = response.json()
        assert 'InvalidAttr' not in data['changes']
    
    def test_update_backup_creation(self, authenticated_client, savegame_character, temp_savegame_dir):
        """Test that backups are created when updating."""
        backup_pattern = f"{temp_savegame_dir}_backup_*"
        backups_before = glob.glob(backup_pattern)
        
        response = authenticated_client.post(f'/api/savegames/{savegame_character.id}/update/', {
            'updates': {
                'attributes': {'Str': 15}
            }
        }, format='json')
        
        assert response.status_code == status.HTTP_200_OK
        assert response.json()['backup_created'] is True
        
        backups_after = glob.glob(backup_pattern)
        assert len(backups_after) == len(backups_before) + 1
    
    def test_update_preserves_file_types(self, authenticated_client, savegame_character, temp_savegame_dir):
        """Test that file type headers are preserved after update."""
        response = authenticated_client.post(f'/api/savegames/{savegame_character.id}/update/', {
            'updates': {
                'attributes': {'Str': 20}
            }
        }, format='json')
        
        assert response.status_code == status.HTTP_200_OK
        
        handler = SaveGameHandler(temp_savegame_dir)
        new_bic = handler.extract_player_bic()
        new_playerlist = handler.extract_player_data()
        
        # Verify file type headers
        assert new_bic[:4].decode('ascii') == 'BIC '
        assert new_playerlist[:4].decode('ascii') == 'IFO '
    
    @patch('parsers.savegame_handler.SaveGameHandler.extract_player_data')
    def test_update_handler_error(self, mock_extract, authenticated_client, savegame_character):
        """Test error handling when SaveGameHandler fails."""
        mock_extract.side_effect = Exception("Failed to extract player data")
        
        response = authenticated_client.post(f'/api/savegames/{savegame_character.id}/update/', {
            'updates': {
                'attributes': {'Str': 20}
            }
        }, format='json')
        
        assert response.status_code == status.HTTP_500_INTERNAL_SERVER_ERROR
        assert 'Failed to extract player data' in response.json()['error']


class TestGetSavegameInfo:
    """Test getting savegame information."""
    
    def test_get_info_success(self, authenticated_client, savegame_character):
        """Test successfully getting savegame info."""
        response = authenticated_client.get(f'/api/savegames/{savegame_character.id}/info/')
        
        assert response.status_code == status.HTTP_200_OK
        
        data = response.json()
        assert 'save_directory' in data
        assert 'original_save_exists' in data
        assert 'backups' in data
        assert 'companions' in data
        assert 'files_in_save' in data
        
        assert isinstance(data['backups'], list)
        assert isinstance(data['companions'], list)
        assert isinstance(data['files_in_save'], list)
        
        # Check required files are listed
        assert 'playerlist.ifo' in data['files_in_save']
        assert 'player.bic' in data['files_in_save']
    
    def test_get_info_with_backups(self, authenticated_client, savegame_character, temp_savegame_dir):
        """Test info includes backup details."""
        # Create a backup
        backup_dir = f"{temp_savegame_dir}_backup_20240101_120000"
        os.makedirs(backup_dir)
        
        response = authenticated_client.get(f'/api/savegames/{savegame_character.id}/info/')
        
        assert response.status_code == status.HTTP_200_OK
        
        data = response.json()
        assert len(data['backups']) == 1
        assert 'path' in data['backups'][0]
        assert 'name' in data['backups'][0]
        assert 'created' in data['backups'][0]
        
        # Cleanup
        shutil.rmtree(backup_dir)
    
    def test_get_info_non_savegame(self, authenticated_client, regular_character):
        """Test info fails for non-savegame character."""
        response = authenticated_client.get(f'/api/savegames/{regular_character.id}/info/')
        
        assert response.status_code == status.HTTP_400_BAD_REQUEST
        assert 'not from a save game' in response.json()['error']
    
    def test_get_info_character_not_found(self, authenticated_client):
        """Test info fails when character doesn't exist."""
        response = authenticated_client.get('/api/savegames/999999/info/')
        
        assert response.status_code == status.HTTP_404_NOT_FOUND
        assert 'Character not found' in response.json()['error']
    
    @patch('parsers.savegame_handler.SaveGameHandler.list_files')
    def test_get_info_handler_error(self, mock_list_files, authenticated_client, savegame_character):
        """Test error handling when SaveGameHandler fails."""
        mock_list_files.side_effect = Exception("Failed to list files")
        
        response = authenticated_client.get(f'/api/savegames/{savegame_character.id}/info/')
        
        assert response.status_code == status.HTTP_500_INTERNAL_SERVER_ERROR
        assert 'Failed to list files' in response.json()['error']


class TestSavegameFieldSynchronization:
    """Test that all relevant fields are synchronized between files."""
    
    def test_all_common_fields_synced(self, temp_savegame_dir):
        """Test that all fields common to both files are synchronized."""
        handler = SaveGameHandler(temp_savegame_dir)
        parser = GFFParser()
        
        bic_data = handler.extract_player_bic()
        playerlist_data = handler.extract_player_data()
        
        bic_element = parser.load(BytesIO(bic_data))
        playerlist_element = parser.load(BytesIO(playerlist_data))
        
        mod_list = playerlist_element.get_field('Mod_PlayerList')
        player_struct = mod_list.value[0]
        
        # Check which fields exist in both
        common_fields = []
        for field in player_struct.fields:
            if bic_element.has_field(field.label):
                common_fields.append(field.label)
        
        # These are some fields we expect to be in both
        expected_common = ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha', 
                          'HitPoints', 'CurrentHitPoints', 'MaxHitPoints']
        
        for field_name in expected_common:
            assert field_name in common_fields, f"{field_name} should be in both files"
            
            # Verify values match
            bic_value = bic_element.get_field(field_name).value
            pl_value = player_struct.get_field(field_name).value
            assert bic_value == pl_value, f"{field_name} values should match"
    
    def test_field_sync_after_update(self, authenticated_client, savegame_character, temp_savegame_dir):
        """Test fields remain synchronized after update."""
        # Update multiple fields
        updates = {
            'Str': 22,
            'Dex': 18,
            'Con': 20
        }
        
        response = authenticated_client.post(f'/api/savegames/{savegame_character.id}/update/', {
            'updates': {
                'attributes': updates
            }
        }, format='json')
        
        assert response.status_code == status.HTTP_200_OK
        
        # Verify synchronization
        handler = SaveGameHandler(temp_savegame_dir)
        parser = GFFParser()
        
        bic_data = handler.extract_player_bic()
        playerlist_data = handler.extract_player_data()
        
        bic_element = parser.load(BytesIO(bic_data))
        playerlist_element = parser.load(BytesIO(playerlist_data))
        
        mod_list = playerlist_element.get_field('Mod_PlayerList')
        player_struct = mod_list.value[0]
        
        for attr, value in updates.items():
            bic_value = bic_element.get_field(attr).value
            pl_value = player_struct.get_field(attr).value
            assert bic_value == value
            assert pl_value == value
            assert bic_value == pl_value


class TestSavegamePerformance:
    """Test performance with large savegames."""
    
    @pytest.fixture
    def large_savegame_dir(self):
        """Create a large savegame for performance testing."""
        temp_dir = tempfile.mkdtemp()
        test_save = os.path.join(temp_dir, 'large_save')
        os.makedirs(test_save)
        
        # Create large resgff.zip
        zip_path = os.path.join(test_save, 'resgff.zip')
        with zipfile.ZipFile(zip_path, 'w') as zf:
            # Add playerlist.ifo with many fields
            parser = GFFParser()
            writer = GFFWriter(file_type='IFO ')
            
            playerlist = GFFElement(GFFFieldType.STRUCT, 0, "", [])
            
            # Create player struct with many fields
            fields = [
                GFFElement(GFFFieldType.BYTE, 0, "Str", 10),
                GFFElement(GFFFieldType.BYTE, 0, "Dex", 12),
                GFFElement(GFFFieldType.BYTE, 0, "Con", 14),
                GFFElement(GFFFieldType.BYTE, 0, "Int", 16),
                GFFElement(GFFFieldType.BYTE, 0, "Wis", 18),
                GFFElement(GFFFieldType.BYTE, 0, "Cha", 20),
            ]
            
            # Add many inventory items
            for i in range(100):
                fields.append(GFFElement(GFFFieldType.DWORD, 0, f"Item_{i}", i))
            
            player_struct = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
            mod_list = GFFElement(GFFFieldType.LIST, 0, "Mod_PlayerList", [player_struct])
            playerlist.fields.append(mod_list)
            
            output = BytesIO()
            writer.save(output, playerlist)
            zf.writestr('playerlist.ifo', output.getvalue())
            
            # Add player.bic
            bic_writer = GFFWriter(file_type='BIC ')
            bic_output = BytesIO()
            bic_writer.save(bic_output, player_struct)
            zf.writestr('player.bic', bic_output.getvalue())
            
            # Add many companions
            for i in range(20):
                companion_writer = GFFWriter(file_type='ROS ')
                companion_struct = GFFElement(GFFFieldType.STRUCT, 0, "", [
                    GFFElement(GFFFieldType.STRING, 0, "FirstName", f"Companion{i}"),
                    GFFElement(GFFFieldType.BYTE, 0, "Str", 10 + i)
                ])
                companion_output = BytesIO()
                companion_writer.save(companion_output, companion_struct)
                zf.writestr(f'companion{i}.ros', companion_output.getvalue())
        
        yield test_save
        shutil.rmtree(temp_dir)
    
    def test_import_large_savegame_performance(self, authenticated_client, large_savegame_dir):
        """Test importing large savegame completes in reasonable time."""
        start_time = time.time()
        
        response = authenticated_client.post('/api/savegames/import/', {
            'save_path': large_savegame_dir
        }, format='json')
        
        elapsed_time = time.time() - start_time
        
        assert response.status_code == status.HTTP_201_CREATED
        assert elapsed_time < 5.0  # Should complete within 5 seconds
    
    def test_update_large_savegame_performance(self, authenticated_client, test_user, large_savegame_dir):
        """Test updating large savegame completes in reasonable time."""
        # Create character
        character = Character.objects.create(
            owner=test_user,
            file_path=large_savegame_dir,
            file_name='large_save',
            first_name='Test',
            last_name='Character',
            is_savegame=True,
            character_level=1
        )
        
        start_time = time.time()
        
        response = authenticated_client.post(f'/api/savegames/{character.id}/update/', {
            'updates': {
                'attributes': {
                    'Str': 25,
                    'Dex': 20,
                    'Con': 18
                }
            }
        }, format='json')
        
        elapsed_time = time.time() - start_time
        
        assert response.status_code == status.HTTP_200_OK
        assert elapsed_time < 3.0  # Should complete within 3 seconds
    
    def test_list_many_companions_performance(self, authenticated_client, test_user, large_savegame_dir):
        """Test listing many companions completes in reasonable time."""
        # Create character
        character = Character.objects.create(
            owner=test_user,
            file_path=large_savegame_dir,
            file_name='large_save',
            first_name='Test',
            last_name='Character',
            is_savegame=True,
            character_level=1
        )
        
        start_time = time.time()
        
        response = authenticated_client.get(f'/api/savegames/{character.id}/companions/')
        
        elapsed_time = time.time() - start_time
        
        assert response.status_code == status.HTTP_200_OK
        assert len(response.json()['companions']) == 20
        assert elapsed_time < 1.0  # Should complete within 1 second


class TestSavegameEdgeCases:
    """Test edge cases and error conditions."""
    
    def test_update_with_empty_mod_playerlist(self, authenticated_client, test_user):
        """Test handling savegame with empty Mod_PlayerList."""
        temp_dir = tempfile.mkdtemp()
        try:
            test_save = os.path.join(temp_dir, 'empty_list_save')
            os.makedirs(test_save)
            
            zip_path = os.path.join(test_save, 'resgff.zip')
            with zipfile.ZipFile(zip_path, 'w') as zf:
                # Add playerlist.ifo with empty Mod_PlayerList
                parser = GFFParser()
                writer = GFFWriter(file_type='IFO ')
                
                playerlist = GFFElement(GFFFieldType.STRUCT, 0, "", [])
                mod_list = GFFElement(GFFFieldType.LIST, 0, "Mod_PlayerList", [])  # Empty list
                playerlist.fields.append(mod_list)
                
                output = BytesIO()
                writer.save(output, playerlist)
                zf.writestr('playerlist.ifo', output.getvalue())
            
            character = Character.objects.create(
                owner=test_user,
                file_path=test_save,
                file_name='empty_list_save',
                first_name='Test',
                last_name='Character',
                is_savegame=True,
                character_level=1
            )
            
            response = authenticated_client.post(f'/api/savegames/{character.id}/update/', {
                'updates': {'attributes': {'Str': 20}}
            }, format='json')
            
            assert response.status_code == status.HTTP_500_INTERNAL_SERVER_ERROR
            assert 'No player data found' in response.json()['error']
        finally:
            shutil.rmtree(temp_dir)
    
    def test_concurrent_updates(self, authenticated_client, savegame_character):
        """Test handling concurrent update requests."""
        import threading
        results = []
        
        def update_character():
            response = authenticated_client.post(f'/api/savegames/{savegame_character.id}/update/', {
                'updates': {
                    'attributes': {'Str': 20}
                }
            }, format='json')
            results.append(response.status_code)
        
        # Create multiple threads
        threads = []
        for _ in range(3):
            t = threading.Thread(target=update_character)
            threads.append(t)
            t.start()
        
        # Wait for all threads
        for t in threads:
            t.join()
        
        # At least one should succeed
        assert status.HTTP_200_OK in results
    
    def test_update_with_unicode_names(self, authenticated_client, test_user):
        """Test handling characters with unicode names."""
        temp_dir = tempfile.mkdtemp()
        try:
            test_save = os.path.join(temp_dir, 'unicode_save')
            os.makedirs(test_save)
            
            zip_path = os.path.join(test_save, 'resgff.zip')
            with zipfile.ZipFile(zip_path, 'w') as zf:
                parser = GFFParser()
                writer = GFFWriter(file_type='IFO ')
                
                playerlist = GFFElement(GFFFieldType.STRUCT, 0, "", [])
                player_struct = GFFElement(GFFFieldType.STRUCT, 0, "", [
                    GFFElement(GFFFieldType.BYTE, 0, "Str", 10),
                    GFFElement(GFFFieldType.STRING, 0, "FirstName", "测试角色"),
                    GFFElement(GFFFieldType.STRING, 0, "LastName", "テスト")
                ])
                mod_list = GFFElement(GFFFieldType.LIST, 0, "Mod_PlayerList", [player_struct])
                playerlist.fields.append(mod_list)
                
                output = BytesIO()
                writer.save(output, playerlist)
                zf.writestr('playerlist.ifo', output.getvalue())
                
                # Add player.bic
                bic_writer = GFFWriter(file_type='BIC ')
                bic_output = BytesIO()
                bic_writer.save(bic_output, player_struct)
                zf.writestr('player.bic', bic_output.getvalue())
            
            character = Character.objects.create(
                owner=test_user,
                file_path=test_save,
                file_name='unicode_save',
                first_name='测试角色',
                last_name='テスト',
                is_savegame=True,
                character_level=1
            )
            
            response = authenticated_client.post(f'/api/savegames/{character.id}/update/', {
                'updates': {'attributes': {'Str': 20}}
            }, format='json')
            
            assert response.status_code == status.HTTP_200_OK
        finally:
            shutil.rmtree(temp_dir)


# Improvements that could be made based on the savegame_views.py analysis:
# 1. Add user permission checks in list_savegame_companions endpoint
# 2. Add validation for attribute values (e.g., Str should be 3-25)
# 3. Add support for updating other character aspects (class, feats, spells, skills)
# 4. Add transaction rollback on backup creation failure
# 5. Add rate limiting for update operations
# 6. Add compression for large savegame backups
# 7. Add endpoint to restore from backup
# 8. Add validation that character.file_path still exists before operations
# 9. Add logging for all operations for audit trail
# 10. Add caching for frequently accessed savegame data