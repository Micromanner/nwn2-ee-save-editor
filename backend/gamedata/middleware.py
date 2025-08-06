"""
Middleware for managing module context in user sessions
"""
from django.utils.deprecation import MiddlewareMixin
from parsers.resource_manager import ResourceManager
from gamedata.services.game_rules_service import GameRulesService
from django.conf import settings
import threading
import logging
import time
from pathlib import Path
from typing import List, Optional
from config.nwn2_settings import nwn2_paths


logger = logging.getLogger(__name__)

# Thread-local storage for request-specific context
_thread_locals = threading.local()

# Global cache for common modules (populated on startup)
_common_module_cache = {}

# Global singleton for base ResourceManager and GameRulesService (prevents redundant 2DA loading)
_global_base_services = {
    'resource_manager': None,
    'game_rules_service': None
}
_services_lock = threading.Lock()
_resource_manager_initializing = False


class ModuleContextMiddleware(MiddlewareMixin):
    """
    Middleware to manage module context per user session.
    
    This middleware:
    1. Tracks the current module for each user session
    2. Creates and caches ResourceManager instances per module
    3. Creates and caches GameRulesService instances per module
    4. Makes them available via thread-local storage
    """
    
    def __init__(self, get_response):
        self.get_response = get_response
        # Cache ResourceManager and GameRulesService instances per module
        self._resource_managers = {}
        self._game_rules_services = {}
        self._cache_generation = 0  # Track cache invalidation
        self._init_lock = threading.Lock()  # Thread safety for initialization
        
        # Don't initialize services here - let them be created on demand
        # This allows Django to start responding immediately
        logger.info("Middleware initialized - services will be created on first request")
        
        # Shared ResourceManager will be created on first request
    
    def process_request(self, request):
        """Process incoming request and set up module context"""
        # Ensure base services are initialized on first request
        if None not in self._resource_managers:
            with self._init_lock:
                if None not in self._resource_managers:  # Double-check after acquiring lock
                    # Use the shared singleton instead of creating a separate instance
                    resource_manager = get_shared_resource_manager()
                    
                    global _global_base_services, _services_lock
                    with _services_lock:
                        if _global_base_services['game_rules_service'] is None:
                            logger.info("Creating GameRulesService with shared ResourceManager...")
                            _global_base_services['game_rules_service'] = GameRulesService(
                                resource_manager=resource_manager, 
                                load_mode='full'
                            )
                    
                    self._resource_managers[None] = resource_manager
                    self._game_rules_services[None] = _global_base_services['game_rules_service']
        
        # Get module from session
        module_path = request.session.get('current_module')
        
        # Get or create ResourceManager for this module
        if module_path and module_path not in self._resource_managers:
            # First check if it's in the warmed cache
            cached_rm, cached_grs = get_cached_module_services(module_path)
            if cached_rm and cached_grs:
                self._resource_managers[module_path] = cached_rm
                self._game_rules_services[module_path] = cached_grs
                logger.debug(f"Using warmed cache for module: {Path(module_path).name}")
            else:
                # Create new ResourceManager
                rm = ResourceManager(str(nwn2_paths.game_folder), suppress_warnings=True)
                
                # Load the module
                if rm.set_module(module_path):
                    self._resource_managers[module_path] = rm
                    
                    # Create corresponding GameRulesService
                    grs = GameRulesService(rm)
                    self._game_rules_services[module_path] = grs
                else:
                    # Failed to load module
                    module_path = None
        
        # Set thread-local context
        if module_path:
            _thread_locals.resource_manager = self._resource_managers.get(module_path)
            _thread_locals.game_rules_service = self._game_rules_services.get(module_path)
        else:
            # Use default (base game only) - already pre-initialized during startup
            _thread_locals.resource_manager = self._resource_managers[None]
            _thread_locals.game_rules_service = self._game_rules_services[None]
        
        # Store current module path in thread locals for views
        _thread_locals.current_module = module_path
    
    def process_response(self, request, response):
        """Clean up thread locals after request"""
        # Clean up thread locals
        if hasattr(_thread_locals, 'resource_manager'):
            del _thread_locals.resource_manager
        if hasattr(_thread_locals, 'game_rules_service'):
            del _thread_locals.game_rules_service
        if hasattr(_thread_locals, 'current_module'):
            del _thread_locals.current_module
        # NOTE: character_managers cache is intentionally NOT cleared here
        # to preserve in-memory changes between requests
        
        return response


def get_resource_manager():
    """Get the current request's ResourceManager"""
    return getattr(_thread_locals, 'resource_manager', None)


def get_game_rules_service():
    """Get the current request's GameRulesService"""
    return getattr(_thread_locals, 'game_rules_service', None)


def get_current_module():
    """Get the current request's module path"""
    return getattr(_thread_locals, 'current_module', None)


def get_character_manager(character_id: int):
    """
    Get cached CharacterManager for the current request.
    
    Args:
        character_id: The character's database ID
        
    Returns:
        CharacterManager instance or None if not cached
    """
    managers = getattr(_thread_locals, 'character_managers', {})
    return managers.get(character_id)


