"""
File browser router - List and browse save files and backups
Handles file system operations for the unified file browser modal
"""

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
    """
    List save files in a directory for the file browser modal.

    Args:
        path: Directory path (defaults to NWN2 saves directory)
        limit: Max files to return
        offset: Files to skip for pagination

    Returns:
        List of files with metadata
    """
    try:
        from config.nwn2_settings import nwn2_paths

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
                if entry.name.startswith('.'):
                    continue

                stat_info = entry.stat()

                # Calculate folder size if it's a directory
                size = stat_info.st_size
                save_name = None
                if entry.is_dir():
                    try:
                        size = sum(f.stat().st_size for f in Path(entry.path).rglob('*') if f.is_file())
                    except (OSError, PermissionError):
                        size = 0

                    # Read save name from savename.txt
                    savename_txt = Path(entry.path) / 'savename.txt'
                    if savename_txt.exists():
                        try:
                            save_name = savename_txt.read_text(encoding='utf-8').strip()
                        except (OSError, UnicodeDecodeError):
                            pass

                file_info = FileInfo(
                    name=entry.name,
                    path=entry.path,
                    size=size,
                    modified=str(stat_info.st_mtime),
                    is_directory=entry.is_dir(),
                    save_name=save_name
                )
                files_list.append(file_info)
            except (OSError, PermissionError) as e:
                logger.warning(f"Failed to stat file {entry.name}: {e}")
                continue

        total_count = len(files_list)

        paginated_files = files_list[offset:offset + limit]

        logger.info(f"Listed saves: path={target_path}, count={len(paginated_files)}, total={total_count}")

        return FileListResponse(
            files=paginated_files,
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
    path: Optional[str] = Query(None, description="Save directory path")
):
    """
    List backups for a specific save or all backups in the backups directory.

    Args:
        save_name: Name of the save to list backups for
        path: Full path to save directory

    Returns:
        List of backup files with metadata
    """
    try:
        from config.nwn2_settings import nwn2_paths

        if not path and not save_name:
            # No path or save name: list all backups in main backups directory
            saves_dir = str(nwn2_paths.saves)
            backups_dir = os.path.join(saves_dir, 'backups')
        elif path:
            # Path provided: use it directly (already points to backups directory)
            backups_dir = path
        else:
            # Save name provided: list backups for that specific save
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
            logger.info(f"Scanning backups directory: {backups_dir}, found {len(entries)} entries")
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

                # Calculate folder size and read save name if it's a directory
                size = stat_info.st_size
                save_name = None
                modified_time = stat_info.st_mtime

                if entry.is_dir():
                    try:
                        size = sum(f.stat().st_size for f in Path(entry.path).rglob('*') if f.is_file())
                    except (OSError, PermissionError):
                        size = 0

                    # Read save name from savename.txt
                    savename_txt = Path(entry.path) / 'savename.txt'
                    if savename_txt.exists():
                        try:
                            save_name = savename_txt.read_text(encoding='utf-8').strip()
                        except (OSError, UnicodeDecodeError):
                            pass

                    # Extract timestamp from backup folder name (format: *_backup_YYYYMMDD_HHMMSS)
                    if '_backup_' in entry.name:
                        try:
                            import re
                            from datetime import datetime
                            match = re.search(r'_backup_(\d{8})_(\d{6})', entry.name)
                            if match:
                                date_str = match.group(1)
                                time_str = match.group(2)
                                timestamp_str = f"{date_str}{time_str}"
                                dt = datetime.strptime(timestamp_str, '%Y%m%d%H%M%S')
                                modified_time = dt.timestamp()
                                logger.info(f"Parsed backup timestamp from {entry.name}: {dt} -> {modified_time}")
                        except Exception as e:
                            logger.warning(f"Failed to parse backup timestamp from {entry.name}: {e}")

                file_info = FileInfo(
                    name=entry.name,
                    path=entry.path,
                    size=size,
                    modified=str(modified_time),
                    is_directory=entry.is_dir(),
                    save_name=save_name
                )
                files_list.append(file_info)
            except (OSError, PermissionError) as e:
                logger.warning(f"Failed to stat backup {entry.name}: {e}")
                continue

        files_list.sort(key=lambda x: x.modified, reverse=True)

        logger.info(f"Listed backups: path={backups_dir}, count={len(files_list)}")

        return FileListResponse(
            files=files_list,
            total_count=len(files_list),
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
    save_path: str
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
    """
    Restore a save from a backup directory.

    Args:
        restore_request: Restore parameters

    Returns:
        Restore result with details
    """
    try:
        from parsers.savegame_handler import SaveGameHandler, SaveGameError

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

        if not os.path.exists(restore_request.save_path):
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Save directory not found: {restore_request.save_path}"
            )

        handler = SaveGameHandler(restore_request.save_path)
        restore_result = handler.restore_from_backup(
            backup_path=restore_request.backup_path,
            create_pre_restore_backup=restore_request.create_pre_restore_backup
        )

        logger.info(f"Restored backup: from={restore_request.backup_path}, to={restore_request.save_path}")

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
    """
    Create a manual backup of a save directory.

    Args:
        backup_request: Backup creation parameters

    Returns:
        Backup creation result
    """
    try:
        from parsers.savegame_handler import SaveGameHandler, SaveGameError
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
    """
    Delete a backup directory.

    Args:
        delete_request: Delete parameters

    Returns:
        Deletion result
    """
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
