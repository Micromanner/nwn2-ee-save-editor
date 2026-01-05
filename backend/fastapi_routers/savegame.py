"""Savegame router - Save game import, export, backup, and management operations."""

import os
import io
from pathlib import Path
from fastapi import APIRouter, Depends, HTTPException, status
from loguru import logger
from services.fastapi.session_registry import get_character_session, save_character_session, get_path_from_id, register_character_path
from fastapi_routers.dependencies import (
    get_character_manager,
    CharacterManagerDep,
    check_system_ready
)
from fastapi_models.savegame_models import SavegameUpdateRequest

router = APIRouter(tags=["savegame"])


def _validate_savegame_character(character_id: int) -> str:
    """Validate that character belongs to a save game."""
    file_path = get_path_from_id(character_id)
    if not file_path:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Character with ID {character_id} not found"
        )
    
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
    """Validate that path is a valid save directory."""
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


@router.post("/savegames/import")
def import_savegame(
    import_request: dict,
    system_ready: bool = Depends(check_system_ready)
):
    """Import a character from a save game directory."""
    try:
        from fastapi_models import SavegameImportResponse
        from services.core.savegame_handler import SaveGameHandler

        if 'save_path' not in import_request:
            raise HTTPException(status_code=400, detail="Invalid import request")

        save_path = _validate_save_directory(import_request['save_path'])

        savegame_handler = SaveGameHandler(save_path)
        
        char_summary = savegame_handler.read_character_summary()
        
        character_id = register_character_path(save_path)
        
        session = get_character_session(save_path)
        
        character_name = char_summary.get('name', "Unknown")
        
        import_result = {
            'success': True,
            'message': f'Successfully imported character {character_name}',
            'character_id': str(character_id),
            'character_name': character_name,
            'save_path': save_path,
            'files_imported': 2, # Approximation, mainly playerlist + bic
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


@router.get("/{character_id}/companions")
def list_savegame_companions(character_id: int):
    """List all companions available in a save game."""
    try:
        from services.core.savegame_handler import SaveGameHandler, SaveGameError
        from fastapi_models import SavegameCompanionsResponse, CompanionInfo
        
        file_path = _validate_savegame_character(character_id)
        
        handler = SaveGameHandler(file_path)
        companion_names = handler.list_companions()
        
        companions = []
        for comp_name in companion_names:
            companion_info = CompanionInfo(
                name=comp_name,
                file_name=f"{comp_name}.ros",
                tag=comp_name,
                level=None,
                class_name=None,
                is_active=False,
                influence=None
            )
            companions.append(companion_info)
        
        logger.info(
            f"Listed savegame companions: character_id={character_id}, "
            f"companions_count={len(companions)}"
        )
        
        return SavegameCompanionsResponse(
            companions=companions,
            count=len(companions),
            active_companions=0
        )
        
    except SaveGameError as e:
        logger.error(f"SaveGameHandler error listing companions: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to list companions: {str(e)}"
        )
    except Exception as e:
        logger.error(f"Failed to list companions: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to list companions: {str(e)}"
        )


@router.get("/{character_id}/info")
def get_savegame_info(character_id: int):
    """Get basic save game information."""
    try:
        from services.core.savegame_handler import SaveGameHandler
        from fastapi_models import SavegameInfoResponse
        
        file_path = _validate_savegame_character(character_id)
        handler = SaveGameHandler(file_path)
        
        files_in_save = handler.list_files()
        companion_names = handler.list_companions()
        module_name = handler.extract_current_module()
        
        info_result = {
            'save_directory': file_path,
            'character_name': "Character",
            'files_in_save': files_in_save,
            'module_name': module_name,
            'companions': [{'name': name, 'file_name': f'{name}.ros'} for name in companion_names],
            'backups': [],
        }
        
        return SavegameInfoResponse(**info_result)
        
    except Exception as e:
        logger.error(f"Failed to get save game info: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get save game information: {str(e)}"
        )


@router.post("/{character_id}/update")
def update_savegame_character(
    character_id: int,
    update_request: SavegameUpdateRequest,
    manager: CharacterManagerDep
):
    """Update character data in save game."""
    try:
        from services.core.savegame_handler import SaveGameHandler
        from fastapi_models import SavegameUpdateResponse
        
        file_path = _validate_savegame_character(character_id)
        
        if update_request.sync_current_state:
            if update_request.updates:
                updates = update_request.updates
                
                # Use manager methods instead of direct GFF access
                if 'deity' in updates:
                    manager.update_deity(updates['deity'])
                    
                if 'biography' in updates:
                    manager.update_biography(updates['biography'])
                    
                first_name = updates.get('first_name')
                last_name = updates.get('last_name')
                if first_name is not None or last_name is not None:
                    manager.update_name(first_name=first_name, last_name=last_name)
            
            success = save_character_session(file_path, create_backup=update_request.create_backup)
            
            return SavegameUpdateResponse(
                success=success,
                message="Character state synchronized to save files",
                changes={'sync_current_state': 'Completed'},
                files_updated=['playerlist.ifo', 'player.bic'],
                backup_created=update_request.create_backup
            )
        
        if update_request.updates:
            handler = SaveGameHandler(file_path)
            for filename, content in update_request.updates.items():
                if isinstance(content, str):
                    content = content.encode('utf-8')
                handler.update_file(filename, content, backup=update_request.create_backup)
            
            return SavegameUpdateResponse(
                success=True,
                message="Files updated",
                changes={f: "Updated" for f in update_request.updates.keys()},
                files_updated=list(update_request.updates.keys()),
                backup_created=update_request.create_backup
            )
        
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="No updates provided"
        )
        
    except Exception as e:
        logger.error(f"Failed to update savegame: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update save game: {str(e)}"
        )


