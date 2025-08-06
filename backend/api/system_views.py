"""
System management API views
"""
from rest_framework.decorators import api_view, permission_classes
from rest_framework.permissions import AllowAny
from django.views.decorators.csrf import csrf_exempt
from django.utils.decorators import method_decorator
from rest_framework.response import Response
from rest_framework import status
from django.conf import settings
from django.core.management import call_command
from pathlib import Path
import json
import subprocess
import sys
from datetime import datetime
import threading
from config.nwn2_settings import nwn2_paths, NWN2PathFinder


@api_view(['GET'])
def health_check(request):
    """Lightweight health check endpoint for sidecar manager"""
    return Response({
        'status': 'ok',
        'timestamp': datetime.now().isoformat()
    }, status=status.HTTP_200_OK)


@api_view(['GET'])
def ready_check(request):
    """Check if all systems are fully initialized and ready"""
    # Check if DynamicGameDataLoader singleton is ready
    loader_ready = False
    loader_tables = 0
    icon_cache_ready = False
    icon_cache_count = 0
    
    try:
        from gamedata.dynamic_loader import singleton
        # Access the private variable to check if initialized
        if hasattr(singleton, '_loader_instance') and singleton._loader_instance is not None:
            loader_ready = True
            loader_tables = len(singleton._loader_instance.table_data)
    except Exception as e:
        loader_ready = False
    
    # Check icon cache status
    try:
        from gamedata.cache import icon_cache as ic
        if hasattr(ic, 'icon_cache') and ic.icon_cache:
            icon_cache_ready = ic.icon_cache._initialized
            if icon_cache_ready:
                stats = ic.icon_cache.get_statistics()
                icon_cache_count = stats.get('total_count', 0)
    except Exception as e:
        icon_cache_ready = False
    
    all_ready = loader_ready and icon_cache_ready
    
    return Response({
        'ready': all_ready,
        'loader_ready': loader_ready,
        'loader_tables': loader_tables,
        'icon_cache_ready': icon_cache_ready,
        'icon_cache_count': icon_cache_count,
        'timestamp': datetime.now().isoformat()
    }, status=status.HTTP_200_OK)


# Store rebuild status
rebuild_status = {
    'is_rebuilding': False,
    'progress': 0,
    'message': '',
    'started_at': None,
    'completed_at': None,
    'error': None
}

# Store initialization status
initialization_status = {
    'stage': 'starting',  # starting, resource_manager, icon_cache, game_data, ready
    'progress': 0,
    'message': 'Django starting up...',
    'started_at': None,
    'completed_at': None,
    'error': None,
    'details': {
        'resource_manager': False,
        'icon_cache': False,
        'game_data': False
    }
}


@api_view(['GET'])
def cache_status(request):
    """Get current cache status and metadata"""
    cache_dir = Path(settings.BASE_DIR) / 'cache'
    metadata_path = cache_dir / 'cache_metadata.json'
    
    response_data = {
        'cache_exists': cache_dir.exists(),
        'file_count': 0,
        'total_size_mb': 0,
        'metadata': None,
        'rebuild_status': rebuild_status
    }
    
    if cache_dir.exists():
        # Count msgpack files
        msgpack_files = list(cache_dir.glob('*.msgpack'))
        response_data['file_count'] = len(msgpack_files)
        
        # Calculate total size
        total_size = sum(f.stat().st_size for f in msgpack_files)
        response_data['total_size_mb'] = round(total_size / (1024 * 1024), 2)
        
        # Load metadata if exists
        if metadata_path.exists():
            with open(metadata_path, 'r') as f:
                response_data['metadata'] = json.load(f)
    
    return Response(response_data)


def rebuild_cache_async(docs_path=None, workshop_path=None):
    """Run cache rebuild in background"""
    global rebuild_status
    
    try:
        rebuild_status['is_rebuilding'] = True
        rebuild_status['progress'] = 0
        rebuild_status['message'] = 'Starting cache rebuild...'
        rebuild_status['started_at'] = datetime.now().isoformat()
        rebuild_status['error'] = None
        
        # Import here to avoid circular imports
        from scripts.cache_with_mods import ModAwareCacher
        
        # Get paths
        nwn2_path = str(nwn2_paths.game_folder)
        
        # Auto-detect docs path if not provided
        if not docs_path:
            common_docs = [
                Path.home() / "Documents" / "Neverwinter Nights 2",
                Path.home() / "My Documents" / "Neverwinter Nights 2",
            ]
            for path in common_docs:
                if path.exists():
                    docs_path = str(path)
                    rebuild_status['message'] = f'Auto-detected documents path: {docs_path}'
                    break
        
        rebuild_status['progress'] = 10
        rebuild_status['message'] = 'Initializing mod-aware cacher...'
        
        cacher = ModAwareCacher(
            nwn2_path=nwn2_path,
            user_docs_path=docs_path,
            workshop_path=workshop_path
        )
        
        rebuild_status['progress'] = 20
        rebuild_status['message'] = 'Scanning for mods...'
        
        output_dir = Path(settings.BASE_DIR) / 'cache'
        
        # We need to modify cache_all_files to update progress
        # For now, just run it
        cacher.cache_all_files(output_dir)
        
        rebuild_status['progress'] = 100
        rebuild_status['message'] = 'Cache rebuild complete!'
        rebuild_status['completed_at'] = datetime.now().isoformat()
        
    except Exception as e:
        rebuild_status['error'] = str(e)
        rebuild_status['message'] = f'Error: {str(e)}'
    finally:
        rebuild_status['is_rebuilding'] = False


