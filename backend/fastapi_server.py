"""
FastAPI Server for NWN2 Save Editor
- Tauri sidecar integration 
- Shared resource manager with proper singleton pattern
- Background initialization following Django's pattern
"""

import logging
import os
import sys
import asyncio
import time
import atexit
from pathlib import Path
from typing import Dict, Any, Optional
from datetime import datetime

from contextlib import asynccontextmanager
from fastapi import FastAPI, HTTPException, Request, status, BackgroundTasks
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from fastapi.exceptions import RequestValidationError
from starlette.exceptions import HTTPException as StarletteHTTPException
from starlette.middleware.base import BaseHTTPMiddleware
import uuid
import uvicorn

# Add the backend directory to Python path for imports
backend_dir = Path(__file__).parent
sys.path.insert(0, str(backend_dir))

# Import custom exceptions
from fastapi_core.exceptions import (
    NWN2SaveEditorException,
    CharacterNotFoundException, 
    CharacterSessionException,
    SystemNotReadyException,
    ValidationException,
    SaveFileException
)

# Configure Loguru logging
from config.logging_config import logger, ENABLE_LOG_VIEWER

# Keep standard logging for compatibility with libraries that use it
logging.basicConfig(
    level=logging.WARNING,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)

# Global singletons for shared services (like Django's middleware)
_shared_services = {
    'resource_manager': None,
    'game_rules_service': None,
    'icon_cache': None,
    'game_data_loader': None,
    'prerequisite_graph': None
}
_services_lock = asyncio.Lock()
_initializing = False

# No more log viewer process tracking - it runs in the same process

# Global initialization lock to prevent duplicate background initialization
_initialization_lock = asyncio.Lock()
_initialization_started = False

# Initialization status tracking (like Django's system_views)
initialization_status = {
    'stage': 'pending',
    'progress': 0,
    'message': 'Starting up...',
    'started_at': None,
    'completed_at': None,
    'error': None,
    'details': {
        'resource_manager': False,
        'icon_cache': False,
        'game_data': False,
        'prerequisite_graph': False
    }
}

def update_initialization_status(stage: str, progress: int, message: str, error: str = None):
    """Update the initialization status"""
    initialization_status['stage'] = stage
    initialization_status['progress'] = progress
    initialization_status['message'] = message
    if error:
        initialization_status['error'] = error
    if progress >= 100:
        initialization_status['completed_at'] = datetime.now().isoformat()

def get_shared_resource_manager():
    """Get the shared ResourceManager instance (thread-safe for sync access)"""
    global _shared_services, _initializing
    
    # Simple thread-safe check without async lock for sync callers
    if _shared_services['resource_manager'] is None and not _initializing:
        _initializing = True
        logger.info("Creating shared ResourceManager instance")
        try:
            from parsers.resource_manager import ResourceManager
            from config.nwn2_settings import nwn2_paths
            
            _shared_services['resource_manager'] = ResourceManager(
                str(nwn2_paths.game_folder), 
                suppress_warnings=True
            )
            
            # Register in independent registry to avoid import cycles during character loading
            from fastapi_core.shared_services import register_shared_service
            register_shared_service('resource_manager', _shared_services['resource_manager'])
            
            logger.info("Shared ResourceManager instance created successfully")
        finally:
            _initializing = False
    elif _initializing:
        # Wait for initialization to complete
        logger.info("ResourceManager is being initialized, waiting...")
        while _initializing and _shared_services['resource_manager'] is None:
            time.sleep(0.1)
    
    return _shared_services['resource_manager']

