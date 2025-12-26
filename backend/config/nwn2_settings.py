"""
NWN2 Installation and Path Configuration (Rust-Powered)

High-performance path discovery using Rust extensions for faster
NWN2 installation detection with Steam/GOG categorization and
detailed performance profiling.
"""
import os
import json
from pathlib import Path
from typing import Optional, List, Dict, Any
import platform
from dotenv import load_dotenv

# Load environment variables
load_dotenv()

from utils.paths import get_writable_dir

# Centralized writable paths
BASE_WRITABLE_DIR = get_writable_dir("") # Base AppData folder
USER_SETTINGS_PATH = BASE_WRITABLE_DIR / "settings.json"

# Import Rust path discovery
try:
    from nwn2_rust import (
        discover_nwn2_paths_rust as discover_nwn2_paths,
        profile_path_discovery_rust as profile_path_discovery,
    )
    RUST_AVAILABLE = True
except ImportError as e:
    raise RuntimeError(f"Rust extensions are required for NWN2 path discovery: {e}. "
                      f"Please ensure nwn2_rust is properly installed.")


class NWN2PathFinder:
    """High-performance NWN2 path detection using Rust implementation"""

    @classmethod
    def auto_discover_nwn2_paths(cls, search_paths: Optional[List[Path]] = None) -> List[Path]:
        """
        Auto-discover NWN2 installations using Rust implementation.

        Args:
            search_paths: Optional list of paths to search. If None, searches common locations.

        Returns:
            List of potential NWN2 installation paths.
        """
        result = discover_nwn2_paths(search_paths)
        return [Path(path) for path in result.nwn2_paths]

    @classmethod
    def find_nwn2_installation(cls) -> Optional[Path]:
        """Try to auto-detect NWN2 installation using Rust-powered search"""
        discovered = cls.auto_discover_nwn2_paths()
        # Prioritize non-Documents folders (actual game installations)
        for path in discovered:
            if 'Documents' not in str(path) and 'My Documents' not in str(path):
                return path
        # Fallback to first discovered path
        return discovered[0] if discovered else None
    
    @classmethod
    def find_steam_installation(cls) -> Optional[Path]:
        """Find Steam-based NWN2 installation"""
        result = discover_nwn2_paths()
        return Path(result.steam_paths[0]) if result.steam_paths else None
    
    @classmethod
    def find_gog_installation(cls) -> Optional[Path]:
        """Find GOG-based NWN2 installation"""
        result = discover_nwn2_paths()
        return Path(result.gog_paths[0]) if result.gog_paths else None
    
    @classmethod
    def get_discovery_timing(cls) -> Dict[str, Any]:
        """Get detailed timing information about path discovery"""
        result = discover_nwn2_paths()
        return {
            'total_time_ms': result.total_time_ms,
            'total_time_seconds': result.total_time_seconds,
            'timing_breakdown': [
                {
                    'operation': timing.operation,
                    'duration_ms': timing.duration_ms,
                    'paths_checked': timing.paths_checked,
                    'paths_found': timing.paths_found
                }
                for timing in result.timing_breakdown
            ]
        }
    
    @classmethod
    def find_documents_folder(cls) -> Optional[Path]:
        """Try to auto-detect NWN2 documents folder using standard locations"""
        # Windows native
        if platform.system() == 'Windows':
            docs_path = Path(os.environ.get('USERPROFILE', '')) / 'Documents' / 'Neverwinter Nights 2'
            if docs_path.exists():
                return docs_path
            # Also try My Documents
            my_docs_path = Path(os.environ.get('USERPROFILE', '')) / 'My Documents' / 'Neverwinter Nights 2'
            if my_docs_path.exists():
                return my_docs_path
        
        # Try Linux paths
        else:
            docs_candidates = [
                Path.home() / 'Documents' / 'Neverwinter Nights 2',
                Path.home() / '.local' / 'share' / 'Neverwinter Nights 2',
            ]
            for candidate in docs_candidates:
                if candidate.exists():
                    return candidate
        
        return None
    
    @classmethod
    def find_steam_workshop(cls) -> Optional[Path]:
        """Try to auto-detect Steam workshop folder for NWN2"""
        system = platform.system()
        search_paths = []
        
        if system == 'Windows':
            program_files = Path(os.environ.get('ProgramFiles', 'C:/Program Files'))
            program_files_x86 = Path(os.environ.get('ProgramFiles(x86)', 'C:/Program Files (x86)'))
            search_paths = [
                program_files / 'Steam' / 'steamapps' / 'workshop' / 'content' / '2738630',
                program_files_x86 / 'Steam' / 'steamapps' / 'workshop' / 'content' / '2738630',
            ]
        else:  # Linux/macOS
            home = Path.home()
            search_paths = [
                home / '.steam' / 'steam' / 'steamapps' / 'workshop' / 'content' / '2738630',
            ]
        
        for path in search_paths:
            if path.exists() and path.is_dir():
                return path
        
        return None

