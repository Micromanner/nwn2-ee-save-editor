"""
System endpoints router - health checks, cache management, configuration
"""

from typing import Dict, Any
from fastapi import APIRouter, HTTPException, status
from loguru import logger

router = APIRouter()


@router.get("/health")
def health_check():
    """Health check endpoint"""
    from fastapi_models import HealthResponse
    return HealthResponse(status="healthy", service="nwn2-save-editor")


@router.get("/system/cache/status")
def cache_status():
    """Get current cache status and metadata"""
    try:
        # Try to get ResourceManager, but fallback to filesystem stats if it fails
        from fastapi_models import CacheStatus
        from datetime import datetime
        from pathlib import Path
        import os
        
        cache_status_data = {
            'status': 'ready',
            'cache_size': 0,
            'last_updated': datetime.now(),
            'cache_type': 'filesystem',
            'memory_usage': 0,
            'hit_rate': 0.0
        }
        
        try:
            # First try to get ResourceManager stats
            from fastapi_server import get_shared_resource_manager
            resource_manager = get_shared_resource_manager()
            
            if resource_manager is not None:
                # Get cache stats from ResourceManager
                cache_stats = resource_manager.get_cache_stats()
                
                # Map ResourceManager stats to CacheStatus model
                status_value = "ready" if cache_stats.get('enabled', False) else "loading"
                
                # Parse hit rate (remove % symbol and convert to float)
                hit_rate_str = cache_stats.get('hit_rate', '0.0%')
                hit_rate = float(hit_rate_str.rstrip('%')) if hit_rate_str and hit_rate_str != '0.0%' else 0.0
                
                # Convert MB to bytes for memory_usage
                current_size_mb = cache_stats.get('current_size_mb', 0)
                memory_usage_bytes = int(current_size_mb * 1024 * 1024) if current_size_mb else 0
                
                cache_status_data = {
                    'status': status_value,
                    'cache_size': cache_stats.get('cached_items', 0),
                    'last_updated': datetime.now(),
                    'cache_type': 'resource_manager',
                    'memory_usage': memory_usage_bytes,
                    'hit_rate': hit_rate
                }
            else:
                raise Exception("ResourceManager not available")
                
        except Exception as rm_error:
            logger.warning(f"ResourceManager unavailable, falling back to filesystem stats: {rm_error}")
            
            # Fallback to filesystem-based cache statistics
            # Use absolute path to cache directory
            cache_dir = Path(__file__).parent.parent / "cache"
            
            if cache_dir.exists():
                # Count cache files and calculate total size
                cache_files = list(cache_dir.rglob("*"))
                cache_files = [f for f in cache_files if f.is_file()]
                
                total_size = sum(f.stat().st_size for f in cache_files)
                
                # Get most recent modification time
                last_modified = None
                if cache_files:
                    last_modified = datetime.fromtimestamp(
                        max(f.stat().st_mtime for f in cache_files)
                    )
                
                cache_status_data = {
                    'status': 'ready',
                    'cache_size': len(cache_files),
                    'last_updated': last_modified,
                    'cache_type': 'filesystem',
                    'memory_usage': total_size,
                    'hit_rate': None  # Not available from filesystem
                }
            else:
                cache_status_data['status'] = 'error'
        
        return CacheStatus(**cache_status_data)
        
    except Exception as e:
        logger.error(f"Failed to get cache status: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get cache status: {str(e)}"
        )




# Temporarily disabled for testing
# @router.post("/system/cache/rebuild/", response_model=CacheRebuildResponse)
# def rebuild_cache(...):


@router.get("/system/config")
def get_config():
    """Get current configuration"""
    try:
        # Use nwn2_settings directly - no service layer needed
        from config.nwn2_settings import nwn2_paths
        from fastapi_models import ConfigResponse
        
        # Get all paths info from nwn2_settings
        paths_info = nwn2_paths.get_all_paths_info()
        
        # Format for ConfigResponse
        config_data = {
            'nwn2_install_dir': str(nwn2_paths.game_folder),
            'cache_enabled': True,
            'debug_mode': False,
            'data_paths': {
                'game_folder': str(nwn2_paths.game_folder),
                'data': str(nwn2_paths.data),
                'dialog_tlk': str(nwn2_paths.dialog_tlk),
                'campaigns': str(nwn2_paths.campaigns),
                'modules': str(nwn2_paths.modules),
                'saves': str(nwn2_paths.saves),
                'localvault': str(nwn2_paths.localvault),
            },
            'feature_flags': {
                'enhanced_edition': nwn2_paths.is_enhanced_edition,
                'steam_installation': nwn2_paths.is_steam_installation,
                'gog_installation': nwn2_paths.is_gog_installation,
            }
        }
        
        return ConfigResponse(**config_data)
        
    except Exception as e:
        logger.error(f"Get config failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get configuration: {str(e)}"
        )


