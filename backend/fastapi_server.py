"""FastAPI backend server running as the Tauri sidecar for character editing and game data management."""

import logging
import os
import sys
import asyncio
import time
from pathlib import Path
from typing import Optional
from datetime import datetime
import tempfile

import psutil
import threading

try:
    with open(os.path.join(tempfile.gettempdir(), "nwn2_fastapi_startup.log"), "a") as f:
        f.write(f"[{datetime.now()}] FastAPI process started. Args: {sys.argv}\n")
except:
    pass

def start_parent_watchdog():
    """
    Monitor the parent process (Tauri) and self-terminate if it dies.
    This prevents orphan background processes.
    """
    try:
        parent_pid = os.getppid()
        if parent_pid <= 1: # No parent or init
            return
            
        def watchdog():
            try:
                parent = psutil.Process(parent_pid)
                while True:
                    if not parent.is_running():
                        break
                    time.sleep(2)
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                pass
            
            # Parent is gone, emergency exit
            os._exit(0)
            
        thread = threading.Thread(target=watchdog, daemon=True)
        thread.start()
    except Exception:
        pass

def panic_log(exc_type, exc_value, exc_traceback):
    try:
        import traceback
        import pprint
        with open(os.path.join(tempfile.gettempdir(), "nwn2_fastapi_panic.log"), "a") as f:
            f.write(f"\n[{datetime.now()}] CRITICAL UNCAUGHT EXCEPTION:\n")
            f.write(f"CWD: {os.getcwd()}\n")
            f.write(f"sys.path: {sys.path}\n")
            f.write("Environment:\n")
            pprint.pprint(dict(os.environ), stream=f)
            f.write("\nTraceback:\n")
            traceback.print_exception(exc_type, exc_value, exc_traceback, file=f)
            f.write("\n" + "="*80 + "\n")
    except:
        pass

sys.excepthook = panic_log
print(f"[{datetime.now()}] Panic logger initialized", flush=True)

from contextlib import asynccontextmanager
from fastapi import FastAPI, Request, status, BackgroundTasks
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from fastapi.exceptions import RequestValidationError
from starlette.exceptions import HTTPException as StarletteHTTPException
from starlette.middleware.base import BaseHTTPMiddleware
import uuid
import uvicorn

backend_dir = Path(__file__).parent
sys.path.insert(0, str(backend_dir))

from services.fastapi.exceptions import SystemNotReadyException

from config.logging_config import logger, ENABLE_LOG_VIEWER

logging.basicConfig(
    level=logging.WARNING,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)

_shared_services = {
    'resource_manager': None,
    'game_rules_service': None,
    'icon_cache': None,
    'game_data_loader': None,
    'prerequisite_graph': None
}
_services_lock = asyncio.Lock()
_initializing = False

_initialization_lock = asyncio.Lock()
_initialization_started = False

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
    
    if _shared_services['resource_manager'] is None and not _initializing:
        _initializing = True
        logger.info("Creating shared ResourceManager instance")
        try:
            from services.core.resource_manager import ResourceManager
            from config.nwn2_settings import nwn2_paths
            
            _shared_services['resource_manager'] = ResourceManager(
                str(nwn2_paths.game_folder), 
                suppress_warnings=True
            )
            
            from services.fastapi.shared_services import register_shared_service
            register_shared_service('resource_manager', _shared_services['resource_manager'])
            
            logger.info("Shared ResourceManager instance created successfully")
        finally:
            _initializing = False
    elif _initializing:
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
                from services.core.resource_manager import ResourceManager
                from config.nwn2_settings import nwn2_paths
                
                _shared_services['resource_manager'] = await asyncio.to_thread(
                    ResourceManager,
                    str(nwn2_paths.game_folder), 
                    suppress_warnings=True
                )
                
                from services.fastapi.shared_services import register_shared_service
                register_shared_service('resource_manager', _shared_services['resource_manager'])
                
                logger.info("Shared ResourceManager instance created successfully (async)")
            finally:
                _initializing = False
        elif _initializing:
            logger.info("ResourceManager is being initialized, waiting... (async)")
            while _initializing and _shared_services['resource_manager'] is None:
                await asyncio.sleep(0.1)
    
    return _shared_services['resource_manager']

