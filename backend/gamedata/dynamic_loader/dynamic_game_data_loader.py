"""Dynamic Game Data Loader - Integration layer for runtime-generated data classes."""
from loguru import logger
import asyncio
import os
from typing import Dict, List, Optional, Any, Tuple
from pathlib import Path

from services.core.resource_manager import ResourceManager
from gamedata.dynamic_loader.data_model_loader import DataModelLoader
from gamedata.dynamic_loader.runtime_class_generator import RuntimeDataClassGenerator
from services.gamedata.data_fetching_rules import with_retry_limit
from utils.performance_profiler import get_profiler


class DynamicGameDataLoader:
    """Integration layer for runtime-generated data classes with async support."""

    def __init__(self, resource_manager: Optional[ResourceManager] = None,
                 use_async: bool = True,
                 progress_callback: Optional[Any] = None,
                 validate_relationships: bool = True,
                 priority_only: Optional[bool] = None):
        profiler = get_profiler()
        
        with profiler.profile("DynamicGameDataLoader.__init__"):
            if resource_manager:
                self.rm = resource_manager
            else:
                with profiler.profile("Create ResourceManager"):
                    self.rm = ResourceManager(suppress_warnings=True)
            
            if priority_only is None:
                priority_only = os.environ.get('FAST_STARTUP', 'false').lower() == 'true'
                if priority_only:
                    logger.info("Fast startup enabled - loading only priority tables initially")
                else:
                    logger.info("Eager loading enabled - loading all game data at startup")
            
            with profiler.profile("Create DataModelLoader"):
                self.loader = DataModelLoader(
                    self.rm,
                    progress_callback=progress_callback,
                    validate_relationships=validate_relationships,
                    priority_only=priority_only
                )
        
            self.table_data: Dict[str, List[Any]] = {}
            self._id_lookup_cache: Dict[str, Dict[int, Any]] = {}
            self._is_ready = False
            self._initialization_error: Optional[Exception] = None
            
            # Load data with better error handling
            with profiler.profile("Load Game Data", async_mode=use_async):
                if use_async:
                    # Run async load in event loop with robust error handling
                    try:
                        try:
                            # Try to get existing event loop
                            loop = asyncio.get_event_loop()
                            if loop.is_running():
                                logger.warning("Event loop is already running, falling back to synchronous loading")
                                self._load_all_rules()
                            else:
                                # Run in existing event loop
                                loop.run_until_complete(self._load_async())
                        except RuntimeError as e:
                            if "no running event loop" in str(e).lower() or "no current event loop" in str(e).lower():
                                logger.info("No event loop found, creating new one for data loading")
                                asyncio.run(self._load_async())
                            else:
                                logger.warning(f"Event loop error ({e}), falling back to synchronous loading")
                                self._load_all_rules()
                    except Exception as e:
                        logger.error(f"Async loading failed: {e}, falling back to synchronous loading")
                        self._load_all_rules()
                else:
                    # Synchronous loading
                    self._load_all_rules()
    
    async def _load_async(self):
        """Load game data asynchronously."""
        try:
            self.table_data = await self.loader.load_game_data()
            self._is_ready = True
            logger.info(f"Async loading complete - {len(self.table_data)} tables loaded")
        except Exception as e:
            self._initialization_error = e
            self._is_ready = False
            logger.error(f"Async loading failed: {e}")
            raise
    
    def _load_all_rules(self):
        """Load game data synchronously."""
        try:
            # For sync loading, we need to wrap the async method
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                logger.info("Starting synchronous game data loading")
                self.table_data = loop.run_until_complete(self.loader.load_game_data())
                logger.info(f"Successfully loaded {len(self.table_data)} data tables")
                self._is_ready = True
            finally:
                loop.close()
                asyncio.set_event_loop(None)
        except Exception as e:
            logger.error(f"Failed to load game data synchronously: {e}")
            # Initialize with empty data to prevent further errors
            self.table_data = {}
            self._initialization_error = e
            self._is_ready = False
            raise RuntimeError(f"Game data loading failed: {e}")
    
    def load_remaining_tables(self) -> bool:
        """Load remaining non-priority tables after initial fast startup."""
        if not self.loader.priority_only:
            return False  # Already fully loaded
        
        logger.info("Loading remaining non-priority tables...")

        full_loader = DataModelLoader(
            self.rm,
            progress_callback=self.loader.progress.callback,
            validate_relationships=self.loader.validate_relationships,
            priority_only=False
        )
        
        try:
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                full_data = loop.run_until_complete(full_loader.load_game_data())
                self.table_data.update(full_data)
                self.loader = full_loader
                logger.info(f"Loaded additional {len(full_data) - len(self.loader.PRIORITY_TABLES)} tables")
                return True
            finally:
                loop.close()
        except Exception as e:
            logger.error(f"Failed to load remaining tables: {e}")
            return False
    
    @with_retry_limit()
    def get_table(self, table_name: str) -> List[Any]:
        """Get all instances for a table by name."""
        if not table_name:
            logger.warning("get_table called with empty table_name")
            return []

        try:
            table = self.table_data.get(table_name, None)
            if table is not None:
                return table

            # Table not in cache - try to load on-demand for prerequisite tables
            if table_name.startswith('cls_pres_'):
                logger.debug(f"Prerequisite table '{table_name}' not found, attempting on-demand load")
                raw_table = self.rm.get_2da_with_overrides(table_name)
                if raw_table:
                    logger.info(f"Loaded prerequisite table '{table_name}' on-demand")
                    empty_list = []
                    self.table_data[table_name] = empty_list
                    return empty_list
                else:
                    logger.debug(f"Could not load prerequisite table '{table_name}'")
                    self.table_data[table_name] = []
                    return []
            
            logger.debug(f"Table '{table_name}' not found")
            return []
        except Exception as e:
            logger.error(f"Error retrieving table '{table_name}': {e}")
            return []
    
    def get_by_id(self, table_name: str, row_id: int) -> Optional[Any]:
        """Get a specific row instance by table name and row ID."""
        if row_id is None or not isinstance(row_id, int):
            return None

        if table_name in self._id_lookup_cache:
            return self._id_lookup_cache[table_name].get(row_id)
        
        instances = self.get_table(table_name)
        if not instances:
            return None

        logger.debug(f"Building ID lookup cache for table '{table_name}'")
        cache = {}
        strategy = self._get_mapping_strategy(table_name)
        
        if strategy['type'] == 'direct':
            for row_id_iter in range(len(instances)):
                cache[row_id_iter] = instances[row_id_iter]
        
        elif strategy['type'] == 'offset':
            for row_index, instance in enumerate(instances):
                actual_id = row_index - strategy['offset']
                cache[actual_id] = instance
        
        elif strategy['type'] == 'sparse':
            for row_index, instance in enumerate(instances):
                actual_id = getattr(instance, 'id', getattr(instance, 'row_index', row_index))
                cache[actual_id] = instance
        
        self._id_lookup_cache[table_name] = cache
        logger.debug(f"Cached {len(cache)} entries for table '{table_name}'")
        return cache.get(row_id)
    
    def _get_mapping_strategy(self, table_name: str) -> Dict[str, Any]:
        """Get the ID mapping strategy for a table."""
        strategies = {
            'creaturesize': {'type': 'direct'},
            'categories': {
                'type': 'direct'
            },
        }
        return strategies.get(table_name, {'type': 'direct'})
    
    def _sparse_lookup(self, table_name: str, row_id: int, instances: List[Any]) -> Optional[Any]:
        """Fallback sparse lookup for tables with non-contiguous IDs."""
        logger.warning(f"Sparse mapping not implemented for {table_name}, using direct mapping")
        if 0 <= row_id < len(instances):
            return instances[row_id]
        return None
    
    def set_module_context(self, module_path: str) -> bool:
        """Set the active module context for mod-aware data loading."""
        return self.rm.set_module(module_path)
    
    def get_stats(self) -> Dict[str, Any]:
        """Get statistics about loaded data."""
        return self.loader.get_stats()
    
    def get_validation_report(self) -> Optional[Any]:
        """Get the relationship validation report if available."""
        return self.loader.get_validation_report()
    
    def get_table_relationships(self, table_name: str) -> Dict[str, Any]:
        """Get relationship information for a specific table."""
        return self.loader.get_table_relationships(table_name)
    
    def get_string(self, str_ref: int) -> str:
        """Get a localized string from TLK files"""
        return self.rm.get_string(str_ref)
    
    def get_strings_batch(self, str_refs: List[int]) -> Dict[int, str]:
        """Get multiple localized strings from TLK files in one batch operation"""
        return self.rm.get_strings_batch(str_refs)
    
    def is_ready(self) -> bool:
        """Check if the loader has finished loading and has data."""
        return self._is_ready and len(self.table_data) > 0
    
    def wait_for_ready(self, timeout: float = 30.0, check_interval: float = 0.1) -> bool:
        """Block until loader is ready or timeout is reached."""
        import time
        start_time = time.time()
        
        while time.time() - start_time < timeout:
            if self._initialization_error:
                raise RuntimeError(f"Loader initialization failed: {self._initialization_error}")
            
            if self.is_ready():
                logger.info(f"DynamicGameDataLoader ready after {time.time() - start_time:.2f}s")
                return True
            
            time.sleep(check_interval)
        
        logger.warning(f"DynamicGameDataLoader not ready after {timeout}s timeout")
        return False
    
    def get_initialization_error(self) -> Optional[Exception]:
        """Get any initialization error that occurred."""
        return self._initialization_error