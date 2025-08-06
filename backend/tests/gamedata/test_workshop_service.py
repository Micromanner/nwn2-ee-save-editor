import pytest
from pathlib import Path
from datetime import datetime, timedelta

from gamedata.workshop_service import SteamWorkshopService


@pytest.fixture
def temp_env(tmp_path):
    """A fixture to create a temporary environment with cache and workshop directories."""
    cache_dir = tmp_path / 'cache'
    cache_dir.mkdir()
    workshop_dir = tmp_path / 'workshop' / 'content' / '2738630'
    workshop_dir.mkdir(parents=True)
    
    mod_ids = ['123456', '789012', '345678']
    for mod_id in mod_ids:
        (workshop_dir / mod_id).mkdir()
        
    return {'cache_dir': cache_dir, 'workshop_dir': workshop_dir, 'mod_ids': mod_ids}


@pytest.fixture
def workshop_service(temp_env):
    """Provides a SteamWorkshopService instance configured with the temporary environment."""
    service = SteamWorkshopService(cache_dir=temp_env['cache_dir'], auto_cleanup=False)
    service._workshop_dirs = [temp_env['workshop_dir']]
    return service


class TestSteamWorkshopService:
    """Test cases for Steam Workshop service."""

    def test_init(self, temp_env):
        """Test service initialization."""
        service = SteamWorkshopService(cache_dir=temp_env['cache_dir'])
        assert service.cache_dir == temp_env['cache_dir']
        assert isinstance(service._mod_cache, dict)

    def test_get_installed_mod_ids(self, workshop_service, temp_env):
        """Test getting installed mod IDs."""
        installed_ids = workshop_service._get_installed_mod_ids()
        assert len(installed_ids) == len(temp_env['mod_ids'])
        assert set(installed_ids) == set(temp_env['mod_ids'])

    def test_scrape_workshop_page_success(self, mocker, workshop_service):
        """Test successful Steam Workshop page scraping."""
        mock_response = mocker.Mock()
        mock_response.status_code = 200
        mock_response.text = '''
        <html>
            <div class="workshopItemTitle">Test Mod Title</div>
            <div class="workshopItemAuthorName"><a>Test Author</a></div>
            <div class="detailsStatRight">1.5 MB</div>
        </html>'''
        mock_get = mocker.patch('gamedata.workshop_service.requests.get', return_value=mock_response)

        metadata = workshop_service._scrape_workshop_page('123456')

        assert metadata is not None
        assert metadata['id'] == '123456'
        assert metadata['title'] == 'Test Mod Title'
        assert metadata['author'] == 'Test Author'
        assert metadata['size'] == '1.5 MB'

    def test_scrape_workshop_page_failure(self, mocker, workshop_service):
        """Test failed Steam Workshop page scraping."""
        mocker.patch('gamedata.workshop_service.requests.get', side_effect=Exception("Network error"))
        metadata = workshop_service._scrape_workshop_page('123456')
        assert metadata is None

    def test_cache_operations(self, temp_env):
        """Test cache save/load operations."""
        service1 = SteamWorkshopService(cache_dir=temp_env['cache_dir'])
        service1._mod_cache['123456'] = {'title': 'Test Mod'}
        service1._save_cache()
        assert service1.cache_file.exists()

        # Create new service instance to test loading
        service2 = SteamWorkshopService(cache_dir=temp_env['cache_dir'])
        assert '123456' in service2._mod_cache
        assert service2._mod_cache['123456']['title'] == 'Test Mod'

    def test_cleanup_stale_cache_entries(self, workshop_service):
        """Test removal of cache entries for uninstalled mods."""
        workshop_service._mod_cache = {
            '123456': {'title': 'Installed Mod 1'},   # Installed
            '999999': {'title': 'Uninstalled Mod'},   # Not installed
            '789012': {'title': 'Installed Mod 2'},   # Installed
        }
        workshop_service._cleanup_stale_cache_entries()

        assert '123456' in workshop_service._mod_cache
        assert '789012' in workshop_service._mod_cache
        assert '999999' not in workshop_service._mod_cache

    def test_get_mod_metadata_with_cache(self, mocker, workshop_service):
        """Test getting mod metadata uses a fresh cache entry."""
        mock_scrape = mocker.patch('gamedata.workshop_service.SteamWorkshopService._scrape_workshop_page')
        cached_data = {
            'id': '123456',
            'title': 'Cached Mod',
            'cached_at': datetime.now().isoformat()
        }
        workshop_service._mod_cache['123456'] = cached_data

        metadata = workshop_service.get_mod_metadata('123456')

        assert metadata['title'] == 'Cached Mod'
        mock_scrape.assert_not_called()

    def test_get_mod_metadata_stale_cache(self, mocker, workshop_service):
        """Test getting mod metadata with a stale cache triggers a re-scrape."""
        old_date = datetime.now() - timedelta(days=8)
        workshop_service._mod_cache['123456'] = {
            'id': '123456',
            'title': 'Old Cached Mod',
            'cached_at': old_date.isoformat()
        }
        mock_scrape = mocker.patch(
            'gamedata.workshop_service.SteamWorkshopService._scrape_workshop_page',
            return_value={'id': '123456', 'title': 'Fresh Mod Data'}
        )

        metadata = workshop_service.get_mod_metadata('123456')

        assert metadata['title'] == 'Fresh Mod Data'
        mock_scrape.assert_called_once_with('123456')

    def test_get_installed_mods(self, mocker, workshop_service):
        """Test getting all installed mods, sorted by title."""
        mock_scrape = mocker.patch(
            'gamedata.workshop_service.SteamWorkshopService._scrape_workshop_page',
            side_effect=[
                {'id': '123456', 'title': 'Mod A'},
                {'id': '789012', 'title': 'Mod C'},
                {'id': '345678', 'title': 'Mod B'},
            ]
        )

        mods = workshop_service.get_installed_mods()

        assert len(mods) == 3
        # Should be sorted by title
        assert mods[0]['title'] == 'Mod A'
        assert mods[1]['title'] == 'Mod B'
        assert mods[2]['title'] == 'Mod C'
        assert all('install_path' in mod for mod in mods)

    def test_clear_cache(self, workshop_service):
        """Test clearing the entire cache."""
        workshop_service._mod_cache['123456'] = {'title': 'Test Mod'}
        workshop_service._save_cache()

        workshop_service.clear_cache()
        assert not workshop_service._mod_cache
        
        # Verify cache file is also empty on reload
        new_service = SteamWorkshopService(cache_dir=workshop_service.cache_dir)
        assert not new_service._mod_cache