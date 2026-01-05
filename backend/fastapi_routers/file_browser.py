"""File browser router - List and browse save files and backups."""

import datetime
import os
from pathlib import Path
from typing import List, Optional
from fastapi import APIRouter, HTTPException, status, Query
from pydantic import BaseModel
from loguru import logger
router = APIRouter(tags=["file_browser"])


class FileInfo(BaseModel):
    name: str
    path: str
    size: int
    modified: str
    is_directory: bool
    save_name: Optional[str] = None
    character_name: Optional[str] = None
    thumbnail: Optional[str] = None


class FileListResponse(BaseModel):
    files: List[FileInfo]
    total_count: int
    path: str
    current_path: str


@router.get("/saves/list")
def list_saves(
    path: Optional[str] = Query(None, description="Directory path to list saves from"),
    limit: int = Query(50, ge=1, le=500, description="Maximum number of files to return"),
    offset: int = Query(0, ge=0, description="Number of files to skip")
):
    """List save files in a directory for the file browser modal."""
    try:
        from config.nwn2_settings import nwn2_paths
        from services.core.playerinfo_service import PlayerInfo

        target_path = path if path else str(nwn2_paths.saves)

        if not target_path or not os.path.exists(target_path):
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Directory not found: {target_path}"
            )

        if not os.path.isdir(target_path):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Path is not a directory"
            )

        files_list = []

        try:
            entries = list(os.scandir(target_path))
        except PermissionError:
            raise HTTPException(
                status_code=status.HTTP_403_FORBIDDEN,
                detail="Permission denied to access directory"
            )

        for entry in entries:
            try:
                if entry.name.startswith('.') or entry.name.lower() == 'backups':
                    continue

                stat_info = entry.stat()
                file_info = {
                    "name": entry.name,
                    "path": entry.path,
                    "modified": str(stat_info.st_mtime),
                    "is_directory": entry.is_dir(),
                    "stat": stat_info
                }
                files_list.append(file_info)
            except (OSError, PermissionError) as e:
                logger.warning(f"Failed to stat file {entry.name}: {e}")
                continue

        total_count = len(files_list)
        files_list.sort(key=lambda x: x["name"], reverse=True)
        paginated_entries = files_list[offset:offset + limit]
        final_files = []
        
        for item in paginated_entries:
            size = item["stat"].st_size
            save_name = None
            thumbnail = None
            
            if item["is_directory"]:
                try:
                    size = sum(f.stat().st_size for f in Path(item["path"]).rglob('*') if f.is_file())
                except (OSError, PermissionError):
                    size = 0

                savename_txt = Path(item["path"]) / 'savename.txt'
                if savename_txt.exists():
                    try:
                        save_name = savename_txt.read_text(encoding='utf-8').strip()
                    except (OSError, UnicodeDecodeError):
                        pass
                
                screen_tga = Path(item["path"]) / 'screen.tga'
                if screen_tga.exists():
                    thumbnail = str(screen_tga)

                playerinfo_bin = Path(item["path"]) / 'playerinfo.bin'
                character_name = PlayerInfo.get_player_name(str(playerinfo_bin))

            final_files.append(FileInfo(
                name=item["name"],
                path=item["path"],
                size=size,
                modified=item["modified"],
                is_directory=item["is_directory"],
                save_name=save_name,
                character_name=character_name if item["is_directory"] else None,
                thumbnail=thumbnail
            ))

        logger.info(f"Listed saves: path={target_path}, count={len(final_files)}, total={total_count}")

        return FileListResponse(
            files=final_files,
            total_count=total_count,
            path=target_path,
            current_path=target_path
        )

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to list saves: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to list saves: {str(e)}"
        )


