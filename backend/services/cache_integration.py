"""
Python integration layer for Rust-based pre-compiled cache system.
Bridges ResourceManager with high-performance Rust cache implementation.
"""
import logging
import time
import os
from pathlib import Path
from typing import Dict, Optional, List, Any
from utils.paths import get_writable_dir

# Import Rust cache components
try:
    from nwn2_rust import CacheBuilder, CacheManager, TDAParser
    RUST_CACHE_AVAILABLE = True
except ImportError:
    RUST_CACHE_AVAILABLE = False
    CacheBuilder = None
    CacheManager = None
    TDAParser = None
    logger = logging.getLogger(__name__)
    logger.warning("Rust cache extensions not available, falling back to dynamic loading")

logger = logging.getLogger(__name__)


class PrecompiledCacheIntegration:
    """Integration layer between Python ResourceManager and Rust cache."""
    
    def __init__(self, resource_manager):
        self.resource_manager = resource_manager
        # Use environment variable or default to True
        self.cache_enabled = os.environ.get('ENABLE_PRECOMPILED_CACHE', 'true').lower() == 'true' and RUST_CACHE_AVAILABLE
        
        if not self.cache_enabled:
            self.cache_manager = None
            return
        
        # Initialize Rust cache manager - use AppData directory
        cache_dir = get_writable_dir("cache")
        self.cache_manager = CacheManager(str(cache_dir))
        self.cache_builder = CacheBuilder(str(cache_dir))
        
        # Smart cache initialization logic
        self._validate_cache_smart()
    
    def get_cached_table(self, table_name: str) -> Optional[TDAParser]:
        """
        Get a 2DA table from cache if available.
        
        Args:
            table_name: Name of the 2DA table (with or without .2da extension)
            
        Returns:
            TDAParser object if cached, None otherwise
        """
        if not self.cache_enabled or not self.cache_manager:
            return None
        
        try:
            # Get raw data from Rust cache
            raw_data = self.cache_manager.get_table_data(table_name)
            if raw_data:
                # Convert from list of ints to bytes if needed
                if isinstance(raw_data, list):
                    raw_data = bytes(raw_data)
                
                # Reconstruct TDAParser from raw data
                parser = self._reconstruct_parser(raw_data)
                if parser:
                    logger.debug(f"Loaded {table_name} from pre-compiled cache")
                    return parser
        except Exception as e:
            logger.error(f"Failed to get cached table {table_name}: {e}")
        
        return None
    
    def build_cache(self) -> bool:
        """
        Build pre-compiled cache from current game state.
        
        Returns:
            True if cache was built successfully
        """
        if not self.cache_enabled:
            return False
        
        start_time = time.time()
        logger.info("Building pre-compiled 2DA cache...")
        
        try:
            # Collect all tables to cache
            tables_data = self._collect_tables_for_caching()
            
            # Generate cache key
            mod_state = self._get_mod_state()
            cache_key = self.cache_builder.generate_cache_key(mod_state)
            
            # Build cache with Rust (pass as dict, not Python object)
            import json
            # Convert to simple dict format for Rust
            rust_tables_data = {}
            for name, data in tables_data.items():
                rust_tables_data[name] = {
                    'section': data['section'],
                    'data': data['data'],  # Already bytes
                    'row_count': data['row_count']
                }
            
            success = self.cache_builder.build_cache(rust_tables_data, cache_key)
            
            if success:
                elapsed = time.time() - start_time
                logger.info(f"Cache built successfully in {elapsed:.2f}s")
                
                # Reload cache manager to use new cache
                self.cache_manager.invalidate_cache()
                self._validate_cache()
            
            return success
            
        except Exception as e:
            logger.error(f"Failed to build cache: {e}", exc_info=True)
            return False
    
    def invalidate_cache(self, reason: str = "manual"):
        """Invalidate the current cache."""
        if self.cache_manager:
            self.cache_manager.invalidate_cache()
            logger.info(f"Cache invalidated: {reason}")
    
    def get_cache_stats(self) -> Dict[str, Any]:
        """Get cache statistics."""
        if not self.cache_manager:
            return {'enabled': False, 'available': False}
        
        try:
            stats = self.cache_manager.get_cache_stats()
            stats['enabled'] = self.cache_enabled
            stats['available'] = True
            return stats
        except Exception as e:
            logger.error(f"Failed to get cache stats: {e}")
            return {'enabled': self.cache_enabled, 'available': False, 'error': str(e)}
    
    def _validate_cache(self):
        """Validate cache against current mod state."""
        if not self.cache_manager:
            return
        
        try:
            # Generate current cache key
            mod_state = self._get_mod_state()
            current_key = self.cache_builder.generate_cache_key(mod_state)
            
            # Validate with Rust cache manager
            is_valid = self.cache_manager.validate_cache_key(current_key)
            
            if is_valid:
                logger.info("Pre-compiled cache is valid and ready")
            else:
                logger.info("Pre-compiled cache is invalid or missing")
                
        except Exception as e:
            logger.error(f"Failed to validate cache: {e}")
    
    def _validate_cache_smart(self):
        """Smart cache validation - only acts on existing cache, lets ResourceManager handle creation."""
        if not self.cache_manager:
            return
            
        try:
            # Check if cache files exist at all
            cache_dir = get_writable_dir("cache/compiled_cache")
            metadata_file = cache_dir / "cache_metadata.json"
            
            if not metadata_file.exists():
                # No cache exists - let ResourceManager create it after full initialization
                logger.info("No pre-compiled cache found - will be created after initialization")
                return
                
            # Cache exists - defer validation until ResourceManager is fully initialized
            # We can't validate the cache key yet because workshop/override paths aren't scanned
            logger.info("Pre-compiled cache found - validation deferred until after initialization")
                    
        except Exception as e:
            logger.error(f"Failed to validate cache: {e}")
    
    def ensure_cache_built(self):
        """Ensure cache is built - called after ResourceManager is fully initialized."""
        if not self.cache_manager:
            return False
            
        try:
            # Check if cache files exist
            cache_dir = get_writable_dir("cache/compiled_cache")
            metadata_file = cache_dir / "cache_metadata.json"
            
            if metadata_file.exists():
                # Cache exists - validate it
                current_key = self.cache_builder.generate_cache_key(self._get_mod_state())
                is_valid = self.cache_manager.validate_cache_key(current_key)
                if is_valid:
                    logger.info("Pre-compiled cache is valid - using existing cache")
                    return True
                else:
                    logger.info("Existing cache is invalid - rebuilding")
            else:
                logger.info("No cache found - building initial cache")
            
            # Build cache
            logger.info("Building pre-compiled 2DA cache...")
            if self.build_cache():
                logger.info("Pre-compiled cache built successfully")
                return True
            else:
                logger.error("Failed to build pre-compiled cache")
                return False
                
        except Exception as e:
            logger.error(f"Failed to ensure cache is built: {e}")
            return False
    
    def _get_mod_state(self) -> Dict[str, Any]:
        """Get current mod state for cache key generation."""
        from config.nwn2_settings import nwn2_paths
        
        mod_state = {
            'install_dir': str(nwn2_paths.game_folder) if nwn2_paths.game_folder else '',
        }
        
        # Add workshop files
        if hasattr(self.resource_manager, '_workshop_file_paths'):
            mod_state['workshop_files'] = list(self.resource_manager._workshop_file_paths.keys())
        
        # Add override files
        if hasattr(self.resource_manager, '_override_file_paths'):
            mod_state['override_files'] = list(self.resource_manager._override_file_paths.keys())
        
        return mod_state
    
    def _get_fast_mod_state(self, workshop_files: List[str], override_files: List[str]) -> Dict[str, Any]:
        """
        Get mod state for fast cache validation using pre-computed file lists.
        This method is used by ResourceManager's fast validation to avoid redundant directory scanning.
        
        Args:
            workshop_files: List of workshop .2da file names (lowercase)
            override_files: List of override .2da file names (lowercase)
            
        Returns:
            Mod state dict for cache key generation
        """
        from config.nwn2_settings import nwn2_paths
        
        return {
            'install_dir': str(nwn2_paths.game_folder) if nwn2_paths.game_folder else '',
            'workshop_files': sorted(workshop_files),  # Sort for consistent ordering
            'override_files': sorted(override_files)   # Sort for consistent ordering
        }
    
    def get_new_custom_tables(self) -> List[str]:
        """
        Get list of new custom 2DA tables that weren't in the previous cache.
        Uses cache key comparison to identify newly added mod content.
        
        Returns:
            List of table names (without .2da extension) that are new custom tables
        """
        if not self.cache_enabled or not self.cache_manager:
            return []
        
        try:
            # Get current mod files
            current_workshop = list(self.resource_manager._workshop_file_paths.keys()) if hasattr(self.resource_manager, '_workshop_file_paths') else []
            current_override = list(self.resource_manager._override_file_paths.keys()) if hasattr(self.resource_manager, '_override_file_paths') else []
            
            # Try to get previous mod state from cache metadata
            cache_dir = get_writable_dir("cache/compiled_cache")
            metadata_file = cache_dir / "cache_metadata.json"
            
            previous_workshop = []
            previous_override = []
            
            if metadata_file.exists():
                import json
                try:
                    with open(metadata_file, 'r') as f:
                        cache_metadata = json.load(f)
                    
                    # Extract previous file lists from cache metadata
                    previous_workshop = cache_metadata.get('mod_state', {}).get('workshop_files', [])
                    previous_override = cache_metadata.get('mod_state', {}).get('override_files', [])
                    
                except Exception as e:
                    logger.warning(f"Could not read previous cache metadata: {e}")
            
            # Find NEW files (in current but not in previous)
            new_workshop = set(current_workshop) - set(previous_workshop)
            new_override = set(current_override) - set(previous_override)
            
            # Combine and clean up names (remove .2da extension)
            new_tables = []
            for filename in new_workshop | new_override:
                table_name = filename.replace('.2da', '') if filename.endswith('.2da') else filename
                new_tables.append(table_name)
            
            if new_tables:
                logger.info(f"Detected {len(new_tables)} new custom tables: {new_tables}")
            
            return sorted(new_tables)
            
        except Exception as e:
            logger.error(f"Failed to detect new custom tables: {e}")
            return []
    
    
    def _collect_tables_for_caching(self) -> Dict[str, Dict[str, Any]]:
        """Collect all 2DA tables for caching."""
        tables_data = {}
        
        # Get the filtered list of character-related tables
        from gamedata.dynamic_loader.data_model_loader import DataModelLoader
        loader = DataModelLoader(self.resource_manager)
        
        # Get base character files and custom mod files
        base_tables = loader._get_base_character_files()
        custom_tables = loader._discover_custom_mod_files()
        
        # Combine all tables
        filtered_tables = list(set(base_tables + custom_tables))
        
        logger.info(f"Collecting {len(filtered_tables)} tables for caching...")
        
        collected_count = 0
        failed_count = 0
        
        for table_name in filtered_tables:
            if not table_name.endswith('.2da'):
                table_name = table_name + '.2da'
            
            try:
                # Determine section (base_game, workshop, override)
                section = self._determine_table_section(table_name)
                
                # Load the table (try different methods)
                parser = self.resource_manager.get_2da_with_overrides(table_name)
                if not parser:
                    # Try without overrides
                    parser = self.resource_manager.get_2da(table_name)
                
                if parser:
                    # Get raw 2DA data
                    raw_data = self._get_raw_table_data(parser)
                    if raw_data:
                        tables_data[table_name] = {
                            'section': section,
                            'data': raw_data,
                            'row_count': parser.get_row_count() if hasattr(parser, 'get_row_count') else 100  # Default estimate
                        }
                        collected_count += 1
                    else:
                        failed_count += 1
                        logger.debug(f"No raw data for {table_name}")
                else:
                    failed_count += 1
                    logger.debug(f"Could not load {table_name}")
                        
            except Exception as e:
                failed_count += 1
                logger.warning(f"Failed to collect table {table_name}: {e}")
        
        logger.info(f"Collected {len(tables_data)} tables for caching (failed: {failed_count})")
        return tables_data
    
    def _determine_table_section(self, table_name: str) -> str:
        """Determine which section a table belongs to."""
        # Check override first (highest priority)
        if hasattr(self.resource_manager, '_override_file_paths'):
            if table_name in self.resource_manager._override_file_paths:
                return 'override'
        
        # Check workshop
        if hasattr(self.resource_manager, '_workshop_file_paths'):
            if table_name in self.resource_manager._workshop_file_paths:
                return 'workshop'
        
        # Default to base game
        return 'base_game'
    
    def _get_raw_table_data(self, parser: TDAParser) -> Optional[bytes]:
        """Get raw 2DA data from parser."""
        try:
            # Use Rust parser's msgpack serialization
            if hasattr(parser, 'to_msgpack_bytes'):
                return parser.to_msgpack_bytes()
            
            # Fallback if method not available
            logger.warning("Parser doesn't have to_msgpack_bytes method")
            return None
            
        except Exception as e:
            logger.error(f"Failed to get raw table data: {e}")
            return None
    
    def _reconstruct_parser(self, raw_data: bytes) -> Optional[TDAParser]:
        """Reconstruct TDAParser from raw data."""
        try:
            # Use Rust parser's msgpack deserialization
            if hasattr(TDAParser, 'from_msgpack_bytes'):
                return TDAParser.from_msgpack_bytes(raw_data)
            
            logger.warning("Parser doesn't have from_msgpack_bytes method")
            return None
            
        except Exception as e:
            logger.error(f"Failed to reconstruct parser: {e}")
            return None