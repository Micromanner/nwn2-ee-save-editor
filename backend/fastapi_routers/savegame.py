"""
Savegame router - Save game import, export, backup, and management operations
Handles operations on NWN2 save game directories and files
All operations use SaveGameHandler to ensure NWN2 save file integrity
"""

import os
import glob
import shutil
import datetime
from typing import Dict, Any, List, Optional
from pathlib import Path
from fastapi import APIRouter, Depends, HTTPException, status
from loguru import logger
from fastapi_core.session_registry import get_character_session, save_character_session, get_path_from_id
from fastapi_routers.dependencies import (
    get_character_manager,
    CharacterManagerDep,
    check_system_ready
)
from fastapi_models.savegame_models import SavegameUpdateRequest # Explicit import for router signature
# from fastapi_models import (...) - moved to lazy loading
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




@router.post("/savegames/import")
def import_savegame(
    import_request: dict,  # Use dict instead of forward reference
    system_ready: bool = Depends(check_system_ready)
):
    """Import a character from a save game directory"""
    try:
        # Lazy imports for performance
        import io
        from fastapi_models import SavegameImportRequest, SavegameImportResponse
        from services.savegame_handler import SaveGameHandler
        from nwn2_rust import GffParser

        # Type validation at runtime
        if 'save_path' not in import_request:
            raise HTTPException(status_code=400, detail="Invalid import request")

        # Use helper function - no duplicated logic
        save_path = _validate_save_directory(import_request['save_path'])

        savegame_handler = SaveGameHandler(save_path)

        # Read and parse playerlist.ifo directly with Rust parser
        raw_files = savegame_handler.batch_read_character_files()
        if 'playerlist.ifo' not in raw_files:
            raise HTTPException(status_code=500, detail="Failed to read playerlist.ifo from save")

        from nwn2_rust import GffParser
        player_data = GffParser.from_bytes(raw_files['playerlist.ifo']).to_dict()
        mod_player_list = player_data.get('Mod_PlayerList', [])
        if not mod_player_list:
            raise HTTPException(status_code=500, detail="No player data found in save game")

        character_data = mod_player_list[0]  # First player in the list
        
        # Register character path and create session properly
        from fastapi_core.session_registry import register_character_path
        
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
            'files_imported': len(raw_files),
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
    """List all companions available in a save game"""
    try:
        # Lazy imports for performance
        from services.savegame_handler import SaveGameHandler, SaveGameError
        from fastapi_models import SavegameCompanionsResponse, CompanionInfo
        
        # Use helper function - no duplicated logic
        file_path = _validate_savegame_character(character_id)
        
        handler = SaveGameHandler(file_path)
        companion_names = handler.list_companions()
        
        # Build companion info list
        companions = []
        for comp_name in companion_names:
            companion_info = CompanionInfo(
                name=comp_name,
                file_name=f"{comp_name}.ros",
                tag=comp_name,
                level=None,  # Would need to parse ROS file for details
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
            active_companions=0  # Would need parsing to determine active companions
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
    """Get basic save game information using SaveGameHandler"""
    try:
        # Lazy imports for performance
        from services.savegame_handler import SaveGameHandler
        from fastapi_models import SavegameInfoResponse
        
        file_path = _validate_savegame_character(character_id)
        handler = SaveGameHandler(file_path)
        
        # Simple relay to SaveGameHandler methods
        files_in_save = handler.list_files()
        companion_names = handler.list_companions()
        module_name = handler.extract_current_module()
        
        info_result = {
            'save_directory': file_path,
            'character_name': "Character",  # Would need session data for actual name
            'files_in_save': files_in_save,
            'module_name': module_name,
            'companions': [{'name': name, 'file_name': f'{name}.ros'} for name in companion_names],
            'backups': [],  # SaveGameHandler doesn't list backups - would need to be added there
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
    """Update character data in save game using existing session save"""
    try:
        # Lazy imports for performance
        from services.savegame_handler import SaveGameHandler
        from fastapi_models import SavegameUpdateResponse
        
        file_path = _validate_savegame_character(character_id)
        
        if update_request.sync_current_state:
            # Handle specific field updates before syncing
            if update_request.updates:
                updates = update_request.updates
                
                # Update raw GFF fields for deity and biography
                if 'deity' in updates:
                    manager.gff.set('Deity', updates['deity'])
                    logger.info(f"Updated deity to: {updates['deity']}")
                    
                if 'biography' in updates:
                    # Description is a CExoLocString with substrings array
                    desc_struct = manager.gff.get('Description', {})
                    if isinstance(desc_struct, dict) and 'substrings' in desc_struct:
                        # Update existing structure
                        if desc_struct['substrings']:
                            desc_struct['substrings'][0]['string'] = updates['biography']
                        else:
                            desc_struct['substrings'] = [{'string': updates['biography'], 'language': 0, 'gender': 0}]
                        manager.gff.set('Description', desc_struct)
                    else:
                        # Create new structure
                        manager.gff.set('Description', {
                            'string_ref': 4294967295,
                            'substrings': [{'string': updates['biography'], 'language': 0, 'gender': 0}]
                        })
                    logger.info(f"Updated biography")
                    
                if 'first_name' in updates:
                    # FirstName is a localized string with 'value' key
                    first_name_struct = manager.gff.get('FirstName', {})
                    if isinstance(first_name_struct, dict):
                        first_name_struct['value'] = updates['first_name']
                        manager.gff.set('FirstName', first_name_struct)
                    else:
                        manager.gff.set('FirstName', {'value': updates['first_name']})
                    logger.info(f"Updated first_name to: {updates['first_name']}")
                    
                if 'last_name' in updates:
                    # LastName is a localized string with 'value' key
                    last_name_struct = manager.gff.get('LastName', {})
                    if isinstance(last_name_struct, dict):
                        last_name_struct['value'] = updates['last_name']
                        manager.gff.set('LastName', last_name_struct)
                    else:
                        manager.gff.set('LastName', {'value': updates['last_name']})
                    logger.info(f"Updated last_name to: {updates['last_name']}")
            
            # Simple relay to existing session save functionality
            success = save_character_session(character_id, create_backup=update_request.create_backup)
            
            return SavegameUpdateResponse(
                success=success,
                message="Character state synchronized to save files",
                changes={'sync_current_state': 'Completed'},
                files_updated=['playerlist.ifo', 'player.bic'],
                backup_created=update_request.create_backup
            )
        
        # For direct file updates, use SaveGameHandler
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
    """List all available backups for a save game"""
    try:
        # Lazy imports for performance
        from services.savegame_handler import SaveGameHandler, SaveGameError
        from fastapi_models import SavegameBackupsResponse, BackupInfo
        
        # Use helper function - no duplicated logic
        file_path = _validate_savegame_character(character_id)
        
        handler = SaveGameHandler(file_path)
        backups_data = handler.list_backups()
        
        # Convert to response model
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
    restore_request  # Type removed for lazy loading
):
    """Restore a save game from backup"""
    try:
        # Lazy imports for performance
        from services.savegame_handler import SaveGameHandler, SaveGameError
        from fastapi_models import SavegameRestoreRequest, SavegameRestoreResponse
        
        # Use helper function - no duplicated logic
        file_path = _validate_savegame_character(character_id)
        
        # Parse request
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
        
        # Clean up old backups after successful restore
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
    """Clean up old backups, keeping only the most recent ones"""
    try:
        # Lazy imports for performance
        from services.savegame_handler import SaveGameHandler, SaveGameError
        
        # Use helper function - no duplicated logic
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