@router.get("/backups/list")
def list_backups_directory(
    save_name: Optional[str] = Query(None, description="Save name to list backups for"),
    path: Optional[str] = Query(None, description="Save directory path"),
    limit: int = Query(50, ge=1, le=500, description="Maximum number of files to return"),
    offset: int = Query(0, ge=0, description="Number of files to skip")
):
    """List backups for a specific save or all backups in the backups directory."""
    try:
        from config.nwn2_settings import nwn2_paths
        from services.core.playerinfo_service import PlayerInfo

        if not path and not save_name:
            saves_dir = str(nwn2_paths.saves)
            backups_dir = os.path.join(saves_dir, 'backups')
        elif path:
            backups_dir = path
        else:
            saves_dir = str(nwn2_paths.saves)
            backups_dir = os.path.join(saves_dir, 'backups', save_name)

        if not os.path.exists(backups_dir):
            return FileListResponse(
                files=[],
                total_count=0,
                path=backups_dir,
                current_path=backups_dir
            )

        if not os.path.isdir(backups_dir):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Backup path is not a directory"
            )

        files_list = []

        try:
            entries = list(os.scandir(backups_dir))
            logger.debug(f"Scanning backups directory: {backups_dir}, found {len(entries)} entries")
        except PermissionError:
            raise HTTPException(
                status_code=status.HTTP_403_FORBIDDEN,
                detail="Permission denied to access backup directory"
            )

        for entry in entries:
            try:
                if entry.name.startswith('.'):
                    continue

                stat_info = entry.stat()
                modified_time = stat_info.st_mtime

                if entry.is_dir() and '_backup_' in entry.name:
                    try:
                        import re
                        from datetime import datetime
                        match = re.search(r'_backup_(\d{8})_(\d{6})', entry.name)
                        if match:
                            date_str = match.group(1)
                            time_str = match.group(2)
                            dt = datetime.strptime(f"{date_str}{time_str}", '%Y%m%d%H%M%S')
                            modified_time = dt.timestamp()
                    except Exception:
                        pass

                files_list.append({
                    "name": entry.name,
                    "path": entry.path,
                    "modified": str(modified_time),
                    "is_directory": entry.is_dir(),
                    "stat": stat_info
                })
            except (OSError, PermissionError) as e:
                logger.warning(f"Failed to stat backup {entry.name}: {e}")
                continue

        files_list.sort(key=lambda x: float(x["modified"]), reverse=True)
        
        total_count = len(files_list)
        paginated_entries = files_list[offset:offset + limit]
        
        final_files = []
        for item in paginated_entries:
            size = item["stat"].st_size
            save_name = None
            character_name = None
            display_name = None
            
            if item["is_directory"]:
                try:
                    size = sum(f.stat().st_size for f in Path(item["path"]).rglob('*') if f.is_file())
                except (OSError, PermissionError):
                    size = 0

                savename_txt = Path(item["path"]) / 'savename.txt'
                if savename_txt.exists():
                    try:
                        save_name = savename_txt.read_text(encoding='utf-8').strip()
                    except (OSError, UnicodeDecodeError):
                        pass

                playerinfo_bin = Path(item["path"]) / 'playerinfo.bin'
                character_name = PlayerInfo.get_player_name(str(playerinfo_bin))

            final_files.append(FileInfo(
                name=item["name"],
                path=item["path"],
                size=size,
                modified=item["modified"],
                is_directory=item["is_directory"],
                save_name=save_name,
                character_name=character_name if item["is_directory"] else None
            ))

        logger.info(f"Listed backups: path={backups_dir}, count={len(final_files)}, total={total_count}")

        return FileListResponse(
            files=final_files,
            total_count=total_count,
            path=backups_dir,
            current_path=backups_dir
        )

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to list backups: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to list backups: {str(e)}"
        )


class RestoreBackupRequest(BaseModel):
    backup_path: str
    save_path: Optional[str] = None
    create_pre_restore_backup: bool = True
    confirm_restore: bool = False


class RestoreBackupResponse(BaseModel):
    success: bool
    message: str
    restored_from: str
    files_restored: int
    pre_restore_backup: Optional[str] = None