# Path management class for centralized access
class NWN2Paths:
    """
    Rust-Powered Centralized Path Management for NWN2 Directories.

    This class provides high-performance access to all critical NWN2 file paths
    using Rust extensions for fast path discovery. It automatically finds the 
    game installation by checking (in order):
    1. 'NWN2_GAME_FOLDER' environment variable.
    2. A 'settings.json' configuration file.
    3. Rust-powered auto-discovery of common installation locations.
    4. A fallback default directory.
    
    Features:
    - Steam/GOG installation detection and categorization
    - Performance timing and profiling
    - Enhanced Edition detection
    - Multi-platform support (Windows, Linux, macOS)
    """
    
    def __init__(self):
        self._game_folder: Optional[Path] = None
        self._documents_folder: Optional[Path] = None
        self._steam_workshop_folder: Optional[Path] = None
        self._custom_override_folders: List[Path] = []
        self._custom_module_folders: List[Path] = []
        self._custom_hak_folders: List[Path] = []
        self._load_config()

    def _load_config(self):
        """Load configuration from various sources."""
        # First check environment variables
        env_game = os.getenv('NWN2_GAME_FOLDER')
        if env_game and Path(env_game).exists():
            self._game_folder = Path(env_game)
        
        env_docs = os.getenv('NWN2_DOCUMENTS_FOLDER')
        if env_docs and Path(env_docs).exists():
            self._documents_folder = Path(env_docs)
        
        env_steam = os.getenv('NWN2_STEAM_WORKSHOP_FOLDER')
        if env_steam and Path(env_steam).exists():
            self._steam_workshop_folder = Path(env_steam)
        
        # Check user settings file
        user_settings_path = USER_SETTINGS_PATH
        if user_settings_path.exists():
            try:
                with open(user_settings_path, 'r') as f:
                    settings = json.load(f)
                    
                    # Load game folder
                    if not self._game_folder and 'game_folder' in settings:
                        path = Path(settings['game_folder'])
                        if path.exists():
                            self._game_folder = path
                    
                    # Load documents folder
                    if not self._documents_folder and 'documents_folder' in settings:
                        path = Path(settings['documents_folder'])
                        if path.exists():
                            self._documents_folder = path
                    
                    # Load Steam workshop folder
                    if not self._steam_workshop_folder and 'steam_workshop_folder' in settings:
                        path = Path(settings['steam_workshop_folder'])
                        if path.exists():
                            self._steam_workshop_folder = path
                    
                    # Load custom folders
                    if 'custom_override_folders' in settings:
                        for folder in settings['custom_override_folders']:
                            path = Path(folder)
                            if path.exists():
                                self._custom_override_folders.append(path)
                    
                    if 'custom_module_folders' in settings:
                        for folder in settings['custom_module_folders']:
                            path = Path(folder)
                            if path.exists():
                                self._custom_module_folders.append(path)
                    
                    if 'custom_hak_folders' in settings:
                        for folder in settings['custom_hak_folders']:
                            path = Path(folder)
                            if path.exists():
                                self._custom_hak_folders.append(path)
                                
            except (json.JSONDecodeError, IOError):
                # Ignore corrupted or unreadable settings file
                pass
        
        # Try Rust-powered auto-discovery for missing paths
        if not self._game_folder:
            found = NWN2PathFinder.find_nwn2_installation()
            if found:
                self._game_folder = found
            else:
                # Default to local data folder if nothing else is found
                self._game_folder = Path(__file__).parent.parent / 'nwn2_ee_data'
        
        if not self._documents_folder:
            found = NWN2PathFinder.find_documents_folder()
            if found:
                self._documents_folder = found
        
        if not self._steam_workshop_folder:
            found = NWN2PathFinder.find_steam_workshop()
            if found:
                self._steam_workshop_folder = found
    
    @property
    def game_folder(self) -> Path:
        """Get the main game installation folder"""
        return self._game_folder
    
    @property
    def data(self) -> Path:
        """Get the data folder containing ZIP files"""
        return self.game_folder / 'data'
    
    @property
    def enhanced(self) -> Path:
        """Get the enhanced folder (Enhanced Edition specific data)"""
        return self.game_folder / 'enhanced'
    
    @property
    def enhanced_data(self) -> Path:
        """Get the enhanced data folder containing Enhanced Edition ZIP files"""
        return self.enhanced / 'data'
    
    @property
    def dialog_tlk(self) -> Path:
        """Get the dialog.tlk file path"""
        return self.game_folder / 'dialog.tlk'
    
    @property
    def campaigns(self) -> Path:
        """Get the campaigns folder"""
        return self.game_folder / 'Campaigns'
    
    @property
    def modules(self) -> Path:
        """Get the modules folder"""
        return self.game_folder / 'Modules'
    
    @property
    def override(self) -> Path:
        """Get the override folder"""
        return self.game_folder / 'override'
    
    @property
    def hak(self) -> Path:
        """Get the hak folder"""
        return self.game_folder / 'hak'
    
    @property
    def steam_workshop_folder(self) -> Optional[Path]:
        """Get the Steam workshop folder"""
        return self._steam_workshop_folder
    
    @property
    def custom_override_folders(self) -> List[Path]:
        """Get custom override folders"""
        return self._custom_override_folders.copy()
    
    @property
    def custom_module_folders(self) -> List[Path]:
        """Get custom module folders"""
        return self._custom_module_folders.copy()
    
    @property
    def custom_hak_folders(self) -> List[Path]:
        """Get custom HAK folders"""
        return self._custom_hak_folders.copy()
    
    @property
    def user_folder(self) -> Path:
        """Get the user documents folder (saves, modules, etc)"""
        # Use configured documents folder if available
        if self._documents_folder:
            return self._documents_folder
            
        # Otherwise auto-detect
        # Fallback to home directory
        docs_candidates = [
            Path.home() / 'Documents' / 'Neverwinter Nights 2',
            Path.home() / 'My Documents' / 'Neverwinter Nights 2',
        ]
        for candidate in docs_candidates:
            if candidate.exists():
                return candidate
        
        # Final fallback
        return Path.home() / 'Documents' / 'Neverwinter Nights 2'
    
    @property
    def saves(self) -> Path:
        """Get the saves folder"""
        return self.user_folder / 'saves'
    
    @property
    def localvault(self) -> Path:
        """Get the localvault folder (character files)"""
        return self.user_folder / 'localvault'
    
    @property
    def servervault(self) -> Path:
        """Get the servervault folder"""
        return self.user_folder / 'servervault'
    
    @property
    def user_modules(self) -> Path:
        """Get the user modules folder"""
        return self.user_folder / 'modules'
    
    @property
    def user_override(self) -> Path:
        """Get the user override folder"""
        return self.user_folder / 'override'
    
    @property
    def user_hak(self) -> Path:
        """Get the user hak folder"""
        return self.user_folder / 'hak'
    
    @property
    def is_enhanced_edition(self) -> bool:
        """Check if this is the Enhanced Edition"""
        return self.enhanced.exists()
    
    @property
    def is_steam_installation(self) -> bool:
        """Check if this is a Steam installation"""
        game_path_str = str(self._game_folder)
        return 'Steam' in game_path_str or 'steamapps' in game_path_str
    
    @property
    def is_gog_installation(self) -> bool:
        """Check if this is a GOG installation"""
        game_path_str = str(self._game_folder)
        return 'GOG' in game_path_str
    
    def get_all_data_folders(self) -> List[Path]:
        """Get all data folders (regular and enhanced if available)"""
        folders = []
        if self.data.exists():
            folders.append(self.data)
        if self.is_enhanced_edition and self.enhanced_data.exists():
            folders.append(self.enhanced_data)
        return folders
    
    def get_path_discovery_performance(self) -> Dict[str, Any]:
        """Get performance metrics for path discovery"""
        return NWN2PathFinder.get_discovery_timing()
    
    def discover_all_nwn2_installations(self) -> Dict[str, List[Path]]:
        """Discover all NWN2 installations with categorization"""
        result = discover_nwn2_paths()
        return {
            'all_installations': [Path(path) for path in result.nwn2_paths],
            'steam_installations': [Path(path) for path in result.steam_paths],
            'gog_installations': [Path(path) for path in result.gog_paths],
            'discovery_time_ms': result.total_time_ms,
            'timing_breakdown': result.timing_breakdown
        }

    def _save_settings(self) -> bool:
        """Save current settings to file"""
        user_settings_path = USER_SETTINGS_PATH
        user_settings_path.parent.mkdir(parents=True, exist_ok=True)
        
        settings = {
            'game_folder': str(self._game_folder.resolve()) if self._game_folder else None,
            'documents_folder': str(self._documents_folder.resolve()) if self._documents_folder else None,
            'steam_workshop_folder': str(self._steam_workshop_folder.resolve()) if self._steam_workshop_folder else None,
            'custom_override_folders': [str(p.resolve()) for p in self._custom_override_folders],
            'custom_module_folders': [str(p.resolve()) for p in self._custom_module_folders],
            'custom_hak_folders': [str(p.resolve()) for p in self._custom_hak_folders],
        }
        
        try:
            with open(user_settings_path, 'w') as f:
                json.dump(settings, f, indent=2)
            return True
        except IOError:
            return False
    
    def set_game_folder(self, path: str) -> bool:
        """Update the game folder path and save it to user settings."""
        path_obj = Path(path)
        if not path_obj.is_dir():
            return False
        
        self._game_folder = path_obj
        return self._save_settings()
    
    def set_documents_folder(self, path: str) -> bool:
        """Update the documents folder path and save it to user settings."""
        path_obj = Path(path)
        if not path_obj.is_dir():
            return False
        
        self._documents_folder = path_obj
        return self._save_settings()
    
    def set_steam_workshop_folder(self, path: str) -> bool:
        """Update the Steam workshop folder path and save it to user settings."""
        path_obj = Path(path)
        if not path_obj.is_dir():
            return False
        
        self._steam_workshop_folder = path_obj
        return self._save_settings()
    
    def add_custom_override_folder(self, path: str) -> bool:
        """Add a custom override folder"""
        path_obj = Path(path)
        if not path_obj.is_dir():
            return False
        
        if path_obj not in self._custom_override_folders:
            self._custom_override_folders.append(path_obj)
            return self._save_settings()
        return True
    
    def add_custom_module_folder(self, path: str) -> bool:
        """Add a custom module folder"""
        path_obj = Path(path)
        if not path_obj.is_dir():
            return False
        
        if path_obj not in self._custom_module_folders:
            self._custom_module_folders.append(path_obj)
            return self._save_settings()
        return True
    
    def add_custom_hak_folder(self, path: str) -> bool:
        """Add a custom HAK folder"""
        path_obj = Path(path)
        if not path_obj.is_dir():
            return False
        
        if path_obj not in self._custom_hak_folders:
            self._custom_hak_folders.append(path_obj)
            return self._save_settings()
        return True
    
    def remove_custom_override_folder(self, path: str) -> bool:
        """Remove a custom override folder"""
        path_obj = Path(path)
        if path_obj in self._custom_override_folders:
            self._custom_override_folders.remove(path_obj)
            return self._save_settings()
        return False
    
    def remove_custom_module_folder(self, path: str) -> bool:
        """Remove a custom module folder"""
        path_obj = Path(path)
        if path_obj in self._custom_module_folders:
            self._custom_module_folders.remove(path_obj)
            return self._save_settings()
        return False
    
    def remove_custom_hak_folder(self, path: str) -> bool:
        """Remove a custom HAK folder"""
        path_obj = Path(path)
        if path_obj in self._custom_hak_folders:
            self._custom_hak_folders.remove(path_obj)
            return self._save_settings()
        return False
    
    def get_all_paths_info(self) -> Dict[str, Any]:
        """Get information about all configured paths with Rust-enhanced details"""
        info = {
            'game_folder': {
                'path': str(self._game_folder) if self._game_folder else None,
                'exists': self._game_folder.exists() if self._game_folder else False,
                'auto_detected': not bool(os.getenv('NWN2_GAME_FOLDER')),
                'is_steam': self.is_steam_installation,
                'is_gog': self.is_gog_installation,
                'is_enhanced_edition': self.is_enhanced_edition,
            },
            'documents_folder': {
                'path': str(self._documents_folder) if self._documents_folder else None,
                'exists': self._documents_folder.exists() if self._documents_folder else False,
                'auto_detected': not bool(os.getenv('NWN2_DOCUMENTS_FOLDER')),
            },
            'steam_workshop_folder': {
                'path': str(self._steam_workshop_folder) if self._steam_workshop_folder else None,
                'exists': self._steam_workshop_folder.exists() if self._steam_workshop_folder else False,
                'auto_detected': not bool(os.getenv('NWN2_STEAM_WORKSHOP_FOLDER')),
            },
            'custom_override_folders': [
                {'path': str(p), 'exists': p.exists()} for p in self._custom_override_folders
            ],
            'custom_module_folders': [
                {'path': str(p), 'exists': p.exists()} for p in self._custom_module_folders
            ],
            'custom_hak_folders': [
                {'path': str(p), 'exists': p.exists()} for p in self._custom_hak_folders
            ],
        }
        
        # Add Rust path discovery performance info
        try:
            info['path_discovery_performance'] = self.get_path_discovery_performance()
        except Exception as e:
            info['path_discovery_performance'] = {'error': str(e)}
            
        return info
        
        
# Create global instance
nwn2_paths = NWN2Paths()

# Legacy compatibility functions
def get_nwn2_data_path():
    """Legacy function - returns game folder"""
    return nwn2_paths.game_folder

def get_data_zip_path():
    """Legacy function - returns data folder"""
    return nwn2_paths.data

def get_dialog_tlk_path():
    """Legacy function - returns dialog.tlk path"""
    return nwn2_paths.dialog_tlk

def update_nwn2_data_path(new_path: str) -> bool:
    """Legacy function - updates game folder"""
    return nwn2_paths.set_game_folder(new_path)

# Legacy global variables
NWN2_DATA_PATH = nwn2_paths.game_folder

# Performance utilities
def profile_path_discovery_performance(iterations: int = 100) -> Dict[str, float]:
    """Profile path discovery performance using Rust implementation"""
    return profile_path_discovery(iterations)

# Global configuration
NWN2_SETTINGS = {
    'installation_path': nwn2_paths.game_folder,
    'enable_custom_content': True,
    'cache_module_data': True,
    'module_cache_dir': get_writable_dir(os.path.join('cache', 'modules')),
    'rust_path_discovery': True,  # New flag indicating Rust-powered path discovery
}