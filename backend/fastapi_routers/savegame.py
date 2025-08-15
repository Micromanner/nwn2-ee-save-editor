"""
Savegame router - Save game import, export, backup, and management operations
Handles operations on NWN2 save game directories and files
"""

import logging
import os
import glob
import shutil
import datetime
from typing import Dict, Any, List
from fastapi import APIRouter, Depends, HTTPException, status
from fastapi_core.session_registry import get_character_session, save_character_session, get_path_from_id
# Removed character_info dependency - use session_registry directly
from parsers.savegame_handler import SaveGameHandler
from character.in_memory_save_manager import InMemorySaveManager
from fastapi_routers.dependencies import (
    get_character_manager,
    CharacterManagerDep,
    check_system_ready
)
from fastapi_models import (
    SavegameImportRequest,
    SavegameImportResponse,
    SavegameCompanionsResponse,
    SavegameInfoResponse,
    SavegameUpdateRequest,
    SavegameUpdateResponse,
    SavegameRestoreRequest,
    SavegameRestoreResponse
)

logger = logging.getLogger(__name__)
router = APIRouter(tags=["savegame"])


def _validate_savegame_character(character_id: int) -> str:
    """Helper function to validate savegame character - no duplicate session creation"""
    # Get file path from character ID (no temporary session creation)
    file_path = get_path_from_id(character_id)
    if not file_path:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Character with ID {character_id} not found"
        )
    
    # Check if path is a directory (savegame) or individual file
    if not os.path.isdir(file_path):
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="This character is not from a save game directory"
        )
    
    if not os.path.exists(file_path):
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Save game directory not found: {file_path}"
        )
    
    return file_path


def _validate_save_directory(save_path: str) -> str:
    """Helper function to validate save directory - no duplicated logic"""
    if not save_path:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="save_path is required"
        )
    
    if not os.path.isdir(save_path):
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Save directory not found: {save_path}"
        )
    
    resgff_path = os.path.join(save_path, 'resgff.zip')
    if not os.path.exists(resgff_path):
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="resgff.zip not found in save directory"
        )
    
    return save_path




@router.post("/savegames/import/", response_model=SavegameImportResponse)
def import_savegame(
    import_request: SavegameImportRequest,
    system_ready: bool = Depends(check_system_ready)
):
    """Import a character from a save game directory"""
    try:
        # Use helper function - no duplicated logic
        save_path = _validate_save_directory(import_request.save_path)
        
        # Create savegame handler to handle import - no duplicated logic
        from parsers.savegame_handler import SaveGameHandler
        from parsers.parallel_gff import extract_and_parse_save_gff_files
        
        savegame_handler = SaveGameHandler(save_path)
        
        # Extract and parse all GFF files
        gff_results = extract_and_parse_save_gff_files(savegame_handler, max_workers=4)
        
        # Get playerlist.ifo data
        if 'playerlist.ifo' not in gff_results or not gff_results['playerlist.ifo']['success']:
            error = gff_results.get('playerlist.ifo', {}).get('error', 'Unknown error')
            raise HTTPException(status_code=500, detail=f"Failed to parse playerlist.ifo: {error}")

        player_data = gff_results['playerlist.ifo']['data']
        mod_player_list = player_data.get('Mod_PlayerList', [])
        if not mod_player_list:
            raise HTTPException(status_code=500, detail="No player data found in save game")

        character_data = mod_player_list[0]  # First player in the list
        
        # Register character path and create session properly
        from fastapi_core.session_registry import register_character_path, get_character_session
        import os
        
        # Get integer ID for the character path
        character_id = register_character_path(save_path)
        
        # Create/get session (this handles the heavy lifting)
        session = get_character_session(save_path)
        
        # Extract character name for response
        first_name = character_data.get('FirstName', {}).get('value', '')
        last_name = character_data.get('LastName', {}).get('value', '')
        character_name = f"{first_name} {last_name}".strip() or "Unknown"
        
        # Create import result in expected format
        import_result = {
            'success': True,
            'message': f'Successfully imported character {character_name}',
            'character_id': str(character_id),  # Use the registered integer ID
            'character_name': character_name,
            'save_path': save_path,
            'files_imported': len(gff_results),
            'backup_created': False
        }
        
        logger.info(
            f"Imported savegame character: id={import_result['character_id']}, "
            f"name={import_result['character_name']}, save_path={save_path}"
        )
        
        return SavegameImportResponse(**import_result)
        
    except Exception as e:
        logger.error(f"Error importing save game: {e}")
        if isinstance(e, HTTPException):
            raise
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to import save game: {str(e)}"
        )


