"""
Gamedata endpoints router - NWN2 paths, game configuration
"""

import logging
from fastapi import APIRouter, HTTPException, status
from pathlib import Path

from config.nwn2_settings import nwn2_paths
# from fastapi_models import (...) - moved to lazy loading

logger = logging.getLogger(__name__)

router = APIRouter()


@router.get("/paths")
def get_nwn2_paths():
    """Get NWN2 installation paths"""
    from fastapi_models import PathInfo, CustomFolderInfo, PathConfig, NWN2PathsResponse
    try:
        # Helper to create PathInfo objects
        def create_path_info(path, auto_detected=True) -> PathInfo:
            if not path:
                return PathInfo(path=None, exists=False, auto_detected=False)
            path_obj = Path(path)
            return PathInfo(
                path=str(path_obj),
                exists=path_obj.exists(),
                auto_detected=auto_detected
            )
        
        # Helper to create CustomFolderInfo objects
        def create_custom_folder_info(path) -> CustomFolderInfo:
            path_obj = Path(path)
            return CustomFolderInfo(
                path=str(path_obj),
                exists=path_obj.exists()
            )
        
        # Get custom folders
        custom_override_folders = []
        custom_hak_folders = []
        
        if hasattr(nwn2_paths, 'custom_override_folders'):
            for folder in nwn2_paths.custom_override_folders:
                custom_override_folders.append(create_custom_folder_info(folder))
        
        if hasattr(nwn2_paths, 'custom_hak_folders'):
            for folder in nwn2_paths.custom_hak_folders:
                custom_hak_folders.append(create_custom_folder_info(folder))
        
        # Create path config
        path_config = PathConfig(
            game_folder=create_path_info(nwn2_paths.game_folder),
            documents_folder=create_path_info(nwn2_paths.user_folder),
            steam_workshop_folder=PathInfo(path=None, exists=False, auto_detected=False),  # Not available
            custom_override_folders=custom_override_folders,
            custom_module_folders=[],  # Not tracking custom module folders yet
            custom_hak_folders=custom_hak_folders
        )
        
        return NWN2PathsResponse(paths=path_config)
        
    except Exception as e:
        logger.error(f"Failed to get NWN2 paths: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get NWN2 paths: {str(e)}"
        )


@router.get("/config")
def get_gamedata_config():
    """Get gamedata configuration"""
    from fastapi_models import GameDataConfigResponse
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