def initialize_background_services():
    """Initialize heavy components in background."""
    global initialization_status, _shared_services, _initialization_started
    
    if _initialization_started or initialization_status['progress'] >= 100:
        logger.info("Background initialization already started or completed, skipping duplicate call")
        return
    
    _initialization_started = True
    
    initialization_status['started_at'] = datetime.now().isoformat()
    update_initialization_status('initializing', 5, 'Background initialization started...')
    logger.info("Starting background initialization of heavy components...")
    
    try:
        update_initialization_status('resource_manager', 10, 'Initializing Resource Manager...')
        rm = asyncio.run(get_shared_resource_manager_async())
        initialization_status['details']['resource_manager'] = True
        update_initialization_status('resource_manager', 25, 'Resource Manager ready')
    except Exception as e:
        logger.error(f"Failed to initialize resource manager: {e}")
        update_initialization_status('resource_manager', 25, f'Resource Manager failed: {e}', error=str(e))
        return
    
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
        # Icons are disabled for 0.1.0 release, so this is expected - don't log as error
        logger.debug(f"Icon cache not available (expected for v0.1.0): {e}")
        update_initialization_status('icon_cache', 60, 'Icon cache skipped (v0.1.0)')
    
    try:
        update_initialization_status('game_data', 65, 'Loading game data...')
        logger.info("Initializing game data loader with shared ResourceManager...")
        from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader
        
        start_time = time.time()
        
        def game_data_progress(message, percent):
            # Map the internal progress (0-100%) to our range (65-90%)
            mapped_progress = 65 + int(percent * 0.25)  # 25% of range (90-65=25)
            update_initialization_status('game_data', mapped_progress, f'Game data: {message}')
        
        loader = get_dynamic_game_data_loader(
            resource_manager=rm, 
            progress_callback=game_data_progress
        )
        _shared_services['game_data_loader'] = loader
        
        from services.fastapi.shared_services import register_shared_service
        register_shared_service('game_data_loader', loader)
        
        init_time = time.time() - start_time
        initialization_status['details']['game_data'] = True
        update_initialization_status('game_data', 90, f'Game data loaded')
        logger.info(f"Game data loader initialized in {init_time:.2f}s with {len(loader.table_data)} tables")
    except Exception as e:
        logger.error(f"Failed to initialize game data loader: {e}")
        update_initialization_status('game_data', 90, f'Game data failed: {e}', error=str(e))
    
    try:
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
            from services.gamedata.prerequisite_graph import get_prerequisite_graph
            
            start_time = time.time()
            
            graph = get_prerequisite_graph(rules_service=_shared_services['game_data_loader'])
            
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
    
    update_initialization_status('ready', 100, 'All systems ready!')
    logger.info("Background initialization complete")


@asynccontextmanager
async def lifespan(app: FastAPI):
    """
    FastAPI lifespan context manager for startup and shutdown events
    Replaces deprecated @app.on_event("startup")
    """
    logger.info("FastAPI server starting up...")
    
    async def preload_imports():
        from utils.import_preloader import preload_heavy_imports
        await preload_heavy_imports()
    
    async def delayed_start():
        await asyncio.sleep(0.5)  # Let FastAPI fully start
        async with _initialization_lock:
            await asyncio.to_thread(initialize_background_services)
    
    asyncio.create_task(preload_imports())
    asyncio.create_task(delayed_start())
    logger.info("Background import preloading and initialization tasks scheduled")
    
    yield

    logger.info("FastAPI server shutting down...")


app = FastAPI(
    title="NWN2 Enhanced Edition Save Editor API",
    description="Standalone offline desktop application for editing NWN2 save files",
    version="0.1.0",
    docs_url=None,
    redoc_url=None,
    openapi_url=None,
    lifespan=lifespan
)



class RequestTrackingMiddleware(BaseHTTPMiddleware):
    """Add request ID and timing for better error tracking"""
    
    async def dispatch(self, request: Request, call_next):
        request_id = str(uuid.uuid4())[:8]
        request.state.request_id = request_id
        
        start_time = time.time()
        
        response = await call_next(request)
        
        duration = time.time() - start_time
        if not request.url.path.startswith("/dev/logs/"):
            logger.info(f"Request {request_id}: {request.method} {request.url.path} - {response.status_code} ({duration:.3f}s)")
        
        response.headers["X-Request-ID"] = request_id
        response.headers["X-Response-Time"] = f"{duration:.3f}s"
        
        return response

app.add_middleware(RequestTrackingMiddleware)

ALLOWED_ORIGINS = [
    "http://localhost:3000",      # Dev mode
    "http://localhost:24314",     # Dev mode (Tauri dev port)
    "tauri://localhost",          # Tauri protocol
    "https://tauri.localhost",    # HTTPS Tauri
    "http://tauri.localhost",     # HTTP Tauri (PRODUCTION FIX)
]

