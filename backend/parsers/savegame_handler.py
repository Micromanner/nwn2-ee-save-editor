"""
NWN2 Save Game Handler
Handles reading and writing save game ZIP files while preserving exact format
"""
import zipfile
import tempfile
import shutil
import os
import logging
from typing import Dict, List, Optional, BinaryIO, Union
from io import BytesIO
from contextlib import contextmanager
from pathlib import Path

logger = logging.getLogger(__name__)


class SaveGameError(Exception):
    """Base exception for save game handler errors."""
    pass


class SaveGameHandler:
    """Handles NWN2 save game ZIP files with proper format preservation"""
    
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
    
    def __init__(self, save_path: Union[str, Path], validate: bool = False):
        """
        Initialize with path to save game directory or resgff.zip
        
        Args:
            save_path: Path to save directory or resgff.zip file
            validate: Whether to validate file formats on operations
        """
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
        self._temp_files = []  # Track temp files for cleanup
        
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
        """
        Validate file content based on file extension.
        
        Args:
            filename: Name of the file
            content: File content to validate
            
        Returns:
            True if valid or validation disabled, raises exception if invalid
        """
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
    
    def extract_file(self, filename: str) -> bytes:
        """
        Extract a single file from the save game zip
        
        Args:
            filename: Name of file to extract (e.g., 'playerlist.ifo')
            
        Returns:
            File contents as bytes
            
        Raises:
            SaveGameError: If file extraction fails
            KeyError: If file not found in zip
        """
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
        """
        Extract playerlist.ifo which contains the actual player data.
        
        Returns:
            playerlist.ifo contents
            
        Raises:
            SaveGameError: If extraction fails
            KeyError: If playerlist.ifo not found
        """
        return self.extract_file('playerlist.ifo')
    
    def extract_player_bic(self) -> Optional[bytes]:
        """
        Extract the player.bic file if it exists.
        
        Returns:
            player.bic contents or None if not found
            
        Raises:
            SaveGameError: If extraction fails (not for missing file)
        """
        try:
            return self.extract_file('player.bic')
        except (KeyError, SaveGameError) as e:
            # Check if it's just a missing file vs other error
            if isinstance(e, SaveGameError) and "There is no item named 'player.bic'" in str(e):
                logger.info("player.bic not found in save (this is normal for some saves)")
                return None
            elif isinstance(e, KeyError):
                logger.info("player.bic not found in save (this is normal for some saves)")
                return None
            else:
                # Re-raise if it's a different error
                raise
    
    def extract_companion(self, companion_name: str) -> bytes:
        """
        Extract a companion's .ros file
        
        Args:
            companion_name: Name without extension (e.g., 'khelgar')
            
        Returns:
            Companion file contents
        """
        return self.extract_file(f'{companion_name}.ros')
    
    def extract_globals_xml(self) -> Optional[str]:
        """
        Extract globals.xml from the save directory (outside resgff.zip).
        This file contains quest states, global variables, and story progress.
        
        Returns:
            XML content as string, or None if file doesn't exist
            
        Raises:
            SaveGameError: If file exists but cannot be read
        """
        globals_path = os.path.join(self.save_dir, 'globals.xml')
        logger.debug(f"Reading globals.xml from: {globals_path}")
        
        if not os.path.exists(globals_path):
            logger.debug("globals.xml not found")
            return None
            
        try:
            with open(globals_path, 'r', encoding='utf-8') as f:
                return f.read()
        except Exception as e:
            raise SaveGameError(f"Failed to read globals.xml: {e}")
    
    def extract_current_module(self) -> Optional[str]:
        """
        Extract current module name from currentmodule.txt.
        
        Returns:
            Module name as string, or None if file doesn't exist
            
        Raises:
            SaveGameError: If file exists but cannot be read
        """
        module_path = os.path.join(self.save_dir, 'currentmodule.txt')
        logger.debug(f"Reading currentmodule.txt from: {module_path}")
        
        if not os.path.exists(module_path):
            logger.debug("currentmodule.txt not found")
            return None
            
        try:
            with open(module_path, 'r', encoding='utf-8') as f:
                return f.read().strip()
        except Exception as e:
            raise SaveGameError(f"Failed to read currentmodule.txt: {e}")
    
    def list_files(self) -> List[str]:
        """
        List all files in the save game zip.
        
        Returns:
            List of filenames in the zip
            
        Raises:
            SaveGameError: If listing fails
        """
        logger.debug("Listing files in save game")
        
        with self._open_zip_safe('r') as zf:
            return zf.namelist()
    
    def list_companions(self) -> List[str]:
        """
        List all companion names (without .ros extension).
        
        Returns:
            List of companion names
            
        Raises:
            SaveGameError: If listing fails
        """
        logger.debug("Listing companions in save game")
        
        companions = []
        for filename in self.list_files():
            if filename.endswith('.ros') and not filename.startswith('npc_'):
                companions.append(filename[:-4])  # Remove .ros extension
        
        logger.debug(f"Found {len(companions)} companions")
        return companions
    
    def update_file(self, filename: str, content: bytes, backup: bool = True):
        """
        Update a file in the save game zip, preserving NWN2 format.
        
        This operation is atomic - either the entire update succeeds or the
        original file remains unchanged.
        
        Args:
            filename: Name of file to update
            content: New file content
            backup: Whether to create a backup first
            
        Raises:
            SaveGameError: If update fails
        """
        logger.info(f"Updating file: {filename} (backup={backup})")
        
        # Validate content if enabled
        self._validate_file_content(filename, content)
        
        if backup:
            self._create_backup()
        
        # Create temporary file for new zip
        temp_fd, temp_path = tempfile.mkstemp(suffix='.zip', dir=self.save_dir)
        os.close(temp_fd)
        self._temp_files.append(temp_path)
        
        try:
            # Track which files we've processed
            files_updated = set()
            
            # Open original and new zip files
            with self._open_zip_safe('r') as old_zip:
                with zipfile.ZipFile(temp_path, 'w', zipfile.ZIP_DEFLATED) as new_zip:
                    # Copy all files except the one we're updating
                    for item in old_zip.filelist:
                        if item.filename != filename:
                            # Preserve all original metadata
                            new_info = zipfile.ZipInfo(item.filename)
                            new_info.date_time = self.NWN2_DATE_TIME
                            new_info.compress_type = item.compress_type
                            new_info.create_system = self.NWN2_CREATE_SYSTEM
                            new_info.extract_version = self.NWN2_EXTRACT_VERSION
                            new_info.flag_bits = item.flag_bits
                            new_info.internal_attr = item.internal_attr
                            new_info.external_attr = item.external_attr
                            
                            # Copy file content
                            new_zip.writestr(new_info, old_zip.read(item.filename))
                        
                        files_updated.add(item.filename)
                    
                    # Add the updated/new file
                    new_info = zipfile.ZipInfo(filename)
                    new_info.date_time = self.NWN2_DATE_TIME
                    new_info.compress_type = zipfile.ZIP_DEFLATED
                    new_info.create_system = self.NWN2_CREATE_SYSTEM
                    new_info.extract_version = self.NWN2_EXTRACT_VERSION
                    new_zip.writestr(new_info, content)
                    
                    if filename not in files_updated:
                        logger.info(f"Added new file to save: {filename}")
            
            # Atomic replace - rename is atomic on most filesystems
            temp_backup = self.zip_path + '.tmp_backup'
            try:
                # Move original to temp backup
                shutil.move(self.zip_path, temp_backup)
                # Move new file to original location
                shutil.move(temp_path, self.zip_path)
                # Remove temp backup
                os.unlink(temp_backup)
            except Exception:
                # Try to restore original if something went wrong
                if os.path.exists(temp_backup) and not os.path.exists(self.zip_path):
                    shutil.move(temp_backup, self.zip_path)
                raise
            
            logger.info(f"Successfully updated {filename}")
            
        except Exception as e:
            # Clean up temp file if something went wrong
            if os.path.exists(temp_path):
                try:
                    os.unlink(temp_path)
                except:
                    pass
            
            logger.error(f"Failed to update {filename}: {e}")
            raise SaveGameError(f"Failed to update {filename}: {e}") from e
    
    def update_player_data(self, content: bytes, backup: bool = True):
        """Update the playerlist.ifo file"""
        self.update_file('playerlist.ifo', content, backup)
    
    def update_player_bic(self, content: bytes, backup: bool = True):
        """Update the player.bic file"""
        self.update_file('player.bic', content, backup)
    
    def update_player_complete(self, playerlist_content: bytes, playerbic_content: bytes, backup: bool = True):
        """Update both playerlist.ifo and player.bic together (required for save games)"""
        if backup:
            self._create_backup()
        
        # TODO: Sync Module Data fields between IFO and globals.xml
        # The following fields in playerlist.ifo need to be synchronized with globals.xml:
        # 1. Module Data (NEEDS SYNC):
        #    - Mod_ModuleList: List of visited modules
        #    - Mod_ModuleList[0].Mod_ID: Module identifier
        #    - Mod_ModuleList[0].Mod_MapDataList: Explored areas
        #    These must be consistent with module progress stored in globals.xml
        
        # TODO: Sync Reputation fields between IFO and globals.xml
        # 2. Reputation System (NEEDS SYNC):
        #    - ReputationList: Faction standings (8 factions in sample)
        #    - ReputationList[0].Amount: Faction reputation values
        #    - PersonalRepList: Personal reputation modifiers
        #    These must be consistent with faction data in globals.xml
        
        # TODO: Check if VarTable needs sync based on variable scope
        # 3. Variables (CONDITIONAL SYNC):
        #    - VarTable[0].Value: Variable storage
        #    May need sync if these are global variables (need to check scope)
        
        # NOTE: The following DO NOT need sync (character-specific):
        # - EffectList: Active effects on character (10 fields)
        # - ActionList: Queued actions (2 fields)
        # - Mod_LastName: Character's last name (1 field)
        
        # Update both files without individual backups
        self.update_file('playerlist.ifo', playerlist_content, backup=False)
        self.update_file('player.bic', playerbic_content, backup=False)
    
    def update_companion(self, companion_name: str, content: bytes, backup: bool = True):
        """Update a companion's .ros file"""
        self.update_file(f'{companion_name}.ros', content, backup)
    
    def _create_backup(self) -> str:
        """
        Create a backup of the entire save directory.
        
        Returns:
            Path to backup directory
            
        Raises:
            SaveGameError: If backup creation fails
        """
        import datetime
        
        # Create backup directory name with timestamp
        timestamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
        backup_dir = f"{self.save_dir}_backup_{timestamp}"
        
        logger.info(f"Creating backup at: {backup_dir}")
        
        try:
            # Copy entire save directory
            shutil.copytree(self.save_dir, backup_dir)
            
            # Update savename.txt to indicate this is a backup
            savename_path = os.path.join(backup_dir, 'savename.txt')
            if os.path.exists(savename_path):
                try:
                    # Read original save name
                    with open(savename_path, 'r', encoding='utf-8', errors='ignore') as f:
                        original_name = f.readline().strip()
                    
                    # Format timestamp for display
                    display_timestamp = datetime.datetime.now().strftime("%m/%d/%Y %H:%M")
                    
                    # Create backup save name (single line only, as NWN2 only reads first line)
                    backup_name = f"Backup of {original_name} at {display_timestamp}"
                    
                    # Write updated savename.txt
                    with open(savename_path, 'w', encoding='utf-8') as f:
                        f.write(backup_name)
                    
                    logger.info(f"Updated backup savename.txt: '{backup_name}'")
                    
                except Exception as e:
                    logger.warning(f"Failed to update backup savename.txt: {e}")
                    # Don't fail the backup creation for this
            
            logger.info(f"Backup created successfully: {backup_dir}")
            return backup_dir
        except Exception as e:
            logger.error(f"Failed to create backup: {e}")
            raise SaveGameError(f"Failed to create backup: {e}") from e
    
    def get_file_info(self, filename: str) -> Dict[str, any]:
        """
        Get information about a file in the save.
        
        Args:
            filename: Name of file to get info for
            
        Returns:
            Dict with file information (size, compressed_size, etc.)
            
        Raises:
            KeyError: If file not found
            SaveGameError: If operation fails
        """
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
        """
        Extract all files for batch editing.
        
        Args:
            temp_dir: Directory to extract to (creates temp if None)
            
        Returns:
            Dict mapping filenames to extracted paths
            
        Raises:
            SaveGameError: If extraction fails
        """
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
        """
        Repack all files from a directory back into the save zip.
        
        This preserves the original file order and NWN2 zip format.
        
        Args:
            source_dir: Directory containing the edited files
            backup: Whether to backup original first
            
        Raises:
            SaveGameError: If repacking fails
        """
        logger.info(f"Repacking files from: {source_dir}")
        
        if not os.path.isdir(source_dir):
            raise SaveGameError(f"Source directory does not exist: {source_dir}")
        
        if backup:
            self._create_backup()
        
        # Get list of files from original zip to preserve order
        try:
            with self._open_zip_safe('r') as old_zip:
                file_order = [item.filename for item in old_zip.filelist]
        except Exception as e:
            raise SaveGameError(f"Failed to read original zip structure: {e}") from e
        
        # Create new zip
        temp_fd, temp_path = tempfile.mkstemp(suffix='.zip', dir=self.save_dir)
        os.close(temp_fd)
        self._temp_files.append(temp_path)
        
        try:
            files_added = 0
            files_skipped = []
            
            with zipfile.ZipFile(temp_path, 'w', zipfile.ZIP_DEFLATED) as new_zip:
                # Add files in original order
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
                            
                        # Validate content if enabled
                        self._validate_file_content(filename, content)
                        
                        new_zip.writestr(info, content)
                        files_added += 1
                        logger.debug(f"Repacked: {filename}")
                    else:
                        files_skipped.append(filename)
                        logger.warning(f"File not found in source directory: {filename}")
            
            if files_skipped:
                logger.warning(f"Skipped {len(files_skipped)} missing files during repack")
            
            # Atomic replace
            temp_backup = self.zip_path + '.tmp_backup'
            try:
                shutil.move(self.zip_path, temp_backup)
                shutil.move(temp_path, self.zip_path)
                os.unlink(temp_backup)
            except Exception:
                # Try to restore original
                if os.path.exists(temp_backup) and not os.path.exists(self.zip_path):
                    shutil.move(temp_backup, self.zip_path)
                raise
            
            logger.info(f"Successfully repacked {files_added} files")
            
        except Exception as e:
            # Clean up temp file
            if os.path.exists(temp_path):
                try:
                    os.unlink(temp_path)
                except:
                    pass
            
            logger.error(f"Failed to repack files: {e}")
            raise SaveGameError(f"Failed to repack files: {e}") from e