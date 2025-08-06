"""
Data Fetching Rules - Prevent infinite retry loops when managers can't find table data

This module provides retry limits and error handling to prevent managers from
making the PC "BRRRRRRRRR" when they can't find table data.
"""

import logging
import time
from typing import Dict, Optional, Any, Callable, Set
from functools import wraps
from threading import Lock
from contextlib import contextmanager

logger = logging.getLogger(__name__)


class DataFetchingRules:
    """
    Manages data fetching rules to prevent infinite retry loops
    
    Features:
    - Retry limits per table/method combination
    - Cooldown periods for failed requests
    - Blacklist for permanently failed tables
    - Thread-safe operation
    """
    
    def __init__(self, max_retries: int = 5, cooldown_seconds: int = 2, blacklist_after: int = 15):
        """
        Initialize data fetching rules
        
        Args:
            max_retries: Maximum retry attempts per table/method
            cooldown_seconds: Seconds to wait before allowing retry
            blacklist_after: Failures before blacklisting a table
        """
        self.max_retries = max_retries
        self.cooldown_seconds = cooldown_seconds
        self.blacklist_after = blacklist_after
        
        # Thread-safe tracking
        self._lock = Lock()
        self._retry_counts: Dict[str, int] = {}
        self._last_attempt: Dict[str, float] = {}
        self._failure_counts: Dict[str, int] = {}
        self._blacklisted: Set[str] = set()
        
        # Scan mode flag to suppress recovery messages
        self._scan_mode = False
        
        logger.info(f"DataFetchingRules initialized: max_retries={max_retries}, "
                   f"cooldown={cooldown_seconds}s, blacklist_after={blacklist_after}")
    
    def _get_cache_key(self, table_name: str, method_name: str = "get_table", record_id: Optional[int] = None) -> str:
        """Generate cache key for table/method combination"""
        base_key = f"{method_name}:{table_name.lower()}"
        
        # For get_by_id, track per specific ID to avoid cross-contamination
        if method_name == "get_by_id" and record_id is not None:
            return f"{base_key}:{record_id}"
        
        # For get_table, use table-level tracking but don't let it affect get_by_id
        return base_key
    
    def is_blacklisted(self, table_name: str, method_name: str = "get_table", record_id: Optional[int] = None) -> bool:
        """Check if a table is blacklisted"""
        cache_key = self._get_cache_key(table_name, method_name, record_id)
        with self._lock:
            return cache_key in self._blacklisted
    
    def should_allow_request(self, table_name: str, method_name: str = "get_table", record_id: Optional[int] = None) -> bool:
        """
        Check if a data request should be allowed
        
        Returns:
            True if request should proceed, False if it should be blocked
        """
        cache_key = self._get_cache_key(table_name, method_name, record_id)
        
        with self._lock:
            # Check blacklist
            if cache_key in self._blacklisted:
                logger.debug(f"Request blocked - blacklisted: {cache_key}")
                return False
            
            # Check retry limit first (but be more lenient with get_table during initialization)
            retry_count = self._retry_counts.get(cache_key, 0)
            effective_max_retries = self.max_retries
            
            # Allow more retries for get_table calls during initialization
            from gamedata.dynamic_loader.singleton import is_loader_ready
            if method_name == "get_table" and not is_loader_ready():
                effective_max_retries = self.max_retries * 6  # Much more lenient during initialization
                
            if retry_count >= effective_max_retries:
                logger.debug(f"Request blocked - max retries exceeded: {cache_key} ({retry_count}/{effective_max_retries})")
                return False
            
            # Check cooldown
            last_attempt = self._last_attempt.get(cache_key, 0)
            if last_attempt > 0:  # Only check cooldown if there was a previous attempt
                time_since_last = time.time() - last_attempt
                if time_since_last < self.cooldown_seconds:
                    remaining = self.cooldown_seconds - time_since_last
                    logger.debug(f"Request blocked - cooldown: {cache_key} ({remaining:.1f}s remaining)")
                    return False
            
            return True
    
    def record_attempt(self, table_name: str, method_name: str = "get_table", record_id: Optional[int] = None) -> None:
        """Record that an attempt was made to fetch data"""
        cache_key = self._get_cache_key(table_name, method_name, record_id)
        
        with self._lock:
            # Check if enough time has passed since last attempt to reset retry count
            last_attempt = self._last_attempt.get(cache_key, 0)
            if last_attempt > 0:
                time_since_last = time.time() - last_attempt
                if time_since_last >= self.cooldown_seconds:
                    # Reset retry count after successful cooldown
                    logger.debug(f"Resetting retry count for {cache_key} after {time_since_last:.1f}s cooldown")
                    self._retry_counts[cache_key] = 0
            
            self._retry_counts[cache_key] = self._retry_counts.get(cache_key, 0) + 1
            self._last_attempt[cache_key] = time.time()
            
            logger.debug(f"Recorded attempt: {cache_key} (attempt #{self._retry_counts[cache_key]})")
            
            # Warn when approaching retry limit
            if self._retry_counts[cache_key] >= self.max_retries - 1:
                logger.warning(f"Approaching retry limit for {cache_key}: {self._retry_counts[cache_key]}/{self.max_retries} attempts")
    
    def record_success(self, table_name: str, method_name: str = "get_table", record_id: Optional[int] = None) -> None:
        """Record that a data fetch was successful"""
        cache_key = self._get_cache_key(table_name, method_name, record_id)
        
        with self._lock:
            # Check if there were previous ACTUAL FAILURES (not just attempts) before resetting
            # Only count as recovery if there were failures or if retry count > 1 (indicating retries)
            retry_count = self._retry_counts.get(cache_key, 0)
            had_failures = (retry_count > 1) or cache_key in self._failure_counts or cache_key in self._blacklisted
            
            # Reset counters on success
            self._retry_counts.pop(cache_key, None)
            self._last_attempt.pop(cache_key, None)
            self._failure_counts.pop(cache_key, None)
            self._blacklisted.discard(cache_key)
            
            # Only log when recovering from failures (but suppress spam)
            if had_failures:
                # Initialize recovery counters if needed
                if not hasattr(self, '_recovery_counts'):
                    self._recovery_counts = {}
                
                # Count recoveries by table type to avoid spam
                table_key = cache_key.split(':')[1] if ':' in cache_key else cache_key
                self._recovery_counts[table_key] = self._recovery_counts.get(table_key, 0) + 1
                
                # Only log milestone recoveries or non-feat tables
                if table_key == 'feat':
                    count = self._recovery_counts[table_key]
                    if count == 1:
                        logger.warning(f"Mass feat recovery started - suppressing individual logs...")
                    elif count % 1000 == 0:
                        logger.info(f"Feat recovery: {count} recovered")
                elif table_key in ['classes', 'spells']:
                    count = self._recovery_counts[table_key]
                    if count == 1:
                        logger.warning(f"Mass {table_key} recovery started - suppressing individual logs...")
                    elif count % 100 == 0:
                        logger.info(f"{table_key.title()} recovery: {count} recovered")
                else:
                    # Log other table recoveries normally (they're usually rare)
                    # But suppress during scan mode
                    if not self._scan_mode:
                        logger.info(f"Recovered: {cache_key}")
            else:
                logger.debug(f"Success: {cache_key}")
    
    def record_failure(self, table_name: str, method_name: str = "get_table", 
                      record_id: Optional[int] = None, error: Optional[Exception] = None) -> None:
        """Record that a data fetch failed"""
        cache_key = self._get_cache_key(table_name, method_name, record_id)
        
        with self._lock:
            self._failure_counts[cache_key] = self._failure_counts.get(cache_key, 0) + 1
            failure_count = self._failure_counts[cache_key]
            
            # Blacklist after too many failures
            if failure_count >= self.blacklist_after:
                self._blacklisted.add(cache_key)
                logger.error(f"BLACKLISTED after {failure_count} failures: {cache_key}")
            
            error_msg = f" (Error: {error})" if error else ""
            logger.warning(f"Recorded failure: {cache_key} (failure #{failure_count}/{self.blacklist_after}){error_msg}")
    
    def reset_table(self, table_name: str, method_name: str = "get_table", record_id: Optional[int] = None) -> None:
        """Reset all counters for a specific table (e.g., after data reload)"""
        cache_key = self._get_cache_key(table_name, method_name, record_id)
        
        with self._lock:
            self._retry_counts.pop(cache_key, None)
            self._last_attempt.pop(cache_key, None)
            self._failure_counts.pop(cache_key, None)
            self._blacklisted.discard(cache_key)
            
            logger.info(f"Reset all counters for: {cache_key}")
    
    @contextmanager
    def scan_mode(self):
        """
        Context manager to suppress recovery messages during table scanning.
        
        Usage:
            with rules.scan_mode():
                # Scan tables without spamming recovery logs
                tables = scan_2da_files()
        """
        old_scan_mode = self._scan_mode
        self._scan_mode = True
        try:
            logger.debug("Entering scan mode - suppressing recovery messages")
            yield
        finally:
            self._scan_mode = old_scan_mode
            logger.debug("Exiting scan mode")
    
    def get_stats(self) -> Dict[str, Any]:
        """Get statistics about data fetching"""
        with self._lock:
            return {
                "retry_counts": dict(self._retry_counts),
                "failure_counts": dict(self._failure_counts),
                "blacklisted": list(self._blacklisted),
                "rules": {
                    "max_retries": self.max_retries,
                    "cooldown_seconds": self.cooldown_seconds,
                    "blacklist_after": self.blacklist_after
                }
            }


