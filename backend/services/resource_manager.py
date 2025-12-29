"""
Resource Manager for efficient loading of NWN2 game data
"""

import os
import re
from loguru import logger
from typing import Dict, Optional, List, Tuple, Any, Union
from pathlib import Path
from datetime import datetime
from collections import OrderedDict
import time
import zlib
import sys

# Django settings import - optional for standalone mode
# Get base directory for the project
BASE_DIR = Path(__file__).parent.parent

# Import Rust parsers - optional for standalone mode
try:
    from nwn2_rust import TDAParser, TLKParser, ErfParser, GffParser
    logger.info("Using high-performance Rust parsers (TDA, TLK, ERF, GFF)")
except ImportError:
    TDAParser = None
    TLKParser = None
    ErfParser = None
    GffParser = None

class ERFResourceType:
    TDA = 2017  # 2DA files
    TLK = 2018  # Talk table files
    GFF = 2037  # Generic file format
    IFO = 2014  # Module info files
from nwn2_rust import GffParser
from .cache_helper import TDACacheHelper
import zipfile

# Rust extensions (optional for standalone mode)
try:
    from nwn2_rust import ResourceScanner as RustResourceScanner, ZipContentReader
except ImportError:
    RustResourceScanner = None
    ZipContentReader = None


        

# Config and services
from config.nwn2_settings import nwn2_paths
# Optional workshop service for standalone mode
try:
    from gamedata.services.workshop_service import SteamWorkshopService
except ImportError:
    SteamWorkshopService = None
from gamedata.cache.safe_cache import SafeCache
from gamedata.services.data_fetching_rules import with_retry_limit
from utils.performance_profiler import get_profiler


class ModuleLRUCache:
    """LRU cache for module data to avoid reloading frequently used modules"""
    
    def __init__(self, max_size: int = 5):
        self.max_size = max_size
        self.cache: OrderedDict[str, Tuple[Any, datetime]] = OrderedDict()
        
    def get(self, key: str) -> Optional[Any]:
        """Get item from cache and move to end (most recently used)"""
        if key not in self.cache:
            return None
        
        # Move to end (most recently used)
        self.cache.move_to_end(key)
        value, timestamp = self.cache[key]
        return value
    
    def put(self, key: str, value: Any):
        """Add item to cache, evicting least recently used if necessary"""
        if key in self.cache:
            # Update existing entry and move to end
            self.cache[key] = (value, datetime.now())
            self.cache.move_to_end(key)
        else:
            # Add new entry
            self.cache[key] = (value, datetime.now())
            
            # Evict least recently used if cache is full
            if len(self.cache) > self.max_size:
                oldest_key = next(iter(self.cache))
                logger.debug(f"LRU cache evicting: {oldest_key}")
                del self.cache[oldest_key]
    
    def clear(self):
        """Clear the cache"""
        self.cache.clear()
    
    def get_stats(self) -> Dict[str, Any]:
        """Get cache statistics"""
        return {
            'size': len(self.cache),
            'max_size': self.max_size,
            'keys': list(self.cache.keys()),
            'timestamps': {k: v[1].isoformat() for k, v in self.cache.items()}
        }