@router.get("/{character_id}/companions/", response_model=SavegameCompanionsResponse)
def list_savegame_companions(character_id: int):
    """List all companions available in a save game"""
    try:
        # Use helper function - no duplicated logic
        file_path = _validate_savegame_character(character_id)
        
        handler = SaveGameHandler(file_path)
        companions = handler.list_companions()
        
        logger.info(
            f"Listed savegame companions: character_id={character_id}, "
            f"companions_count={len(companions)}"
        )
        
        return SavegameCompanionsResponse(
            companions=companions,
            count=len(companions)
        )
        
    except Exception as e:
        logger.error(f"Failed to list companions: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to list companions: {str(e)}"
        )


@router.get("/{character_id}/info/", response_model=SavegameInfoResponse)
def get_savegame_info(character_id: int):
    """Get information about the save game, including backup status"""
    try:
        # Use helper function - no duplicated logic
        file_path = _validate_savegame_character(character_id)
        
        # Use savegame service - no duplicated logic
        from character.services.savegame_service import SavegameService
        savegame_service = SavegameService()
        
        info_result = savegame_service.get_savegame_info(file_path)
        
        logger.info(
            f"Retrieved savegame info: character_id={character_id}, "
            f"backups_count={len(info_result['backups'])}"
        )
        
        return SavegameInfoResponse(**info_result)
        
    except Exception as e:
        logger.error(f"Failed to get save game info: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get save game information: {str(e)}"
        )


@router.post("/{character_id}/update/", response_model=SavegameUpdateResponse)
def update_savegame_character(
    character_id: int,
    update_request: SavegameUpdateRequest,
    manager: CharacterManagerDep = Depends(get_character_manager)
):
    """Update character data in a save game"""
    try:
        # Use helper function - no duplicated logic
        file_path = _validate_savegame_character(character_id)
        
        if not update_request.sync_current_state and not update_request.updates:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="No updates provided and sync_current_state not requested"
            )
        
        # Use savegame service - no duplicated logic
        from character.services.savegame_service import SavegameService
        savegame_service = SavegameService()
        
        update_result = savegame_service.update_savegame(
            character_id, 
            update_request.sync_current_state, 
            update_request.updates
        )
        
        logger.info(
            f"Updated savegame character: character_id={character_id}, "
            f"changes={update_result['changes']}, backup_created={update_result['backup_created']}"
        )
        
        return SavegameUpdateResponse(**update_result)
        
    except Exception as e:
        logger.error(f"Failed to update savegame: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update save game: {str(e)}"
        )


@router.post("/{character_id}/restore/", response_model=SavegameRestoreResponse)
def restore_savegame_backup(
    character_id: int,
    restore_request: SavegameRestoreRequest
):
    """Restore a save game from a backup"""
    try:
        # Use helper function - no duplicated logic
        file_path = _validate_savegame_character(character_id)
        
        # Use savegame service - no duplicated logic
        from character.services.savegame_service import SavegameService
        savegame_service = SavegameService()
        
        restore_result = savegame_service.restore_backup(
            file_path,
            restore_request.backup_path
        )
        
        logger.info(
            f"Restored savegame from backup: character_id={character_id}, "
            f"backup_path={restore_request.backup_path}, "
            f"pre_restore_backup={restore_result['pre_restore_backup']}"
        )
        
        return SavegameRestoreResponse(**restore_result)
        
    except Exception as e:
        logger.error(f"Failed to restore backup: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to restore backup: {str(e)}"
        )






