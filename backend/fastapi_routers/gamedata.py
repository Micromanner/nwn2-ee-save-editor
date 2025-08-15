"""
Gamedata endpoints router - NWN2 paths, game configuration
"""

import logging
from fastapi import APIRouter, HTTPException, status
from pathlib import Path

from config.nwn2_settings import nwn2_paths
from fastapi_models import (
    NWN2PathInfo,
    NWN2PathsResponse,
    GameDataConfigResponse
)

logger = logging.getLogger(__name__)

router = APIRouter()


@router.get("/paths/")
def get_nwn2_paths():
    """Get NWN2 installation paths"""
    try:
        # Helper function to create path info - no duplicated logic
        def _create_path_info(path) -> NWN2PathInfo:
            if not path:
                return NWN2PathInfo(path="", exists=False, readable=False, writable=False)
            
            path_obj = Path(path)
            return NWN2PathInfo(
                path=str(path_obj),
                exists=path_obj.exists(),
                readable=path_obj.is_file() or path_obj.is_dir() if path_obj.exists() else False,
                writable=path_obj.parent.exists() and path_obj.parent.is_dir() if not path_obj.exists() else True,
                size_bytes=path_obj.stat().st_size if path_obj.is_file() else None
            )
        
        # Check if installation is valid
        installation_valid = (
            nwn2_paths.game_folder and Path(nwn2_paths.game_folder).exists() and
            nwn2_paths.user_folder and Path(nwn2_paths.user_folder).exists()
        )
        
        return {
            "saves": str(nwn2_paths.saves) if nwn2_paths.saves else "",
            "game_folder": str(nwn2_paths.game_folder) if nwn2_paths.game_folder else "",
            "user_folder": str(nwn2_paths.user_folder) if nwn2_paths.user_folder else "",
            "data": str(nwn2_paths.data) if nwn2_paths.data else "",
            "campaigns": str(nwn2_paths.campaigns) if nwn2_paths.campaigns else "",
            "modules": str(nwn2_paths.modules) if nwn2_paths.modules else "",
            "localvault": str(nwn2_paths.localvault) if nwn2_paths.localvault else "",
            "tlk": str(nwn2_paths.dialog_tlk) if hasattr(nwn2_paths, 'dialog_tlk') and nwn2_paths.dialog_tlk else "",
            "installation_valid": installation_valid,
            "version": None,
            "expansions": []
        }
        
    except Exception as e:
        logger.error(f"Failed to get NWN2 paths: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get NWN2 paths: {str(e)}"
        )


@router.get("/config/", response_model=GameDataConfigResponse)
def get_gamedata_config():
    """Get gamedata configuration"""
    try:
        # Use path string conversion helper - no duplicated logic
        def _path_str(path):
            return str(path) if path else ""
        
        return GameDataConfigResponse(
            nwn2_install_path=_path_str(nwn2_paths.game_folder),
            nwn2_user_path=_path_str(nwn2_paths.user_folder),
            saves_path=_path_str(nwn2_paths.saves),
            data_path=_path_str(nwn2_paths.data),
            dialog_tlk_path=_path_str(nwn2_paths.dialog_tlk)
        )
        
    except Exception as e:
        logger.error(f"Failed to get gamedata config: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get gamedata config: {str(e)}"
        )