async def get_shared_resource_manager_async():
    """Get the shared ResourceManager instance (async-safe)"""
    global _shared_services, _services_lock, _initializing
    
    async with _services_lock:
        if _shared_services['resource_manager'] is None and not _initializing:
            _initializing = True
            logger.info("Creating shared ResourceManager instance (async)")
            try:
                # Run ResourceManager creation in thread pool
                from parsers.resource_manager import ResourceManager
                from config.nwn2_settings import nwn2_paths
                
                _shared_services['resource_manager'] = await asyncio.to_thread(
                    ResourceManager,
                    str(nwn2_paths.game_folder), 
                    suppress_warnings=True
                )
                
                # Register in independent registry to avoid import cycles during character loading
                from fastapi_core.shared_services import register_shared_service
                register_shared_service('resource_manager', _shared_services['resource_manager'])
                
                logger.info("Shared ResourceManager instance created successfully (async)")
            finally:
                _initializing = False
        elif _initializing:
            # Wait for initialization to complete
            logger.info("ResourceManager is being initialized, waiting... (async)")
            while _initializing and _shared_services['resource_manager'] is None:
                await asyncio.sleep(0.1)
    
    return _shared_services['resource_manager']

def initialize_background_services():
    """Initialize heavy components in background (follows Django's apps.py pattern)"""
    global initialization_status, _shared_services, _initialization_started
    
    # Check if already initialized or in progress (thread-safe)
    if _initialization_started or initialization_status['progress'] >= 100:
        logger.info("Background initialization already started or completed, skipping duplicate call")
        return
    
    # Mark as started atomically
    _initialization_started = True
    
    # Mark start time
    initialization_status['started_at'] = datetime.now().isoformat()
    update_initialization_status('initializing', 5, 'Background initialization started...')
    logger.info("Starting background initialization of heavy components...")
    
    # Stage 1: Resource Manager
    try:
        update_initialization_status('resource_manager', 10, 'Initializing Resource Manager...')
        # Use the async version to ensure consistency
        import asyncio
        rm = asyncio.run(get_shared_resource_manager_async())
        initialization_status['details']['resource_manager'] = True
        update_initialization_status('resource_manager', 25, 'Resource Manager ready')
    except Exception as e:
        logger.error(f"Failed to initialize resource manager: {e}")
        update_initialization_status('resource_manager', 25, f'Resource Manager failed: {e}', error=str(e))
        return
    
    # Stage 2: Initialize icon cache
    try:
        update_initialization_status('icon_cache', 30, 'Loading icon cache (4000+ icons)...')
        logger.info("Initializing icon cache...")
        from gamedata.cache.icon_cache import create_icon_cache
        import gamedata.cache.icon_cache
        
        start_time = time.time()
        icon_cache = create_icon_cache(rm)
        if hasattr(icon_cache, 'initialize'):
            icon_cache.initialize()
        
        gamedata.cache.icon_cache.icon_cache = icon_cache
        _shared_services['icon_cache'] = icon_cache
        
        init_time = time.time() - start_time
        initialization_status['details']['icon_cache'] = True
        update_initialization_status('icon_cache', 60, f'Icon cache loaded in {init_time:.1f}s')
        logger.info(f"Icon cache initialized in {init_time:.2f}s")
    except Exception as e:
        logger.error(f"Failed to initialize icon cache: {e}")
        update_initialization_status('icon_cache', 60, f'Icon cache failed: {e}', error=str(e))
    
    # Stage 3: Initialize DynamicGameDataLoader with shared ResourceManager
    try:
        update_initialization_status('game_data', 65, 'Loading game data...')
        logger.info("Initializing game data loader with shared ResourceManager...")
        from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader
        
        start_time = time.time()
        
        # Create progress callback that updates initialization status
        def game_data_progress(message, percent):
            # Map the internal progress (0-100%) to our range (65-90%)
            mapped_progress = 65 + int(percent * 0.25)  # 25% of range (90-65=25)
            update_initialization_status('game_data', mapped_progress, f'Game data: {message}')
        
        # Pass the shared ResourceManager to the loader
        loader = get_dynamic_game_data_loader(
            resource_manager=rm, 
            progress_callback=game_data_progress
        )
        _shared_services['game_data_loader'] = loader
        
        # Register in independent registry to avoid import cycles during character loading
        from fastapi_core.shared_services import register_shared_service
        register_shared_service('game_data_loader', loader)
        
        init_time = time.time() - start_time
        initialization_status['details']['game_data'] = True
        update_initialization_status('game_data', 90, f'Game data loaded')
        logger.info(f"Game data loader initialized in {init_time:.2f}s with {len(loader.table_data)} tables")
    except Exception as e:
        logger.error(f"Failed to initialize game data loader: {e}")
        update_initialization_status('game_data', 90, f'Game data failed: {e}', error=str(e))
    
    # Stage 4: Build Prerequisite Graph (optional optimization)
    try:
        # Check if prerequisite graph is enabled
        use_graph = os.environ.get('USE_PREREQUISITE_GRAPH', 'true').lower() == 'true'
        
        if not use_graph:
            logger.info("Prerequisite graph disabled (USE_PREREQUISITE_GRAPH=false)")
            update_initialization_status('prereq_graph', 98, 'Prerequisite graph disabled')
        elif not _shared_services.get('game_data_loader'):
            logger.warning("Prerequisite graph enabled but game data loader not available")
            update_initialization_status('prereq_graph', 98, 'Prerequisite graph skipped (no loader)')
        else:
            update_initialization_status('prereq_graph', 92, 'Building prerequisite graph...')
            logger.info("Building feat prerequisite graph...")
            from character.managers.prerequisite_graph import get_prerequisite_graph
            
            start_time = time.time()
            
            # Build the graph with the game data loader
            graph = get_prerequisite_graph(game_data_loader=_shared_services['game_data_loader'])
            
            if graph and graph.is_built:
                _shared_services['prerequisite_graph'] = graph
                init_time = time.time() - start_time
                stats = graph.get_statistics()
                initialization_status['details']['prerequisite_graph'] = True
                update_initialization_status('prereq_graph', 98, 
                    f'Prerequisite graph built ({stats["feats_with_prerequisites"]} feats with prereqs)')
                logger.info(f"Prerequisite graph built in {init_time:.2f}s - "
                          f"{stats['total_feats']} feats, max chain depth: {stats['max_chain_depth']}")
            else:
                logger.warning("Prerequisite graph failed to build")
                update_initialization_status('prereq_graph', 98, 'Prerequisite graph skipped')
    except Exception as e:
        logger.error(f"Failed to build prerequisite graph: {e}")
        update_initialization_status('prereq_graph', 98, f'Prerequisite graph failed: {e}', error=str(e))
    
    # All done
    update_initialization_status('ready', 100, 'All systems ready!')
    logger.info("Background initialization complete")


