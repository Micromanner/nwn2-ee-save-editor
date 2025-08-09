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
                
                # Immediately update status when background thread starts
                initialization_status['started_at'] = datetime.now().isoformat()
                update_initialization_status('initializing', 5, 'Background initialization started...')
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
                        
                        # Check if icon cache has progress callback capability
                        cache_obj = gamedata.cache.icon_cache.icon_cache
                        if hasattr(cache_obj, 'initialize'):
                            # Try to add progress updates during icon cache loading
                            update_initialization_status('icon_cache', 35, 'Scanning for icons...')
                            cache_obj.initialize()
                            update_initialization_status('icon_cache', 45, 'Processing icons...')
                            time.sleep(0.1)  # Brief pause to show progress
                            update_initialization_status('icon_cache', 55, 'Building icon index...')
                        
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
                    
                    # Create progress callback that updates initialization status
                    def game_data_progress(message, percent):
                        # Map the internal progress (0-100%) to our range (65-90%)
                        mapped_progress = 65 + int(percent * 0.25)  # 25% of range (90-65=25)
                        update_initialization_status('game_data', mapped_progress, f'Game data: {message}')
                    
                    # Get shared ResourceManager and pass it to the loader
                    shared_rm = get_shared_resource_manager()
                    loader = get_dynamic_game_data_loader(
                        resource_manager=shared_rm, 
                        progress_callback=game_data_progress
                    )
                    
                    init_time = time.time() - start_time
                    initialization_status['details']['game_data'] = True
                    update_initialization_status('game_data', 90, f'Game data loaded')
                    logger.info(f"Background: Game data loader initialized in {init_time:.2f}s with {len(loader.table_data)} tables")
                except Exception as e:
                    logger.error(f"Background: Failed to initialize game data loader: {e}")
                    update_initialization_status('game_data', 90, f'Game data failed: {e}', error=str(e))
                
                # Stage 4: Build Prerequisite Graph (optional optimization)
                try:
                    # Check if prerequisite graph is enabled
                    import os
                    use_graph = os.environ.get('USE_PREREQUISITE_GRAPH', 'true').lower() == 'true'
                    
                    if not use_graph:
                        logger.info("Prerequisite graph disabled (USE_PREREQUISITE_GRAPH=false)")
                        update_initialization_status('prereq_graph', 98, 'Prerequisite graph disabled')
                    elif 'loader' not in locals() or not loader:
                        logger.warning("Prerequisite graph enabled but game data loader not available")
                        update_initialization_status('prereq_graph', 98, 'Prerequisite graph skipped (no loader)')
                    else:
                        update_initialization_status('prereq_graph', 92, 'Building prerequisite graph...')
                        logger.info("Background: Building feat prerequisite graph...")
                        from character.managers.prerequisite_graph import get_prerequisite_graph
                        start_time = time.time()
                        
                        # Build the graph with the game data loader
                        graph = get_prerequisite_graph(game_data_loader=loader)
                        
                        if graph and graph.is_built:
                            init_time = time.time() - start_time
                            stats = graph.get_statistics()
                            initialization_status['details']['prerequisite_graph'] = True
                            update_initialization_status('prereq_graph', 98, 
                                f'Prerequisite graph built ({stats["feats_with_prerequisites"]} feats with prereqs)')
                            logger.info(f"Background: Prerequisite graph built in {init_time:.2f}s - "
                                      f"{stats['total_feats']} feats, max chain depth: {stats['max_chain_depth']}")
                        else:
                            logger.warning("Background: Prerequisite graph failed to build")
                            update_initialization_status('prereq_graph', 98, 'Prerequisite graph skipped')
                except Exception as e:
                    logger.error(f"Background: Failed to build prerequisite graph: {e}")
                    update_initialization_status('prereq_graph', 98, f'Prerequisite graph failed: {e}', error=str(e))
                
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
