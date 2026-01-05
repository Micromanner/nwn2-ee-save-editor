"""NWN2 save game ZIP handler with format preservation."""
import zipfile
import tempfile
import shutil
import os
import datetime
from typing import Dict, List, Optional, Union, Any
from contextlib import contextmanager
from pathlib import Path
from loguru import logger

try:
    from nwn2_rust import GffParser
except ImportError:
    GffParser = None

from services.core.playerinfo_service import PlayerInfo


class SaveGameError(Exception):
    """Base exception for save game handler errors."""
    pass


class BatchReader:
    """Helper class for batch reading files from an open ZIP."""

    def __init__(self, zip_file: zipfile.ZipFile, validate_func):
        """Initialize batch reader with an open ZIP file."""
        self.zf = zip_file
        self.validate = validate_func

    def read(self, filename: str) -> Optional[bytes]:
        """Read a file from the open ZIP, returns bytes or None if not found."""
        try:
            content = self.zf.read(filename)
            self.validate(filename, content)
            return content
        except KeyError:
            logger.debug(f"File not found in save: {filename}")
            return None
    
    def list_files(self) -> List[str]:
        """Get list of all files in the ZIP."""
        return self.zf.namelist()


class SaveGameHandler:
    """Handles NWN2 save game ZIP files with proper format preservation."""

    # NWN2 saves always use these settings
    NWN2_DATE_TIME = (1980, 0, 0, 0, 0, 0)
    NWN2_CREATE_SYSTEM = 0  # MS-DOS
    NWN2_EXTRACT_VERSION = 20
    
    # File type headers for validation
    FILE_HEADERS = {
        '.bic': b'BIC ',
        '.ros': b'ROS ',
        '.ifo': b'IFO ',
        '.uti': b'UTI ',
        '.utc': b'UTC ',
        '.ute': b'UTE ',
    }
    
    _backup_created_for_saves = set()

    def __init__(self, save_path: Union[str, Path], validate: bool = False, create_load_backup: bool = True):
        """Initialize with path to save game directory or resgff.zip."""
        save_path = Path(save_path)
        
        if save_path.is_dir():
            self.save_dir = str(save_path)
            self.zip_path = str(save_path / 'resgff.zip')
        else:
            self.zip_path = str(save_path)
            self.save_dir = str(save_path.parent)
            
        if not os.path.exists(self.zip_path):
            raise FileNotFoundError(f"resgff.zip not found at {self.zip_path}")
            
        self.validate = validate
        self._temp_files = []

        if create_load_backup and self.save_dir not in SaveGameHandler._backup_created_for_saves:
            try:
                backup_path = self._create_backup()
                SaveGameHandler._backup_created_for_saves.add(self.save_dir)
                logger.info(f"Created load backup: {backup_path}")
            except Exception as e:
                logger.warning(f"Could not create load backup: {e}")
        
        logger.debug(f"Initialized SaveGameHandler for {self.zip_path}")
    
    def __enter__(self):
        """Context manager entry."""
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit - cleanup temp files."""
        self._cleanup_temp_files()
        return False
    
    def _cleanup_temp_files(self):
        """Clean up any temporary files created during operations."""
        for temp_file in self._temp_files:
            try:
                if os.path.exists(temp_file):
                    os.unlink(temp_file)
                    logger.debug(f"Cleaned up temp file: {temp_file}")
            except Exception as e:
                logger.warning(f"Failed to cleanup temp file {temp_file}: {e}")
        self._temp_files.clear()
    
    def _validate_file_content(self, filename: str, content: bytes) -> bool:
        """Validate file content based on file extension."""
        if not self.validate:
            return True
            
        ext = os.path.splitext(filename)[1].lower()
        if ext in self.FILE_HEADERS:
            expected_header = self.FILE_HEADERS[ext]
            if len(content) < len(expected_header):
                raise SaveGameError(f"File {filename} too small to be valid {ext} file")
            if content[:len(expected_header)] != expected_header:
                raise SaveGameError(f"File {filename} has invalid header for {ext} file")
        
        return True
    
    @contextmanager
    def _open_zip_safe(self, mode: str = 'r'):
        """Safely open zip file with error handling."""
        try:
            with zipfile.ZipFile(self.zip_path, mode) as zf:
                yield zf
        except zipfile.BadZipFile as e:
            raise SaveGameError(f"Corrupted save game zip: {e}")
        except PermissionError as e:
            raise SaveGameError(f"Permission denied accessing save game: {e}")
        except Exception as e:
            raise SaveGameError(f"Error accessing save game: {e}")
    
    @contextmanager
    def batch_read_context(self):
        """Context manager for batch reading files from the ZIP."""
        with self._open_zip_safe('r') as zf:
            yield BatchReader(zf, self._validate_file_content)
    
    def extract_file(self, filename: str) -> bytes:
        """Extract a single file from the save game zip."""
        logger.debug(f"Extracting file: {filename}")
        
        with self._open_zip_safe('r') as zf:
            try:
                content = zf.read(filename)
                self._validate_file_content(filename, content)
                return content
            except KeyError:
                logger.warning(f"File not found in save: {filename}")
                raise
    
    def extract_player_data(self) -> bytes:
        """Extract playerlist.ifo which contains the actual player data."""
        return self.extract_file('playerlist.ifo')
    
    def extract_player_bic(self) -> Optional[bytes]:
        """Extract the player.bic file if it exists."""
        try:
            return self.extract_file('player.bic')
        except KeyError:
            logger.info("player.bic not found in save (normal for some saves)")
            return None
    
    def extract_companion(self, companion_name: str) -> bytes:
        """Extract a companion's .ros file."""
        return self.extract_file(f'{companion_name}.ros')
    
    def batch_read_character_files(self) -> Dict[str, bytes]:
        """Batch read all character-related files from the save."""
        result = {}
        
        with self.batch_read_context() as reader:
            # Read playerlist.ifo (required)
            player_data = reader.read('playerlist.ifo')
            if not player_data:
                raise SaveGameError("Could not read playerlist.ifo - save is invalid")
            result['playerlist.ifo'] = player_data
            
            # Read player.bic (optional)
            bic_data = reader.read('player.bic')
            if bic_data:
                result['player.bic'] = bic_data
            
            # Read all companion .ros files
            for filename in reader.list_files():
                if filename.endswith('.ros'):
                    ros_data = reader.read(filename)
                    if ros_data:
                        result[filename] = ros_data
        
        logger.debug(f"Batch read {len(result)} character files")
        return result
    
    def extract_globals_xml(self) -> str:
        """Extract globals.xml from the save directory (outside resgff.zip)."""
        globals_path = os.path.join(self.save_dir, 'globals.xml')
        logger.debug(f"Reading globals.xml from: {globals_path}")

        if not os.path.exists(globals_path):
            raise SaveGameError("globals.xml not found - save is invalid")

        try:
            with open(globals_path, 'r', encoding='utf-8') as f:
                return f.read()
        except Exception as e:
            raise SaveGameError(f"Failed to read globals.xml: {e}")
    
    def extract_current_module(self) -> str:
        """Extract current module name from currentmodule.txt."""
        module_path = os.path.join(self.save_dir, 'currentmodule.txt')
        logger.debug(f"Reading currentmodule.txt from: {module_path}")

        if not os.path.exists(module_path):
            raise SaveGameError("currentmodule.txt not found - save is invalid")

        try:
            with open(module_path, 'r', encoding='utf-8') as f:
                return f.read().strip()
        except Exception as e:
            raise SaveGameError(f"Failed to read currentmodule.txt: {e}")

    def extract_module_ifo(self) -> bytes:
        """Extract module.ifo from the save directory."""
        module_ifo_path = os.path.join(self.save_dir, 'module.ifo')
        logger.debug(f"Reading module.ifo from: {module_ifo_path}")

        if not os.path.exists(module_ifo_path):
            raise SaveGameError("module.ifo not found - save is invalid")

        try:
            with open(module_ifo_path, 'rb') as f:
                return f.read()
        except Exception as e:
            raise SaveGameError(f"Failed to read module.ifo: {e}")

    def update_module_ifo(self, module_ifo_data: bytes) -> None:
        """Write updated module.ifo back to the save directory."""
        module_ifo_path = os.path.join(self.save_dir, 'module.ifo')
        logger.debug(f"Writing module.ifo to: {module_ifo_path}")

        try:
            with open(module_ifo_path, 'wb') as f:
                f.write(module_ifo_data)
            logger.info("Successfully updated module.ifo in save directory")
        except Exception as e:
            raise SaveGameError(f"Failed to write module.ifo: {e}")

    def list_files(self) -> List[str]:
        """List all files in the save game zip."""
        logger.debug("Listing files in save game")
        
        with self._open_zip_safe('r') as zf:
            return zf.namelist()
    
    def list_companions(self) -> List[str]:
        """List all companion names (without .ros extension)."""
        logger.debug("Listing companions in save game")
        
        companions = []
        for filename in self.list_files():
            if filename.endswith('.ros') and not filename.startswith('npc_'):
                companions.append(filename[:-4])  # Remove .ros extension
        
        logger.debug(f"Found {len(companions)} companions")
        return companions
    
    def update_file(self, filename: str, content: bytes):
        """Update a file in the save game zip, preserving NWN2 format."""
        logger.info(f"Updating file: {filename}")
        self._validate_file_content(filename, content)
        temp_fd, temp_path = tempfile.mkstemp(suffix='.zip', dir=self.save_dir)
        os.close(temp_fd)
        self._temp_files.append(temp_path)
        
        try:
            files_updated = set()

            with self._open_zip_safe('r') as old_zip:
                with zipfile.ZipFile(temp_path, 'w', zipfile.ZIP_DEFLATED) as new_zip:
                    for item in old_zip.filelist:
                        if item.filename != filename:
                            new_info = zipfile.ZipInfo(item.filename)
                            new_info.date_time = self.NWN2_DATE_TIME
                            new_info.compress_type = item.compress_type
                            new_info.create_system = self.NWN2_CREATE_SYSTEM
                            new_info.extract_version = self.NWN2_EXTRACT_VERSION
                            new_info.flag_bits = item.flag_bits
                            new_info.internal_attr = item.internal_attr
                            new_info.external_attr = item.external_attr
                            new_zip.writestr(new_info, old_zip.read(item.filename))

                        files_updated.add(item.filename)

                    new_info = zipfile.ZipInfo(filename)
                    new_info.date_time = self.NWN2_DATE_TIME
                    new_info.compress_type = zipfile.ZIP_DEFLATED
                    new_info.create_system = self.NWN2_CREATE_SYSTEM
                    new_info.extract_version = self.NWN2_EXTRACT_VERSION
                    new_zip.writestr(new_info, content)
                    
                    if filename not in files_updated:
                        logger.info(f"Added new file to save: {filename}")

            temp_backup = self.zip_path + '.tmp_backup'
            try:
                shutil.move(self.zip_path, temp_backup)
                shutil.move(temp_path, self.zip_path)
                os.unlink(temp_backup)
            except Exception:
                if os.path.exists(temp_backup) and not os.path.exists(self.zip_path):
                    shutil.move(temp_backup, self.zip_path)
                raise
            
            logger.info(f"Successfully updated {filename}")
            
        except Exception as e:
            if os.path.exists(temp_path):
                try:
                    os.unlink(temp_path)
                except Exception:
                    pass
            logger.error(f"Failed to update {filename}: {e}")
            raise SaveGameError(f"Failed to update {filename}: {e}") from e
    
    def update_player_data(self, content: bytes):
        """Update the playerlist.ifo file"""
        self.update_file('playerlist.ifo', content)
    
    def update_player_bic(self, content: bytes):
        """Update the player.bic file"""
        self.update_file('player.bic', content)
    
    def update_player_complete(self, playerlist_content: bytes, playerbic_content: bytes,
                              base_stats: dict = None, char_summary: dict = None):
        """Update both playerlist.ifo and player.bic together (required for save games)."""
        self.update_file('playerlist.ifo', playerlist_content)
        self.update_file('player.bic', playerbic_content)
        self._sync_playerinfo_bin(base_stats=base_stats, char_summary=char_summary)
    
    def update_companion(self, companion_name: str, content: bytes):
        """Update a companion's .ros file"""
        self.update_file(f'{companion_name}.ros', content)
    
    def _sync_playerinfo_bin(self, base_stats: dict = None, char_summary: dict = None):
        """Synchronize playerinfo.bin with character data from managers."""
        playerinfo_path = os.path.join(self.save_dir, 'playerinfo.bin')

        if not os.path.exists(playerinfo_path):
            raise SaveGameError("playerinfo.bin not found - save is invalid")

        try:
            player_info = PlayerInfo(playerinfo_path)
        except Exception as e:
            raise SaveGameError(f"playerinfo.bin is corrupted: {e}")

        if char_summary:
            player_info.data.first_name = char_summary.get('first_name', '')
            player_info.data.last_name = char_summary.get('last_name', '')
            player_info.data.name = char_summary.get('name', '')
            player_info.data.subrace = char_summary.get('subrace', '')
            align_val = char_summary.get('alignment_string', '')

            if not align_val:
                align_raw = char_summary.get('alignment')
                if isinstance(align_raw, str):
                    align_val = align_raw
                else:
                    align_val = ""

            player_info.data.alignment = align_val
            player_info.data.deity = char_summary.get('deity', '')

            player_info.data.classes = []
            for cls in char_summary.get('classes', []):
                from services.core.playerinfo_service import PlayerClassEntry
                if isinstance(cls, dict):
                    player_info.data.classes.append(PlayerClassEntry(cls['name'], cls['level']))
                elif hasattr(cls, 'get'):
                    player_info.data.classes.append(PlayerClassEntry(cls['name'], cls['level']))
                else:
                    raise SaveGameError(f"Invalid class data format: {type(cls)}")

        if base_stats:
            player_info.data.str = base_stats['str']
            player_info.data.dex = base_stats['dex']
            player_info.data.con = base_stats['con']
            player_info.data.int = base_stats['int']
            player_info.data.wis = base_stats['wis']
            player_info.data.cha = base_stats['cha']

        player_info.save(playerinfo_path)
        logger.info("Successfully synced playerinfo.bin")
    
    def _create_backup(self) -> str:
        """Create a backup of the entire save directory in saves/backups/."""
        save_folder_name = os.path.basename(self.save_dir)
        saves_root = os.path.dirname(self.save_dir)
        backups_root = os.path.join(saves_root, 'backups')
        os.makedirs(backups_root, exist_ok=True)

        timestamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
        backup_dir = os.path.join(backups_root, f"{save_folder_name}_backup_{timestamp}")

        base_backup_dir = backup_dir
        counter = 1
        while os.path.exists(backup_dir):
            backup_dir = f"{base_backup_dir}_{counter}"
            counter += 1

        logger.info(f"Creating backup at: {backup_dir}")

        try:
            shutil.copytree(self.save_dir, backup_dir)

            savename_path = os.path.join(backup_dir, 'savename.txt')
            if os.path.exists(savename_path):
                try:
                    with open(savename_path, 'r', encoding='utf-8', errors='ignore') as f:
                        original_name = f.readline().strip()

                    backup_name = f"Backup of {original_name}"

                    with open(savename_path, 'w', encoding='utf-8') as f:
                        f.write(backup_name)

                    logger.info(f"Updated backup savename.txt: '{backup_name}'")

                except Exception as e:
                    logger.warning(f"Failed to update backup savename.txt: {e}")
            
            logger.info(f"Backup created successfully: {backup_dir}")
            return backup_dir
        except Exception as e:
            logger.error(f"Failed to create backup: {e}")
            raise SaveGameError(f"Failed to create backup: {e}") from e
    
    def get_file_info(self, filename: str) -> Dict[str, any]:
        """Get information about a file in the save."""
        with self._open_zip_safe('r') as zf:
            try:
                info = zf.getinfo(filename)
                return {
                    'filename': info.filename,
                    'file_size': info.file_size,
                    'compress_size': info.compress_size,
                    'date_time': info.date_time,
                    'compress_type': info.compress_type,
                    'create_system': info.create_system,
                    'extract_version': info.extract_version,
                }
            except KeyError:
                logger.warning(f"File not found: {filename}")
                raise
    
    def extract_for_editing(self, temp_dir: Optional[str] = None) -> Dict[str, str]:
        """Extract all files for batch editing to a temporary directory."""
        if temp_dir is None:
            temp_dir = tempfile.mkdtemp(prefix='nwn2_save_')
            logger.info(f"Created temp directory for extraction: {temp_dir}")
        else:
            # Create directory if it doesn't exist
            os.makedirs(temp_dir, exist_ok=True)
        
        logger.info(f"Extracting all files to: {temp_dir}")
        
        extracted = {}
        try:
            with self._open_zip_safe('r') as zf:
                for filename in zf.namelist():
                    extract_path = os.path.join(temp_dir, filename)
                    zf.extract(filename, temp_dir)
                    extracted[filename] = extract_path
                    logger.debug(f"Extracted: {filename}")
            
            logger.info(f"Extracted {len(extracted)} files")
            return extracted
            
        except Exception as e:
            logger.error(f"Failed to extract files: {e}")
            raise SaveGameError(f"Failed to extract files: {e}") from e
    
    def repack_from_directory(self, source_dir: str, backup: bool = True):
        """Repack all files from a directory back into the save zip."""
        logger.info(f"Repacking files from: {source_dir}")

        if not os.path.isdir(source_dir):
            raise SaveGameError(f"Source directory does not exist: {source_dir}")

        if backup:
            self._create_backup()

        try:
            with self._open_zip_safe('r') as old_zip:
                file_order = [item.filename for item in old_zip.filelist]
        except Exception as e:
            raise SaveGameError(f"Failed to read original zip structure: {e}") from e

        temp_fd, temp_path = tempfile.mkstemp(suffix='.zip', dir=self.save_dir)
        os.close(temp_fd)
        self._temp_files.append(temp_path)
        
        try:
            files_added = 0
            files_skipped = []

            with zipfile.ZipFile(temp_path, 'w', zipfile.ZIP_DEFLATED) as new_zip:
                for filename in file_order:
                    file_path = os.path.join(source_dir, filename)
                    if os.path.exists(file_path):
                        info = zipfile.ZipInfo(filename)
                        info.date_time = self.NWN2_DATE_TIME
                        info.compress_type = zipfile.ZIP_DEFLATED
                        info.create_system = self.NWN2_CREATE_SYSTEM
                        info.extract_version = self.NWN2_EXTRACT_VERSION

                        with open(file_path, 'rb') as f:
                            content = f.read()

                        self._validate_file_content(filename, content)
                        new_zip.writestr(info, content)
                        files_added += 1
                        logger.debug(f"Repacked: {filename}")
                    else:
                        files_skipped.append(filename)
                        logger.warning(f"File not found in source directory: {filename}")
            
            if files_skipped:
                logger.warning(f"Skipped {len(files_skipped)} missing files during repack")

            temp_backup = self.zip_path + '.tmp_backup'
            try:
                shutil.move(self.zip_path, temp_backup)
                shutil.move(temp_path, self.zip_path)
                os.unlink(temp_backup)
            except Exception:
                if os.path.exists(temp_backup) and not os.path.exists(self.zip_path):
                    shutil.move(temp_backup, self.zip_path)
                raise
            
            logger.info(f"Successfully repacked {files_added} files")
            
        except Exception as e:
            if os.path.exists(temp_path):
                try:
                    os.unlink(temp_path)
                except Exception:
                    pass
            logger.error(f"Failed to repack files: {e}")
            raise SaveGameError(f"Failed to repack files: {e}") from e
    
    def list_backups(self) -> List[Dict[str, Any]]:
        """List all available backups for this save file."""
        save_folder_name = os.path.basename(self.save_dir)
        saves_root = os.path.dirname(self.save_dir)
        backups_root = os.path.join(saves_root, 'backups')

        backups = []

        if not os.path.exists(backups_root):
            return backups

        backup_prefix = f"{save_folder_name}_backup_"

        try:
            for item in os.listdir(backups_root):
                if item.startswith(backup_prefix) and os.path.isdir(os.path.join(backups_root, item)):
                    backup_path = os.path.join(backups_root, item)
                    timestamp_str = item[len(backup_prefix):]

                    try:
                        backup_time = datetime.datetime.strptime(timestamp_str, "%Y%m%d_%H%M%S")
                    except ValueError:
                        continue

                    backup_save_name = save_folder_name
                    savename_path = os.path.join(backup_path, 'savename.txt')
                    if os.path.exists(savename_path):
                        try:
                            with open(savename_path, 'r', encoding='utf-8', errors='ignore') as f:
                                backup_save_name = f.readline().strip()
                        except Exception:
                            pass

                    backup_size = 0
                    try:
                        for root, dirs, files in os.walk(backup_path):
                            backup_size += sum(os.path.getsize(os.path.join(root, file)) for file in files)
                    except Exception:
                        pass

                    backups.append({
                        'path': backup_path,
                        'folder_name': item,
                        'timestamp': backup_time.isoformat(),
                        'display_name': backup_save_name,
                        'size_bytes': backup_size,
                        'original_save': save_folder_name
                    })

            backups.sort(key=lambda x: x['timestamp'], reverse=True)

        except Exception as e:
            logger.warning(f"Error listing backups: {e}")

        return backups
    
    def restore_from_backup(self, backup_path: str, create_pre_restore_backup: bool = True) -> Dict[str, Any]:
        """Restore save from a backup directory."""
        if not os.path.exists(backup_path):
            raise SaveGameError(f"Backup directory not found: {backup_path}")

        if not os.path.isdir(backup_path):
            raise SaveGameError(f"Backup path is not a directory: {backup_path}")

        logger.info(f"Restoring save from backup: {backup_path}")
        pre_restore_backup = None

        try:
            if create_pre_restore_backup and os.path.exists(self.save_dir):
                pre_restore_backup = self._create_backup()
                logger.info(f"Created pre-restore backup: {pre_restore_backup}")

            if os.path.exists(self.save_dir):
                shutil.rmtree(self.save_dir)

            shutil.copytree(backup_path, self.save_dir)

            savename_path = os.path.join(self.save_dir, 'savename.txt')
            if os.path.exists(savename_path):
                try:
                    with open(savename_path, 'r', encoding='utf-8', errors='ignore') as f:
                        current_name = f.readline().strip()

                    if current_name.startswith("Backup of "):
                        parts = current_name.split(" at ")
                        if len(parts) >= 2:
                            original_name = current_name[10:].split(" at ")[0]
                        else:
                            original_name = current_name[10:]

                        with open(savename_path, 'w', encoding='utf-8') as f:
                            f.write(original_name)

                        logger.info(f"Updated savename.txt: '{original_name}'")
                except Exception as e:
                    logger.warning(f"Failed to update savename.txt: {e}")

            restored_files = []
            for root, dirs, files in os.walk(self.save_dir):
                restored_files.extend(files)

            logger.info(f"Successfully restored {len(restored_files)} files from backup")

            return {
                'success': True,
                'restored_from': backup_path,
                'files_restored': len(restored_files),
                'pre_restore_backup': pre_restore_backup,
                'restore_timestamp': datetime.datetime.now().isoformat()
            }

        except Exception as e:
            logger.error(f"Failed to restore from backup: {e}")

            if pre_restore_backup and os.path.exists(pre_restore_backup):
                try:
                    if os.path.exists(self.save_dir):
                        shutil.rmtree(self.save_dir)
                    shutil.copytree(pre_restore_backup, self.save_dir)
                    logger.info("Restored original save after failed restore")
                except Exception as restore_error:
                    logger.error(f"Failed to restore original save: {restore_error}")

            raise SaveGameError(f"Failed to restore from backup: {e}") from e
    
    def cleanup_old_backups(self, keep_count: int = 10) -> Dict[str, Any]:
        """Clean up old backups, keeping only the most recent ones."""
        backups_root = os.path.join(os.path.dirname(self.save_dir), 'backups')

        if not os.path.exists(backups_root):
            return {'cleaned_up': 0, 'kept': 0, 'errors': []}

        backups = self.list_backups()

        if len(backups) <= keep_count:
            return {'cleaned_up': 0, 'kept': len(backups), 'errors': []}

        backups_to_remove = backups[keep_count:]
        errors = []
        cleaned_up = 0

        for backup in backups_to_remove:
            try:
                shutil.rmtree(backup['path'])
                cleaned_up += 1
                logger.info(f"Removed old backup: {backup['folder_name']}")
            except Exception as e:
                error_msg = f"Failed to remove backup {backup['folder_name']}: {e}"
                errors.append(error_msg)
                logger.warning(error_msg)

        return {'cleaned_up': cleaned_up, 'kept': keep_count, 'errors': errors}

    def read_character_summary(self) -> Dict[str, Any]:
        """Read and parse basic character summary from playerlist.ifo."""
        player_data_bytes = self.extract_player_data()
        if not player_data_bytes:
            raise SaveGameError("Could not read playerlist.ifo")

        if GffParser is None:
            raise SaveGameError("GffParser not available (rust module missing)")

        player_data = GffParser.from_bytes(player_data_bytes).to_dict()
        mod_player_list = player_data.get('Mod_PlayerList', [])

        if not mod_player_list:
            raise SaveGameError("No player data found in save game")

        character_data = mod_player_list[0]

        first_name = character_data.get('FirstName', {}).get('value', '')
        last_name = character_data.get('LastName', {}).get('value', '')
        full_name = f"{first_name} {last_name}".strip()

        return {
            'name': full_name,
            'first_name': first_name,
            'last_name': last_name,
        }

    @staticmethod
    def infer_save_path_from_backup(backup_path_str: str) -> str:
        """Infer the original save path from a backup path."""
        try:
            backup_path = Path(backup_path_str)

            if 'backups' not in str(backup_path):
                raise SaveGameError("Backup path does not appear to be in a valid backups directory")

            backup_name = backup_path.name
            save_name = backup_name

            if '_backup_' in backup_name:
                save_name = backup_name.split('_backup_')[0]
            elif backup_path.is_file() and backup_name.endswith('.cam'):
                save_name = backup_name

            current = backup_path
            while current.parent and current.name != 'backups' and 'backups' in str(current.parent):
                current = current.parent

            saves_dir = None
            if current.name == 'backups':
                saves_dir = current.parent
            elif current.parent.name == 'backups':
                saves_dir = current.parent.parent
            else:
                parts = list(backup_path.parts)
                try:
                    reversed_parts = parts[::-1]
                    if 'backups' in reversed_parts:
                        idx = len(parts) - 1 - reversed_parts.index('backups')
                        saves_dir = Path(*parts[:idx])
                except ValueError:
                    pass

            if not saves_dir:
                saves_dir = backup_path.parent.parent

            inferred_path = saves_dir / save_name
            return str(inferred_path)

        except Exception as e:
            raise SaveGameError(f"Failed to infer save path from backup: {e}") from e