@router.get("/{character_id}/backups")
def list_savegame_backups(character_id: int):
    """List all available backups for a save game."""
    try:
        from services.core.savegame_handler import SaveGameHandler
        from fastapi_models import SavegameBackupsResponse, BackupInfo
        
        file_path = _validate_savegame_character(character_id)
        
        handler = SaveGameHandler(file_path)
        backups_data = handler.list_backups()
        
        backups = []
        for backup in backups_data:
            backup_info = BackupInfo(
                path=backup['path'],
                folder_name=backup['folder_name'],
                timestamp=backup['timestamp'],
                display_name=backup['display_name'],
                size_bytes=backup['size_bytes'],
                original_save=backup['original_save']
            )
            backups.append(backup_info)
        
        logger.info(f"Listed backups: character_id={character_id}, backups_count={len(backups)}")
        
        return SavegameBackupsResponse(
            backups=backups,
            count=len(backups)
        )
        
    except Exception as e:
        logger.error(f"Error listing backups: {e}")
        if isinstance(e, HTTPException):
            raise
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to list backups: {str(e)}"
        )


@router.post("/{character_id}/restore")
def restore_savegame_backup(
    character_id: int,
    restore_request: dict
):
    """Restore a save game from backup."""
    try:
        from services.core.savegame_handler import SaveGameHandler
        from fastapi_models import SavegameRestoreRequest, SavegameRestoreResponse
        
        file_path = _validate_savegame_character(character_id)
        
        if not isinstance(restore_request, SavegameRestoreRequest):
            request_data = SavegameRestoreRequest.model_validate(restore_request)
        else:
            request_data = restore_request
        
        if not request_data.confirm_restore:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Restore confirmation required"
            )
        
        handler = SaveGameHandler(file_path)
        restore_result = handler.restore_from_backup(
            backup_path=request_data.backup_path,
            create_pre_restore_backup=request_data.create_pre_restore_backup
        )
        
        cleanup_result = handler.cleanup_old_backups(keep_count=10)
        
        logger.info(f"Restored backup: character_id={character_id}, backup_path={request_data.backup_path}")
        
        return SavegameRestoreResponse(
            success=restore_result['success'],
            restored_from=restore_result['restored_from'],
            files_restored=restore_result['files_restored'],
            pre_restore_backup=restore_result.get('pre_restore_backup'),
            restore_timestamp=restore_result['restore_timestamp'],
            backups_cleaned_up=cleanup_result['cleaned_up']
        )
        
    except Exception as e:
        logger.error(f"Error restoring backup: {e}")
        if isinstance(e, HTTPException):
            raise
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to restore backup: {str(e)}"
        )


@router.post("/{character_id}/cleanup-backups")
def cleanup_savegame_backups(
    character_id: int,
    keep_count: int = 10
):
    """Clean up old backups."""
    try:
        from services.core.savegame_handler import SaveGameHandler
        
        file_path = _validate_savegame_character(character_id)
        
        handler = SaveGameHandler(file_path)
        cleanup_result = handler.cleanup_old_backups(keep_count=keep_count)
        
        logger.info(f"Cleaned up backups: character_id={character_id}, cleaned={cleanup_result['cleaned_up']}, kept={cleanup_result['kept']}")
        
        return cleanup_result
        
    except Exception as e:
        logger.error(f"Error cleaning up backups: {e}")
        if isinstance(e, HTTPException):
            raise
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to clean up backups: {str(e)}"
        )
