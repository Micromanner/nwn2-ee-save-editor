"""
NWN2 Save Game Handler
Handles reading and writing save game ZIP files while preserving exact format
"""
import zipfile
import tempfile
import shutil
import os
import logging
from typing import Dict, List, Optional, BinaryIO, Union, Any
from io import BytesIO
from contextlib import contextmanager
from pathlib import Path
import datetime

try:
    from nwn2_rust import GffParser
except ImportError:
    GffParser = None

from services.playerinfo_service import PlayerInfo

logger = logging.getLogger(__name__)


class SaveGameError(Exception):
    """Base exception for save game handler errors."""
    pass


class BatchReader:
    """Helper class for batch reading files from an open ZIP."""
    
    def __init__(self, zip_file: zipfile.ZipFile, validate_func):
        """
        Initialize batch reader with an open ZIP file.
        
        Args:
            zip_file: Open ZipFile object
            validate_func: Function to validate file content
        """
        self.zf = zip_file
        self.validate = validate_func
        
    def read(self, filename: str) -> Optional[bytes]:
        """
        Read a file from the open ZIP.
        
        Args:
            filename: Name of file to read
            
        Returns:
            File contents as bytes, or None if not found
            
        Raises:
            SaveGameError: If read fails (not for missing file)
        """
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
    
    # Class variable to track which saves have already been backed up this session
    _backup_created_for_saves = set()
    
    def __init__(self, save_path: Union[str, Path], validate: bool = False, create_load_backup: bool = True):
        """
        Initialize with path to save game directory or resgff.zip
        
        Args:
            save_path: Path to save directory or resgff.zip file
            validate: Whether to validate file formats on operations
            create_load_backup: Whether to create backup when loading save (recommended)
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
        
        # Create backup on load for undo/restore functionality (only once per save)
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
    
    @contextmanager
    def batch_read_context(self):
        """
        Context manager for batch reading files from the ZIP.
        Keeps the ZIP file open for multiple read operations.
        
        Usage:
            with save_handler.batch_read_context() as reader:
                player_data = reader.read('playerlist.ifo')
                bic_data = reader.read('player.bic')
                
        Yields:
            BatchReader: Object with read() and list_files() methods
        """
        with self._open_zip_safe('r') as zf:
            yield BatchReader(zf, self._validate_file_content)
    
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
    
    def batch_read_character_files(self) -> Dict[str, bytes]:
        """
        Batch read all character-related files from the save.
        Opens the ZIP once and reads all needed files.
        
        Returns:
            Dictionary mapping filenames to file contents:
            - 'playerlist.ifo': Always present
            - 'player.bic': Present if exists
            - '*.ros': All companion files
            
        Raises:
            SaveGameError: If playerlist.ifo cannot be read
        """
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

    def extract_module_ifo(self) -> Optional[bytes]:
        """
        Extract module.ifo from the save directory.
        This contains the current state of module variables (VarTable).

        Returns:
            module.ifo contents as bytes, or None if file doesn't exist

        Raises:
            SaveGameError: If file exists but cannot be read
        """
        module_ifo_path = os.path.join(self.save_dir, 'module.ifo')
        logger.debug(f"Reading module.ifo from: {module_ifo_path}")

        if not os.path.exists(module_ifo_path):
            logger.debug("module.ifo not found in save directory")
            return None

        try:
            with open(module_ifo_path, 'rb') as f:
                return f.read()
        except Exception as e:
            raise SaveGameError(f"Failed to read module.ifo: {e}")

    def update_module_ifo(self, module_ifo_data: bytes) -> None:
        """
        Write updated module.ifo back to the save directory.
        This updates module variables (VarTable) in the save file.

        Args:
            module_ifo_data: Updated module.ifo contents as bytes

        Raises:
            SaveGameError: If write fails
        """
        module_ifo_path = os.path.join(self.save_dir, 'module.ifo')
        logger.debug(f"Writing module.ifo to: {module_ifo_path}")

        try:
            with open(module_ifo_path, 'wb') as f:
                f.write(module_ifo_data)
            logger.info("Successfully updated module.ifo in save directory")
        except Exception as e:
            raise SaveGameError(f"Failed to write module.ifo: {e}")

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
    
    def update_file(self, filename: str, content: bytes):
        """
        Update a file in the save game zip, preserving NWN2 format.
        
        This operation is atomic - either the entire update succeeds or the
        original file remains unchanged.
        
        Args:
            filename: Name of file to update
            content: New file content
            
        Raises:
            SaveGameError: If update fails
        """
        logger.info(f"Updating file: {filename}")
        
        # Validate content if enabled
        self._validate_file_content(filename, content)
        
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
    
    def update_player_data(self, content: bytes):
        """Update the playerlist.ifo file"""
        self.update_file('playerlist.ifo', content)
    
    def update_player_bic(self, content: bytes):
        """Update the player.bic file"""
        self.update_file('player.bic', content)
    
    def update_player_complete(self, playerlist_content: bytes, playerbic_content: bytes, 
                              base_stats: dict = None, char_summary: dict = None):
        """Update both playerlist.ifo and player.bic together (required for save games)"""
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
        
        # Update both files in resgff.zip
        self.update_file('playerlist.ifo', playerlist_content)
        self.update_file('player.bic', playerbic_content)
        
        # Sync playerinfo.bin (game engine requirement for save loading)
        self._sync_playerinfo_bin(base_stats=base_stats, char_summary=char_summary)
    
    def update_companion(self, companion_name: str, content: bytes):
        """Update a companion's .ros file"""
        self.update_file(f'{companion_name}.ros', content)
    
    def _sync_playerinfo_bin(self, base_stats: dict = None, char_summary: dict = None):
        """Synchronize playerinfo.bin with character data from managers."""
        playerinfo_path = os.path.join(self.save_dir, 'playerinfo.bin')
        
        try:
            if os.path.exists(playerinfo_path):
                try:
                    player_info = PlayerInfo(playerinfo_path)
                    logger.debug("Loaded existing playerinfo.bin for update")
                except Exception as e:
                    logger.warning(f"Existing playerinfo.bin is corrupted ({e}), creating fresh file")
                    player_info = PlayerInfo()
            else:
                player_info = PlayerInfo()
                logger.debug("Creating new playerinfo.bin")
            
            if char_summary:
                player_info.data.first_name = char_summary.get('first_name', '')
                player_info.data.last_name = char_summary.get('last_name', '')
                player_info.data.name = char_summary.get('name', '')
                player_info.data.subrace = char_summary.get('subrace', '')
                # specific handling for alignment to ensure string
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
                    from services.playerinfo_service import PlayerClassEntry
                    if isinstance(cls, dict):
                        player_info.data.classes.append(PlayerClassEntry(cls.get('name', ''), cls.get('level', 1)))
                    elif hasattr(cls, 'get'): # Duck typing for other dict-likes
                        player_info.data.classes.append(PlayerClassEntry(cls.get('name', ''), cls.get('level', 1)))
                    else:
                         # Fallback for strings or other types
                        player_info.data.classes.append(PlayerClassEntry(str(cls), 1))
            
            if base_stats:
                player_info.data.str = base_stats.get('str', 10)
                player_info.data.dex = base_stats.get('dex', 10)
                player_info.data.con = base_stats.get('con', 10)
                player_info.data.int = base_stats.get('int', 10)
                player_info.data.wis = base_stats.get('wis', 10)
                player_info.data.cha = base_stats.get('cha', 10)
            
            player_info.save(playerinfo_path)
            logger.info("Successfully synced playerinfo.bin")
            
        except Exception as e:
            logger.error(f"Failed to sync playerinfo.bin: {e}")
    
    def _create_backup(self) -> str:
        """
        Create a backup of the entire save directory in saves/backups/ folder.
        
        Returns:
            Path to backup directory
            
        Raises:
            SaveGameError: If backup creation fails
        """
        import datetime
        
        # Get save folder name (e.g. "MyCharacter")
        save_folder_name = os.path.basename(self.save_dir)
        
        # Create backups directory in the parent saves folder
        saves_root = os.path.dirname(self.save_dir)
        backups_root = os.path.join(saves_root, 'backups')
        
        # Create backups directory if it doesn't exist
        os.makedirs(backups_root, exist_ok=True)
        
        # Create backup directory name with save name and timestamp
        timestamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
        backup_dir = os.path.join(backups_root, f"{save_folder_name}_backup_{timestamp}")
        
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
                    
                    # Create backup save name (single line only, as NWN2 only reads first line)
                    backup_name = f"Backup of {original_name}"
                    
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
    
    def list_backups(self) -> List[Dict[str, Any]]:
        """
        List all available backups for this save file.
        
        Returns:
            List of backup info dictionaries with path, timestamp, etc.
        """
        save_folder_name = os.path.basename(self.save_dir)
        saves_root = os.path.dirname(self.save_dir)
        backups_root = os.path.join(saves_root, 'backups')
        
        backups = []
        
        if not os.path.exists(backups_root):
            return backups
        
        # Find backups for this save file
        backup_prefix = f"{save_folder_name}_backup_"
        
        try:
            for item in os.listdir(backups_root):
                if item.startswith(backup_prefix) and os.path.isdir(os.path.join(backups_root, item)):
                    backup_path = os.path.join(backups_root, item)
                    
                    # Extract timestamp from folder name
                    timestamp_str = item[len(backup_prefix):]
                    
                    # Get backup creation time
                    try:
                        backup_time = datetime.datetime.strptime(timestamp_str, "%Y%m%d_%H%M%S")
                    except ValueError:
                        # Skip if timestamp format is invalid
                        continue
                    
                    # Read backup save name if available
                    backup_save_name = save_folder_name
                    savename_path = os.path.join(backup_path, 'savename.txt')
                    if os.path.exists(savename_path):
                        try:
                            with open(savename_path, 'r', encoding='utf-8', errors='ignore') as f:
                                backup_save_name = f.readline().strip()
                        except Exception:
                            pass
                    
                    # Get backup size
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
            
            # Sort by timestamp (newest first)
            backups.sort(key=lambda x: x['timestamp'], reverse=True)
            
        except Exception as e:
            logger.warning(f"Error listing backups: {e}")
        
        return backups
    
    def restore_from_backup(self, backup_path: str, create_pre_restore_backup: bool = True) -> Dict[str, Any]:
        """
        Restore save from a backup directory.
        
        Args:
            backup_path: Full path to backup directory
            create_pre_restore_backup: Whether to backup current state before restore
            
        Returns:
            Dict with restore results
            
        Raises:
            SaveGameError: If restore fails
        """
        import datetime
        
        if not os.path.exists(backup_path):
            raise SaveGameError(f"Backup directory not found: {backup_path}")
        
        if not os.path.isdir(backup_path):
            raise SaveGameError(f"Backup path is not a directory: {backup_path}")
        
        logger.info(f"Restoring save from backup: {backup_path}")
        
        pre_restore_backup = None
        
        try:
            # Create pre-restore backup if requested
            if create_pre_restore_backup and os.path.exists(self.save_dir):
                pre_restore_backup = self._create_backup()
                logger.info(f"Created pre-restore backup: {pre_restore_backup}")
            
            # Remove current save directory if it exists
            if os.path.exists(self.save_dir):
                shutil.rmtree(self.save_dir)
            
            # Copy backup to save location
            shutil.copytree(backup_path, self.save_dir)
            
            # Update savename.txt to remove "Backup of" prefix if present
            savename_path = os.path.join(self.save_dir, 'savename.txt')
            if os.path.exists(savename_path):
                try:
                    with open(savename_path, 'r', encoding='utf-8', errors='ignore') as f:
                        current_name = f.readline().strip()
                    
                    # Remove backup prefix if present
                    if current_name.startswith("Backup of "):
                        # Extract original name (remove "Backup of X at Y")
                        parts = current_name.split(" at ")
                        if len(parts) >= 2:
                            original_name = current_name[10:].split(" at ")[0]  # Remove "Backup of "
                        else:
                            original_name = current_name[10:]  # Just remove "Backup of "
                        
                        with open(savename_path, 'w', encoding='utf-8') as f:
                            f.write(original_name)
                        
                        logger.info(f"Updated savename.txt: '{original_name}'")
                except Exception as e:
                    logger.warning(f"Failed to update savename.txt: {e}")
            
            # Count restored files
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
            
            # Try to restore pre-restore backup if something went wrong
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
        """
        Clean up old backups, keeping only the most recent ones.
        
        Args:
            keep_count: Number of recent backups to keep per save file
            
        Returns:
            Dict with cleanup results
        """
        save_folder_name = os.path.basename(self.save_dir)
        saves_root = os.path.dirname(self.save_dir)
        backups_root = os.path.join(saves_root, 'backups')
        
        if not os.path.exists(backups_root):
            return {'cleaned_up': 0, 'kept': 0, 'errors': []}
        
        backups = self.list_backups()
        
        if len(backups) <= keep_count:
            return {'cleaned_up': 0, 'kept': len(backups), 'errors': []}
        
        # Keep the most recent backups, remove the rest
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
        
        return {
            'cleaned_up': cleaned_up,
            'kept': len(backups) - len(backups_to_remove),
            'errors': errors
        }