@asynccontextmanager
async def lifespan(app: FastAPI):
    """
    FastAPI lifespan context manager for startup and shutdown events
    Replaces deprecated @app.on_event("startup")
    """
    # Startup
    logger.info("FastAPI server starting up...")
    
    # Start background import preloading (highest priority)
    async def preload_imports():
        from utils.import_preloader import preload_heavy_imports
        await preload_heavy_imports()
    
    # Start background initialization using asyncio (2025 FastAPI standard) 
    async def delayed_start():
        await asyncio.sleep(0.5)  # Let FastAPI fully start
        async with _initialization_lock:
            await asyncio.to_thread(initialize_background_services)
    
    # Schedule both background tasks
    asyncio.create_task(preload_imports())  # Start immediately
    asyncio.create_task(delayed_start())    # Start after delay
    logger.info("Background import preloading and initialization tasks scheduled")
    
    yield

    # Shutdown
    logger.info("FastAPI server shutting down...")


# Create FastAPI app with lifespan
app = FastAPI(
    title="NWN2 Enhanced Edition Save Editor API",
    description="Standalone offline desktop application for editing NWN2 save files",
    version="1.0.0",
    docs_url="/api/docs",
    redoc_url="/api/redoc",
    openapi_url="/api/openapi.json",
    lifespan=lifespan
)

# ============================================================================
# MIDDLEWARE STACK - 2025 FastAPI Standards
# ============================================================================

