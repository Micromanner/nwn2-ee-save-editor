"""
System endpoints router - health checks, cache management, configuration
"""

import logging
from typing import Dict, Any
from fastapi import APIRouter, HTTPException, status

from config.nwn2_settings import nwn2_paths
from gamedata.dynamic_loader.singleton import is_loader_ready
from fastapi_models import (
    HealthResponse, 
    ReadyResponse, 
    CacheStatus, 
    ConfigResponse,
    SystemInfo,
    CacheRebuildResponse,
    ConfigUpdateRequest,
    ConfigUpdateResponse,
    NWN2PathResponse,
    AutoDiscoverResponse,
    BackgroundLoadingTriggerResponse,
    BackgroundLoadingStatusResponse,
    InitializationStatusResponse
)

logger = logging.getLogger(__name__)

router = APIRouter()


@router.get("/health/", response_model=HealthResponse)
def health_check():
    """Health check endpoint"""
    return HealthResponse(status="healthy", service="nwn2-save-editor")


@router.get("/ready/", response_model=ReadyResponse)
def ready_check():
    """Readiness check endpoint"""
    try:
        # Check if NWN2 paths are configured
        paths_configured = bool(nwn2_paths.get('nwn2_install_dir'))
        loader_ready = is_loader_ready()
        
        ready = paths_configured and loader_ready
        
        return ReadyResponse(
            status="ready" if ready else "not_ready",
            nwn2_paths_configured=paths_configured
        )
    except Exception as e:
        logger.error(f"Ready check failed: {e}")
        return ReadyResponse(
            status="not_ready", 
            nwn2_paths_configured=False,
            error=str(e)
        )


@router.get("/system/cache/status/", response_model=CacheStatus)
def cache_status():
    """Get current cache status and metadata"""
    try:
        # Use cache service - no duplicated logic
        from services.cache_service import CacheService
        cache_service = CacheService()
        
        cache_status_data = cache_service.get_cache_status()
        
        return CacheStatus(**cache_status_data)
        
    except Exception as e:
        logger.error(f"Failed to get cache status: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get cache status: {str(e)}"
        )




@router.post("/system/cache/rebuild/", response_model=CacheRebuildResponse)
def rebuild_cache(docs_path: str = None, workshop_path: str = None):
    """Trigger cache rebuild with mod support"""
    try:
        # Use cache service - no duplicated logic
        from services.cache_service import CacheService
        cache_service = CacheService()
        
        rebuild_result = cache_service.rebuild_cache(docs_path, workshop_path)
        
        return CacheRebuildResponse(**rebuild_result)
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to rebuild cache: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to rebuild cache: {str(e)}"
        )


@router.get("/system/config/", response_model=ConfigResponse)
def get_config():
    """Get current configuration"""
    try:
        # Use configuration service - no duplicated logic
        from services.config_service import ConfigService
        config_service = ConfigService()
        
        config_data = config_service.get_config()
        
        return ConfigResponse(**config_data)
        
    except Exception as e:
        logger.error(f"Get config failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get configuration: {str(e)}"
        )


@router.post("/system/config/update/", response_model=ConfigUpdateResponse)
def update_config(config_request: ConfigUpdateRequest):
    """Update configuration"""
    try:
        # Use configuration service - no duplicated logic
        from services.config_service import ConfigService
        config_service = ConfigService()
        
        update_result = config_service.update_config(config_request.config_updates)
        
        return ConfigUpdateResponse(**update_result)
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Config update failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update configuration: {str(e)}"
        )


@router.get("/system/nwn2-data-path/", response_model=NWN2PathResponse)
def nwn2_data_path_config():
    """Get NWN2 data path configuration"""
    try:
        # Use configuration service - no duplicated logic
        from services.config_service import ConfigService
        config_service = ConfigService()
        
        path_config = config_service.get_nwn2_paths()
        
        return NWN2PathResponse(**path_config)
        
    except Exception as e:
        logger.error(f"NWN2 data path config failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get NWN2 data path configuration: {str(e)}"
        )


@router.post("/system/auto-discover-nwn2/", response_model=AutoDiscoverResponse)
def auto_discover_nwn2():
    """Auto-discover NWN2 installation paths"""
    try:
        # Use configuration service - no duplicated logic
        from services.config_service import ConfigService
        config_service = ConfigService()
        
        discovery_result = config_service.auto_discover_nwn2()
        
        return AutoDiscoverResponse(**discovery_result)
            
    except Exception as e:
        logger.error(f"Auto-discover NWN2 failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to auto-discover NWN2: {str(e)}"
        )


@router.post("/system/background-loading/trigger/", response_model=BackgroundLoadingTriggerResponse)
def trigger_background_loading():
    """Trigger background loading of game data"""
    try:
        # Use loading service - no duplicated logic
        from services.loading_service import LoadingService
        loading_service = LoadingService()
        
        trigger_result = loading_service.trigger_background_loading()
        
        return BackgroundLoadingTriggerResponse(**trigger_result)
        
    except Exception as e:
        logger.error(f"Background loading trigger failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to trigger background loading: {str(e)}"
        )


@router.get("/system/background-loading/status/", response_model=BackgroundLoadingStatusResponse)
def background_loading_status():
    """Get background loading status"""
    try:
        # Use loading service - no duplicated logic
        from services.loading_service import LoadingService
        loading_service = LoadingService()
        
        status_result = loading_service.get_background_loading_status()
        
        return BackgroundLoadingStatusResponse(**status_result)
        
    except Exception as e:
        logger.error(f"Background loading status check failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get background loading status: {str(e)}"
        )


@router.get("/system/initialization/status/", response_model=InitializationStatusResponse)
def initialization_status():
    """Get system initialization status"""
    try:
        # Use system service - no duplicated logic
        from services.system_service import SystemService
        system_service = SystemService()
        
        init_status = system_service.get_initialization_status()
        
        return InitializationStatusResponse(**init_status)
        
    except Exception as e:
        logger.error(f"Initialization status check failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get initialization status: {str(e)}"
        )