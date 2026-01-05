"""
System endpoints router - health checks, cache management, configuration
"""

from typing import Dict, Any
from fastapi import APIRouter, HTTPException, status
from loguru import logger

router = APIRouter()


@router.get("/health")
def health_check():
    """Health check endpoint."""
    from fastapi_models import HealthResponse
    return HealthResponse(status="healthy", service="nwn2-save-editor")


@router.get("/system/cache/status")
def cache_status():
    """Get current cache status and metadata."""
    try:
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
            from fastapi_server import get_shared_resource_manager
            resource_manager = get_shared_resource_manager()
            
            if resource_manager is not None:
                cache_stats = resource_manager.get_cache_stats()
                
                status_value = "ready" if cache_stats.get('enabled', False) else "loading"
                
                hit_rate_str = cache_stats.get('hit_rate', '0.0%')
                hit_rate = float(hit_rate_str.rstrip('%')) if hit_rate_str and hit_rate_str != '0.0%' else 0.0
                
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
            
            cache_dir = Path(__file__).parent.parent / "cache"
            
            if cache_dir.exists():
                cache_files = list(cache_dir.rglob("*"))
                cache_files = [f for f in cache_files if f.is_file()]
                
                total_size = sum(f.stat().st_size for f in cache_files)
                
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

@router.get("/system/config")
def get_config():
    """Get current configuration."""
    try:
        from config.nwn2_settings import nwn2_paths
        from fastapi_models import ConfigResponse
        
        paths_info = nwn2_paths.get_all_paths_info()
        
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


@router.get("/system/nwn2-data-path")
def nwn2_data_path_config():
    """Get NWN2 data path configuration."""
    try:
        from config.nwn2_settings import nwn2_paths
        from fastapi_models import NWN2PathResponse
        
        path_config = {
            'nwn2_game_folder': str(nwn2_paths.game_folder),

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
    """Auto-discover NWN2 installation paths."""
    try:
        from config.nwn2_settings import nwn2_paths
        from fastapi_models import AutoDiscoverResponse
        
        discovery_data = nwn2_paths.discover_all_nwn2_installations()
        
        installations = []
        for path in discovery_data['all_installations']:
            installation = {
                'path': str(path),
                'name': path.name,
                'valid': True,
                'has_data_folder': (path / 'data').exists() or (path / 'Data').exists(),
                'has_dialog_tlk': (path / 'Dialog.tlk').exists() or (path / 'dialog.tlk').exists(),
                'type': 'unknown'
            }
            
            if path in discovery_data['steam_installations']:
                installation['type'] = 'steam'
            elif path in discovery_data['gog_installations']:
                installation['type'] = 'gog'
            elif 'Enhanced Edition' in str(path):
                installation['type'] = 'enhanced_edition'
            
            installations.append(installation)
        
        paths_found = {
            'nwn2_game_folder': str(nwn2_paths.game_folder),

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