# Global instance
_data_fetching_rules = None
_rules_lock = Lock()


def get_data_fetching_rules() -> DataFetchingRules:
    """Get the global DataFetchingRules instance (singleton)"""
    global _data_fetching_rules
    
    if _data_fetching_rules is None:
        with _rules_lock:
            if _data_fetching_rules is None:
                _data_fetching_rules = DataFetchingRules()
    
    return _data_fetching_rules


def with_retry_limit(table_name_param: str = "table_name", method_name: str = None):
    """
    Decorator to add retry limits to data fetching methods
    
    Args:
        table_name_param: Name of the parameter containing the table name
        method_name: Name of the method (defaults to function name)
    """
    def decorator(func: Callable) -> Callable:
        actual_method_name = method_name or func.__name__
        
        @wraps(func)
        def wrapper(*args, **kwargs):
            rules = get_data_fetching_rules()
            
            # Extract table name from parameters
            table_name = None
            record_id = None
            
            # Try to get from kwargs first
            if table_name_param in kwargs:
                table_name = kwargs[table_name_param]
            else:
                # Try to get from args by inspecting function signature
                import inspect
                sig = inspect.signature(func)
                param_names = list(sig.parameters.keys())
                
                if table_name_param in param_names:
                    param_index = param_names.index(table_name_param)
                    if param_index < len(args):
                        table_name = args[param_index]
            
            # For get_by_id methods, also extract the record ID
            if actual_method_name == "get_by_id":
                # Look for row_id parameter (second parameter after table_name)
                if 'row_id' in kwargs:
                    record_id = kwargs['row_id']
                else:
                    import inspect
                    sig = inspect.signature(func)
                    param_names = list(sig.parameters.keys())
                    
                    if 'row_id' in param_names:
                        row_id_index = param_names.index('row_id')
                        if row_id_index < len(args):
                            record_id = args[row_id_index]
            
            if not table_name:
                logger.warning(f"Could not extract table name from {actual_method_name} parameters, blocking request.")
                return None
            
            # Check if system is still initializing
            from gamedata.dynamic_loader.singleton import is_loader_ready
            is_initializing = not is_loader_ready()
            
            # Check if request should be allowed
            if not rules.should_allow_request(table_name, actual_method_name, record_id):
                # Get detailed blocking reason
                cache_key = rules._get_cache_key(table_name, actual_method_name, record_id)
                retry_count = rules._retry_counts.get(cache_key, 0)
                is_blacklisted = cache_key in rules._blacklisted
                
                if is_initializing:
                    # During initialization, be more lenient
                    logger.info(f"Data request during initialization: {actual_method_name}({table_name}{f', {record_id}' if record_id is not None else ''}) - system still loading")
                    return None
                elif is_blacklisted:
                    logger.warning(f"Data request BLACKLISTED: {actual_method_name}({table_name}{f', {record_id}' if record_id is not None else ''}) - permanently blocked after repeated failures")
                else:
                    logger.warning(f"Data request BLOCKED: {actual_method_name}({table_name}{f', {record_id}' if record_id is not None else ''}) - retry count: {retry_count}/{rules.max_retries}")
                return None
            
            # Record attempt
            rules.record_attempt(table_name, actual_method_name, record_id)
            
            # Special logging for feat table issues to track initialization problems
            if table_name == 'feat' and actual_method_name in ['get_table', 'get_by_id']:
                if not hasattr(rules, '_feat_access_started'):
                    rules._feat_access_started = time.time()
                    logger.warning(f"First feat table access detected: {actual_method_name}({table_name}{f', {record_id}' if record_id is not None else ''}) - tracking initialization timing")
            
            try:
                # Execute the function
                result = func(*args, **kwargs)
                
                # For get_by_id methods, None is a valid result (ID doesn't exist)
                # Only treat None as failure for get_table methods
                if result is not None:
                    # Success
                    rules.record_success(table_name, actual_method_name, record_id)
                elif actual_method_name == "get_by_id":
                    # For get_by_id, None is valid (ID not found) - still record as success
                    rules.record_success(table_name, actual_method_name, record_id)
                    # Only log if this ID was previously problematic
                    cache_key = rules._get_cache_key(table_name, actual_method_name, record_id)
                    if cache_key in rules._retry_counts or cache_key in rules._failure_counts:
                        logger.info(f"Data fetch resolved: get_by_id({table_name}, {record_id}) returned None (ID does not exist - this is normal)")
                else:
                    # For other methods (like get_table), None is a failure
                    logger.warning(f"Data fetch FAILURE: {actual_method_name}({table_name}) returned None unexpectedly")
                    rules.record_failure(table_name, actual_method_name, record_id)
                
                return result
                
            except Exception as e:
                # Exception occurred
                logger.error(f"Data fetch EXCEPTION: {actual_method_name}({table_name}{f', {record_id}' if record_id is not None else ''}) raised {type(e).__name__}: {e}")
                rules.record_failure(table_name, actual_method_name, record_id, e)
                raise
        
        return wrapper
    return decorator


def reset_data_fetching_rules() -> None:
    """Reset the global data fetching rules (useful for tests)"""
    global _data_fetching_rules
    with _rules_lock:
        _data_fetching_rules = None

def mark_initialization_complete() -> None:
    """Mark that game data initialization is complete"""
    rules = get_data_fetching_rules()
    rules._initialization_complete = True
    logger.info("Game data initialization marked as complete - stricter retry limits now active")

def clear_all_data_fetching_blocks() -> None:
    """Clear all existing blocks and counters"""
    rules = get_data_fetching_rules()
    with rules._lock:
        retry_count = len(rules._retry_counts)
        failure_count = len(rules._failure_counts)
        blacklist_count = len(rules._blacklisted)
        
        rules._retry_counts.clear()
        rules._last_attempt.clear()
        rules._failure_counts.clear()
        rules._blacklisted.clear()
        
        # Reset recovery counters
        if hasattr(rules, '_recovery_counts'):
            rules._recovery_counts.clear()
        if hasattr(rules, '_feat_recovery_count'):
            rules._feat_recovery_count = 0
        
        logger.info(f"Cleared all data fetching blocks: {retry_count} retry entries, {failure_count} failure entries, {blacklist_count} blacklisted entries")