@router.post("/backups/restore")
def restore_backup(restore_request: RestoreBackupRequest):
    """Restore a save from a backup directory."""
    try:
        from services.core.savegame_handler import SaveGameHandler, SaveGameError
        import shutil
        import os
        from pathlib import Path

        logger.info(f"Restore request received for backup: {restore_request.backup_path}")

        if not restore_request.confirm_restore:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Restore confirmation required"
            )

        if not os.path.exists(restore_request.backup_path):
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Backup not found: {restore_request.backup_path}"
            )

        backup_path_lower = restore_request.backup_path.lower()
        if backup_path_lower.endswith('.cam') and 'campaign_backups' in backup_path_lower:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Campaign backups must be restored through /api/characters/{id}/campaign/restore endpoint"
            )

        save_path = restore_request.save_path
        
        try:
            backup_path = Path(restore_request.backup_path)
            
            if not save_path:
                try:
                    save_path = SaveGameHandler.infer_save_path_from_backup(str(restore_request.backup_path))
                    logger.info(f"Inferred save path: {save_path} (from backup: {restore_request.backup_path})")
                except SaveGameError as e:
                     raise HTTPException(status_code=400, detail=str(e))

            target_path = Path(save_path).resolve()
            backup_source = Path(restore_request.backup_path).resolve()
            
            if target_path.name == 'backups':
                raise ValueError("Dangerous restore detected: inferred target is the 'backups' folder itself. Aborting.")
            
            if 'backups' in str(target_path) and target_path != backup_source:
                 if target_path.parent.name == 'backups':
                     raise ValueError(f"Dangerous restore: Target '{target_path}' is inside backups directory. Aborting.")

            if target_path == backup_source:
                 raise ValueError("Source and destination are the same.")
                 
            if target_path in backup_source.parents:
                 raise ValueError(f"Dangerous restore: Target {target_path} is a parent of source {backup_source}")

        except Exception as e:
            logger.error(f"Failed to infer/validate save path: {e}")
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Invalid restore target: {e}"
            )

        pre_restore_backup = None
        temp_original_path = None

        if os.path.exists(save_path) and restore_request.create_pre_restore_backup:
            if os.path.isdir(save_path):
                temp_handler = SaveGameHandler(save_path)
                pre_restore_backup = temp_handler._create_backup()
                logger.info(f"Created pre-restore backup: {pre_restore_backup}")

        try:
            if os.path.exists(save_path):
                timestamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S_%f")
                temp_original_path = f"{save_path}_restoring_{timestamp}"
                os.rename(save_path, temp_original_path)
                logger.info(f"Renamed original to temp: {temp_original_path}")

            if os.path.isdir(restore_request.backup_path):
                shutil.copytree(restore_request.backup_path, save_path)
                files_count = sum(len(files) for _, _, files in os.walk(save_path))
            elif os.path.isfile(restore_request.backup_path):
                shutil.copy2(restore_request.backup_path, save_path)
                files_count = 1
            else:
                raise FileNotFoundError(f"Source not found: {restore_request.backup_path}")

            if temp_original_path and os.path.exists(temp_original_path):
                if os.path.isdir(temp_original_path):
                    shutil.rmtree(temp_original_path)
                else:
                    os.remove(temp_original_path)
                logger.info(f"Deleted temp original: {temp_original_path}")

            if os.path.isdir(save_path):
                savename_path = os.path.join(save_path, 'savename.txt')
                if os.path.exists(savename_path):
                    try:
                        with open(savename_path, 'r', encoding='utf-8', errors='ignore') as f:
                            content = f.read().strip()

                        if content.startswith("Backup of "):
                            original_name = content[10:].split(" at ")[0] if " at " in content else content[10:]
                            with open(savename_path, 'w', encoding='utf-8') as f:
                                f.write(original_name)
                    except Exception:
                        pass

            restore_result = {
                'success': True,
                'restored_from': restore_request.backup_path,
                'files_restored': files_count,
                'pre_restore_backup': pre_restore_backup,
                'restore_timestamp': datetime.datetime.now().isoformat()
            }

        except Exception as e:
            if temp_original_path and os.path.exists(temp_original_path):
                logger.warning(f"Restore failed, rolling back from {temp_original_path}")
                if os.path.exists(save_path):
                    if os.path.isdir(save_path):
                        shutil.rmtree(save_path)
                    else:
                        os.remove(save_path)
                os.rename(temp_original_path, save_path)
                logger.info(f"Rollback complete: restored original at {save_path}")

            logger.error(f"Restore operation failed: {e}")
            raise HTTPException(status_code=500, detail=f"Restore failed: {e}")

        logger.info(f"Restored backup: from={restore_request.backup_path}, to={save_path}")

        return RestoreBackupResponse(
            success=restore_result['success'],
            message=f"Successfully restored from backup",
            restored_from=restore_result['restored_from'],
            files_restored=restore_result['files_restored'],
            pre_restore_backup=restore_result.get('pre_restore_backup')
        )

    except SaveGameError as e:
        logger.error(f"SaveGameError restoring backup: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to restore backup: {str(e)}"
        )
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to restore backup: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to restore backup: {str(e)}"
        )


class CreateBackupRequest(BaseModel):
    save_path: str
    backup_name: Optional[str] = None


class CreateBackupResponse(BaseModel):
    success: bool
    message: str
    backup_path: str
    backup_name: str
    timestamp: str


@router.post("/backups/create")
def create_manual_backup(backup_request: CreateBackupRequest):
    """Create a manual backup of a save directory."""
    try:
        from services.core.savegame_handler import SaveGameHandler, SaveGameError
        import datetime

        if not os.path.exists(backup_request.save_path):
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Save directory not found: {backup_request.save_path}"
            )

        if not os.path.isdir(backup_request.save_path):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Path is not a directory"
            )

        handler = SaveGameHandler(backup_request.save_path)
        backup_path = handler.create_backup()

        timestamp = datetime.datetime.now().isoformat()

        logger.info(f"Created manual backup: save={backup_request.save_path}, backup={backup_path}")

        return CreateBackupResponse(
            success=True,
            message="Backup created successfully",
            backup_path=backup_path,
            backup_name=os.path.basename(backup_path),
            timestamp=timestamp
        )

    except SaveGameError as e:
        logger.error(f"SaveGameError creating backup: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to create backup: {str(e)}"
        )
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to create backup: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to create backup: {str(e)}"
        )


class DeleteBackupRequest(BaseModel):
    backup_path: str


@router.delete("/backups/delete")
def delete_backup(delete_request: DeleteBackupRequest):
    """Delete a backup directory."""
    try:
        import shutil

        if not os.path.exists(delete_request.backup_path):
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Backup not found: {delete_request.backup_path}"
            )

        if not os.path.isdir(delete_request.backup_path):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Path is not a directory"
            )

        if 'backups' not in delete_request.backup_path:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Can only delete from backups directory"
            )

        shutil.rmtree(delete_request.backup_path)

        logger.info(f"Deleted backup: {delete_request.backup_path}")

        return {
            "success": True,
            "message": "Backup deleted successfully",
            "deleted_path": delete_request.backup_path
        }

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to delete backup: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to delete backup: {str(e)}"
        )