def set_character_manager(character_id: int, manager):
    """
    Cache a CharacterManager instance for the current request.
    
    Args:
        character_id: The character's database ID
        manager: CharacterManager instance to cache
    """
    if not hasattr(_thread_locals, 'character_managers'):
        _thread_locals.character_managers = {}
    _thread_locals.character_managers[character_id] = manager
    logger.debug(f"Cached CharacterManager for character {character_id}")


def clear_character_manager(character_id: int = None):
    """
    Clear cached CharacterManager(s).
    
    Args:
        character_id: Specific character ID to clear, or None to clear all
    """
    if hasattr(_thread_locals, 'character_managers'):
        if character_id is not None:
            _thread_locals.character_managers.pop(character_id, None)
            logger.debug(f"Cleared CharacterManager cache for character {character_id}")
        else:
            _thread_locals.character_managers.clear()
            logger.debug("Cleared all CharacterManager caches")


def set_module_for_session(request, module_path: str):
    """
    Set the module for the current user session
    
    Args:
        request: Django request object
        module_path: Path to .mod file or module directory
    """
    request.session['current_module'] = module_path
    request.session.save()


def get_shared_resource_manager() -> ResourceManager:
    """Get the shared ResourceManager instance"""
    global _global_base_services, _services_lock, _resource_manager_initializing
    
    with _services_lock:
        if _global_base_services['resource_manager'] is None and not _resource_manager_initializing:
            _resource_manager_initializing = True
            logger.info("Creating shared ResourceManager instance")
            try:
                _global_base_services['resource_manager'] = ResourceManager(
                    str(nwn2_paths.game_folder), 
                    suppress_warnings=True
                )
                logger.info("Shared ResourceManager instance created successfully")
            finally:
                _resource_manager_initializing = False
        elif _resource_manager_initializing:
            # Wait for initialization to complete in another thread
            import time
            logger.info("ResourceManager is being initialized by another thread, waiting...")
            while _resource_manager_initializing and _global_base_services['resource_manager'] is None:
                time.sleep(0.1)
        
    return _global_base_services['resource_manager']


def warm_common_modules(module_paths: Optional[List[str]] = None):
    """
    Pre-load commonly used modules into cache on startup.
    
    Args:
        module_paths: List of module paths to preload. If None, loads default common modules.
    """
    global _common_module_cache
    
    if module_paths is None:
        # Default common modules - official campaigns and popular mods
        module_paths = []
        
        # Check for official campaign modules
        campaigns_path = nwn2_paths.campaigns
        if campaigns_path.exists():
            # Original Campaign
            nwn_oc = campaigns_path / "Neverwinter Nights 2 Campaign"
            if nwn_oc.exists():
                # Add key modules from the original campaign
                module_paths.extend([
                    str(nwn_oc / "1000_Neverwinter_A1.mod"),
                    str(nwn_oc / "1100_West_Harbor.mod"),
                    str(nwn_oc / "1700_Merchant_Quarter.mod"),
                ])
            
            # Mask of the Betrayer
            motb = campaigns_path / "Neverwinter Nights 2 Campaign_X1"
            if motb.exists():
                module_paths.append(str(motb / "3000_Rashemen.mod"))
            
            # Storm of Zehir  
            soz = campaigns_path / "Neverwinter Nights 2 Campaign_X2"
            if soz.exists():
                module_paths.append(str(soz / "N_X2_Overland.mod"))
        
        # Check for popular modded content
        modules_path = nwn2_paths.modules
        if modules_path.exists():
            # Add any frequently used custom modules here based on usage patterns
            # For now, we'll just check if specific popular mods exist
            popular_mods = [
                "Kaedrin's PrC Pack.mod",  # Example popular mod
            ]
            
            for mod_name in popular_mods:
                mod_path = modules_path / mod_name
                if mod_path.exists():
                    module_paths.append(str(mod_path))
    
    logger.info(f"Starting cache warming for {len(module_paths)} modules...")
    
    for module_path in module_paths:
        if not Path(module_path).exists():
            continue
            
        try:
            # Create ResourceManager and load module
            rm = ResourceManager(str(nwn2_paths.game_folder), suppress_warnings=True)
            
            if rm.set_module(module_path):
                # Create GameRulesService
                grs = GameRulesService(rm)
                
                # Pre-load commonly accessed data to warm the cache
                # Access properties to trigger loading
                _ = grs.classes
                _ = grs.feats
                _ = grs.races
                _ = grs.skills
                _ = grs.spells
                
                # Store in global cache
                _common_module_cache[module_path] = {
                    'resource_manager': rm,
                    'game_rules_service': grs
                }
                
                logger.info(f"Successfully warmed cache for module: {Path(module_path).name}")
            else:
                logger.warning(f"Failed to load module for cache warming: {module_path}")
                
        except Exception as e:
            logger.error(f"Error warming cache for module {module_path}: {e}")
    
    logger.info(f"Cache warming complete. Loaded {len(_common_module_cache)} modules.")


def get_cached_module_services(module_path: str):
    """
    Get ResourceManager and GameRulesService from cache if available.
    
    Returns:
        tuple: (ResourceManager, GameRulesService) or (None, None) if not cached
    """
    cached = _common_module_cache.get(module_path)
    if cached:
        return cached['resource_manager'], cached['game_rules_service']
    return None, None