@api_view(['POST'])
def rebuild_cache(request):
    """Trigger cache rebuild with mod support"""
    global rebuild_status
    
    if rebuild_status['is_rebuilding']:
        return Response({
            'error': 'Cache rebuild already in progress',
            'status': rebuild_status
        }, status=status.HTTP_409_CONFLICT)
    
    # Get paths from request
    docs_path = request.data.get('docs_path')
    workshop_path = request.data.get('workshop_path')
    
    # Start rebuild in background thread
    thread = threading.Thread(
        target=rebuild_cache_async,
        args=(docs_path, workshop_path)
    )
    thread.daemon = True
    thread.start()
    
    return Response({
        'message': 'Cache rebuild started',
        'status': rebuild_status
    })


@api_view(['GET'])
def get_config(request):
    """Get current configuration"""
    config = {
        'nwn2_game_folder': str(nwn2_paths.game_folder),
        'nwn2_user_folder': str(nwn2_paths.user_folder),
        'nwn2_data_path': str(nwn2_paths.data),  # Legacy name
        'nwn2_docs_path': str(nwn2_paths.user_folder),
        'nwn2_workshop_path': getattr(settings, 'NWN2_WORKSHOP_PATH', None),
        'auto_detected_paths': {}
    }
    
    # Try to auto-detect paths
    common_docs = [
        Path.home() / "Documents" / "Neverwinter Nights 2",
        Path.home() / "My Documents" / "Neverwinter Nights 2",
    ]
    
    for path in common_docs:
        if path.exists():
            config['auto_detected_paths']['documents'] = str(path)
            break
    
    # Auto-detect Steam workshop
    steam_paths = [
        Path.home() / ".steam/steam/steamapps/workshop/content/209000",
        Path("C:/Program Files (x86)/Steam/steamapps/workshop/content/209000"),
        Path("C:/Program Files/Steam/steamapps/workshop/content/209000"),
    ]
    
    for path in steam_paths:
        if path.exists():
            config['auto_detected_paths']['workshop'] = str(path)
            break
    
    return Response(config)


@api_view(['POST'])
def update_config(request):
    """Update configuration (saved to user preferences)"""
    # In a real app, you'd save this to database or user preferences
    # For now, just validate the paths
    
    docs_path = request.data.get('docs_path')
    workshop_path = request.data.get('workshop_path')
    
    errors = []
    
    if docs_path and not Path(docs_path).exists():
        errors.append(f"Documents path does not exist: {docs_path}")
    
    if workshop_path and not Path(workshop_path).exists():
        errors.append(f"Workshop path does not exist: {workshop_path}")
    
    if errors:
        return Response({
            'errors': errors
        }, status=status.HTTP_400_BAD_REQUEST)
    
    # Save to user session or database
    # For now, just return success
    return Response({
        'message': 'Configuration updated',
        'docs_path': docs_path,
        'workshop_path': workshop_path
    })


@api_view(['GET', 'POST'])
def nwn2_data_path_config(request):
    """Get or update the NWN2 game folder configuration"""
    if request.method == 'GET':
        # Get current configuration
        return Response({
            'nwn2_game_folder': str(nwn2_paths.game_folder),
            'nwn2_data_path': str(nwn2_paths.game_folder),  # Legacy compatibility
            'exists': nwn2_paths.game_folder.exists(),
            'has_data_folder': nwn2_paths.data.exists(),
            'has_dialog_tlk': nwn2_paths.dialog_tlk.exists(),
            'paths': {
                'game_folder': str(nwn2_paths.game_folder),
                'data': str(nwn2_paths.data),
                'dialog_tlk': str(nwn2_paths.dialog_tlk),
                'campaigns': str(nwn2_paths.campaigns),
                'modules': str(nwn2_paths.modules),
                'user_folder': str(nwn2_paths.user_folder),
                'saves': str(nwn2_paths.saves),
                'localvault': str(nwn2_paths.localvault),
            }
        })
    
    elif request.method == 'POST':
        # Update the NWN2 game folder
        new_path = request.data.get('nwn2_game_folder') or request.data.get('nwn2_data_path')  # Support both names
        if not new_path:
            return Response({
                'error': 'nwn2_game_folder is required'
            }, status=status.HTTP_400_BAD_REQUEST)
        
        # Validate the path
        path_obj = Path(new_path)
        if not path_obj.exists():
            return Response({
                'error': f'Path does not exist: {new_path}'
            }, status=status.HTTP_400_BAD_REQUEST)
        
        # Check if it looks like a valid NWN2 installation
        if not NWN2PathFinder._is_valid_nwn2_path(path_obj):
            return Response({
                'error': 'Invalid NWN2 installation: missing data folder or dialog.tlk',
                'hint': 'Path should point to the NWN2 installation directory'
            }, status=status.HTTP_400_BAD_REQUEST)
        
        # Update the path
        success = nwn2_paths.set_game_folder(new_path)
        if success:
            return Response({
                'message': 'NWN2 game folder updated successfully',
                'nwn2_game_folder': str(nwn2_paths.game_folder),
                'saved_to': str(Path.home() / '.nwn2_editor' / 'settings.json')
            })
        else:
            return Response({
                'error': 'Failed to save settings'
            }, status=status.HTTP_500_INTERNAL_SERVER_ERROR)