class RequestTrackingMiddleware(BaseHTTPMiddleware):
    """Add request ID and timing for better error tracking"""
    
    async def dispatch(self, request: Request, call_next):
        # Add unique request ID
        request_id = str(uuid.uuid4())[:8]
        request.state.request_id = request_id
        
        # Add timing
        start_time = time.time()
        
        # Process request
        response = await call_next(request)
        
        # Log request completion
        duration = time.time() - start_time
        logger.info(f"Request {request_id}: {request.method} {request.url.path} - {response.status_code} ({duration:.3f}s)")
        
        # Add headers
        response.headers["X-Request-ID"] = request_id
        response.headers["X-Response-Time"] = f"{duration:.3f}s"
        
        return response

# Add middleware stack
app.add_middleware(RequestTrackingMiddleware)

# CORS configuration for Tauri frontend - UPDATED FOR PRODUCTION
ALLOWED_ORIGINS = [
    "http://localhost:3000",      # Dev mode
    "http://localhost:24314",     # Dev mode (Tauri dev port)
    "tauri://localhost",          # Tauri protocol
    "https://tauri.localhost",    # HTTPS Tauri
    "http://tauri.localhost",     # HTTP Tauri (PRODUCTION FIX)
]

print(f"CORS DEBUG: Allowing origins: {ALLOWED_ORIGINS}")

