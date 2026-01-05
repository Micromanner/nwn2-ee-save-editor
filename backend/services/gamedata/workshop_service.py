"""Steam Workshop metadata service for NWN2 mod discovery."""
import re
import time
import requests
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from utils.paths import get_writable_dir
from datetime import datetime, timedelta
from bs4 import BeautifulSoup
from loguru import logger

from config.nwn2_settings import nwn2_paths
from services.core.safe_cache import SafeCache


class SteamWorkshopService:
    """Fetch and cache metadata for installed Steam Workshop mods without API keys."""
    
    def __init__(self, cache_dir: Optional[Path] = None, auto_cleanup: bool = True):
        # Use provided cache dir or default to AppData cache/workshop
        if cache_dir is None:
            self.cache_dir = get_writable_dir('cache/workshop')
        else:
            self.cache_dir = Path(cache_dir)
        
        self.cache_dir.mkdir(exist_ok=True)
        self.cache_file = self.cache_dir / 'workshop_metadata'
        self.auto_cleanup = auto_cleanup
        
        # Rate limiting
        self._last_request_time = 0
        self._request_delay = 1.0  # 1 second between requests
        
        # Get workshop directory from centralized nwn2_paths
        self._workshop_dirs = []
        if nwn2_paths.steam_workshop_folder and nwn2_paths.steam_workshop_folder.exists():
            self._workshop_dirs.append(nwn2_paths.steam_workshop_folder)
        
        # Load cache (after workshop dirs are set)
        self._mod_cache: Dict[str, Dict] = {}
        self._load_cache()
    
    
    def _get_installed_mod_ids(self) -> set:
        """Return IDs of currently installed workshop mods."""
        installed_ids = set()
        
        for workshop_dir in self._workshop_dirs:
            if not workshop_dir.exists():
                continue
            
            for mod_dir in workshop_dir.iterdir():
                if mod_dir.is_dir() and mod_dir.name.isdigit():
                    installed_ids.add(mod_dir.name)
        
        return installed_ids
    
    def _cleanup_stale_cache_entries(self):
        """Remove cached entries for mods no longer installed."""
        installed_ids = self._get_installed_mod_ids()
        stale_entries = []
        for mod_id in self._mod_cache:
            if mod_id not in installed_ids:
                stale_entries.append(mod_id)

        if stale_entries:
            for mod_id in stale_entries:
                del self._mod_cache[mod_id]
            logger.info(f"Removed {len(stale_entries)} stale entries: {stale_entries}")
            self._save_cache()
    
    def _load_cache(self):
        """Load cached mod metadata from disk."""
        if SafeCache.exists(self.cache_file):
            try:
                data = SafeCache.load(self.cache_file)
                if data is not None:
                    self._mod_cache = data
                    logger.info(f"Loaded workshop cache: {len(self._mod_cache)} entries")
                    if self.auto_cleanup:
                        self._cleanup_stale_cache_entries()
                else:
                    self._mod_cache = {}
            except Exception as e:
                logger.error(f"Error loading workshop cache: {e}")
                self._mod_cache = {}
        else:
            self._mod_cache = {}
    
    def _save_cache(self):
        """Save mod metadata cache to disk."""
        try:
            SafeCache.save(self.cache_file, self._mod_cache)
            logger.debug("Saved workshop cache")
        except Exception as e:
            logger.error(f"Error saving workshop cache: {e}")
    
    def _rate_limit(self):
        """Enforce rate limiting between requests."""
        current_time = time.time()
        time_since_last = current_time - self._last_request_time
        
        if time_since_last < self._request_delay:
            sleep_time = self._request_delay - time_since_last
            time.sleep(sleep_time)
        
        self._last_request_time = time.time()
    
    def _scrape_workshop_page(self, mod_id: str) -> Optional[Dict]:
        """Scrape Steam Workshop page for mod metadata."""
        url = f"https://steamcommunity.com/sharedfiles/filedetails/?id={mod_id}"
        
        try:
            self._rate_limit()

            response = requests.get(url, timeout=10)
            response.raise_for_status()

            soup = BeautifulSoup(response.text, 'html.parser')

            title_elem = soup.find('div', class_='workshopItemTitle')
            title = title_elem.text.strip() if title_elem else f"Workshop Item {mod_id}"

            desc_elem = soup.find('div', class_='workshopItemDescription')
            description = ""
            if desc_elem:
                desc_text = desc_elem.text.strip()
                description = desc_text[:500] + "..." if len(desc_text) > 500 else desc_text

            size = "Unknown"
            stat_elements = soup.find_all('div', class_='detailsStatRight')
            for elem in stat_elements:
                text = elem.text.strip()
                if 'KB' in text or 'MB' in text or 'GB' in text:
                    size = text
                    break

            author = "Unknown"
            author_elem = soup.find('div', class_='workshopItemAuthorName')
            if author_elem:
                author_link = author_elem.find('a')
                if author_link:
                    author = author_link.text.strip()

            updated = "Unknown"
            for elem in stat_elements:
                text = elem.text.strip()
                if '@' in text:  # Date format like "20 Jul @ 1:38pm"
                    updated = text
                    break
            
            metadata = {
                'id': mod_id,
                'title': title,
                'description': description,
                'author': author,
                'size': size,
                'updated': updated,
                'url': url,
                'cached_at': datetime.now().isoformat()
            }
            
            logger.info(f"Successfully scraped metadata for mod {mod_id}: {title}")
            return metadata
            
        except requests.RequestException as e:
            logger.error(f"Error fetching workshop page for {mod_id}: {e}")
            return None
        except Exception as e:
            logger.error(f"Error parsing workshop page for {mod_id}: {e}")
            return None
    
    def get_installed_mods(self, force_refresh: bool = False) -> List[Dict]:
        """Return metadata for all installed workshop mods."""
        mods = []
        
        for workshop_dir in self._workshop_dirs:
            if not workshop_dir.exists():
                continue
            
            for mod_dir in workshop_dir.iterdir():
                if not mod_dir.is_dir():
                    continue

                mod_id = mod_dir.name
                
                if not mod_id.isdigit():
                    continue

                metadata = self.get_mod_metadata(mod_id, force_refresh)
                if metadata:
                    metadata['install_path'] = str(mod_dir)
                    mods.append(metadata)

        mods.sort(key=lambda x: x.get('title', ''))
        
        return mods
    
    def get_mod_metadata(self, mod_id: str, force_refresh: bool = False) -> Optional[Dict]:
        """Return metadata for a specific mod, using cache when available."""
        if not force_refresh and mod_id in self._mod_cache:
            cached = self._mod_cache[mod_id]

            if 'cached_at' in cached:
                cached_time = datetime.fromisoformat(cached['cached_at'])
                if datetime.now() - cached_time < timedelta(days=7):
                    logger.debug(f"Using cached metadata for mod {mod_id}")
                    return cached

        metadata = self._scrape_workshop_page(mod_id)
        
        if metadata:
            self._mod_cache[mod_id] = metadata
            self._save_cache()
        
        return metadata
    
    def get_mod_name(self, mod_id: str) -> str:
        """Return mod title or fallback string."""
        metadata = self.get_mod_metadata(mod_id)
        if metadata:
            return metadata['title']
        return f"Workshop Item {mod_id}"
    
    def clear_cache(self):
        """Clear all cached metadata."""
        self._mod_cache.clear()
        self._save_cache()
        logger.info("Cleared workshop metadata cache")
    
    def cleanup_cache(self):
        """Remove cache entries for uninstalled mods and return count removed."""
        initial_count = len(self._mod_cache)
        self._cleanup_stale_cache_entries()
        final_count = len(self._mod_cache)
        removed = initial_count - final_count
        logger.info(f"Cache cleanup complete: removed {removed} stale entries")
        return removed
    
    def get_cache_stats(self) -> Dict:
        """Return statistics about the cache."""
        stats = {
            'total_cached': len(self._mod_cache),
            'cache_file': str(self.cache_file),
            'cache_size_kb': self.cache_file.stat().st_size / 1024 if self.cache_file.exists() else 0
        }

        if self._mod_cache:
            cache_times = []
            for mod_data in self._mod_cache.values():
                if 'cached_at' in mod_data:
                    cache_times.append(datetime.fromisoformat(mod_data['cached_at']))
            
            if cache_times:
                stats['oldest_cache'] = min(cache_times).isoformat()
                stats['newest_cache'] = max(cache_times).isoformat()
        
        return stats
    
    def find_mod_by_name(self, search_term: str) -> List[Dict]:
        """Search installed mods by name."""
        search_lower = search_term.lower()
        matches = []

        for mod_id, mod_data in self._mod_cache.items():
            if search_lower in mod_data.get('title', '').lower():
                matches.append(mod_data)

        if not matches:
            for mod in self.get_installed_mods():
                if search_lower in mod.get('title', '').lower():
                    matches.append(mod)

        return matches