@api_view(['GET'])
def auto_discover_nwn2(request):
    """Auto-discover NWN2 installations on the system"""
    # Get optional search paths from query params
    search_paths = request.GET.getlist('search_paths')
    if search_paths:
        search_paths = [Path(p) for p in search_paths]
    else:
        search_paths = None
    
    # Run auto-discovery
    found_paths = NWN2PathFinder.auto_discover_nwn2_paths(search_paths)
    
    # Validate each found path and gather details
    installations = []
    for path in found_paths:
        installation = {
            'path': str(path),
            'name': path.name,
            'valid': True,
            'has_data_folder': (path / 'data').exists() or (path / 'Data').exists(),
            'has_dialog_tlk': (path / 'dialog.tlk').exists() or (path / 'Dialog.tlk').exists(),
            'type': 'unknown'
        }
        
        # Try to determine installation type
        if 'Steam' in str(path) or 'steamapps' in str(path):
            installation['type'] = 'steam'
        elif 'GOG' in str(path):
            installation['type'] = 'gog'
        elif 'Enhanced Edition' in str(path):
            installation['type'] = 'enhanced_edition'
        
        installations.append(installation)
    
    # Sort by type and name
    installations.sort(key=lambda x: (x['type'], x['name']))
    
    return Response({
        'found_installations': installations,
        'count': len(installations),
        'current_path': str(nwn2_paths.game_folder),
    })


# Background loading status
background_loading_status = {
    'is_loading': False,
    'started_at': None,
    'completed_at': None,
    'tables_loaded': 0,
    'total_tables': 0,
    'error': None
}


def _background_load_complete_data():
    """Background thread function to load remaining game data."""
    from gamedata.middleware import ModuleContextMiddleware, get_shared_resource_manager
    from gamedata.services.game_rules_service import GameRulesService
    from config.nwn2_settings import nwn2_paths
    import time
    
    try:
        background_loading_status['is_loading'] = True
        background_loading_status['started_at'] = datetime.now().isoformat()
        background_loading_status['error'] = None
        
        # Use shared ResourceManager instance to avoid duplicate initialization
        rm = get_shared_resource_manager()
        full_service = GameRulesService(rm)  # Full loading mode
        
        # Update the middleware's cached service with the full one
        # Access the middleware instance (this is a bit hacky but works)
        from django.conf import settings
        from django.utils.module_loading import import_string
        
        # Find the middleware instance
        for middleware_path in settings.MIDDLEWARE:
            if 'ModuleContextMiddleware' in middleware_path:
                middleware_class = import_string(middleware_path)
                # Replace the cached service (assumes singleton behavior)
                break
        
        background_loading_status['completed_at'] = datetime.now().isoformat()
        background_loading_status['is_loading'] = False
        
    except Exception as e:
        background_loading_status['error'] = str(e)
        background_loading_status['is_loading'] = False


@csrf_exempt
def trigger_background_loading(request):
    """Trigger background loading of complete game data."""
    from django.http import JsonResponse
    import logging
    logger = logging.getLogger(__name__)
    logger.info(f"Background loading endpoint hit - User: {getattr(request, 'user', 'None')}, Method: {request.method}")
    
    if request.method != 'POST':
        return JsonResponse({'error': 'Method not allowed'}, status=405)
    
    if background_loading_status['is_loading']:
        return JsonResponse({
            'message': 'Background loading already in progress',
            'status': background_loading_status
        })
    
    # Start background loading in a thread
    thread = threading.Thread(target=_background_load_complete_data, daemon=True)
    thread.start()
    
    return JsonResponse({
        'message': 'Background loading started',
        'status': background_loading_status
    })


@api_view(['GET'])
def background_loading_status_endpoint(request):
    """Get the status of background loading."""
    return Response({
        'status': background_loading_status
    })


@api_view(['GET'])
def initialization_status_endpoint(request):
    """Get the status of Django initialization."""
    return Response(initialization_status)


def update_initialization_status(stage, progress, message, error=None):
    """Update the initialization status."""
    global initialization_status
    initialization_status['stage'] = stage
    initialization_status['progress'] = progress
    initialization_status['message'] = message
    if error:
        initialization_status['error'] = error
    if stage == 'ready':
        initialization_status['completed_at'] = datetime.now().isoformat()