app.add_middleware(
    CORSMiddleware,
    allow_origins=ALLOWED_ORIGINS,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
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
    try:
        path = request.url.path
    except Exception:
        path = ""

    if exc.status_code == 404 and path.startswith("/api/gamedata/icons/"):
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
    """Get initialization status."""
    return initialization_status

@app.post("/api/system/background-loading/trigger/")
def trigger_background_loading(background_tasks: BackgroundTasks):
    """Trigger background loading of complete game data using FastAPI BackgroundTasks"""
    global initialization_status, _initialization_started
    
    if _initialization_started or initialization_status['progress'] >= 100:
        return {
            "status": "already_complete" if initialization_status['progress'] >= 100 else "in_progress",
            "message": "Background loading already completed" if initialization_status['progress'] >= 100 else "Background loading already in progress",
            "progress": initialization_status['progress']
        }
    
    def background_init_sync():
        """Sync background initialization task for BackgroundTasks"""
        try:
            if initialization_status['progress'] >= 100:
                logger.info("Background initialization already completed, skipping manual trigger")
                return
                
            if initialization_status['stage'] != 'pending' and initialization_status['progress'] > 0:
                logger.info("Background initialization already in progress, skipping manual trigger")
                return
                
            initialize_background_services()
            logger.info("Background initialization completed via manual trigger")
        except Exception as e:
            logger.error(f"Background initialization failed: {e}")
            update_initialization_status("failed", 0, str(e))
    
    try:
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
    
    from threading import Thread
    shutdown_thread = Thread(target=shutdown_task, daemon=True)
    shutdown_thread.start()
    
    return {
        "status": "shutting_down",
        "message": "Server shutdown initiated"
    }


routers_loaded = []

try:
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
        from fastapi_routers import file_browser
        logger.info(f"file_browser router import: {time.time() - start:.3f}s")
        
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
    app.include_router(gamedata.router, prefix="/api", tags=["gamedata"])
    routers_loaded.append("gamedata")
    
    app.include_router(content.router, prefix="/api", tags=["content"])
    routers_loaded.append("content")



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
    
    logger.info(f"Core routers loaded: {', '.join(routers_loaded)}")
    
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
        
        logger.info("Character editing routers loaded: attributes, skills, feats, combat, spells")
        
    except Exception as e:
        logger.warning(f"Failed to load character editing routers: {e}")
    
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

        logger.info("Additional routers loaded: inventory, classes, race, saves")
        
    except Exception as e:
        logger.warning(f"Failed to load additional routers: {e}")

except Exception as e:
    logger.warning(f"Failed to include core routers: {e}")
    routers_loaded = []


def main():
    """Main entry point for FastAPI server"""
    start_parent_watchdog()
    
    logger.info("Starting NWN2 Save Editor FastAPI backend...")

    should_open_browser = False
    if ENABLE_LOG_VIEWER:
        logger.info("Mounting development log viewer at /dev/logs...")
        from dev_log_viewer import app as log_app
        app.mount("/dev/logs", log_app)
        logger.info("Log viewer available at http://localhost:<port>/dev/logs")
        should_open_browser = True

    port = int(os.environ.get("PORT", "8000"))
    host = os.environ.get("HOST", "127.0.0.1")
    debug = os.environ.get("DEBUG", "False").lower() == "true"

    if "--port" in sys.argv:
        try:
            idx = sys.argv.index("--port")
            port = int(sys.argv[idx + 1])
        except (ValueError, IndexError):
            pass
    if "--host" in sys.argv:
        try:
            idx = sys.argv.index("--host")
            host = sys.argv[idx + 1]
        except IndexError:
            pass

    logger.info(f"Raw CLI args: {sys.argv}")
    logger.info(f"Raw ENV PORT: {os.environ.get('PORT')}")
    logger.info(f"Server configuration: {host}:{port} (debug={debug})")
    
    config = uvicorn.Config(
        app=app,
        host=host,
        port=port,
        log_level="info" if debug else "warning",
        reload=False
    )
    
    server = uvicorn.Server(config)
    
    class PortReportingServer(uvicorn.Server):
        async def startup(self, sockets=None):
            await super().startup(sockets)

            for server in self.servers:
                for socket in server.sockets:
                    actual_port = socket.getsockname()[1]
                    print(f"FASTAPI_ACTUAL_PORT={actual_port}", flush=True)
                    logger.info(f"FASTAPI_ACTUAL_PORT={actual_port}")
                    logger.info(f"FastAPI server bound to port: {actual_port}")

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
    
    server.__class__ = PortReportingServer
    
    try:
        server.run()
    except Exception as e:
        logger.error(f"Failed to start FastAPI server: {e}")
        raise

if __name__ == "__main__":
    main()