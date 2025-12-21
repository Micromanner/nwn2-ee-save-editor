"""
Enhanced icon cache using Rust implementation for high performance.
"""
import logging
from typing import Dict, Optional, Tuple, List
from pathlib import Path

from nwn2_rust import ErfParser
from config.nwn2_settings import nwn2_paths

logger = logging.getLogger(__name__)

# Import Rust icon cache - required, no fallback
try:
    from rust_icon_cache import RustIconCache  # type: ignore
except ImportError as e:
    raise ImportError(
        "Rust icon cache module not found. Please ensure it's built with: "
        "cd backend/parsers/rust_icon_cache && maturin develop"
    ) from e

class RustIconCacheWrapper:
    """Wrapper to make Rust cache API compatible with existing Python interface"""
    
    def __init__(self, rust_cache: RustIconCache, resource_manager=None):
        self.rust_cache = rust_cache
        self._resource_manager = resource_manager
        self._initialized = False
        self._initializing = False
        
        # Keep format info for compatibility
        self.icon_format = 'WebP'
        self.icon_mimetype = 'image/webp'
    
    def set_resource_manager(self, resource_manager):
        """Set or update the resource manager."""
        self._resource_manager = resource_manager
    
    def initialize(self, force_reload: bool = False, background: bool = True):  # background kept for compatibility
        """Initialize the cache"""
        if self._initialized and not force_reload:
            return
        
        if self._initializing:
            return
        
        self._initializing = True
        
        try:
            # Use the synchronous initialization method which handles Tokio runtime internally
            logger.info("Initializing Rust icon cache using synchronous method")
            self.rust_cache.initialize_sync(force_reload)
            self._initialized = True
            logger.info("Rust icon cache initialized successfully")
        except Exception as e:
            logger.error(f"Error initializing Rust icon cache: {e}")
            # Don't raise - allow app to continue without icon cache
            self._initialized = False
        finally:
            self._initializing = False
    
    def get_icon(self, icon_name: str) -> Optional[Tuple[bytes, str]]:
        """Get icon data following the override hierarchy with case-insensitive fallback."""
        if not self._initialized and not self._initializing:
            try:
                self.initialize()
            except Exception as e:
                logger.warning(f"Failed to initialize icon cache on demand: {e}")
                return None, None
        
        if not self._initialized:
            # Cache failed to initialize
            return None, None
        
        # Try exact match first
        result = self.rust_cache.get_icon(icon_name)
        if result:
            data, mime = result
            return data, mime
        
        # If not found, try lowercase version for case-insensitive lookup
        if icon_name != icon_name.lower():
            logger.debug(f"Icon '{icon_name}' not found, trying lowercase version")
            result = self.rust_cache.get_icon(icon_name.lower())
            if result:
                data, mime = result
                logger.debug(f"Found icon with lowercase name: '{icon_name.lower()}'")
                return data, mime
        
        # If still not found, try filename-only lookup as fallback
        # This handles cases where frontend sends "is_calllightning" but cache has "evocation/spell/is_calllightning"
        if '/' not in icon_name:
            logger.debug(f"Icon '{icon_name}' not found with direct lookup, searching by filename...")
            # This is a temporary solution - ideally frontend should send correct paths
            # For now, we'll need the Rust cache to support filename-only lookup
            pass
        
        return None, None
    
    def get_icon_by_path(self, path: str) -> Optional[Tuple[bytes, str]]:
        """Get icon by path - just delegates to get_icon"""
        return self.get_icon(path)
    
    def set_module_haks(self, hak_list: List[str]):
        """Set the HAK files for the current module."""
        try:
            self.rust_cache.set_module_haks_sync(hak_list)
        except Exception as e:
            logger.error(f"Error setting module HAKs: {e}")
    
    def load_hak_icons(self, hak_name: str):  # hak_name kept for compatibility
        """Load icons from a HAK file."""
        # Handled internally by Rust cache through set_module_haks
        pass
    
    def load_module_icons(self, module_parser: ERFParser):
        """Load icons from a module file."""
        # Would need to pass module path to Rust cache
        if hasattr(module_parser, 'filename'):
            try:
                self.rust_cache.load_module_icons_sync(str(module_parser.filename))
            except Exception as e:
                logger.error(f"Error loading module icons: {e}")
    
    def get_statistics(self) -> Dict[str, int]:
        """Get current cache statistics."""
        try:
            return self.rust_cache.get_statistics()
        except Exception as e:
            logger.error(f"Error getting statistics: {e}")
            return {
                'total_count': 0,
                'base_count': 0,
                'override_count': 0,
                'workshop_count': 0,
                'hak_count': 0,
                'module_count': 0,
                'total_size': 0,
            }
    
    # For compatibility with tests/code that check these attributes
    @property
    def _loaded_haks(self):
        """Compatibility property for loaded HAKs"""
        return set()  # Rust cache manages this internally


def create_icon_cache(resource_manager=None):
    """Create the Rust icon cache implementation"""
    cache_dir = getattr(nwn2_paths, 'cache_dir', Path('cache')) / 'icons'
    cache_dir.mkdir(parents=True, exist_ok=True)
    
    # Get NWN2 game directory
    game_folder = getattr(nwn2_paths, 'game_folder', None)
    if game_folder:
        game_folder_str = str(game_folder)
        logger.info(f"Creating Rust icon cache with NWN2 game folder: {game_folder_str}")
    else:
        game_folder_str = None
        logger.warning("No NWN2 game folder found in nwn2_paths")
    
    try:
        rust_cache = RustIconCache(str(cache_dir), game_folder_str)
        logger.info("Created Rust icon cache instance")
        return RustIconCacheWrapper(rust_cache, resource_manager)
    except Exception as e:
        logger.error(f"Failed to create Rust icon cache: {e}")
        raise RuntimeError(f"Rust icon cache is required but failed to initialize: {e}")


# Global icon cache instance - will be initialized with ResourceManager later
icon_cache = None