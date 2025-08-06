from django.apps import AppConfig
from django.core.management import call_command
import logging

logger = logging.getLogger(__name__)


class GamedataConfig(AppConfig):
    default_auto_field = 'django.db.models.BigAutoField'
    name = 'gamedata'
    
    def ready(self):
        """Run startup tasks when the app is ready."""
        # Only run in main process, not in migrations or commands
        import sys
        import os
        
        # Skip initialization in the reloader process (but not with --noreload)
        if os.environ.get('RUN_MAIN') != 'true' and 'runserver' in sys.argv and '--noreload' not in sys.argv:
            return
            
        if 'runserver' in sys.argv or 'gunicorn' in sys.argv[0]:
            # Initialize minimal components for quick startup
            import threading
            
            # Create icon cache instance but don't initialize it yet
            try:
                from .middleware import get_shared_resource_manager
                resource_manager = get_shared_resource_manager()
                
                from .cache.icon_cache import create_icon_cache
                import gamedata.cache.icon_cache
                gamedata.cache.icon_cache.icon_cache = create_icon_cache(resource_manager)
                logger.info("Icon cache instance created (not initialized yet)")
            except Exception as e:
                logger.warning(f"Failed to create icon cache instance: {e}")
            
            # Start background initialization thread
            def initialize_heavy_components():
                """Initialize heavy components in background."""
                import time
                from api.system_views import update_initialization_status, initialization_status
                from datetime import datetime
                
                initialization_status['started_at'] = datetime.now().isoformat()
                logger.info("Starting background initialization of heavy components...")
                
                # Stage 1: Resource Manager
                try:
                    update_initialization_status('resource_manager', 10, 'Initializing Resource Manager...')
                    from .middleware import get_shared_resource_manager
                    rm = get_shared_resource_manager()
                    initialization_status['details']['resource_manager'] = True
                    update_initialization_status('resource_manager', 25, 'Resource Manager ready')
                except Exception as e:
                    logger.error(f"Background: Failed to initialize resource manager: {e}")
                    update_initialization_status('resource_manager', 25, f'Resource Manager failed: {e}', error=str(e))
                
                # Stage 2: Initialize icon cache
                try:
                    if hasattr(gamedata.cache.icon_cache, 'icon_cache') and gamedata.cache.icon_cache.icon_cache:
                        update_initialization_status('icon_cache', 30, 'Loading icon cache (4000+ icons)...')
                        logger.info("Background: Initializing icon cache...")
                        start_time = time.time()
                        gamedata.cache.icon_cache.icon_cache.initialize()
                        init_time = time.time() - start_time
                        initialization_status['details']['icon_cache'] = True
                        update_initialization_status('icon_cache', 60, f'Icon cache loaded in {init_time:.1f}s')
                        logger.info(f"Background: Icon cache initialized in {init_time:.2f}s")
                except Exception as e:
                    logger.error(f"Background: Failed to initialize icon cache: {e}")
                    update_initialization_status('icon_cache', 60, f'Icon cache failed: {e}', error=str(e))
                
                # Stage 3: Initialize DynamicGameDataLoader with shared ResourceManager
                try:
                    update_initialization_status('game_data', 65, 'Loading game data...')
                    logger.info("Background: Initializing game data loader with shared ResourceManager...")
                    from .middleware import get_shared_resource_manager
                    from .dynamic_loader.singleton import get_dynamic_game_data_loader
                    start_time = time.time()
                    
                    # Get shared ResourceManager and pass it to the loader
                    shared_rm = get_shared_resource_manager()
                    loader = get_dynamic_game_data_loader(resource_manager=shared_rm)
                    
                    init_time = time.time() - start_time
                    initialization_status['details']['game_data'] = True
                    update_initialization_status('game_data', 95, f'Game data loaded')
                    logger.info(f"Background: Game data loader initialized in {init_time:.2f}s with {len(loader.table_data)} tables")
                except Exception as e:
                    logger.error(f"Background: Failed to initialize game data loader: {e}")
                    update_initialization_status('game_data', 95, f'Game data failed: {e}', error=str(e))
                
                # All done
                update_initialization_status('ready', 100, 'All systems ready!')
                logger.info("Background initialization complete")
            
            # Start background thread with a small delay to ensure Django is ready
            def delayed_start():
                import time
                time.sleep(0.5)  # Let Django fully start
                initialize_heavy_components()
            
            init_thread = threading.Thread(target=delayed_start, daemon=True)
            init_thread.start()
            logger.info("Background initialization thread scheduled")
            
            
            # Module cache warming - skip in DEBUG or if explicitly disabled
            import os
            from django.conf import settings
            warm_cache = os.environ.get('WARM_MODULE_CACHE', 'true').lower() == 'true'
            
            if warm_cache and not settings.DEBUG:
                try:
                    # Warm module cache for common modules
                    from .middleware import warm_common_modules
                    warm_common_modules()
                    logger.info("Module cache warming completed")
                except Exception as e:
                    logger.warning(f"Failed to warm module cache: {e}")
            else:
                logger.info("Module cache warming skipped (DEBUG mode or WARM_MODULE_CACHE=false)")
            
            # Enable hot reload in development mode
            from django.conf import settings
            if settings.DEBUG:
                try:
                    from .hot_reload import enable_hot_reload
                    if enable_hot_reload():
                        logger.info("Hot reload enabled for development")
                except Exception as e:
                    logger.warning(f"Failed to enable hot reload: {e}")
