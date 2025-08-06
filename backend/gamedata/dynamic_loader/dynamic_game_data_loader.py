"""
Dynamic Game Data Loader - Integration layer for runtime-generated data classes

This module provides a dynamic data loader that uses runtime-generated classes
instead of hardcoded dataclasses, enabling full mod compatibility.
"""
import logging
import asyncio
import os
from typing import Dict, List, Optional, Any, Tuple
from pathlib import Path

from parsers.resource_manager import ResourceManager
from gamedata.dynamic_loader.data_model_loader import DataModelLoader
from gamedata.dynamic_loader.runtime_class_generator import RuntimeDataClassGenerator
from gamedata.data_fetching_rules import with_retry_limit

logger = logging.getLogger(__name__)


class DynamicGameDataLoader:
    """
    Dynamic data access layer that generates classes at runtime.
    
    This loader:
    - Generates classes dynamically to support mod-added columns
    - Provides direct access to dynamically loaded data
    - Supports both sync and async loading modes
    """
    
    def __init__(self, resource_manager: Optional[ResourceManager] = None,
                 use_async: bool = True,
                 progress_callback: Optional[Any] = None,
                 validate_relationships: bool = True,
                 priority_only: Optional[bool] = None):
        """
        Initialize dynamic game data loader.
        
        Args:
            resource_manager: Optional ResourceManager instance
            use_async: Whether to use async loading (default True)
            progress_callback: Optional callback for progress updates
            validate_relationships: Whether to validate table relationships
            priority_only: If None, check FAST_STARTUP env var (default True for faster startup)
        """
        # Create resource manager if not provided
        if resource_manager:
            self.rm = resource_manager
        else:
            self.rm = ResourceManager(suppress_warnings=True)
        
        # Check environment variable for fast startup (priority-only loading)
        # Default to eager loading (FAST_STARTUP=false) for better UX
        if priority_only is None:
            priority_only = os.environ.get('FAST_STARTUP', 'false').lower() == 'true'
            if priority_only:
                logger.info("Fast startup enabled - loading only priority tables initially")
            else:
                logger.info("Eager loading enabled - loading all game data at startup")
        
        # Create data model loader
        self.loader = DataModelLoader(
            self.rm,
            progress_callback=progress_callback,
            validate_relationships=validate_relationships,
            priority_only=priority_only
        )
        
        # Storage for dynamically loaded data
        self.table_data: Dict[str, List[Any]] = {}
        
        # Cache for O(1) ID lookups - table_name -> {id -> data}
        self._id_lookup_cache: Dict[str, Dict[int, Any]] = {}
        
        # Track initialization state
        self._is_ready = False
        self._initialization_error: Optional[Exception] = None
        
        # Load data with better error handling
        if use_async:
            # Run async load in event loop with robust error handling
            try:
                try:
                    # Try to get existing event loop
                    loop = asyncio.get_event_loop()
                    if loop.is_running():
                        # If we're already in an async context, we can't run another loop
                        # Fall back to sync loading
                        logger.warning("Event loop is already running, falling back to synchronous loading")
                        self._load_all_rules()
                    else:
                        # Run in existing event loop
                        loop.run_until_complete(self._load_async())
                except RuntimeError as e:
                    if "no running event loop" in str(e).lower() or "no current event loop" in str(e).lower():
                        # No event loop exists, create one
                        logger.info("No event loop found, creating new one for data loading")
                        asyncio.run(self._load_async())
                    else:
                        # Other RuntimeError, fall back to sync
                        logger.warning(f"Event loop error ({e}), falling back to synchronous loading")
                        self._load_all_rules()
            except Exception as e:
                logger.error(f"Async loading failed: {e}, falling back to synchronous loading")
                self._load_all_rules()
        else:
            # Synchronous loading
            self._load_all_rules()
    
    async def _load_async(self):
        """Load all data asynchronously."""
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
        """Load all data synchronously."""
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
        """
        Load remaining tables if initially loaded in priority-only mode.
        
        Returns:
            True if additional tables were loaded, False if already fully loaded
        """
        if not self.loader.priority_only:
            return False  # Already fully loaded
        
        logger.info("Loading remaining non-priority tables...")
        
        # Create a new loader without priority_only restriction 
        full_loader = DataModelLoader(
            self.rm,
            progress_callback=self.loader.progress.callback,
            validate_relationships=self.loader.validate_relationships,
            priority_only=False
        )
        
        # Load all tables
        try:
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                full_data = loop.run_until_complete(full_loader.load_game_data())
                # Update with new data (priority tables will be replaced, new ones added)
                self.table_data.update(full_data)
                # Update loader state
                self.loader = full_loader
                logger.info(f"Loaded additional {len(full_data) - len(self.loader.PRIORITY_TABLES)} tables")
                return True
            finally:
                loop.close()
        except Exception as e:
            logger.error(f"Failed to load remaining tables: {e}")
            return False
    
    # Direct access methods
    @with_retry_limit()
    def get_table(self, table_name: str) -> List[Any]:
        """
        Get all instances for a table with retry limits to prevent infinite loops.
        
        Args:
            table_name: Name of the table (without .2da extension)
            
        Returns:
            List of data instances or empty list if not found/blacklisted
        """
        if not table_name:
            logger.warning("get_table called with empty table_name")
            return []
        
        try:
            table = self.table_data.get(table_name, None)
            if table is not None:
                # Table found in cache (could be empty list or populated list)
                return table
            
            # Table not in cache - try to load on-demand for prerequisite tables
            if table_name.startswith('cls_pres_'):
                logger.debug(f"Prerequisite table '{table_name}' not found in loaded data, attempting on-demand load")
                
                # Try to get the table from ResourceManager (which can auto-create missing prerequisite tables)
                raw_table = self.rm.get_2da_with_overrides(table_name)
                if raw_table:
                    logger.info(f"Successfully loaded missing prerequisite table '{table_name}' on-demand")
                    
                    # For prerequisite tables, cache an empty list since they're typically empty or have minimal data
                    # This prevents the data fetching rules from blocking while still allowing the system to function
                    empty_list = []
                    self.table_data[table_name] = empty_list
                    logger.info(f"Cached empty prerequisite table '{table_name}' to prevent future retries")
                    return empty_list
                else:
                    # ResourceManager couldn't create the table, cache empty list to prevent retries
                    logger.debug(f"Could not load prerequisite table '{table_name}', caching empty list")
                    empty_list = []
                    self.table_data[table_name] = empty_list
                    return empty_list
            
            # Regular table not found - return empty list instead of None to prevent retry loop
            logger.debug(f"Table '{table_name}' not found, returning empty list")
            return []
        except Exception as e:
            logger.error(f"Error retrieving table '{table_name}': {e}")
            # Return empty list instead of raising to prevent retry loops
            return []
    
    @with_retry_limit()
    def get_by_id(self, table_name: str, row_id: int) -> Optional[Any]:
        """
        Get a specific row by ID with mapping strategy support and retry limits.
        Now with O(1) caching for fast lookups.
        
        Args:
            table_name: Name of the table
            row_id: Row ID/index
            
        Returns:
            Data instance or None
        """
        # Handle None or invalid row_id
        if row_id is None or not isinstance(row_id, int):
            return None
        
        # Check if we have a cached lookup table for this table
        if table_name in self._id_lookup_cache:
            # Use O(1) cached lookup
            return self._id_lookup_cache[table_name].get(row_id)
        
        # First time accessing this table - build the cache
        instances = self.get_table(table_name)
        if not instances:
            return None
        
        # Build ID lookup cache for this table
        logger.debug(f"Building ID lookup cache for table '{table_name}'")
        cache = {}
        
        # Get mapping strategy for this table
        strategy = self._get_mapping_strategy(table_name)
        
        if strategy['type'] == 'direct':
            # Standard implementation: row_index == row_id
            for row_id_iter in range(len(instances)):
                cache[row_id_iter] = instances[row_id_iter]
        
        elif strategy['type'] == 'offset':
            # Offset mapping: row_index = row_id + offset
            for row_index, instance in enumerate(instances):
                actual_id = row_index - strategy['offset']
                cache[actual_id] = instance
        
        elif strategy['type'] == 'sparse':
            # Sparse mapping: build cache using custom logic
            for row_index, instance in enumerate(instances):
                # Try to get the ID from the instance itself
                actual_id = getattr(instance, 'id', getattr(instance, 'row_index', row_index))
                cache[actual_id] = instance
        
        # Store the cache
        self._id_lookup_cache[table_name] = cache
        logger.debug(f"Cached {len(cache)} entries for table '{table_name}'")
        
        # Return the requested item
        return cache.get(row_id)
    
    def _get_mapping_strategy(self, table_name: str) -> Dict[str, Any]:
        """
        Get mapping strategy for a table.
        
        Args:
            table_name: Name of the table
            
        Returns:
            Dictionary containing mapping strategy configuration
        """
        # Define mapping strategies for specific tables
        strategies = {
            'creaturesize': {
                'type': 'offset',
                'offset': -1,
                'description': 'creaturesize.2da uses row_index = id - 1'
            },
            'categories': {
                'type': 'offset',
                'offset': -1,
                'description': 'categories.2da uses row_index = id - 1'
            },
            # Partial offset tables with lower confidence - use direct for now
            # 'phenotype': {'type': 'direct'},  # Only 66% confidence, gaps present
            # 'spelltarget': {'type': 'direct'},  # Only 65% confidence
            # Add more strategies as discovered
            # 'appearance': {'type': 'sparse', 'lookup_field': 'LABEL'},
        }
        
        # Return strategy or default to direct mapping
        return strategies.get(table_name, {'type': 'direct'})
    
    def _sparse_lookup(self, table_name: str, row_id: int, instances: List[Any]) -> Optional[Any]:
        """
        Handle sparse mapping lookups for complex table patterns.
        
        Args:
            table_name: Name of the table
            row_id: ID to look up
            instances: List of table instances
            
        Returns:
            Data instance or None
        """
        # Future implementation for sparse/custom mappings
        # For now, fallback to direct mapping
        logger.warning(f"Sparse mapping not implemented for {table_name}, falling back to direct mapping")
        if 0 <= row_id < len(instances):
            return instances[row_id]
        return None
    
    def set_module_context(self, module_path: str) -> bool:
        """Set module context for loading module-specific overrides."""
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
        """
        Check if the loader is fully initialized and ready to use.
        
        Returns:
            True if all data is loaded and ready, False otherwise
        """
        return self._is_ready and len(self.table_data) > 0
    
    def wait_for_ready(self, timeout: float = 30.0, check_interval: float = 0.1) -> bool:
        """
        Wait for the loader to be ready with timeout.
        
        Args:
            timeout: Maximum time to wait in seconds (default 30s)
            check_interval: How often to check ready status in seconds (default 0.1s)
            
        Returns:
            True if ready within timeout, False if timeout exceeded
            
        Raises:
            RuntimeError: If initialization failed with an error
        """
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