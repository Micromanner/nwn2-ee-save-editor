"""
Hot reload functionality for development - watches override directories for changes
"""
import logging
import threading
from pathlib import Path
from typing import Set, Optional
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler, FileSystemEvent
from config.nwn2_settings import nwn2_paths
from .middleware import _common_module_cache


logger = logging.getLogger(__name__)


class OverrideFileHandler(FileSystemEventHandler):
    """
    Handles file system events in override directories.
    When 2DA files are modified, it clears relevant caches.
    """
    
    def __init__(self):
        self._affected_modules: Set[str] = set()
        self._cache_lock = threading.Lock()
    
    def on_any_event(self, event: FileSystemEvent):
        """Handle any file system event"""
        if event.is_directory:
            return
            
        # Check if it's a 2DA file
        path = Path(event.src_path)
        if path.suffix.lower() != '.2da':
            return
            
        # Log the change
        event_type = event.event_type
        logger.info(f"2DA file {event_type}: {path.name}")
        
        # Clear caches for affected modules
        self._clear_affected_caches(path)
    
    def _clear_affected_caches(self, changed_file: Path):
        """Clear caches that might be affected by this file change"""
        with self._cache_lock:
            # Clear middleware cache for all modules
            # Since override affects all modules, we need to clear everything
            if _common_module_cache:
                logger.info(f"Clearing module cache due to change in {changed_file.name}")
                _common_module_cache.clear()
            
            # Also clear any ResourceManager instances in middleware
            # This will be done on next request via the middleware
            
            # TODO: Implement more granular cache invalidation
            # For now, we clear everything to ensure consistency


class HotReloadManager:
    """
    Manages hot reload functionality for development.
    Watches override directories and clears caches when files change.
    """
    
    def __init__(self):
        self.observer: Optional[Observer] = None
        self.handler = OverrideFileHandler()
        self._watched_paths: Set[Path] = set()
        self._running = False
    
    def start(self):
        """Start watching override directories"""
        if self._running:
            logger.warning("Hot reload already running")
            return
            
        self.observer = Observer()
        
        # Watch game override directory
        if nwn2_paths.override.exists():
            self._watch_directory(nwn2_paths.override)
        
        # Watch user override directory
        if nwn2_paths.user_override.exists():
            self._watch_directory(nwn2_paths.user_override)
        
        # Watch Steam Workshop directories
        workshop_base = nwn2_paths.game_folder / "steamapps" / "workshop" / "content" / "2738630"
        if workshop_base.exists():
            # Watch each workshop item's override directory
            for workshop_item in workshop_base.iterdir():
                if workshop_item.is_dir():
                    override_dir = workshop_item / "override"
                    if override_dir.exists():
                        self._watch_directory(override_dir)
        
        # Start the observer
        self.observer.start()
        self._running = True
        logger.info(f"Hot reload started, watching {len(self._watched_paths)} directories")
    
    def _watch_directory(self, directory: Path):
        """Add a directory to the watch list"""
        try:
            self.observer.schedule(self.handler, str(directory), recursive=True)
            self._watched_paths.add(directory)
            logger.debug(f"Watching directory: {directory}")
        except Exception as e:
            logger.error(f"Failed to watch directory {directory}: {e}")
    
    def stop(self):
        """Stop watching directories"""
        if not self._running or not self.observer:
            return
            
        self.observer.stop()
        self.observer.join(timeout=5)
        self._running = False
        self._watched_paths.clear()
        logger.info("Hot reload stopped")
    
    def is_running(self) -> bool:
        """Check if hot reload is running"""
        return self._running
    
    def get_watched_paths(self) -> Set[Path]:
        """Get the list of watched paths"""
        return self._watched_paths.copy()


# Global instance
hot_reload_manager = HotReloadManager()


def enable_hot_reload():
    """Enable hot reload for development"""
    try:
        hot_reload_manager.start()
        return True
    except Exception as e:
        logger.error(f"Failed to enable hot reload: {e}")
        return False


def disable_hot_reload():
    """Disable hot reload"""
    try:
        hot_reload_manager.stop()
        return True
    except Exception as e:
        logger.error(f"Failed to disable hot reload: {e}")
        return False


def is_hot_reload_enabled() -> bool:
    """Check if hot reload is enabled"""
    return hot_reload_manager.is_running()