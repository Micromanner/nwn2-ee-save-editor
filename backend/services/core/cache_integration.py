"""Python integration layer for Rust-based pre-compiled cache system."""
import time
import os
from pathlib import Path
from typing import Dict, List, Any
from loguru import logger
from utils.paths import get_writable_dir

try:
    from nwn2_rust import CacheBuilder, CacheManager, TDAParser
    RUST_CACHE_AVAILABLE = True
except ImportError:
    RUST_CACHE_AVAILABLE = False
    CacheBuilder = None
    CacheManager = None
    TDAParser = None
    logger.warning("Rust cache extensions not available")


class CacheError(Exception):
    """Raised when cache operations fail."""
    pass


class PrecompiledCacheIntegration:
    """Integration layer between Python ResourceManager and Rust cache."""

    def __init__(self, resource_manager):
        self.resource_manager = resource_manager
        self.cache_enabled = os.environ.get('ENABLE_PRECOMPILED_CACHE', 'true').lower() == 'true' and RUST_CACHE_AVAILABLE

        if not self.cache_enabled:
            self.cache_manager = None
            return

        cache_dir = get_writable_dir("cache")
        self.cache_manager = CacheManager(str(cache_dir))
        self.cache_builder = CacheBuilder(str(cache_dir))
        self._validate_cache_smart()

    def get_cached_table(self, table_name: str) -> TDAParser:
        """Get a 2DA table from cache."""
        if not self.cache_enabled or not self.cache_manager:
            raise CacheError("Cache not enabled")

        raw_data = self.cache_manager.get_table_data(table_name)
        if not raw_data:
            raise CacheError(f"Table {table_name} not found in cache")

        if isinstance(raw_data, list):
            raw_data = bytes(raw_data)

        parser = self._reconstruct_parser(raw_data)
        # logger.debug(f"Loaded {table_name} from pre-compiled cache")
        return parser

    def build_cache(self) -> bool:
        """Build pre-compiled cache from current game state."""
        if not self.cache_enabled:
            return False

        start_time = time.time()
        logger.info("Building pre-compiled 2DA cache...")

        tables_data = self._collect_tables_for_caching()
        mod_state = self._get_mod_state()
        cache_key = self.cache_builder.generate_cache_key(mod_state)

        rust_tables_data = {}
        for name, data in tables_data.items():
            rust_tables_data[name] = {
                'section': data['section'],
                'data': data['data'],
                'row_count': data['row_count']
            }

        success = self.cache_builder.build_cache(rust_tables_data, cache_key)

        if success:
            elapsed = time.time() - start_time
            logger.info(f"Cache built successfully in {elapsed:.2f}s")
            self.cache_manager.invalidate_cache()
            self._validate_cache()

        return success

    def invalidate_cache(self, reason: str = "manual"):
        """Invalidate the current cache."""
        if self.cache_manager:
            self.cache_manager.invalidate_cache()
            logger.info(f"Cache invalidated: {reason}")

    def get_cache_stats(self) -> Dict[str, Any]:
        """Get cache statistics."""
        if not self.cache_manager:
            return {'enabled': False, 'available': False}

        stats = self.cache_manager.get_cache_stats()
        stats['enabled'] = self.cache_enabled
        stats['available'] = True
        return stats

    def _validate_cache(self):
        """Validate cache against current mod state."""
        if not self.cache_manager:
            return

        mod_state = self._get_mod_state()
        current_key = self.cache_builder.generate_cache_key(mod_state)
        is_valid = self.cache_manager.validate_cache_key(current_key)

        if is_valid:
            logger.info("Pre-compiled cache is valid and ready")
        else:
            logger.info("Pre-compiled cache is invalid or missing")

    def _validate_cache_smart(self):
        """Smart cache validation - defers until ResourceManager is fully initialized."""
        if not self.cache_manager:
            return

        cache_dir = get_writable_dir("cache/compiled_cache")
        metadata_file = cache_dir / "cache_metadata.json"

        if not metadata_file.exists():
            logger.info("No pre-compiled cache found - will be created after initialization")
            return

        logger.info("Pre-compiled cache found - validation deferred")

    def ensure_cache_built(self) -> bool:
        """Ensure cache is built - called after ResourceManager is fully initialized."""
        if not self.cache_manager:
            return False

        cache_dir = get_writable_dir("cache/compiled_cache")
        metadata_file = cache_dir / "cache_metadata.json"

        if metadata_file.exists():
            current_key = self.cache_builder.generate_cache_key(self._get_mod_state())
            if self.cache_manager.validate_cache_key(current_key):
                logger.info("Pre-compiled cache is valid - using existing cache")
                return True
            logger.info("Existing cache is invalid - rebuilding")
        else:
            logger.info("No cache found - building initial cache")

        return self.build_cache()

    def _get_mod_state(self) -> Dict[str, Any]:
        """Get current mod state for cache key generation."""
        from config.nwn2_settings import nwn2_paths

        mod_state = {
            'install_dir': str(nwn2_paths.game_folder) if nwn2_paths.game_folder else '',
        }

        if hasattr(self.resource_manager, '_workshop_file_paths'):
            mod_state['workshop_files'] = list(self.resource_manager._workshop_file_paths.keys())

        if hasattr(self.resource_manager, '_override_file_paths'):
            mod_state['override_files'] = list(self.resource_manager._override_file_paths.keys())

        return mod_state

    def _get_fast_mod_state(self, workshop_files: List[str], override_files: List[str]) -> Dict[str, Any]:
        """Get mod state for fast cache validation using pre-computed file lists."""
        from config.nwn2_settings import nwn2_paths

        return {
            'install_dir': str(nwn2_paths.game_folder) if nwn2_paths.game_folder else '',
            'workshop_files': sorted(workshop_files),
            'override_files': sorted(override_files)
        }

    def get_new_custom_tables(self) -> List[str]:
        """Get list of new custom 2DA tables that weren't in the previous cache."""
        if not self.cache_enabled or not self.cache_manager:
            return []

        current_workshop = list(self.resource_manager._workshop_file_paths.keys()) if hasattr(self.resource_manager, '_workshop_file_paths') else []
        current_override = list(self.resource_manager._override_file_paths.keys()) if hasattr(self.resource_manager, '_override_file_paths') else []

        cache_dir = get_writable_dir("cache/compiled_cache")
        metadata_file = cache_dir / "cache_metadata.json"

        previous_workshop = []
        previous_override = []

        if metadata_file.exists():
            import json
            try:
                with open(metadata_file, 'r') as f:
                    cache_metadata = json.load(f)
                previous_workshop = cache_metadata.get('mod_state', {}).get('workshop_files', [])
                previous_override = cache_metadata.get('mod_state', {}).get('override_files', [])
            except Exception as e:
                logger.warning(f"Could not read previous cache metadata: {e}")

        new_workshop = set(current_workshop) - set(previous_workshop)
        new_override = set(current_override) - set(previous_override)

        new_tables = []
        for filename in new_workshop | new_override:
            table_name = filename.replace('.2da', '') if filename.endswith('.2da') else filename
            new_tables.append(table_name)

        if new_tables:
            logger.info(f"Detected {len(new_tables)} new custom tables: {new_tables}")

        return sorted(new_tables)

    def _collect_tables_for_caching(self) -> Dict[str, Dict[str, Any]]:
        """Collect all 2DA tables for caching."""
        tables_data = {}

        base_tables = self._get_base_character_files()
        custom_tables = self._get_custom_mod_files()
        filtered_tables = list(set(base_tables + custom_tables))

        logger.info(f"Collecting {len(filtered_tables)} tables for caching...")

        collected_count = 0
        failed_count = 0

        for table_name in filtered_tables:
            if not table_name.endswith('.2da'):
                table_name = table_name + '.2da'

            try:
                section = self._determine_table_section(table_name)
                parser = self.resource_manager.get_2da_with_overrides(table_name)

                if parser:
                    raw_data = self._get_raw_table_data(parser)
                    tables_data[table_name] = {
                        'section': section,
                        'data': raw_data,
                        'row_count': parser.get_row_count() if hasattr(parser, 'get_row_count') else 0
                    }
                    collected_count += 1
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
        if hasattr(self.resource_manager, '_override_file_paths'):
            if table_name in self.resource_manager._override_file_paths:
                return 'override'

        if hasattr(self.resource_manager, '_workshop_file_paths'):
            if table_name in self.resource_manager._workshop_file_paths:
                return 'workshop'

        return 'base_game'

    def _get_base_character_files(self) -> List[str]:
        """Get the base list of character-related 2DA files from config."""
        import json

        filter_file = Path(__file__).parent.parent / 'config' / 'nw2_data_filtered.json'

        if filter_file.exists():
            with open(filter_file, 'r') as f:
                filter_data = json.load(f)
            character_files = filter_data.get('character_files', [])
            return [f.replace('.2da', '') for f in character_files]

        return ['classes', 'racialtypes', 'feat', 'skills', 'spells', 'baseitems', 'appearance']

    def _get_custom_mod_files(self) -> List[str]:
        """Get custom mod 2DA files from ResourceManager's already-scanned paths."""
        custom_files = set()
        rm = self.resource_manager

        if hasattr(rm, '_workshop_file_paths'):
            for filename in rm._workshop_file_paths.keys():
                if filename.lower().endswith('.2da'):
                    custom_files.add(filename.replace('.2da', '').replace('.2DA', ''))

        if hasattr(rm, '_override_file_paths'):
            for filename in rm._override_file_paths.keys():
                if filename.lower().endswith('.2da'):
                    custom_files.add(filename.replace('.2da', '').replace('.2DA', ''))

        if hasattr(rm, '_custom_override_paths'):
            for filename in rm._custom_override_paths.keys():
                if filename.lower().endswith('.2da'):
                    custom_files.add(filename.replace('.2da', '').replace('.2DA', ''))

        return sorted(list(custom_files))

    def _get_raw_table_data(self, parser: TDAParser) -> bytes:
        """Get raw 2DA data from parser."""
        return parser.to_msgpack_bytes()

    def _reconstruct_parser(self, raw_data: bytes) -> TDAParser:
        """Reconstruct TDAParser from raw data."""
        return TDAParser.from_msgpack_bytes(raw_data)