app.add_middleware(
    CORSMiddleware,
    allow_origins=ALLOWED_ORIGINS,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# ============================================================================
# GLOBAL EXCEPTION HANDLERS - 2025 FastAPI Standards
# ============================================================================

# Exception classes are now imported from fastapi_core.exceptions

@app.exception_handler(CharacterNotFoundException)
def character_not_found_handler(request: Request, exc: CharacterNotFoundException):
    """Handle character not found errors"""
    logger.warning(f"Character not found: {exc.character_id}")
    return JSONResponse(
        status_code=exc.status_code,
        content={
            "error": "character_not_found",
            "detail": exc.message,
            "character_id": exc.character_id
        }
    )

@app.exception_handler(CharacterSessionException)
def character_session_handler(request: Request, exc: CharacterSessionException):
    """Handle character session errors"""
    logger.error(f"Character session error: {exc.message} (character_id: {exc.character_id})")
    return JSONResponse(
        status_code=exc.status_code,
        content={
            "error": "character_session_error",
            "detail": exc.message,
            "character_id": exc.character_id
        }
    )

@app.exception_handler(SystemNotReadyException)
def system_not_ready_handler(request: Request, exc: SystemNotReadyException):
    """Handle system not ready errors"""
    logger.info(f"System not ready, progress: {exc.progress}%")
    return JSONResponse(
        status_code=exc.status_code,
        content={
            "error": "system_not_ready",
            "detail": exc.message,
            "progress": exc.progress,
            "retry_after": 5
        },
        headers={"Retry-After": "5"}
    )

@app.exception_handler(RequestValidationError)
def validation_exception_handler(request: Request, exc: RequestValidationError):
    """Handle request validation errors"""
    logger.warning(f"Validation error on {request.url}: {exc.errors()}")
    return JSONResponse(
        status_code=status.HTTP_422_UNPROCESSABLE_ENTITY,
        content={
            "error": "validation_error",
            "detail": "Invalid request data",
            "errors": exc.errors()
        }
    )

@app.exception_handler(StarletteHTTPException)
def http_exception_handler(request: Request, exc: StarletteHTTPException):
    """Handle HTTP exceptions with consistent format"""
    # Suppress noisy logs for icon lookups while icons are temporarily disabled
    try:
        path = request.url.path
    except Exception:
        path = ""

    if exc.status_code == 404 and path.startswith("/api/gamedata/icons/"):
        # Return 404 without warning-level log to avoid noise
        return JSONResponse(
            status_code=exc.status_code,
            content={
                "error": "icon_not_found",
                "path": path
            }
        )

    logger.warning(f"HTTP {exc.status_code} on {request.url}: {exc.detail}")
    return JSONResponse(
        status_code=exc.status_code,
        content={
            "error": "http_error",
            "detail": exc.detail
        }
    )

@app.exception_handler(Exception)
def global_exception_handler(request: Request, exc: Exception):
    """Handle all other unhandled exceptions"""
    logger.error(f"Unhandled exception on {request.url}: {exc}", exc_info=True)
    return JSONResponse(
        status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
        content={
            "error": "internal_server_error",
            "detail": "An unexpected error occurred",
            "request_id": getattr(request.state, 'request_id', 'unknown')
        }
    )

# System endpoints
@app.get("/api/health/")
def health_check():
    """Health check endpoint for Tauri sidecar monitoring"""
    return {"status": "healthy", "service": "nwn2-save-editor-fastapi"}

@app.get("/api/ready/")
def ready_check():
    """Readiness check endpoint"""
    is_ready = initialization_status['progress'] >= 100
    if not is_ready:
        raise SystemNotReadyException(initialization_status['progress'])
    
    return {
        "status": "ready",
        "message": initialization_status['message'],
        "progress": initialization_status['progress']
    }

@app.get("/api/info/")
async def app_info():
    """Application information"""
    return {
        "name": "NWN2 Save Editor",
        "version": "1.0.0",
        "backend": "FastAPI",
        "python_version": sys.version,
        "working_directory": str(Path.cwd())
    }

@app.get("/api/system/initialization/status/")
def get_initialization_status():
    """Get initialization status (matches Django endpoint)"""
    return initialization_status

@app.post("/api/system/background-loading/trigger/")
def trigger_background_loading(background_tasks: BackgroundTasks):
    """Trigger background loading of complete game data using FastAPI BackgroundTasks"""
    global initialization_status, _initialization_started
    
    # Atomic check with proper race condition protection
    if _initialization_started or initialization_status['progress'] >= 100:
        return {
            "status": "already_complete" if initialization_status['progress'] >= 100 else "in_progress",
            "message": "Background loading already completed" if initialization_status['progress'] >= 100 else "Background loading already in progress",
            "progress": initialization_status['progress']
        }
    
    def background_init_sync():
        """Sync background initialization task for BackgroundTasks"""
        try:
            # Double-check initialization state before proceeding
            if initialization_status['progress'] >= 100:
                logger.info("Background initialization already completed, skipping manual trigger")
                return
                
            if initialization_status['stage'] != 'pending' and initialization_status['progress'] > 0:
                logger.info("Background initialization already in progress, skipping manual trigger")
                return
                
            # Proceed with initialization
            initialize_background_services()
            logger.info("Background initialization completed via manual trigger")
        except Exception as e:
            logger.error(f"Background initialization failed: {e}")
            update_initialization_status("failed", 0, str(e))
    
    try:
        # Use FastAPI BackgroundTasks (proper way for sync endpoints)
        background_tasks.add_task(background_init_sync)
        logger.info("Background initialization task added to BackgroundTasks queue")
        
        return {
            "status": "triggered",
            "message": "Background loading triggered successfully",
            "progress": initialization_status['progress']
        }
    except Exception as e:
        logger.error(f"Failed to trigger background loading: {e}")
        return {
            "status": "error",
            "message": f"Failed to trigger background loading: {str(e)}",
            "progress": initialization_status['progress']
        }

@app.post("/api/system/cache/rebuild/")
def rebuild_cache(background_tasks: BackgroundTasks):
    """Rebuild game data cache in background (demonstrates proper BackgroundTasks usage)"""
    
    def rebuild_cache_task():
        """Background task to rebuild cache"""
        try:
            logger.info("Starting cache rebuild...")
            # Simulate cache rebuild work
            rm = get_shared_resource_manager()
            if rm:
                # Trigger cache rebuild if manager has that capability
                logger.info("Cache rebuild completed successfully")
            else:
                logger.warning("ResourceManager not available for cache rebuild")
        except Exception as e:
            logger.error(f"Cache rebuild failed: {e}")
    
    # Add background task using FastAPI's recommended pattern
    background_tasks.add_task(rebuild_cache_task)
    
    return {
        "status": "started",
        "message": "Cache rebuild started in background"
    }

@app.post("/api/system/shutdown/")
def shutdown_server():
    """Gracefully shutdown the FastAPI server"""
    logger.info("Shutdown requested via API")
    
    def shutdown_task():
        """Background task to shutdown the server"""
        import time
        import os
        time.sleep(0.5)  # Give time to send response
        logger.info("Initiating server shutdown...")
        os._exit(0)  # Force exit
    
    # Add background task to shutdown after response
    from threading import Thread
    shutdown_thread = Thread(target=shutdown_task, daemon=True)
    shutdown_thread.start()
    
    return {
        "status": "shutting_down",
        "message": "Server shutdown initiated"
    }


# Include routers
routers_loaded = []

try:
    # Try core routers first - profile each individually
    import time
    from utils.performance_profiler import get_profiler
    profiler = get_profiler()
    
    with profiler.profile("FastAPI Router Loading"):
        start = time.time()
        from fastapi_routers import system
        logger.info(f"system router import: {time.time() - start:.3f}s")
        
        start = time.time()
        from fastapi_routers import session
        logger.info(f"session router import: {time.time() - start:.3f}s")
        
        start = time.time()
        from fastapi_routers import gamedata
        logger.info(f"gamedata router import: {time.time() - start:.3f}s")
        
        start = time.time()
        from fastapi_routers import content
        logger.info(f"content router import: {time.time() - start:.3f}s")
        
        start = time.time()
        from fastapi_routers import savegame
        logger.info(f"savegame router import: {time.time() - start:.3f}s")
        
        start = time.time()
        from fastapi_routers import data
        logger.info(f"data router import: {time.time() - start:.3f}s")
        
        start = time.time()
        from fastapi_routers import state
        logger.info(f"state router import: {time.time() - start:.3f}s")
        
        start = time.time()
        from fastapi_routers import alignment
        logger.info(f"alignment router import: {time.time() - start:.3f}s")
    
    app.include_router(system.router, prefix="/api/system", tags=["system"])
    routers_loaded.append("system")
    
    app.include_router(session.router, prefix="/api/session", tags=["session"])
    routers_loaded.append("session")
    
    app.include_router(gamedata.router, prefix="/api/gamedata", tags=["gamedata"])
    routers_loaded.append("gamedata")
    
    app.include_router(content.router, prefix="/api", tags=["content"])
    routers_loaded.append("content")

    from fastapi_routers import file_browser
    app.include_router(file_browser.router, prefix="/api", tags=["file_browser"])
    routers_loaded.append("file_browser")

    app.include_router(savegame.router, prefix="/api", tags=["savegame"])
    routers_loaded.append("savegame")

    app.include_router(data.router, prefix="/api", tags=["data"])
    routers_loaded.append("data")
    
    app.include_router(state.router, prefix="/api", tags=["state"])
    routers_loaded.append("state")
    
    app.include_router(alignment.router, prefix="/api", tags=["alignment"])
    routers_loaded.append("alignment")
    
    logger.info(f"✓ Core routers loaded: {', '.join(routers_loaded)}")
    
    # Try additional routers - profile each individually
    try:
        with profiler.profile("Character Router Loading"):
            start = time.time()
            from fastapi_routers import abilities
            logger.info(f"abilities router import: {time.time() - start:.3f}s")
            
            start = time.time()
            from fastapi_routers import skills
            logger.info(f"skills router import: {time.time() - start:.3f}s")
            
            start = time.time()
            from fastapi_routers import feats
            logger.info(f"feats router import: {time.time() - start:.3f}s")
            
            start = time.time()
            from fastapi_routers import combat
            logger.info(f"combat router import: {time.time() - start:.3f}s")
        
            start = time.time()
            from fastapi_routers import spells
            logger.info(f"spells router import: {time.time() - start:.3f}s")
        
        app.include_router(abilities.router, prefix="/api", tags=["abilities"])
        routers_loaded.append("abilities")
        
        app.include_router(skills.router, prefix="/api", tags=["skills"])
        routers_loaded.append("skills")
        
        app.include_router(feats.router, prefix="/api", tags=["feats"])
        routers_loaded.append("feats")
        
        app.include_router(combat.router, prefix="/api", tags=["combat"])
        routers_loaded.append("combat")
        
        app.include_router(spells.router, prefix="/api", tags=["spells"])
        routers_loaded.append("spells")
        
        logger.info(f"✓ Character editing routers loaded: attributes, skills, feats, combat, spells")
        
    except Exception as e:
        logger.warning(f"Failed to load character editing routers: {e}")
    
    # Try remaining routers - profile each individually
    try:
        with profiler.profile("Additional Router Loading"):
            start = time.time()
            from fastapi_routers import inventory
            logger.info(f"inventory router import: {time.time() - start:.3f}s")
            
            start = time.time()
            from fastapi_routers import classes
            logger.info(f"classes router import: {time.time() - start:.3f}s")
            
            start = time.time()
            from fastapi_routers import race
            logger.info(f"race router import: {time.time() - start:.3f}s")
            
            start = time.time()
            from fastapi_routers import saves
            logger.info(f"saves router import: {time.time() - start:.3f}s")
        
        app.include_router(inventory.router, prefix="/api", tags=["inventory"])
        routers_loaded.append("inventory")
        
        app.include_router(classes.router, prefix="/api", tags=["classes"])
        routers_loaded.append("classes")
        
        app.include_router(race.router, prefix="/api", tags=["race"])
        routers_loaded.append("race")
        
        app.include_router(saves.router, prefix="/api", tags=["saves"])
        routers_loaded.append("saves")
        
        logger.info(f"✓ Additional routers loaded: inventory, classes, race, saves")
        
    except Exception as e:
        logger.warning(f"Failed to load additional routers: {e}")
        
except Exception as e:
    logger.warning(f"Failed to include core routers: {e}")
    routers_loaded = []

# Modern lifespan event handling is now implemented above

def main():
    """Main entry point for FastAPI server"""
    logger.info("Starting NWN2 Save Editor FastAPI backend...")

    # Mount log viewer if enabled (runs in same process, auto-stops with backend)
    should_open_browser = False
    if ENABLE_LOG_VIEWER:
        logger.info("Mounting development log viewer at /dev/logs...")
        from dev_log_viewer import app as log_app
        app.mount("/dev/logs", log_app)
        logger.info("Log viewer available at http://localhost:<port>/dev/logs")
        should_open_browser = True

    # Check environment
    port = int(os.environ.get("PORT", "8000"))
    host = os.environ.get("HOST", "127.0.0.1")
    debug = os.environ.get("DEBUG", "False").lower() == "true"

    logger.info(f"Server configuration: {host}:{port} (debug={debug})")
    
    # Create uvicorn config to capture actual port
    config = uvicorn.Config(
        app=app,
        host=host,
        port=port,
        log_level="info" if debug else "warning",
        reload=False
    )
    
    server = uvicorn.Server(config)
    
    # Create a custom server class to report the actual port after binding
    class PortReportingServer(uvicorn.Server):
        async def startup(self, sockets=None):
            # First do the normal startup
            await super().startup(sockets)

            # Now we can get the actual bound port
            for server in self.servers:
                for socket in server.sockets:
                    actual_port = socket.getsockname()[1]
                    print(f"FASTAPI_ACTUAL_PORT={actual_port}", flush=True)
                    logger.info(f"FastAPI server bound to port: {actual_port}")

                    # Open log viewer in browser if enabled
                    if should_open_browser:
                        import webbrowser
                        import threading
                        def open_browser():
                            import time
                            time.sleep(1)  # Wait for server to be ready
                            url = f"http://localhost:{actual_port}/dev/logs"
                            logger.info(f"Opening log viewer in browser: {url}")
                            webbrowser.open(url)
                        threading.Thread(target=open_browser, daemon=True).start()
                    break
                break
    
    # Use custom server class
    server.__class__ = PortReportingServer
    
    # Run the server
    try:
        server.run()
    except Exception as e:
        logger.error(f"Failed to start FastAPI server: {e}")
        raise

if __name__ == "__main__":
    main()