class ResourceManager:
    """
    Centralized manager for NWN2 game resources.
    
    Coordinates all NWN2 file parsers (GFF, 2DA, TLK, ERF/HAK) with caching,
    mod detection, and resource scanning capabilities.
    """
    
    # ============================================================================
    # INITIALIZATION & CONFIGURATION
    # ============================================================================
    
    def __init__(self, nwn2_path: Optional[str] = None, cache_dir: str = "cache", suppress_warnings: bool = False):
        profiler = get_profiler()
        from utils.paths import get_writable_dir
        
        with profiler.profile("ResourceManager.__init__"):
            # Use provided path or default from nwn2_settings
            self.nwn2_path = Path(nwn2_path) if nwn2_path else nwn2_paths.game_folder
            
            # Resolve cache directory using centralized utility if relative
            cache_path = Path(cache_dir)
            if not cache_path.is_absolute():
                self.cache_dir = get_writable_dir(cache_dir)
            else:
                self.cache_dir = cache_path.resolve()
                self.cache_dir.mkdir(parents=True, exist_ok=True)
        self.suppress_warnings = suppress_warnings
        
        # ZIP file handles - now opened on-demand and closed after use
        # No longer kept open to make ResourceManager pickle-safe
        self._zip_files: Dict[str, zipfile.ZipFile] = {}
        
        # In-memory cache - using OrderedDict for proper LRU
        # self._2da_cache: OrderedDict[str, Union[TDAParser, bytes]] = OrderedDict()
        # self._2da_compressed: Dict[str, bool] = {}  # Track compressed entries
        self._tlk_cache: Optional[TLKParser] = None
        self._custom_tlk_cache: Optional[TLKParser] = None
        
        # File modification time tracking for cache invalidation
        self._file_mod_times: Dict[str, float] = {}  # path -> mtime
        self._cache_timestamps: Dict[str, datetime] = {}  # cache_key -> when cached
        
        # Track what 2DA files are in which ZIP
        self._2da_locations: Dict[str, Tuple[str, str]] = {}
        # Track where item templates are located (zip_path, internal_path)
        self._template_locations: Dict[str, Tuple[str, str]] = {}
        
        # ERF/HAK file cache
        self._erf_parsers: Dict[str, ErfParser] = {}
        self._module_overrides: Dict[str, TDAParser] = {}
        self._hak_overrides: List[Dict[str, TDAParser]] = []  # Ordered list of HAK overrides
        self._override_dir_overrides: Dict[str, TDAParser] = {}  # Override directory
        self._workshop_overrides: Dict[str, TDAParser] = {}  # Steam Workshop
        
        # File path indexes for on-demand parsing
        self._override_file_paths: Dict[str, Path] = {}
        self._workshop_file_paths: Dict[str, Path] = {}
        self._custom_override_paths: Dict[str, Path] = {}
        self._custom_override_dirs: List[Path] = []

        # Campaign overrides
        self._campaign_overrides: Dict[str, TDAParser] = {}
        self._campaign_override_paths: Dict[str, Path] = {}
        self._current_campaign_folder: Optional[Path] = None
        self._current_campaign_id: Optional[str] = None

        # Module information
        self._current_module: Optional[str] = None
        self._module_parser: Optional[ErfParser] = None
        self._module_info: Optional[Dict[str, Any]] = None
        self._module_path: Optional[Path] = None
        
        # Module LRU cache
        self._module_cache = ModuleLRUCache(max_size=5)
        
        # Track override directories for reporting
        self._override_dirs: List[Path] = []
        
        # Steam Workshop service
        self._workshop_service = SteamWorkshopService(cache_dir=self.cache_dir) if SteamWorkshopService else None
        
        # Module-to-HAK mapping for save-context-aware loading
        self._module_to_haks: Dict[str, Dict[str, Any]] = {}
        self._modules_indexed = False
        
        # Expansion file loading tracking
        self._expansion_files_loaded: Dict[str, int] = {}  # expansion name -> count
        self._expansion_summary_logged = False
        
        # Official modules to skip during indexing (we already have their content)
        self._vanilla_modules = {
            # Main Campaign
            '0_tutorial.mod',
            '0100_uninvitedguests.mod', 
            '1000_neverwinter_a1.mod',
            '1100_west_harbor.mod',
            '1200_highcliff.mod',
            '1300_old_owl_well.mod',
            '1600_githyanki_caves.mod',
            '1700_merchant_quarter.mod',
            '1800_skymirror.mod',
            '1900_slums.mod',
            '2000_neverwinter.mod',
            '2100_crossroad_keep_a2.mod',
            '2200_port_llast.mod',
            '2300_crossroad_keep_adv.mod',
            '2400_illefarn_ruins.mod',
            '2600_aj_haven.mod',
            '3000_neverwinter_a3.mod',
            '3400_merdelain.mod',
            '3500_crossroad_keep_siege.mod',
            
            # Expansion Modules
            'a_x1.mod',
            'b_x1.mod',
            'c_x1.mod',
            'd_x1.mod',
            'e_x1.mod',
            'f_x2.mod',
            'g_x2.mod',
            'm_x2.mod',
            'n_x2.mod',
            'o_x2.mod',
            's_x2.mod',
            't_x2.mod',
            'x_x2.mod',
            'z_x1.mod',
            
            # Westgate (official DLC content)
            'westgate_ar1500.mod',
            'westgate_ar1600.mod',
            'westgate_ar1700.mod',
            'westgate_ar1800.mod'
        }
        
        # Use Rust extensions for high-performance resource scanning
        logger.info("Using Rust extensions for high-performance resource scanning")
        # Direct usage of Rust scanner - no wrapper needed
        # Note: Rust methods expect string paths, so we convert Paths to strings at call sites
        rust_scanner = RustResourceScanner()
        self._python_scanner = rust_scanner
        self._zip_indexer = rust_scanner
        self._directory_walker = rust_scanner
        
        # Initialize Rust ZIP content reader for efficient file access
        self._zip_reader = ZipContentReader()
        
        # In-memory cache settings
        # TODO: Remove memory cache settings - not needed with manager pattern
        # Setting to False to disable while keeping compatibility
        self._memory_cache_enabled = False  # getattr(settings, 'NWN2_MEMORY_CACHE', False)
        self._preload_on_init = False      # getattr(settings, 'NWN2_PRELOAD_2DA', True)
        # self._cache_max_mb = getattr(settings, 'NWN2_CACHE_MAX_MB', 50)
        # self._compression_enabled = getattr(settings, 'NWN2_COMPRESS_CACHE', True)
        # self._compression_threshold = getattr(settings, 'NWN2_COMPRESS_THRESHOLD_KB', 100)  # Compress if > 100KB
        
        # Initialize pre-compiled cache integration
        from .cache_integration import PrecompiledCacheIntegration
        self._precompiled_cache = PrecompiledCacheIntegration(self)
        
        # Track memory usage and cache statistics
        # self._cache_memory_bytes = 0
        self._cache_hits = 0
        self._cache_misses = 0
        # self._compression_ratio_sum = 0.0
        # self._compression_count = 0
        
        # Try fast cache validation first to avoid heavy scanning
        cache_valid_fast = False
        with profiler.profile("Fast Cache Validation"):
            cache_valid_fast = self._fast_cache_validation()
        
        if cache_valid_fast:
            # Fast path: Cache is valid, skip heavy scanning
            logger.info("Fast cache validation passed - using cached data, skipping heavy scans")
            
            # Load cached file path mappings without heavy scanning
            with profiler.profile("Load Cached Mappings"):
                self._load_cached_data_fast()
            
            # CRITICAL FIX: Run workshop override detection in fast path
            # This ensures workshop files take precedence over expansion files
            with profiler.profile("Workshop Override Detection"):
                self._detect_workshop_overrides_fast()
            
            # CRITICAL FIX: Also scan for TLK files in fast path
            # TLK files are needed for proper string resolution in modded content
            with profiler.profile("Workshop TLK Detection"):
                self._scan_workshop_tlk_fast()
            
            # Still need ZIP locations for base game files (but this is much faster than heavy parsing)
            with profiler.profile("Scan ZIP Files"):
                self._scan_zip_files()

            # Module scanning removed - HAKs now extracted from save's module.ifo directly
            # via load_haks_for_save() which is called when loading a save
        else:
            # Slow path: Cache invalid or missing, do full scanning
            logger.info("Fast cache validation failed - using full scanning path")
            
            # Initialize with full heavy scanning
            with profiler.profile("Scan ZIP Files"):
                self._scan_zip_files()
            
            # Scan override directories immediately for save compatibility
            # This ensures custom mod content is available for DynamicGameDataLoader
            with profiler.profile("Scan Override Directories"):
                self._scan_override_directories()

            # Module scanning removed - HAKs now extracted from save's module.ifo directly
            # via load_haks_for_save() which is called when loading a save

            # Skip preloading if pre-compiled cache is valid
            if self._precompiled_cache.cache_enabled and self._precompiled_cache.cache_manager:
                # Check if pre-compiled cache is valid
                cache_stats = self._precompiled_cache.get_cache_stats()
                if cache_stats.get('valid', False):
                    logger.info("Pre-compiled cache is valid, skipping preload")
                else:
                    # Preload if cache not valid
                    # Preloading disabled - not needed with manager pattern
                    pass
            else:
                # Preload if no precompiled cache
                # Preloading disabled - not needed with manager pattern
                pass
            
            # Now that ResourceManager is fully initialized, ensure cache is built
            with profiler.profile("Ensure Precompiled Cache"):
                self._precompiled_cache.ensure_cache_built()
        
    # ============================================================================
    # RESOURCE SCANNING & DISCOVERY
    # ============================================================================
    
    def _scan_zip_files(self):
        """Scan ZIP files to build index of 2DA locations using optimized Python scanner"""
        data_dir = self.nwn2_path / "data"
        
        # Later zips override earlier ones
        zip_files = [
            "2da.zip",      # Base game
            "2da_x1.zip",   # Mask of the Betrayer
            "2da_x2.zip",   # Storm of Zehir
            "Templates.zip",    # Base game items
            "Templates_X1.zip", # MotB items
            "Templates_X2.zip", # SoZ items
        ]
        
        # Build list of ZIP paths to scan
        zip_paths = []
        
        if nwn2_paths.is_enhanced_edition:
            data_dirs = [data_dir, nwn2_paths.enhanced_data]
        else:
            data_dirs = [data_dir]
        
        for data_dir in data_dirs:
            for zip_name in zip_files:
                zip_path = data_dir / zip_name
                if zip_path.exists():
                    zip_paths.append(str(zip_path))
        
        if not zip_paths:
            logger.warning("No ZIP files found for indexing")
            return
        
        start_time = time.time()
        
        # Always use parallel processing - even 3 ZIPs benefit from it
        # The Rust parallel implementation is highly optimized
        # Explicit conversion to list of strings for Rust binding
        resource_locations = self._python_scanner.scan_zip_files([str(p) for p in zip_paths])
        
        self._zip_reader.preopen_zip_archives(zip_paths)
        
        # Convert ResourceLocation objects to the legacy format
        for resource_name, resource_location in resource_locations.items():
            zip_path_str = resource_location.source_path
            internal_path = resource_location.internal_path
            
            if resource_name.lower().endswith('.uti'):
                self._template_locations[resource_name.lower()] = (zip_path_str, internal_path)
            else:
                self._2da_locations[resource_name.lower()] = (zip_path_str, internal_path)
        
        scan_time_ms = int((time.time() - start_time) * 1000)
        logger.info(f"ZIP scan completed: {len(resource_locations)} 2DA files from {len(zip_paths)} ZIPs in {scan_time_ms}ms")
        
        # Check if critical expansion files were found
        expansion_files = ['classes.2da', 'spells.2da', 'feat.2da']
        expansion_files_detected = []
        for filename in expansion_files:
            if filename in self._2da_locations:
                zip_path, internal_path = self._2da_locations[filename]
                zip_name = Path(zip_path).name
                logger.info(f"Expansion check: {filename} loaded from {zip_name} ({internal_path})")
                
                if 'x1' in zip_name.lower() or 'x2' in zip_name.lower():
                    expansion_files_detected.append(filename)
            else:
                logger.warning(f"Expansion check: {filename} NOT FOUND in ZIP locations")
        
        self._invalidate_cache_for_overrides(expansion_files_detected, 'expansion')
    
    def _invalidate_cache_for_overrides(self, override_files: List[str], source_type: str = 'unknown'):
        """Invalidate cache for files that have override sources"""
        if not override_files:
            return
            
        logger.info(f"Invalidating cache for {source_type} files: {override_files}")
        invalidated_count = 0
        
        for filename in override_files:
            # Remove from memory cache
            # if filename in self._2da_cache:
            #     del self._2da_cache[filename]
            #     logger.debug(f"Removed {filename} from memory cache")
            #     invalidated_count += 1
            # 
            # if filename in self._2da_compressed:
            #     del self._2da_compressed[filename]
            
            # Disk cache removed - precompiled cache handles invalidation
            invalidated_count += 1
        
        logger.info(f"Cache invalidation complete: {invalidated_count} {source_type} files processed")
    
    def _fast_cache_validation(self) -> bool:
        """
        Fast cache validation - checks if cache is valid without heavy scanning.
        
        Returns:
            True if cache appears valid and can be used, False if full scanning is needed
        """
        if not self._precompiled_cache or not self._precompiled_cache.cache_enabled:
            return False
        
        try:
            # First check if cache files actually exist
            from utils.paths import get_writable_dir
            cache_dir = get_writable_dir("cache/compiled_cache")
            metadata_file = cache_dir / "cache_metadata.json"
            
            if not metadata_file.exists():
                logger.debug("Fast cache validation: No cache files found")
                return False
            
            # Generate lightweight cache key based on file listings only
            fast_mod_state = self._generate_fast_mod_state()
            if not fast_mod_state:
                logger.debug("Fast cache validation: Could not generate mod state")
                return False
            
            # Use the cache integration's method for consistency
            cache_key = self._precompiled_cache.cache_builder.generate_cache_key(fast_mod_state)
            is_valid = self._precompiled_cache.cache_manager.validate_cache_key(cache_key)
            
            if is_valid:
                logger.debug("Fast cache validation: cache key is valid")
                return True
            else:
                logger.debug("Fast cache validation: cache key is invalid or missing")
                logger.debug(f"Fast mod state: {fast_mod_state}")
                return False
                
        except Exception as e:
            logger.warning(f"Fast cache validation failed with error: {e}", exc_info=True)
            return False
    
    def _generate_fast_mod_state(self) -> Optional[Dict[str, Any]]:
        """
        Generate mod state for cache validation using only lightweight directory listings.
        This avoids heavy ZIP scanning and 2DA parsing.
        
        Returns:
            Mod state dict or None if generation failed
        """
        try:
            from config.nwn2_settings import nwn2_paths
            
            mod_state = {
                'install_dir': str(nwn2_paths.game_folder) if nwn2_paths.game_folder else '',
                'workshop_files': [],
                'override_files': []
            }
            
            # Fast workshop files listing using same Rust scanner as heavy scan
            workshop_files = []
            if nwn2_paths.steam_workshop_folder and nwn2_paths.steam_workshop_folder.exists():
                for workshop_item in nwn2_paths.steam_workshop_folder.iterdir():
                    if workshop_item.is_dir():
                        workshop_override = workshop_item / 'override'
                        if workshop_override.exists():
                            # Check subdirectories like override/2DA/ using Rust scanner
                            tda_subdir = workshop_override / '2DA'
                            if tda_subdir.exists():
                                resource_locations = self._directory_walker.index_directory(str(tda_subdir), recursive=True)
                                for resource_name in resource_locations.keys():
                                    workshop_files.append(resource_name.lower())
                            # Also check root override using Rust scanner (recursive like heavy scan)
                            resource_locations = self._directory_walker.index_directory(str(workshop_override), recursive=True)
                            for resource_name in resource_locations.keys():
                                workshop_files.append(resource_name.lower())
            
            # Fast override files listing using same Rust scanner as heavy scan
            override_files = []
            override_dir = nwn2_paths.user_override
            if override_dir and override_dir.exists():
                resource_locations = self._directory_walker.index_directory(str(override_dir), recursive=True)
                for resource_name in resource_locations.keys():
                    override_files.append(resource_name.lower())
            
            mod_state['workshop_files'] = list(set(workshop_files))  # Remove duplicates
            mod_state['override_files'] = list(set(override_files))   # Remove duplicates
            
            logger.debug(f"Fast mod state: {len(mod_state['workshop_files'])} workshop, {len(mod_state['override_files'])} override files")
            return mod_state
            
        except Exception as e:
            logger.error(f"Failed to generate fast mod state: {e}")
            return None
    
    def _load_cached_data_fast(self) -> bool:
        """
        Load cached data when skipping heavy scanning.
        Populates the file path mappings from cached information.
        
        Returns:
            True if cached data was loaded successfully
        """
        try:
            from config.nwn2_settings import nwn2_paths
            
            # Generate the same fast mod state to ensure consistency
            fast_mod_state = self._generate_fast_mod_state()
            if not fast_mod_state:
                return False
            
            # Populate file path mappings from the fast mod state
            # This avoids having to scan directories again
            workshop_files = fast_mod_state.get('workshop_files', [])
            override_files = fast_mod_state.get('override_files', [])
            
            # Build workshop file paths mapping using same Rust scanner
            if nwn2_paths.steam_workshop_folder and nwn2_paths.steam_workshop_folder.exists():
                for workshop_item in nwn2_paths.steam_workshop_folder.iterdir():
                    if workshop_item.is_dir():
                        workshop_override = workshop_item / 'override'
                        if workshop_override.exists():
                            # Check subdirectories like override/2DA/ using Rust scanner
                            tda_subdir = workshop_override / '2DA'
                            if tda_subdir.exists():
                                resource_locations = self._directory_walker.index_directory(str(tda_subdir), recursive=True)
                                for resource_name, resource_location in resource_locations.items():
                                    if resource_name.lower() in workshop_files:
                                        self._workshop_file_paths[resource_name.lower()] = Path(resource_location.source_path)
                            # Also check root override using Rust scanner (recursive like heavy scan)
                            resource_locations = self._directory_walker.index_directory(str(workshop_override), recursive=True)
                            for resource_name, resource_location in resource_locations.items():
                                if resource_name.lower() in workshop_files:
                                    self._workshop_file_paths[resource_name.lower()] = Path(resource_location.source_path)
            
            # Build override file paths mapping using same Rust scanner
            override_dir = nwn2_paths.user_override
            if override_dir and override_dir.exists():
                resource_locations = self._directory_walker.index_directory(str(override_dir), recursive=True)
                for resource_name, resource_location in resource_locations.items():
                    if resource_name.lower() in override_files:
                        self._override_file_paths[resource_name.lower()] = Path(resource_location.source_path)
            
            logger.info(f"Loaded cached mappings: {len(self._workshop_file_paths)} workshop, {len(self._override_file_paths)} override files")
            return True
            
        except Exception as e:
            logger.error(f"Failed to load cached data fast: {e}")
            return False
    
    def _detect_workshop_overrides_fast(self) -> None:
        """
        Detect workshop override conflicts and invalidate cache in fast validation path.
        
        This method runs the same override precedence logic as _scan_override_directories()
        but without heavy directory scanning (since file mappings are already cached).
        
        Critical for ensuring workshop > expansion > base precedence is maintained.
        """
        # Check for workshop mod overrides that need cache invalidation
        # This is the same logic from _scan_override_directories() lines 966-982
        workshop_overrides = []
        critical_files = ['classes.2da', 'spells.2da', 'feat.2da', 'baseitems.2da', 'appearance.2da']
        
        for filename in critical_files:
            # Check if workshop mods have this file
            if filename in self._workshop_file_paths:
                workshop_overrides.append(filename)
                logger.info(f"Workshop override detected: {filename} in {self._workshop_file_paths[filename]}")
            # Also check override directories  
            elif filename in self._override_file_paths:
                workshop_overrides.append(filename)
                logger.info(f"User override detected: {filename} in {self._override_file_paths[filename]}")
        
        # Invalidate cache for workshop/override files
        # This ensures expansion files don't take precedence over workshop files
        if workshop_overrides:
            self._invalidate_cache_for_overrides(workshop_overrides, 'workshop_mod')
    
    # ============================================================================
    # HAK/ERF & MODULE OPERATIONS
    # ============================================================================
    
    # Module Loading & Management
    
    def set_module(self, module_path: str) -> bool:
        """
        Load a module and set up its override chain
        
        Args:
            module_path: Path to the .mod file or campaign module directory
            
        Returns:
            True if module loaded successfully
        """
        try:
            module_path = Path(module_path)
            cache_key = str(module_path)
            
            # Check LRU cache first
            cached_data = self._module_cache.get(cache_key)
            if cached_data:
                logger.info(f"Loading module from cache: {cache_key}")
                # Restore cached data
                self._current_module = cached_data['current_module']
                self._module_path = cached_data['module_path']
                self._module_parser = cached_data['module_parser']
                self._module_info = cached_data['module_info']
                self._module_overrides = cached_data['module_overrides']
                self._hak_overrides = cached_data['hak_overrides']
                self._custom_tlk_cache = cached_data.get('custom_tlk')
                self._campaign_overrides = cached_data.get('campaign_overrides', {})
                self._campaign_override_paths = cached_data.get('campaign_override_paths', {})
                self._current_campaign_folder = cached_data.get('campaign_folder')
                self._current_campaign_id = cached_data.get('campaign_id')
                return True
            
            self._current_module = str(module_path)
            self._module_path = module_path
            
            # Check if it's a directory or file (.mod)
            if module_path.is_dir():
                # Directory - not a valid module in NWN2
                logger.error(f"{module_path} is a directory, not a .mod file")
                return False
            else:
                # Regular .mod file - extract info and get parser
                extraction_result = self._extract_module_info(module_path)
                if not extraction_result:
                    logger.error(f"Failed to extract info from {module_path}")
                    return False
                
                self._module_parser = extraction_result['parser']
                module_info = extraction_result['info']
            
            # Format module info for storage
            hak_list = module_info.get('haks', [])
            self._module_info = {
                'Mod_Name': module_info.get('name', ''),
                'Mod_ID': module_info.get('mod_id', ''),
                'Mod_Version': 1,  # Default version
                'Mod_Entry_Area': module_info.get('entry_area', ''),
                'Mod_CustomTlk': module_info.get('custom_tlk', ''),
                'Mod_HakList': [{'Mod_Hak': hak} for hak in hak_list],  # Convert to expected format
            }
            logger.info(f"Module '{self._module_info['Mod_Name']}' has {len(hak_list)} HAKs")
            
            # Clear previous overrides
            self._module_overrides.clear()
            self._hak_overrides.clear()
            self._campaign_overrides.clear()
            self._campaign_override_paths.clear()
            self._current_campaign_folder = None
            self._current_campaign_id = None
            self._custom_tlk_cache = None

            # Load module's own 2DA overrides
            self._load_module_2das()
            
            # Load custom TLK if specified
            custom_tlk = self._module_info.get('Mod_CustomTlk', '')
            if custom_tlk:
                self._load_custom_tlk(custom_tlk)
            
            # Load HAKs in order
            for hak_name in hak_list:
                if hak_name:
                    self._load_hakpak_to_override_chain(hak_name)

            # Load campaign overrides if Campaign_ID present
            campaign_id = module_info.get('campaign_id', '')
            if campaign_id:
                self._current_campaign_id = campaign_id
                campaign_folder = self._find_campaign_folder_by_guid(campaign_id)
                if campaign_folder:
                    self._load_campaign_2das(campaign_folder)

            # Scan override directories
            self._scan_override_directories()

            # Cache the module data
            cache_data = {
                'current_module': self._current_module,
                'module_path': self._module_path,
                'module_parser': self._module_parser,
                'module_info': self._module_info.copy() if self._module_info else None,
                'module_overrides': self._module_overrides.copy(),
                'hak_overrides': self._hak_overrides.copy(),
                'custom_tlk': self._custom_tlk_cache,
                'campaign_overrides': self._campaign_overrides.copy(),
                'campaign_override_paths': self._campaign_override_paths.copy(),
                'campaign_folder': self._current_campaign_folder,
                'campaign_id': self._current_campaign_id,
            }
            self._module_cache.put(cache_key, cache_data)
            logger.info(f"Cached module data for: {cache_key}")
            
            return True
            
        except Exception as e:
            logger.error(f"Error loading module {module_path}: {e}")
            return False
    
    def find_campaign(self, campaign_path: str) -> dict:
        """
        Find campaign information by searching for .cam files
        
        Args:
            campaign_path: Path to the campaign directory
            
        Returns:
            Dict with campaign info or None if not found
        """
        try:
            campaign_path = Path(campaign_path)
            
            # Search for .cam files
            cam_files = list(campaign_path.glob("*.cam"))
            if not cam_files:
                # Search in subdirectories
                cam_files = list(campaign_path.glob("*/*.cam"))
            
            if cam_files:
                # Parse the first .cam file found
                cam_file = cam_files[0]
                logger.debug(f"Found campaign file: {cam_file}")
                
                # .cam files are GFF format
                campaign_data = GffParser(str(cam_file)).to_dict()
                
                # Extract campaign info using helper
                display_name = self._extract_gff_string(campaign_data.get('DisplayName'), 'Unknown Campaign')
                description = self._extract_gff_string(campaign_data.get('Description'), '')
                
                # Extract module names from list
                module_list = campaign_data.get('ModNames', [])
                module_names = []
                if isinstance(module_list, list):
                    for mod in module_list:
                        if isinstance(mod, dict) and 'ModuleName' in mod:
                            module_names.append(mod['ModuleName'])
                
                campaign_info = {
                    'file': str(cam_file),
                    'name': display_name,
                    'description': description,
                    'modules': module_names,
                    'start_module': campaign_data.get('StartModule', ''),
                    'directory': str(campaign_path),
                    'level_cap': campaign_data.get('LvlCap', 20),
                    'xp_cap': campaign_data.get('XPCap', 0),
                    'party_size': campaign_data.get('Cam_PartySize', 4)
                }
                
                return campaign_info
            
            return None
            
        except Exception as e:
            logger.error(f"Error finding campaign in {campaign_path}: {e}")
            return None

    def _find_campaign_folder_by_guid(self, campaign_guid: str) -> Optional[Path]:
        """
        Find campaign folder by matching Campaign_ID GUID from module.ifo
        to GUID in campaign.cam files.

        Searches both install folder and user documents folder.

        Args:
            campaign_guid: The Campaign_ID GUID from module.ifo

        Returns:
            Path to campaign folder or None if not found
        """
        if not campaign_guid:
            return None

        # Search both install folder and user documents folder
        campaigns_dirs = [
            nwn2_paths.campaigns,                    # Install folder (vanilla campaigns)
            nwn2_paths.user_folder / 'campaigns',    # User documents folder (custom campaigns)
        ]

        for campaigns_dir in campaigns_dirs:
            if not campaigns_dir or not campaigns_dir.exists():
                continue

            for campaign_name in os.listdir(campaigns_dir):
                campaign_path = campaigns_dir / campaign_name

                if not campaign_path.is_dir():
                    continue

                # Find campaign.cam (case-insensitive)
                campaign_file = None
                for f in campaign_path.iterdir():
                    if f.is_file() and f.name.lower() == 'campaign.cam':
                        campaign_file = f
                        break
                if not campaign_file:
                    continue

                try:
                    campaign_data = GffParser(str(campaign_file)).to_dict()

                    file_guid_raw = campaign_data.get('GUID', '')
                    if isinstance(file_guid_raw, bytes):
                        file_guid = file_guid_raw.hex()
                    else:
                        file_guid = str(file_guid_raw) if file_guid_raw else ''

                    if file_guid == campaign_guid:
                        logger.info(f"Found campaign folder: {campaign_name} for GUID {campaign_guid[:16]}...")
                        return campaign_path

                except Exception as e:
                    logger.debug(f"Failed to parse {campaign_file}: {e}")

        logger.debug(f"No campaign found for GUID: {campaign_guid[:16] if campaign_guid else 'empty'}...")
        return None

    def _load_campaign_2das(self, campaign_folder: Path):
        """
        Load 2DA file paths from campaign folder for lazy loading.

        NWN2 spec: Files in deeper subdirectories have higher priority.
        So we scan recursively and let deeper files override shallower ones.

        Args:
            campaign_folder: Path to the campaign folder
        """
        self._campaign_overrides.clear()
        self._campaign_override_paths.clear()
        self._current_campaign_folder = campaign_folder

        if not campaign_folder or not campaign_folder.exists():
            return

        files_by_depth: Dict[int, List[Tuple[str, Path]]] = {}

        for tda_file in campaign_folder.rglob('*.2da'):
            relative_path = tda_file.relative_to(campaign_folder)
            depth = len(relative_path.parts) - 1
            name = tda_file.name.lower()

            if depth not in files_by_depth:
                files_by_depth[depth] = []
            files_by_depth[depth].append((name, tda_file))

        for depth in sorted(files_by_depth.keys()):
            for name, file_path in files_by_depth[depth]:
                self._campaign_override_paths[name] = file_path
                self._file_mod_times[str(file_path)] = file_path.stat().st_mtime

        if self._campaign_override_paths:
            logger.info(f"Indexed {len(self._campaign_override_paths)} 2DAs in campaign folder: {campaign_folder.name}")

    def set_campaign_by_guid(self, campaign_guid: str) -> bool:
        """
        Set campaign context directly by GUID (for savegame loading).

        This loads campaign 2DA overrides without needing a .mod file.
        Used when loading saves where we have the Campaign_ID but not a module path.

        Args:
            campaign_guid: Campaign GUID hex string from save file

        Returns:
            True if campaign was found and loaded, False otherwise
        """
        if not campaign_guid:
            return False

        # Skip if already loaded
        if self._current_campaign_id == campaign_guid:
            logger.debug(f"Campaign {campaign_guid[:16]}... already loaded")
            return True

        # Clear previous campaign data
        self._campaign_overrides.clear()
        self._campaign_override_paths.clear()
        self._current_campaign_folder = None
        self._current_campaign_id = None

        # Find and load campaign
        campaign_folder = self._find_campaign_folder_by_guid(campaign_guid)
        if campaign_folder:
            self._current_campaign_id = campaign_guid
            self._load_campaign_2das(campaign_folder)
            return True
        else:
            logger.debug(f"Campaign not found for GUID: {campaign_guid[:16] if campaign_guid else 'empty'}...")
            return False

    def load_haks_for_save(self, hak_list: list, custom_tlk: str = '', campaign_guid: str = '') -> bool:
        """
        Load HAKs directly from a save's module.ifo data (no .mod file scanning needed).

        This is the preferred method for save editors - get HAK list from the save's
        module.ifo and pass it here directly, avoiding expensive module scanning.

        Args:
            hak_list: List of HAK names from Mod_HakList in module.ifo
            custom_tlk: Custom TLK filename from Mod_CustomTlk (optional)
            campaign_guid: Campaign GUID for loading campaign 2DAs (optional)

        Returns:
            True if HAKs loaded successfully
        """
        try:
            # Clear previous HAK overrides
            self._hak_overrides.clear()

            # Load custom TLK if specified
            if custom_tlk:
                self._load_custom_tlk(custom_tlk)

            # Load HAKs in order (first HAK = highest priority)
            loaded_count = 0
            for hak_name in hak_list:
                if hak_name:
                    if self._load_hakpak_to_override_chain(hak_name):
                        loaded_count += 1

            if loaded_count > 0:
                logger.info(f"Loaded {loaded_count}/{len(hak_list)} HAKs from save module.ifo")

            # Load campaign 2DAs if campaign_guid provided
            if campaign_guid:
                self.set_campaign_by_guid(campaign_guid)

            return True

        except Exception as e:
            logger.error(f"Error loading HAKs for save: {e}")
            return False

    def _load_custom_tlk(self, tlk_filename: str):
        """Load custom TLK file for the module"""
        if not tlk_filename:
            return
            
        # Add .tlk extension if not present
        if not tlk_filename.endswith('.tlk'):
            tlk_filename += '.tlk'
        
        try:
            # First try to find it in the module itself
            if self._module_parser:
                tlk_data = self._module_parser.extract_resource(tlk_filename)
                if tlk_data:
                    # Save to temp file and parse
                    import tempfile
                    with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as tmp:
                        tmp.write(tlk_data)
                        tmp_path = tmp.name
                    
                    parser = TLKParser()
                    parser.read(tmp_path)
                    os.unlink(tmp_path)
                    
                    self._custom_tlk_cache = parser
                    logger.info(f"Loaded custom TLK from module: {tlk_filename}")
                    return
            
            # Try to find in tlk directory
            tlk_dirs = [
                self.nwn2_path / "tlk",
                self.nwn2_path / "localization" / "english" / "tlk",
            ]
            
            for tlk_dir in tlk_dirs:
                tlk_path = tlk_dir / tlk_filename
                if tlk_path.exists():
                    parser = TLKParser()
                    parser.read(str(tlk_path))
                    self._custom_tlk_cache = parser
                    logger.info(f"Loaded custom TLK: {tlk_path}")
                    return
            
            # Check Steam Workshop mods
            for override_dir in self._override_dirs:
                if 'workshop' in str(override_dir):
                    # Check in mod root (where many workshop mods put TLK files)
                    mod_root = override_dir.parent
                    tlk_path = mod_root / tlk_filename
                    if tlk_path.exists():
                        parser = TLKParser()
                        parser.read(str(tlk_path))
                        self._custom_tlk_cache = parser
                        logger.info(f"Loaded custom TLK from workshop: {tlk_path}")
                        return
                    
                    # Also check in tlk subdirectory
                    tlk_path = mod_root / 'tlk' / tlk_filename
                    if tlk_path.exists():
                        parser = TLKParser()
                        parser.read(str(tlk_path))
                        self._custom_tlk_cache = parser
                        logger.info(f"Loaded custom TLK from workshop tlk dir: {tlk_path}")
                        return
            
            logger.warning(f"Custom TLK file not found: {tlk_filename}")
            
        except Exception as e:
            logger.error(f"Error loading custom TLK {tlk_filename}: {e}")
    
    def _load_module_2das(self):
        """Load 2DA files from the module itself"""
        if self._module_parser:
            # Regular .mod file - extract 2DAs from ERF
            for resource in self._module_parser.list_resources(resource_type=ERFResourceType.TDA):
                name = resource['name'].lower()
                if name.endswith('.2da'):
                    try:
                        data = self._module_parser.extract_resource(resource['name'])
                        tda_parser = self._parse_2da_from_bytes(data)
                        if tda_parser:
                            self._module_overrides[name] = tda_parser
                            logger.debug(f"Loaded module override for {name}")
                    except Exception as e:
                        logger.error(f"Error loading module 2DA {name}: {e}")
        elif self._module_path and self._module_path.is_dir():
            # Module directory (e.g., from campaign) - look for 2DA files in directory
            for tda_file in self._module_path.glob('*.2da'):
                try:
                    # Track modification time
                    self._file_mod_times[str(tda_file)] = tda_file.stat().st_mtime
                    
                    parser = TDAParser()
                    parser.read(str(tda_file))
                    name = tda_file.name.lower()
                    self._module_overrides[name] = parser
                    logger.debug(f"Loaded module override from directory for {name}")
                except Exception as e:
                    logger.error(f"Error loading campaign 2DA {tda_file.name}: {e}")
    
    def _load_hakpak_to_override_chain(self, hakpak_name: str):
        """Load a hakpak into the ordered override chain"""
        if not hakpak_name.endswith('.hak'):
            hakpak_name += '.hak'
        
        hakpak_path = self._find_hakpak(hakpak_name)
        if not hakpak_path:
            logger.warning(f"Hakpak '{hakpak_name}' not found")
            return
        
        try:
            parser = ErfParser()
            parser.read(str(hakpak_path))
            self._erf_parsers[hakpak_name] = parser
            
            # Check for associated TLK file if we don't already have a custom TLK
            if not self._custom_tlk_cache:
                self._check_for_hak_tlk(hakpak_path)
            
            # Create override dict for this HAK
            hak_overrides = {}
            
            for resource in parser.list_resources(resource_type=ERFResourceType.TDA):
                name = resource['name'].lower()
                if name.endswith('.2da'):
                    try:
                        data = parser.extract_resource(resource['name'])
                        tda_parser = self._parse_2da_from_bytes(data)
                        if tda_parser:
                            hak_overrides[name] = tda_parser
                    except Exception as e:
                        logger.error(f"Error loading 2DA {name} from {hakpak_name}: {e}")
            
            if hak_overrides:
                self._hak_overrides.append(hak_overrides)
                logger.info(f"Loaded {len(hak_overrides)} 2DA overrides from {hakpak_name}")
                
        except Exception as e:
            logger.error(f"Error loading hakpak {hakpak_name}: {e}")
    
    def _scan_override_directories(self, module_context: Optional[Dict[str, Any]] = None):
        """
        Scan traditional override and Steam Workshop directories
        
        Args:
            module_context: Optional dict containing module information with keys:
                - 'module_name': Name of the module
                - 'module_path': Path to the .mod file
        """
        # Clear previous scans
        self._override_dir_overrides.clear()
        self._workshop_overrides.clear()
        self._override_dirs.clear()
        
        # Also clear file path mappings for overrides
        self._override_file_paths = {}  # Maps 2da name to file path
        self._workshop_file_paths = {}  # Maps 2da name to file path
        self._custom_override_paths = {}  # Maps 2da name to file path for custom dirs
        
        # Use nwn2_paths for user override directory
        override_dir = nwn2_paths.user_override
        
        if override_dir.exists():
            self._override_dirs.append(override_dir)
            self._index_directory_for_2das(override_dir, self._override_file_paths)
            logger.info(f"Found {len(self._override_file_paths)} 2DAs in override directory")
        
        # Use configured Steam workshop folder from nwn2_paths only
        workshop_dirs = []
        if nwn2_paths.steam_workshop_folder and nwn2_paths.steam_workshop_folder.exists():
            workshop_dirs.append(nwn2_paths.steam_workshop_folder)
        
        for workshop_dir in workshop_dirs:
            if workshop_dir.exists():
                for workshop_item in workshop_dir.iterdir():
                    if workshop_item.is_dir():
                        # Check for custom TLK in workshop item root
                        if not self._custom_tlk_cache:
                            self._check_workshop_item_for_tlk(workshop_item)
                        
                        workshop_override = workshop_item / 'override'
                        if workshop_override.exists():
                            self._override_dirs.append(workshop_override)
                            # Check subdirectories like override/2DA/
                            tda_subdir = workshop_override / '2DA'
                            if tda_subdir.exists():
                                self._index_directory_for_2das(tda_subdir, self._workshop_file_paths)
                            # Also check root override
                            self._index_directory_for_2das(workshop_override, self._workshop_file_paths)
                
                if self._workshop_file_paths:
                    logger.info(f"Found {len(self._workshop_file_paths)} 2DAs in Steam Workshop")
                break  # Found workshop dir, no need to check others
        
        # Scan custom override directories from nwn2_paths
        for custom_dir in nwn2_paths.custom_override_folders:
            if custom_dir.exists():
                self._custom_override_dirs.append(custom_dir)
                self._index_directory_for_2das(custom_dir, self._custom_override_paths)
        
        # Also scan any directories added via add_custom_override_directory
        for custom_dir in self._custom_override_dirs:
            if custom_dir not in nwn2_paths.custom_override_folders and custom_dir.exists():
                self._index_directory_for_2das(custom_dir, self._custom_override_paths)
        
        if self._custom_override_paths:
            logger.info(f"Found {len(self._custom_override_paths)} 2DAs in custom override directories")
        
        # Check for workshop mod overrides that need cache invalidation
        workshop_overrides = []
        critical_files = ['classes.2da', 'spells.2da', 'feat.2da', 'baseitems.2da', 'appearance.2da']
        
        for filename in critical_files:
            # Check if workshop mods have this file
            if filename in self._workshop_file_paths:
                workshop_overrides.append(filename)
                logger.info(f"Workshop override detected: {filename} in {self._workshop_file_paths[filename]}")
            # Also check override directories
            elif filename in self._override_file_paths:
                workshop_overrides.append(filename) 
                logger.info(f"User override detected: {filename} in {self._override_file_paths[filename]}")
        
        # Invalidate cache for workshop/override files
        if workshop_overrides:
            self._invalidate_cache_for_overrides(workshop_overrides, 'workshop_mod')
        
        # Load module-specific content if context is provided
        if module_context:
            module_name = module_context.get('module_name', '')
            module_path = module_context.get('module_path', '')
            
            if module_name and module_path:
                logger.info(f"Loading module context: {module_name}")
                try:
                    # Load the specific module for this context
                    if self.set_module(module_path):
                        logger.info(f"Successfully loaded module: {module_name}")
                    else:
                        logger.warning(f"Failed to load module: {module_name}")
                except Exception as e:
                    logger.error(f"Error loading module {module_name}: {e}")
            else:
                logger.info("Module context provided but missing module_name or module_path")
        else:
            logger.debug("No module context provided - scanning overrides only")
    
    def _index_directory_for_2das(self, directory: Path, target_dict: Dict[str, Path]):
        """Index 2DA files in a directory without parsing them using optimized scanner"""
        # Use optimized directory walker
        resource_locations = self._directory_walker.index_directory(str(directory), recursive=True)
        
        # Convert to legacy format
        for resource_name, resource_location in resource_locations.items():
            file_path = Path(resource_location.source_path)
            
            # Track modification time
            self._file_mod_times[str(file_path)] = resource_location.modified_time
            
            # Store the path in target dict
            target_dict[resource_name.lower()] = file_path
            
        logger.debug(f"Directory indexing: {len(resource_locations)} 2DA files in {directory}")
    
    def _preload_all_base_2das(self):
        """Preload all base game 2DAs into memory"""
        logger.info("Preloading base game 2DAs into memory...")
        logger.info(f"Cache directory: {self.cache_dir}")
        start_time = time.time()
        
        # Check if we should use smart preloading
        smart_preload = getattr(settings, 'NWN2_SMART_PRELOAD', True)
        if smart_preload:
            return self._smart_preload_2das()
        
        # Load all .msgpack files directly into memory cache
        loaded = 0
        failed = 0
        
        if self.cache_dir.exists():
            for msgpack_file in self.cache_dir.glob('*.msgpack'):
                if msgpack_file.stem == 'tlk_cache':
                    continue  # Skip TLK cache
                
                name = msgpack_file.stem.lower()
                if not name.endswith('.2da'):
                    name = name + '.2da'
                
                try:
                    # Load from cache directly
                    parser = TDACacheHelper.load_tda(msgpack_file.with_suffix(''))
                    if parser is None:
                        failed += 1
                        continue
                    
                    # Store in memory cache with base game key (no module context)
                    self._2da_cache[name] = parser
                    self._2da_compressed[name] = False
                    loaded += 1
                    logger.debug(f"Preloaded {name} from cache")
                    
                except Exception as e:
                    logger.warning(f"Failed to load cached {name}: {e}")
                    failed += 1
        
        elapsed = time.time() - start_time
        logger.info(f"Preloaded {loaded} 2DAs in {elapsed:.2f}s ({failed} failed)")
        
        # Update memory usage
        self._update_cache_memory_usage()
        logger.info(f"Current 2DA cache memory usage: {self._cache_memory_bytes / 1024 / 1024:.1f} MB")
    
    # HAK Discovery & Loading
    
    def _find_hakpak(self, hakpak_name: str) -> Optional[Path]:
        """Find hakpak in standard locations"""
        # Check custom HAK folders first (highest priority)
        for custom_hak_dir in nwn2_paths.custom_hak_folders:
            custom_hak = custom_hak_dir / hakpak_name
            if custom_hak.exists():
                return custom_hak
        
        # Check user hak directory
        user_hak = nwn2_paths.user_hak / hakpak_name
        if user_hak.exists():
            return user_hak
        
        # Check NWN2 installation hak directory
        install_hak = nwn2_paths.hak / hakpak_name
        if install_hak.exists():
            return install_hak
        
        return None

    def find_module(self, module_name: str) -> Optional[str]:
        """
        Find a module file or campaign module in standard locations
        
        Args:
            module_name: Name of module (with or without .mod extension)
                        Can also be a campaign module directory name
            
        Returns:
            Full path to module file or campaign module directory, or None
        """
        # First check if it's a .mod file
        if not module_name.endswith('.mod'):
            mod_file_name = module_name + '.mod'
        else:
            mod_file_name = module_name
            module_name = module_name[:-4]  # Remove .mod for campaign search
        
        # Check custom module folders first (highest priority)
        for custom_module_dir in nwn2_paths.custom_module_folders:
            custom_module = custom_module_dir / mod_file_name
            if custom_module.exists():
                return str(custom_module)
        
        # Check user modules
        user_modules = nwn2_paths.user_modules / mod_file_name
        if user_modules.exists():
            return str(user_modules)
        
        # Check NWN2 installation modules
        install_modules = nwn2_paths.modules / mod_file_name
        if install_modules.exists():
            return str(install_modules)
        
        # Check campaign modules (unpacked directories with MODULE.IFO)
        campaigns_dirs = [
            nwn2_paths.campaigns,
            nwn2_paths.game_folder / 'campaigns'  # lowercase version
        ]
        
        for campaigns_dir in campaigns_dirs:
            if campaigns_dir.exists():
                # Look in each campaign directory
                for campaign_dir in campaigns_dir.iterdir():
                    if campaign_dir.is_dir():
                        # Check for .mod file in campaign
                        module_path = campaign_dir / mod_file_name
                        if module_path.exists():
                            return str(module_path)
                        
                        # Check for unpacked module directory
                        module_dir = campaign_dir / module_name
                        if module_dir.is_dir():
                            # Verify it has MODULE.IFO
                            module_ifo = module_dir / 'MODULE.IFO'
                            if module_ifo.exists():
                                return str(module_dir)
        
        return None

    def _build_module_hak_index(self):
        """
        Build an index mapping module names to their required HAKs.
        This enables save-context-aware loading without performance overhead.
        """
        if self._modules_indexed:
            return
        
        start_time = time.time()
        
        self._module_to_haks.clear()
        
        # Search all module directories
        module_dirs = [
            nwn2_paths.modules,           # Installation modules
            nwn2_paths.user_modules,      # User modules  
            nwn2_paths.campaigns,         # Campaign modules
        ]
        
        # Add custom module folders
        for custom_dir in nwn2_paths.custom_module_folders:
            module_dirs.append(custom_dir)
        
        modules_found = 0
        modules_skipped = 0
        total_haks = 0
        for module_dir in module_dirs:
            if not module_dir.exists():
                continue
                
            # Scan for .mod files
            for mod_file in module_dir.glob('*.mod'):
                # Skip vanilla modules - we already have their content
                if mod_file.name in self._vanilla_modules:
                    logger.debug(f"Skipping vanilla module: {mod_file.name}")
                    modules_skipped += 1
                    continue
                
                try:
                    extraction_result = self._extract_module_info(mod_file)
                    if extraction_result:
                        module_info = extraction_result['info']
                        module_name = module_info['name']
                        hak_list = module_info['haks']
                        
                        self._module_to_haks[module_name] = {
                            'mod_file': str(mod_file),
                            'haks': hak_list,
                            'custom_tlk': module_info.get('custom_tlk', ''),
                            'indexed_at': time.time()
                        }
                        
                        modules_found += 1
                        total_haks += len(hak_list)
                        logger.debug(f"Indexed custom module '{module_name}' with {len(hak_list)} HAKs")
                        
                except Exception as e:
                    logger.debug(f"Error indexing module {mod_file}: {e}")
                    continue
        
        self._modules_indexed = True
        elapsed_ms = int((time.time() - start_time) * 1000)
        logger.info(f"Module scanning: found {modules_found} custom modules with {total_haks} total HAKs in {elapsed_ms}ms")

    # Module Information Extraction
    
    def _extract_gff_string(self, gff_data: Any, default: str = '') -> str:
        """
        Extract a string from a potentially localized GFF data structure.
        
        Args:
            gff_data: The GFF field data which might be a string or localized dict
            default: Default value if extraction fails
            
        Returns:
            Extracted string value
        """
        if isinstance(gff_data, dict) and 'substrings' in gff_data:
            substrings = gff_data.get('substrings', [])
            if substrings and len(substrings) > 0:
                return substrings[0].get('string', default)
            return default
        elif isinstance(gff_data, str):
            return gff_data
        return default

    def _extract_campaign_id(self, campaign_id_raw) -> str:
        """Extract Campaign_ID GUID, handling bytes or string format"""
        if isinstance(campaign_id_raw, bytes):
            return campaign_id_raw.hex()
        return str(campaign_id_raw) if campaign_id_raw else ''

    def _extract_module_info(self, mod_file: Path) -> Optional[Dict[str, Any]]:
        """
        Extract module name and HAK list from a .mod file.
        
        Args:
            mod_file: Path to .mod file
            
        Returns:
            Dict with 'info' containing module info and 'parser' containing the ErfParser instance,
            or None if extraction fails
        """
        try:
            # Parse .mod file as ERF archive
            parser = ErfParser()
            parser.read(str(mod_file))
            
            # Extract module.ifo
            module_ifo_data = parser.extract_resource('module.ifo')
            if not module_ifo_data:
                logger.debug(f"No module.ifo found in {mod_file}")
                return None
            
            # Parse module.ifo as GFF
            import tempfile
            with tempfile.NamedTemporaryFile(suffix='.ifo', delete=False) as tmp:
                tmp.write(module_ifo_data)
                tmp_path = tmp.name
            
            module_data = GffParser(tmp_path).to_dict()
            os.unlink(tmp_path)
            
            # Get module name using helper
            mod_name = self._extract_gff_string(module_data.get('Mod_Name'), '')
            
            # Get HAK list
            hak_list = module_data.get('Mod_HakList', [])
            hak_names = []
            for hak_entry in hak_list:
                hak_name = hak_entry.get('Mod_Hak', '')
                if hak_name:
                    hak_names.append(hak_name)
            
            # Get custom TLK
            custom_tlk = module_data.get('Mod_CustomTlk', '')
            
            module_info = {
                'name': mod_name,
                'haks': hak_names,
                'custom_tlk': custom_tlk,
                'mod_id': module_data.get('Mod_ID', ''),
                'entry_area': module_data.get('Mod_Entry_Area', ''),
                'campaign_id': self._extract_campaign_id(module_data.get('Campaign_ID', ''))
            }
            
            return {'info': module_info, 'parser': parser}
            
        except Exception as e:
            logger.debug(f"Error extracting info from {mod_file}: {e}")
            return None

    def load_save_context(self, save_folder_path: Union[str, Path]) -> bool:
        """
        Load module context from a save folder for accurate content loading.
        
        This method reads CURRENTMODULE.TXT from a save folder and loads
        the appropriate module and its HAKs for save compatibility.
        
        Args:
            save_folder_path: Path to save folder containing CURRENTMODULE.TXT
            
        Returns:
            True if context was loaded successfully
        """
        save_path = Path(save_folder_path)
        
        if not save_path.exists() or not save_path.is_dir():
            logger.warning(f"Save folder not found: {save_path}")
            return False
        
        # Build module index if not already done
        self._build_module_hak_index()
        
        # Read current module from save
        current_module_file = save_path / "CURRENTMODULE.TXT"
        if not current_module_file.exists():
            logger.info(f"No CURRENTMODULE.TXT found in {save_path} - using base game content only")
            return True  # Not an error - might be base campaign save
        
        try:
            module_name = current_module_file.read_text(encoding='utf-8').strip()
            logger.info(f"Save was created in module: '{module_name}'")
            
            # Find module in our index
            if module_name not in self._module_to_haks:
                logger.warning(f"Module '{module_name}' not found in index. Available modules: {list(self._module_to_haks.keys())[:5]}...")
                return False
            
            # Load the module and its HAKs
            module_info = self._module_to_haks[module_name]
            mod_file_path = module_info['mod_file']
            required_haks = module_info['haks']
            
            logger.info(f"Loading module context: {len(required_haks)} HAKs required")
            for hak in required_haks:
                logger.debug(f"  - {hak}")
            
            # Use existing set_module method to load everything
            success = self.set_module(mod_file_path)
            if success:
                logger.info(f"Successfully loaded save context for module '{module_name}'")
            else:
                logger.error(f"Failed to load module '{module_name}' from {mod_file_path}")
            
            return success
            
        except Exception as e:
            logger.error(f"Error loading save context from {save_path}: {e}")
            return False

    def get_module_index_stats(self) -> Dict[str, Any]:
        """Get statistics about the module-to-HAK index."""
        if not self._modules_indexed:
            self._build_module_hak_index()
        
        total_haks = 0
        modules_with_haks = 0
        
        for module_info in self._module_to_haks.values():
            hak_count = len(module_info['haks'])
            total_haks += hak_count
            if hak_count > 0:
                modules_with_haks += 1
        
        return {
            'total_modules': len(self._module_to_haks),
            'modules_with_haks': modules_with_haks,
            'total_haks_referenced': total_haks,
            'vanilla_modules_skipped': len(self._vanilla_modules),
            'indexed': self._modules_indexed,
            'sample_modules': list(self._module_to_haks.keys())[:10]
        }
    
    # ============================================================================
    # CACHE MANAGEMENT
    # ============================================================================
    
    def _compress_parser(self, parser: TDAParser) -> Tuple[bytes, int, int]:
        """Compress a TDAParser object using zlib.
        
        Returns:
            Tuple of (compressed_data, original_size, compressed_size)
        """
        # Use Rust parser's built-in compressed msgpack serialization
        try:
            # For Rust parser, use its own compressed serialization
            compressed = parser.to_msgpack_bytes()
            original_size = len(compressed) * 3  # Approximate uncompressed size
            compressed_size = len(compressed)
            return compressed, original_size, compressed_size
        except AttributeError:
            # Fallback for Python parser compatibility
            import msgpack
            serializable = SafeCache._make_serializable(parser)
            packed = msgpack.packb(serializable, use_bin_type=True)
            original_size = len(packed)
            compressed = zlib.compress(packed, level=6)
            compressed_size = len(compressed)
            return compressed, original_size, compressed_size
    
    def _decompress_parser(self, compressed_data: bytes) -> TDAParser:
        """Decompress and unpack a TDAParser object."""
        try:
            # For Rust parser, use its own deserialization
            return TDAParser.from_msgpack_bytes(compressed_data)
        except AttributeError:
            # Fallback for Python parser compatibility
            import msgpack
            decompressed = zlib.decompress(compressed_data)
            data = msgpack.unpackb(decompressed, raw=False)
            restored = SafeCache._restore_objects(data)
            
            # Reconstruct TDAParser from dict
            if isinstance(restored, dict):
                parser = TDAParser()
                for key, value in restored.items():
                    setattr(parser, key, value)
                return parser
            elif isinstance(restored, TDAParser):
                return restored
            else:
                raise ValueError(f"Unexpected type after decompression: {type(restored)}")
    
    def _should_compress(self, parser: TDAParser) -> bool:
        """Determine if a parser should be compressed based on size."""
        if not self._compression_enabled:
            return False
        
        # Estimate size
        try:
            # For Rust parser, estimate based on row/column count
            if hasattr(parser, 'get_resource_count'):
                # Rust parser - estimate size based on content
                rows = parser.get_resource_count()
                cols = parser.get_column_count() if hasattr(parser, 'get_column_count') else 10
                estimated_size_kb = (rows * cols * 20) / 1024  # Rough estimate: 20 bytes per cell
                return estimated_size_kb > self._compression_threshold
            else:
                # Python parser fallback
                import msgpack
                serializable = SafeCache._make_serializable(parser)
                packed = msgpack.packb(serializable, use_bin_type=True)
                size_kb = len(packed) / 1024
                return size_kb > self._compression_threshold
        except:
            return False
    
    def _update_cache_memory_usage(self):
        """Estimate memory usage of cached 2DAs"""
        self._cache_memory_bytes = 0
        for name, data in self._2da_cache.items():
            try:
                if name in self._2da_compressed and self._2da_compressed[name]:
                    # Compressed entry - just use the byte size
                    size = len(data) if isinstance(data, bytes) else sys.getsizeof(data)
                else:
                    # Uncompressed parser - estimate size more accurately
                    # Use msgpack to get a better size estimate
                    import msgpack
                    serialized = msgpack.packb(SafeCache._make_serializable(data), use_bin_type=True)
                    size = len(serialized)
                self._cache_memory_bytes += int(size)
            except Exception as e:
                logger.debug(f"Could not estimate size of {name}: {e}")
    
    def _build_cache_key(self, name: str) -> str:
        """Build a cache key that includes module context"""
        # Include module path in cache key to handle different override chains
        if self._current_module:
            return f"{self._current_module}:{name}"
        return name
    
    def _evict_lru_items(self):
        """Evict least recently used items when cache is too large"""
        logger.info(f"Cache size exceeded {self._cache_max_mb} MB, evicting LRU items...")
        
        # Since we're using OrderedDict, the first items are the least recently used
        to_remove = int(len(self._2da_cache) * 0.2)
        if to_remove < 1:
            to_remove = 1
            
        # Remove oldest items
        removed = 0
        keys_to_remove = []
        for key in self._2da_cache:
            keys_to_remove.append(key)
            removed += 1
            if removed >= to_remove:
                break
        
        for key in keys_to_remove:
            del self._2da_cache[key]
            if key in self._2da_compressed:
                del self._2da_compressed[key]
            
        logger.info(f"Evicted {len(keys_to_remove)} items from cache")
        self._update_cache_memory_usage()

    def _is_file_modified(self, filepath: Path) -> bool:
        """Check if a file has been modified since it was cached"""
        str_path = str(filepath)
        
        # Get current modification time
        try:
            current_mtime = filepath.stat().st_mtime
        except (OSError, IOError):
            # File might have been deleted
            return True
        
        # Check if we have a recorded modification time
        if str_path not in self._file_mod_times:
            # First time seeing this file
            self._file_mod_times[str_path] = current_mtime
            return False
        
        # Compare modification times with small tolerance for file system precision
        cached_mtime = self._file_mod_times[str_path]
        if abs(current_mtime - cached_mtime) < 0.001:  # Less than 1ms difference
            # File hasn't changed (within tolerance)
            return False
        elif current_mtime > cached_mtime:
            # File has been modified
            self._file_mod_times[str_path] = current_mtime
            return True
        
        return False

    def _invalidate_cache_for_file(self, filepath: Path):
        """Invalidate caches related to a modified file"""
        filename = filepath.name.lower()
        
        # Remove from override caches
        if filename in self._override_dir_overrides:
            del self._override_dir_overrides[filename]
            logger.info(f"Invalidated override cache for {filename}")
        
        if filename in self._workshop_overrides:
            del self._workshop_overrides[filename]
            logger.info(f"Invalidated workshop cache for {filename}")
        
        # Also remove from main 2DA cache if present
        if filename.endswith('.2da'):
            # Need to check all cache keys that might contain this file
            keys_to_remove = []
            for cache_key in list(self._2da_cache.keys()):
                if cache_key.endswith(filename) or cache_key.endswith(filename[:-4]):
                    keys_to_remove.append(cache_key)
            
            for key in keys_to_remove:
                del self._2da_cache[key]
                if key in self._2da_compressed:
                    del self._2da_compressed[key]
                logger.info(f"Invalidated cache for {key}")
    
    # ============================================================================
    # 2DA OPERATIONS
    # ============================================================================
    
    def _parse_2da_file(self, file_path: Path) -> Optional[TDAParser]:
        """Parse a single 2DA file from disk"""
        try:
            parser = TDAParser()
            parser.read(str(file_path))
            return parser
        except Exception as e:
            logger.error(f"Error parsing 2DA file {file_path}: {e}")
            return None
    
    def _parse_2da_from_bytes(self, data: bytes) -> Optional[TDAParser]:
        """Parse 2DA data from bytes - let Rust parser handle all validation"""
        try:
            # Basic validation - check if we have data
            if not data.strip():
                logger.warning("Empty 2DA data provided")
                return None
            
            # Parse using TDAParser - it handles all validation including headers
            parser = TDAParser()
            parser.parse_from_bytes(data)
            
            # Validate that we got some data (using Rust parser API)
            if parser.get_resource_count() == 0:
                logger.warning("2DA parsed but contains no rows")
                return None
                
            return parser
        except UnicodeDecodeError as e:
            logger.error(f"Failed to decode 2DA data as UTF-8: {e}")
            return None
        except Exception as e:
            logger.error(f"Unexpected error parsing 2DA from bytes: {type(e).__name__}: {e}")
            return None
    
    @with_retry_limit(table_name_param="name")
    def get_2da(self, name: str) -> Optional[TDAParser]:
        """
        Get a parsed 2DA file by name (base game files only) with retry limits to prevent infinite loops.
        
        Args:
            name: Name of 2DA file (e.g., "classes" or "classes.2da")
            
        Returns:
            Parsed TDAParser object or None if not found
        """
        # Normalize name
        if not name.lower().endswith('.2da'):
            name = name + '.2da'
        name = name.lower()
        
        # Check pre-compiled cache first (fastest)
        cached_parser = self._precompiled_cache.get_cached_table(name)
        if cached_parser:
            self._cache_hits += 1
            return cached_parser
        
        # Check memory cache next
        # if self._memory_cache_enabled and name in self._2da_cache:
        #     self._cache_hits += 1
        #         self._2da_compressed[name] = False
        #         return decompressed
        #     return self._2da_cache[name]
        
        if self._memory_cache_enabled:
            self._cache_misses += 1
        
        # Old disk cache removed - precompiled cache handles persistence now
        
        # Load from ZIP if not in cache
        if name not in self._2da_locations:
            logger.debug(f"2DA file '{name}' not found in base game")
            return None
            
        zip_path_str, file_path = self._2da_locations[name]
        
        # Track expansion loading for summary
        zip_name = Path(zip_path_str).name
        if 'x1' in zip_name.lower() or 'x2' in zip_name.lower():
            expansion_key = 'x1' if 'x1' in zip_name.lower() else 'x2'
            self._expansion_files_loaded[expansion_key] = self._expansion_files_loaded.get(expansion_key, 0) + 1
        
        parser = TDAParser()
        try:
            # Read file using Rust reader
            data = self._zip_reader.read_file_from_zip(zip_path_str, file_path)
            
            self._last_successful_content = (name, data.decode('utf-8', errors='ignore')[:200])
            
            parser.parse_from_bytes(data)
        except ValueError as e:
            logger.error(f"ERROR parsing {name} from {zip_path_str}/{file_path}: {e}")
            # Get text content for debugging
            try:
                text = data.decode('utf-8', errors='ignore')
                logger.error(f"First 200 chars of content: {repr(text[:200])}")
                logger.error(f"First 5 lines:")
                lines = text.split('\n')[:5]
                for i, line in enumerate(lines):
                    logger.error(f"  Line {i}: {repr(line)}")
            except:
                logger.error("Could not decode content for debugging")
            
            # Print previous successful file
            if hasattr(self, '_last_successful_content'):
                prev_name, prev_content = self._last_successful_content
                logger.error(f"\nPrevious successful file was: {prev_name}")
                logger.error(f"Its first 200 chars: {repr(prev_content)}")
            raise
            
        self._add_to_cache(name, parser)
        
        # Log expansion summary if this was an expansion file
        zip_name = Path(zip_path_str).name
        if ('x1' in zip_name.lower() or 'x2' in zip_name.lower()) and not self._expansion_summary_logged:
            self._log_expansion_summary()
        
        return parser
    
    def _add_to_cache(self, name: str, parser: TDAParser):
        """Add a parser to the cache with optional compression."""
        if not self._memory_cache_enabled:
            return
            
        if self._should_compress(parser):
            compressed, original_size, compressed_size = self._compress_parser(parser)
            self._2da_cache[name] = compressed
            self._2da_compressed[name] = True
            
            compression_ratio = 1.0 - (compressed_size / original_size)
            self._compression_ratio_sum += compression_ratio
            self._compression_count += 1
            
            logger.debug(f"Compressed {name}: {original_size/1024:.1f}KB -> {compressed_size/1024:.1f}KB (ratio: {compression_ratio:.1%})")
        else:
            self._2da_cache[name] = parser
            self._2da_compressed[name] = False
            
        self._update_cache_memory_usage()
        if self._cache_memory_bytes > self._cache_max_mb * 1024 * 1024:
            self._evict_lru_items()
    

    def _log_expansion_summary(self):
        """Log a summary of loaded expansion files, called only once"""
        if self._expansion_summary_logged or not self._expansion_files_loaded:
            return
            
        total_files = sum(self._expansion_files_loaded.values())
        expansion_names = {'x1': 'Mask of the Betrayer', 'x2': 'Storm of Zehir'}
        
        if total_files > 0:
            details = []
            for exp_key, count in self._expansion_files_loaded.items():
                exp_name = expansion_names.get(exp_key, exp_key.upper())
                details.append(f"{exp_name}: {count}")
            
            logger.info(f"Loaded expansion campaign 2da files {total_files}/{total_files} successfully ({', '.join(details)})")
            self._expansion_summary_logged = True
    
    # ============================================================================
    # TLK OPERATIONS
    # ============================================================================
    
    # Core TLK Access
    
    def get_tlk(self, language: str = "english") -> Optional[TLKParser]:
        """Get the TLK parser for game text"""
        if self._tlk_cache:
            return self._tlk_cache
            
        # Use nwn2_paths for dialog.tlk
        tlk_paths = [
            nwn2_paths.dialog_tlk,
            nwn2_paths.game_folder / "localization" / language / "dialog.tlk",
            nwn2_paths.game_folder / "tlk" / "dialog.tlk",
        ]
        
        for tlk_path in tlk_paths:
            if tlk_path.exists():
                parser = TLKParser()
                parser.read(str(tlk_path))
                self._tlk_cache = parser
                return parser
                
        if not self.suppress_warnings:
            logger.warning("dialog.tlk not found")
        return None
    
    def get_string(self, str_ref: int) -> str:
        """Get a localized string from TLK, checking custom TLK first"""
        # Check custom TLK first if loaded
        if self._custom_tlk_cache:
            custom_string = self._custom_tlk_cache.get_string(str_ref)
            if custom_string:
                return custom_string
        
        # Fall back to base game TLK
        tlk = self.get_tlk()
        if not tlk:
            return f"{{StrRef:{str_ref}}}"
        return tlk.get_string(str_ref) or f"{{StrRef:{str_ref}}}"
    
    def get_strings_batch(self, str_refs: List[int]) -> Dict[int, str]:
        """
        Get multiple localized strings from TLK in one batch operation for performance.
        
        This method leverages the Rust TLK parser's high-performance batch retrieval
        to resolve thousands of string references quickly during data loading.
        
        Args:
            str_refs: List of string reference IDs to resolve
            
        Returns:
            Dictionary mapping str_ref -> resolved string
        """
        result = {}
        
        if not str_refs:
            return result
        
        # Process custom TLK first if available
        remaining_refs = []
        if self._custom_tlk_cache and hasattr(self._custom_tlk_cache, 'get_strings_batch'):
            try:
                # Use Rust TLK parser's batch method if available
                batch_result = self._custom_tlk_cache.get_strings_batch(str_refs)
                for str_ref in str_refs:
                    if str_ref in batch_result:
                        result[str_ref] = batch_result[str_ref]
                    else:
                        remaining_refs.append(str_ref)
            except (AttributeError, Exception):
                # Fallback to individual lookups for custom TLK
                for str_ref in str_refs:
                    custom_string = self._custom_tlk_cache.get_string(str_ref)
                    if custom_string:
                        result[str_ref] = custom_string
                    else:
                        remaining_refs.append(str_ref)
        else:
            remaining_refs = str_refs.copy()
        
        # Process remaining refs with base game TLK
        if remaining_refs:
            tlk = self.get_tlk()
            if tlk and hasattr(tlk, 'get_strings_batch'):
                try:
                    # Use Rust TLK parser's batch method
                    batch_result = tlk.get_strings_batch(remaining_refs)
                    for str_ref in remaining_refs:
                        if str_ref in batch_result:
                            result[str_ref] = batch_result[str_ref]
                        else:
                            result[str_ref] = f"{{StrRef:{str_ref}}}"
                except (AttributeError, Exception):
                    # Fallback to individual lookups
                    for str_ref in remaining_refs:
                        string_val = tlk.get_string(str_ref)
                        result[str_ref] = string_val or f"{{StrRef:{str_ref}}}"
            else:
                # No TLK available or no batch method
                for str_ref in remaining_refs:
                    if tlk:
                        string_val = tlk.get_string(str_ref)
                        result[str_ref] = string_val or f"{{StrRef:{str_ref}}}"
                    else:
                        result[str_ref] = f"{{StrRef:{str_ref}}}"
        
        return result
    
    
    # Old disk cache methods removed - using precompiled cache instead
    
    def _smart_preload_2das(self):
        """Smart preload based on ignore list from nw2_data_filtered.json"""
        logger.info("Using smart preload strategy...")
        start_time = time.time()
        
        # Load ignore prefixes from nw2_data_filtered.json
        ignore_prefixes = self._load_ignore_prefixes()
        if not ignore_prefixes:
            # Fallback to hardcoded list if file not found
            logger.warning("Failed to load ignore prefixes, using fallback")
            ignore_prefixes = [
                'ambientmusic', 'ambientsound', 'appearancesndset', 'bodybag',
                'container_preference', 'crafting', 'crft_', 'cursors',
                'des_', 'environ', 'footstepsounds', 'grass', 'hen_',
                'inventorysnds', 'itm_rand_', 'light', 'nwn2_', 'placeable',
                'sound', 'tcn01', 'tdc01', 'tde01', 'tdm01', 'tdr01', 'tds01',
                'texture', 'tile', 'time', 'treas_', 'trees', 'ttr01', 'tts01',
                'vfx_', 'video', 'visualeffects', 'water', 'waypoint'
            ]
        
        loaded = 0
        ignored = 0
        
        # Load all msgpack files from cache directory and module overrides
    
    def get_all_base_items(self) -> List[Dict[str, Any]]:
        """Get all available base items for creation"""
        # ... (implemented in inventory_manager, this seems like a placeholder or utility if present)
        pass

    def get_all_item_templates(self) -> Dict[str, Any]:
        """
        Get all available item templates from all sources (Zips, Overrides, Module).
        Returns a dict mapping ResRef (lowercased) to location info.
        """
        all_templates = {}
        
        # 1. Base Game & Expansions (Templates.zip) - Lowest Priority
        for resref, location in self._template_locations.items():
            all_templates[resref] = {
                'resref': resref,
                'source': 'base' if 'templates.zip' in location[0].lower() else 'expansion',
                'path': location[0],
                'internal_path': location[1],
                'container_type': 'zip'
            }
            
        # 2. Workshop Content
        for resref, path in self._workshop_file_paths.items():
            if resref.endswith('.uti'):
                all_templates[resref] = {
                    'resref': resref,
                    'source': 'workshop',
                    'path': str(path),
                    'container_type': 'file'
                }

        # 3. User Overrides
        for resref, path in self._override_file_paths.items():
            if resref.endswith('.uti'):
                all_templates[resref] = {
                    'resref': resref,
                    'source': 'override',
                    'path': str(path),
                    'container_type': 'file'
                }
                
        if self._module_parser:
             try:
                if hasattr(self._module_parser, 'get_resource_list'):
                    for res_name in self._module_parser.get_resource_list():
                        if res_name.lower().endswith('.uti'):
                            all_templates[res_name.lower()] = {
                                'resref': res_name.lower(),
                                'source': 'module',
                                'path': self._current_module,
                                'internal_path': res_name,
                                'container_type': 'erf'
                            }
             except Exception:
                 pass

        return all_templates

    def build_item_template_index(self) -> List[Dict[str, Any]]:
        all_templates = self.get_all_item_templates()

        baseitems_parser = self.get_2da('baseitems')
        baseitems_count = baseitems_parser.row_count() if baseitems_parser else 0

        index = []

        for resref, template_info in all_templates.items():
            try:
                data = None
                container_type = template_info.get('container_type')
                path = template_info.get('path')

                if container_type == 'zip':
                    internal_path = template_info.get('internal_path')
                    data = self._zip_reader.read_file_from_zip(path, internal_path)
                elif container_type == 'file':
                    with open(path, 'rb') as f:
                        data = f.read()
                elif container_type == 'erf' and self._module_parser:
                    data = self._module_parser.extract_resource(template_info.get('internal_path'))

                if not data:
                    continue

                gff = GffParser.from_bytes(data)
                base_item = gff.get_field('BaseItem') or 0
                loc_name = gff.get_field('LocalizedName') or {}

                strref = loc_name.get('string_ref', -1) if isinstance(loc_name, dict) else -1
                substrings = loc_name.get('substrings', []) if isinstance(loc_name, dict) else []

                if substrings:
                    name = substrings[0].get('string', '')
                elif strref >= 0:
                    name = self.get_string(strref)
                else:
                    name = resref.replace('.uti', '')

                # Strip NWN2 formatting tags like <color=...>, </color>, etc.
                if name and '<' in name:
                    name = re.sub(r'<[^>]+>', '', name)

                category = 4
                if baseitems_parser and 0 <= base_item < baseitems_count:
                    store_panel = baseitems_parser.get_string(base_item, 'StorePanel')
                    if store_panel and store_panel != '****':
                        try:
                            category = int(store_panel)
                        except ValueError:
                            pass

                index.append({
                    'resref': resref,
                    'name': name or resref.replace('.uti', ''),
                    'base_item': base_item,
                    'category': category,
                    'source': template_info.get('source', 'unknown')
                })

            except Exception as e:
                logger.debug(f"Failed to index template {resref}: {e}")
                continue

        index.sort(key=lambda x: x['name'].lower())
        return index

    def get_item_template_data(self, template_info: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        try:
            container_type = template_info.get('container_type')
            path = template_info.get('path')
            
            data = None
            
            if container_type == 'zip':
                internal_path = template_info.get('internal_path')
                with zipfile.ZipFile(path, 'r') as zf:
                    data = zf.read(internal_path)
                    
            elif container_type == 'file':
                with open(path, 'rb') as f:
                    data = f.read()
                    
            elif container_type == 'erf':
                if self._module_parser:
                    data = self._module_parser.extract_resource(template_info.get('internal_path'))
            
            if data:
                if GffParser:
                    if hasattr(GffParser, 'from_bytes'):
                         return GffParser.from_bytes(data).to_dict()
                    else:
                        import tempfile
                        with tempfile.NamedTemporaryFile(delete=False) as tmp:
                            tmp.write(data)
                            tmp_path = tmp.name
                        
                        try:
                            parsed = GffParser(tmp_path).to_dict()
                            return parsed
                        finally:
                           try:
                               os.unlink(tmp_path)
                           except:
                               pass
                               
            return None
        except Exception as e:
            logger.error(f"Error loading template {template_info}: {e}")
            return None
        if self.cache_dir.exists():
            # First load from base cache
            for msgpack_file in self.cache_dir.glob('*.msgpack'):
                if msgpack_file.stem == 'tlk_cache':
                    continue
                
                # Check if should ignore based on prefix
                stem = msgpack_file.stem.lower()
                if any(stem.startswith(prefix) for prefix in ignore_prefixes):
                    ignored += 1
                    continue
                
                name = stem if stem.endswith('.2da') else stem + '.2da'
                
                if name not in self._2da_cache:
                    try:
                        parser = TDACacheHelper.load_tda(msgpack_file.with_suffix(''))
                        if parser is not None:
                            self._2da_cache[name] = parser
                            self._2da_compressed[name] = False
                            loaded += 1
                    except Exception as e:
                        logger.debug(f"Failed to preload {name}: {e}")
                
                # Check memory limit periodically
                if loaded % 50 == 0:
                    self._update_cache_memory_usage()
                    if self._cache_memory_bytes > self._cache_max_mb * 1024 * 1024 * 0.8:
                        logger.info(f"Stopping preload at {loaded} files - approaching memory limit")
                        break
        
        # Always update memory usage at the end
        self._update_cache_memory_usage()
        
        elapsed = time.time() - start_time
        logger.info(f"Smart preloaded {loaded} 2DAs ({ignored} ignored) in {elapsed:.2f}s")
        logger.info(f"Current cache size: {self._cache_memory_bytes / 1024 / 1024:.1f} MB")
        
        return loaded
    
    def _load_ignore_prefixes(self) -> List[str]:
        """Load ignore prefixes from nw2_data_filtered.json"""
        try:
            json_path = self.cache_dir.parent / 'config' / 'nw2_data_filtered.json'
            if json_path.exists():
                import json
                with open(json_path, 'r') as f:
                    data = json.load(f)
                    return data.get('ignore_prefixes', [])
        except Exception as e:
            logger.debug(f"Failed to load ignore prefixes: {e}")
        return []
    
    def preload_common_tables(self):
        """Preload commonly used 2DA files"""
        common_tables = [
            'classes', 'racialtypes', 'racialsubtypes', 'feat', 'skills',
            'spells', 'baseitems', 'appearance', 'gender'
        ]
        
        logger.info("Preloading common 2DA tables...")
        for table in common_tables:
            self.get_2da(table)
    
    # String Resolution & Utilities
    
    def get_race_name(self, race_id: int) -> str:
        """Get race name from ID"""
        races = self.get_2da('racialtypes')  # Correct table name
        if races and 0 <= race_id < races.get_resource_count():
            name_ref = races.get_int(race_id, 'Name')
            if name_ref:
                return self.get_string(name_ref)
        return f"Unknown Race ({race_id})"
    
    def get_class_name(self, class_id: int) -> str:
        """Get class name from ID"""
        classes = self.get_2da('classes')
        if classes and 0 <= class_id < classes.get_resource_count():
            name_ref = classes.get_int(class_id, 'Name')
            if name_ref:
                return self.get_string(name_ref)
        return f"Unknown Class ({class_id})"
    
    def close(self):
        """Clean up resources (ZIP files are no longer kept open)"""
        # ZIP files are now opened on-demand and closed after use
        # So we just clear the location mappings
        self._zip_files.clear()
        
        # Clear ERF parsers
        self._erf_parsers.clear()
    
    # TLK Discovery & Loading
    
    def _check_for_hak_tlk(self, hakpak_path: Path):
        """Check for TLK file associated with a HAK file"""
        # Common patterns for TLK files:
        # 1. Same name as HAK: Kaedrin_PrC_Pack.hak -> Kaedrin_PrC_Pack.tlk
        # 2. In same directory as HAK
        # 3. In override folder
        # 4. In a tlk subdirectory
        
        hak_base = hakpak_path.stem  # filename without extension
        hak_dir = hakpak_path.parent
        
        # Locations to check for TLK files
        tlk_locations = [
            # Same directory as HAK
            hak_dir / f"{hak_base}.tlk",
            # Override directory
            nwn2_paths.user_override / f"{hak_base}.tlk",
            # tlk subdirectory in HAK directory
            hak_dir / "tlk" / f"{hak_base}.tlk",
            # tlk subdirectory in override
            nwn2_paths.user_override / "tlk" / f"{hak_base}.tlk",
        ]
        
        # Check Steam Workshop directories if this HAK is in workshop
        if 'workshop' in str(hakpak_path):
            workshop_root = hakpak_path
            while workshop_root.parent.name != 'workshop' and workshop_root.parent != workshop_root:
                workshop_root = workshop_root.parent
            if workshop_root.parent.name == 'workshop':
                # Get the specific workshop item directory
                item_dir = None
                for parent in hakpak_path.parents:
                    if parent.parent == workshop_root / 'content' / '2738630':
                        item_dir = parent
                        break
                
                if item_dir:
                    # Check workshop-specific locations
                    tlk_locations.extend([
                        item_dir / f"{hak_base}.tlk",
                        item_dir / "override" / f"{hak_base}.tlk",
                        item_dir / "tlk" / f"{hak_base}.tlk",
                        item_dir / "override" / "tlk" / f"{hak_base}.tlk",
                    ])
        
        # Try each location
        for tlk_path in tlk_locations:
            if tlk_path.exists():
                try:
                    parser = TLKParser()
                    parser.read(str(tlk_path))
                    self._custom_tlk_cache = parser
                    logger.info(f"Loaded TLK for HAK {hakpak_path.name}: {tlk_path}")
                    logger.info(f"Found and loaded TLK file for {hakpak_path.name}: {tlk_path.name}")
                    return
                except Exception as e:
                    logger.error(f"Error loading TLK {tlk_path}: {e}")
        
        logger.debug(f"No TLK file found for HAK {hakpak_path.name}")
    
    def _check_workshop_item_for_tlk(self, workshop_item: Path):
        """Check for custom TLK files in a Steam Workshop item"""
        # Workshop items typically have dialog.tlk in the root
        # Check both .tlk and .TLK extensions
        tlk_candidates = [
            workshop_item / 'dialog.tlk',
            workshop_item / 'dialog.TLK',
            workshop_item / 'Dialog.tlk',
            workshop_item / 'Dialog.TLK',
        ]
        
        # Also check for any .tlk file in root
        tlk_files = list(workshop_item.glob('*.tlk'))
        tlk_files.extend(workshop_item.glob('*.TLK'))
        
        # Try dialog.tlk first
        for tlk_path in tlk_candidates:
            if tlk_path.exists():
                try:
                    parser = TLKParser()
                    parser.read(str(tlk_path))
                    self._custom_tlk_cache = parser
                    logger.info(f"Loaded workshop TLK: {workshop_item.name}/dialog.tlk")
                    return
                except Exception as e:
                    logger.error(f"Error loading workshop TLK {tlk_path}: {e}")
        
        # If no dialog.tlk, try any .tlk file
        for tlk_path in tlk_files:
            if tlk_path.name.lower() != 'dialog.tlk':  # Skip if already tried
                try:
                    parser = TLKParser()
                    parser.read(str(tlk_path))
                    self._custom_tlk_cache = parser
                    logger.info(f"Loaded workshop TLK: {workshop_item.name}/{tlk_path.name}")
                    return
                except Exception as e:
                    logger.error(f"Error loading workshop TLK {tlk_path}: {e}")
    
    def _scan_workshop_tlk_fast(self):
        """Fast workshop TLK scanning using the same logic as slow path but without full directory indexing"""
        if self._custom_tlk_cache:
            logger.debug("TLK already loaded, skipping workshop TLK scan")
            return
            
        try:
            # Use the same workshop directory discovery as _scan_override_directories()
            from config.nwn2_settings import nwn2_paths
            
            # Use configured Steam workshop folder from nwn2_paths only (same as slow path lines 975-976)
            if nwn2_paths.steam_workshop_folder and nwn2_paths.steam_workshop_folder.exists():
                workshop_dir = nwn2_paths.steam_workshop_folder
                
                # Iterate through workshop items same as slow path (lines 980-984)
                for workshop_item in workshop_dir.iterdir():
                    if workshop_item.is_dir():
                        # Check for custom TLK in workshop item root (same call as slow path)
                        if not self._custom_tlk_cache:
                            self._check_workshop_item_for_tlk(workshop_item)
                        
                        # Stop at first TLK found for efficiency
                        if self._custom_tlk_cache:
                            break
            else:
                logger.debug("No Steam workshop directory configured or found")
                        
        except Exception as e:
            logger.warning(f"Error during fast workshop TLK scan: {e}")
    
    @with_retry_limit(table_name_param="name")
    def get_2da_with_overrides(self, name: str) -> Optional[TDAParser]:
        """
        Get a 2DA file, checking the full override chain with NWN2 engine priority:
        1. HAK overrides (first HAK in module.ifo = highest priority)
        2. Custom override directories (user-specified)
        3. Steam Workshop overrides
        4. Traditional override directory
        5. Campaign folder overrides
        6. Module content (2DAs inside .mod file)
        7. Base game files

        Args:
            name: Name of 2DA file

        Returns:
            Parsed TDAParser object or None
        """
        # Normalize name
        if not name.lower().endswith('.2da'):
            name = name + '.2da'
        name = name.lower()

        # Build cache key that includes module context
        cache_key = self._build_cache_key(name)

        # Check memory cache first if enabled
        if self._memory_cache_enabled:
            if cache_key in self._2da_cache:
                self._cache_hits += 1
                self._2da_cache.move_to_end(cache_key)
                if cache_key in self._2da_compressed and self._2da_compressed[cache_key]:
                    return self._decompress_parser(self._2da_cache[cache_key])
                return self._2da_cache[cache_key]

        self._cache_misses += 1

        # 1. Check HAK overrides (first HAK wins - no reverse!)
        for hak_overrides in self._hak_overrides:
            if name in hak_overrides:
                result = hak_overrides[name]
                if self._memory_cache_enabled and result:
                    self._add_to_cache(cache_key, result)
                return result

        # 2. Check custom override directories
        if name in self._custom_override_paths:
            parser = self._parse_2da_file(self._custom_override_paths[name])
            if parser:
                if self._memory_cache_enabled:
                    self._add_to_cache(cache_key, parser)
                return parser

        # 3. Check Steam Workshop overrides
        if name in self._workshop_overrides:
            result = self._workshop_overrides[name]
            if self._memory_cache_enabled and result:
                self._add_to_cache(cache_key, result)
            return result
        elif name in self._workshop_file_paths:
            parser = self._parse_2da_file(self._workshop_file_paths[name])
            if parser:
                self._workshop_overrides[name] = parser
                if self._memory_cache_enabled:
                    self._add_to_cache(cache_key, parser)
                return parser

        # 4. Check traditional override directory
        if name in self._override_dir_overrides:
            result = self._override_dir_overrides[name]
            if self._memory_cache_enabled and result:
                self._add_to_cache(cache_key, result)
            return result
        elif name in self._override_file_paths:
            parser = self._parse_2da_file(self._override_file_paths[name])
            if parser:
                self._override_dir_overrides[name] = parser
                if self._memory_cache_enabled:
                    self._add_to_cache(cache_key, parser)
                return parser

        # 5. Check campaign folder overrides (lazy load)
        if name in self._campaign_override_paths:
            if name not in self._campaign_overrides:
                parser = self._parse_2da_file(self._campaign_override_paths[name])
                if parser:
                    self._campaign_overrides[name] = parser
            if name in self._campaign_overrides:
                result = self._campaign_overrides[name]
                if self._memory_cache_enabled and result:
                    self._add_to_cache(cache_key, result)
                return result

        # 6. Check module overrides (2DAs inside .mod file)
        if name in self._module_overrides:
            result = self._module_overrides[name]
            if self._memory_cache_enabled and result:
                self._add_to_cache(cache_key, result)
            return result

        # 7. Fall back to base game
        result = self.get_2da(name)

        # 8. If still not found and it's a prerequisite table, create an empty one
        if not result and name.startswith('cls_pres_'):
            logger.info(f"Creating empty prerequisite table for missing file: {name}")
            result = self._create_empty_prerequisite_table(name)
            if self._memory_cache_enabled and result:
                cache_key = self._build_cache_key(name)
                self._add_to_cache(cache_key, result)

        # Cache the result if memory caching is enabled
        if self._memory_cache_enabled and result:
            cache_key = self._build_cache_key(name)
            self._add_to_cache(cache_key, result)
            self._update_cache_memory_usage()
            if self._cache_memory_bytes > self._cache_max_mb * 1024 * 1024:
                self._evict_lru_items()

        return result
    
    def _is_file_modified(self, filepath: Path) -> bool:
        """Check if a file has been modified since it was cached"""
        str_path = str(filepath)
        
        # Get current modification time
        try:
            current_mtime = filepath.stat().st_mtime
        except (OSError, IOError):
            # File might have been deleted
            return True
        
        # Check if we have a recorded modification time
        if str_path not in self._file_mod_times:
            # First time seeing this file
            self._file_mod_times[str_path] = current_mtime
            return False
        
        # Compare modification times with small tolerance for file system precision
        cached_mtime = self._file_mod_times[str_path]
        if abs(current_mtime - cached_mtime) < 0.001:  # Less than 1ms difference
            # File hasn't changed (within tolerance)
            return False
        elif current_mtime > cached_mtime:
            # File has been modified
            self._file_mod_times[str_path] = current_mtime
            return True
        
        return False
    
    def _invalidate_cache_for_file(self, filepath: Path):
        """Invalidate caches related to a modified file"""
        filename = filepath.name.lower()
        
        # Remove from override caches
        if filename in self._override_dir_overrides:
            del self._override_dir_overrides[filename]
            logger.info(f"Invalidated override cache for {filename}")
        
        if filename in self._workshop_overrides:
            del self._workshop_overrides[filename]
            logger.info(f"Invalidated workshop cache for {filename}")
        
        # Also remove from main 2DA cache if present
        if filename.endswith('.2da'):
            # Need to check all cache keys that might contain this file
            keys_to_remove = []
            for cache_key in list(self._2da_cache.keys()):
                if cache_key.endswith(filename) or cache_key.endswith(filename[:-4]):
                    keys_to_remove.append(cache_key)
            
            for key in keys_to_remove:
                del self._2da_cache[key]
                if key in self._2da_compressed:
                    del self._2da_compressed[key]
                logger.info(f"Invalidated cache for {key}")
    
    def check_for_modifications(self):
        """Check all tracked files for modifications and invalidate caches as needed"""
        modified_files = []
        
        # Check override directories
        for override_dir in self._override_dirs:
            if override_dir.exists():
                for file_path in override_dir.glob('*.2da'):
                    if self._is_file_modified(file_path):
                        self._invalidate_cache_for_file(file_path)
                        modified_files.append(file_path)
        
        # Check module files if loaded
        if self._module_path and self._module_path.exists():
            if self._is_file_modified(self._module_path):
                # Module file changed, need to reload it
                logger.warning(f"Module file {self._module_path} has been modified, consider reloading")
                modified_files.append(self._module_path)
        
        return modified_files
    
    def get_module_cache_stats(self) -> Dict[str, Any]:
        """Get statistics about the module LRU cache"""
        return self._module_cache.get_stats()
    
    def clear_module_cache(self):
        """Clear the module LRU cache"""
        self._module_cache.clear()
    
    def add_custom_override_directory(self, directory_path: Union[str, Path]) -> bool:
        """Add a custom override directory to the search path"""
        path = Path(directory_path)
        
        # Validate path
        if not path.exists():
            logger.error(f"Custom override directory does not exist: {path}")
            return False
        
        if not path.is_dir():
            logger.error(f"Custom override path is not a directory: {path}")
            return False
        
        # Check if already added
        if path in self._custom_override_dirs:
            logger.info(f"Custom override directory already added: {path}")
            return True
        
        # Add to list
        self._custom_override_dirs.append(path)
        
        # Index the directory
        self._index_directory_for_2das(path, self._custom_override_paths)
        
        logger.info(f"Added custom override directory: {path} (found {len(self._custom_override_paths)} 2DAs)")
        
        # Clear cached data to force re-evaluation with new overrides
        self._clear_override_caches()
        
        return True
    
    def remove_custom_override_directory(self, directory_path: Union[str, Path]) -> bool:
        """Remove a custom override directory from the search path"""
        path = Path(directory_path)
        
        if path not in self._custom_override_dirs:
            logger.warning(f"Custom override directory not found: {path}")
            return False
        
        # Remove from list
        self._custom_override_dirs.remove(path)
        
        # Remove indexed files from this directory
        files_to_remove = []
        for filename, filepath in self._custom_override_paths.items():
            if filepath.parent == path or path in filepath.parents:
                files_to_remove.append(filename)
        
        for filename in files_to_remove:
            del self._custom_override_paths[filename]
        
        logger.info(f"Removed custom override directory: {path} (removed {len(files_to_remove)} 2DAs)")
        
        # Clear cached data
        self._clear_override_caches()
        
        return True
    
    def get_custom_override_directories(self) -> List[str]:
        """Get list of currently configured custom override directories"""
        return [str(path) for path in self._custom_override_dirs]
    
    def _clear_override_caches(self):
        """Clear all override-related caches"""
        self._override_dir_overrides.clear()
        self._workshop_overrides.clear()
        self._module_overrides.clear()
        self._campaign_overrides.clear()
        for hak_override in self._hak_overrides:
            hak_override.clear()

        logger.info("Cleared all override caches")
    
    def get_workshop_mods(self, force_refresh: bool = False) -> List[Dict]:
        """
        Get metadata for all installed Steam Workshop mods
        
        Args:
            force_refresh: Force re-scraping of all mod data
            
        Returns:
            List of mod metadata dictionaries
        """
        return self._workshop_service.get_installed_mods(force_refresh=force_refresh)
    
    def get_workshop_mod(self, mod_id: str, force_refresh: bool = False) -> Optional[Dict]:
        """
        Get metadata for a specific workshop mod
        
        Args:
            mod_id: Steam Workshop item ID
            force_refresh: Force re-scraping even if cached
            
        Returns:
            Mod metadata dictionary or None
        """
        return self._workshop_service.get_mod_metadata(mod_id, force_refresh=force_refresh)
    
    def search_workshop_mods(self, search_term: str) -> List[Dict]:
        """
        Search installed workshop mods by name
        
        Args:
            search_term: Text to search for in mod titles
            
        Returns:
            List of matching mod metadata
        """
        return self._workshop_service.find_mod_by_name(search_term)
    
    def get_workshop_cache_stats(self) -> Dict:
        """Get statistics about the workshop metadata cache"""
        return self._workshop_service.get_cache_stats()
    
    def clear_workshop_cache(self):
        """Clear all cached workshop metadata"""
        self._workshop_service.clear_cache()
    
    def cleanup_workshop_cache(self):
        """Remove cache entries for uninstalled workshop mods"""
        return self._workshop_service.cleanup_cache()
    
    # ============================================================================
    # UTILITY & STATS METHODS
    # ============================================================================
    
    
    def get_cache_size_mb(self) -> float:
        """Get current cache size in MB"""
        return self._cache_memory_bytes / 1024 / 1024
    
    def get_cached_count(self) -> int:
        """Get number of cached 2DAs"""
        return len(self._2da_cache)
    
    def get_available_2da_files(self) -> List[str]:
        """
        Get list of all available 2DA files.
        
        Returns:
            List of 2DA filenames (without path, with .2da extension)
        """
        return list(self._2da_locations.keys())
    
    def get_cache_stats(self) -> Dict[str, Any]:
        """Get detailed cache statistics"""
        self._update_cache_memory_usage()
        
        # Calculate hit rate
        total_accesses = self._cache_hits + self._cache_misses
        hit_rate = (self._cache_hits / total_accesses * 100) if total_accesses > 0 else 0.0
        
        # Calculate average compression ratio
        avg_compression_ratio = (self._compression_ratio_sum / self._compression_count * 100) if self._compression_count > 0 else 0.0
        
        # Count compressed vs uncompressed
        compressed_count = sum(1 for v in self._2da_compressed.values() if v)
        
        return {
            'enabled': self._memory_cache_enabled,
            'preload_enabled': self._preload_on_init,
            'compression_enabled': self._compression_enabled,
            'max_size_mb': self._cache_max_mb,
            'current_size_mb': self.get_cache_size_mb(),
            'cached_items': self.get_cached_count(),
            'compressed_items': compressed_count,
            'compression_ratio': f"{avg_compression_ratio:.1f}%",
            'cache_hits': self._cache_hits,
            'cache_misses': self._cache_misses,
            'hit_rate': f"{hit_rate:.1f}%",
            '2da_cache_keys': list(self._2da_cache.keys())[:20],  # First 20 keys
            'module_cache_stats': self._module_cache.get_stats() if hasattr(self, '_module_cache') else {}
        }
    
    def clear_memory_cache(self):
        """Clear all in-memory caches"""
        self._2da_cache.clear()
        self._2da_compressed.clear()
        if hasattr(self, '_decompressed_cache'):
            self._decompressed_cache.clear()
        self._module_overrides.clear()
        self._hak_overrides.clear()
        self._override_dir_overrides.clear()
        self._workshop_overrides.clear()
        self._cache_memory_bytes = 0
        self._cache_hits = 0
        self._cache_misses = 0
        self._compression_ratio_sum = 0.0
        self._compression_count = 0
        logger.info("Cleared all in-memory caches")
    
    def set_context(self, module_info: Dict[str, Any]):
        """
        Set up resource manager context based on save file module information
        
        Args:
            module_info: Dictionary containing module information from _detect_module_info()
        """
        logger.info(f"Setting ResourceManager context for module: {module_info.get('module_name', 'None')}")
        
        # Extract module context
        module_context = None
        if module_info.get('module_name') and module_info.get('module_path'):
            module_context = {
                'module_name': module_info['module_name'],
                'module_path': module_info['module_path']
            }
        
        # Only scan override directories if they haven't been scanned yet
        # This prevents redundant scanning during save imports
        if not self._workshop_file_paths and not self._override_file_paths:
            logger.info("Override directories not yet scanned, performing scan...")
            self._scan_override_directories(module_context)
        else:
            logger.info("Override directories already scanned, skipping redundant scan")
            # Still need to load module-specific content (HAKs) if context is provided
            if module_context:
                module_name = module_context.get('module_name', '')
                module_path = module_context.get('module_path', '')
                
                if module_name and module_path:
                    logger.info(f"Loading module-specific context: {module_name}")
                    try:
                        # Load the specific module for this context
                        if self.set_module(module_path):
                            logger.info(f"Successfully loaded module: {module_name}")
                        else:
                            logger.warning(f"Failed to load module: {module_name}")
                    except Exception as e:
                        logger.error(f"Error loading module {module_name}: {e}")
        
        logger.info(f"ResourceManager context set successfully")
    
    def comprehensive_resource_scan(
        self,
        workshop_dirs: Optional[List[str]] = None,
        custom_override_dirs: Optional[List[str]] = None,
        enhanced_data_dir: Optional[str] = None
    ) -> Dict[str, Any]:
        """
        Perform a comprehensive resource scan using the optimized Python scanner
        
        Args:
            workshop_dirs: Optional list of workshop directories to scan
            custom_override_dirs: Optional list of custom override directories
            enhanced_data_dir: Optional enhanced edition data directory
            
        Returns:
            Dictionary with scan results and performance statistics
        """
        try:
            # Use nwn2_paths for default values
            if workshop_dirs is None:
                workshop_folder = nwn2_paths.steam_workshop_folder
                workshop_dirs = [str(workshop_folder)] if workshop_folder and workshop_folder.exists() else []
            
            if custom_override_dirs is None:
                custom_override_dirs = [str(d) for d in nwn2_paths.custom_override_folders if d.exists()]
            
            if enhanced_data_dir is None and nwn2_paths.is_enhanced_edition:
                enhanced_data_dir = str(nwn2_paths.enhanced_data)
            
            # Perform comprehensive scan
            scan_results = self._python_scanner.comprehensive_scan(
                nwn2_data_dir=str(self.nwn2_path / "data"),
                workshop_dirs=workshop_dirs,
                custom_override_dirs=custom_override_dirs,
                enhanced_data_dir=enhanced_data_dir
            )
            
            # Get performance statistics
            performance_stats = self._python_scanner.get_performance_stats()
            
            return {
                'scan_results': scan_results.to_dict(),
                'performance_stats': performance_stats,
                'timestamp': time.time()
            }
            
        except Exception as e:
            logger.error(f"Comprehensive resource scan failed: {e}")
            return {
                'error': str(e),
                'timestamp': time.time()
            }
    
    def get_resource_scanner_stats(self) -> Dict[str, Any]:
        """Get performance statistics from all scanners"""
        return {
            'main_scanner': self._python_scanner.get_performance_stats(),
            'zip_indexer': self._zip_indexer.get_stats(),
            'directory_walker': self._directory_walker.get_stats()
        }
    
    def reset_scanner_stats(self):
        """Reset performance statistics for all scanners"""
        self._python_scanner.reset_stats()
        self._zip_indexer.reset_stats()
        self._directory_walker.reset_stats()
        logger.info("All scanner statistics reset")
    
    def _create_empty_prerequisite_table(self, table_name: str) -> Optional[TDAParser]:
        """
        Create an empty prerequisite table to prevent errors when referenced tables are missing.
        
        Args:
            table_name: Name of the prerequisite table to create (e.g., 'cls_pres_fershift.2da')
            
        Returns:
            Empty TDAParser with basic prerequisite table structure
        """
        try:
            # Create a minimal empty prerequisite table structure
            # Standard prerequisite tables typically have these columns
            empty_2da_content = """2DA V2.0

   ReqType ReqParam1 ReqParam2 Label       
0  ****    ****      ****      ****        
"""
            
            # Create a temporary file to parse
            import tempfile
            import os
            
            with tempfile.NamedTemporaryFile(mode='w', suffix='.2da', delete=False) as f:
                f.write(empty_2da_content.strip())
                temp_path = f.name
            
            try:
                # Parse the temporary file
                parser = TDAParser()
                parser.read(temp_path)
                
                logger.info(f"Created empty prerequisite table: {table_name}")
                return parser
                
            finally:
                # Clean up temporary file
                try:
                    os.unlink(temp_path)
                except OSError:
                    pass
                    
        except Exception as e:
            logger.error(f"Failed to create empty prerequisite table {table_name}: {e}")
            return None
    
    def __enter__(self):
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()