# @router.post("/system/config/update/", response_model=ConfigUpdateResponse)
def update_config_disabled():
    """Update configuration"""
    try:
        # Use ResourceManager for configuration - no duplicated logic
        from fastapi_server import get_shared_resource_manager
        rm = get_shared_resource_manager()
        
        if rm and hasattr(config_request, 'config_updates'):
            # Use ResourceManager methods for configuration
            update_result = {"status": "success", "message": "Configuration updated"}
        else:
            update_result = {"status": "error", "message": "ResourceManager not available or invalid request"}
        
        return ConfigUpdateResponse(**update_result)
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Config update failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update configuration: {str(e)}"
        )


@router.get("/system/nwn2-data-path")
def nwn2_data_path_config():
    """Get NWN2 data path configuration"""
    try:
        # Use nwn2_settings directly
        from config.nwn2_settings import nwn2_paths
        from fastapi_models import NWN2PathResponse
        
        path_config = {
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
        }
        
        return NWN2PathResponse(**path_config)
        
    except Exception as e:
        logger.error(f"NWN2 data path config failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get NWN2 data path configuration: {str(e)}"
        )


@router.post("/system/auto-discover-nwn2")
def auto_discover_nwn2():
    """Auto-discover NWN2 installation paths"""
    try:
        # Use nwn2_settings directly
        from config.nwn2_settings import nwn2_paths
        from fastapi_models import AutoDiscoverResponse
        
        # Get all installations with categorization
        discovery_data = nwn2_paths.discover_all_nwn2_installations()
        
        # Format installations for response
        installations = []
        for path in discovery_data['all_installations']:
            installation = {
                'path': str(path),
                'name': path.name,
                'valid': True,
                'has_data_folder': (path / 'data').exists() or (path / 'Data').exists(),
                'has_dialog_tlk': (path / 'dialog.tlk').exists() or (path / 'Dialog.tlk').exists(),
                'type': 'unknown'
            }
            
            # Determine installation type
            if path in discovery_data['steam_installations']:
                installation['type'] = 'steam'
            elif path in discovery_data['gog_installations']:
                installation['type'] = 'gog'
            elif 'Enhanced Edition' in str(path):
                installation['type'] = 'enhanced_edition'
            
            installations.append(installation)
        
        # Create NWN2PathResponse for current paths
        paths_found = {
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
        }
        
        discovery_result = {
            'success': True,
            'message': f'Found {len(installations)} NWN2 installations using Rust-enhanced discovery',
            'paths_found': paths_found,
            'discovery_method': 'rust_enhanced'
        }
        
        return AutoDiscoverResponse(**discovery_result)
            
    except Exception as e:
        logger.error(f"Auto-discover NWN2 failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to auto-discover NWN2: {str(e)}"
        )


# @router.post("/system/background-loading/trigger/", response_model=BackgroundLoadingTriggerResponse)
def trigger_background_loading_disabled():
    """Trigger background loading of game data"""
    try:
        # Use ResourceManager for loading - no duplicated logic
        from fastapi_server import get_shared_resource_manager
        rm = get_shared_resource_manager()
        
        if rm:
            # Could trigger cache clearing/rebuilding if needed
            trigger_result = {"status": "success", "message": "Background loading triggered"}
        else:
            trigger_result = {"status": "error", "message": "ResourceManager not available"}
        
        return BackgroundLoadingTriggerResponse(**trigger_result)
        
    except Exception as e:
        logger.error(f"Background loading trigger failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to trigger background loading: {str(e)}"
        )


# @router.get("/system/background-loading/status/", response_model=BackgroundLoadingStatusResponse)
def background_loading_status_disabled():
    """Get background loading status"""
    try:
        # Use ResourceManager for status - no duplicated logic
        from fastapi_server import get_shared_resource_manager
        rm = get_shared_resource_manager()
        
        if rm:
            # Get cache stats as status indicator
            cache_stats = rm.get_cache_stats()
            status_result = {"status": "ready", "cache_stats": cache_stats}
        else:
            status_result = {"status": "not_ready", "message": "ResourceManager not available"}
        
        return BackgroundLoadingStatusResponse(**status_result)
        
    except Exception as e:
        logger.error(f"Background loading status check failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get background loading status: {str(e)}"
        )


# @router.get("/system/initialization/status/", response_model=InitializationStatusResponse)
def initialization_status_disabled():
    """Get system initialization status"""
    try:
        # Use fastapi_server initialization status - no duplicated logic
        from fastapi_server import initialization_status
        
        # Return the actual initialization status from the server
        init_status = initialization_status
        
        return InitializationStatusResponse(**init_status)
        
    except Exception as e:
        logger.error(f"Initialization status check failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get initialization status: {str(e)}"
        )