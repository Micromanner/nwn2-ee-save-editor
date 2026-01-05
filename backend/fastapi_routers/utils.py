"""Shared utilities for FastAPI routers."""

import time
from functools import wraps
from loguru import logger


class SimpleCache:
    """Simple in-memory cache implementation."""
    
    def __init__(self):
        self._cache = {}

    def get(self, key, default=None):
        return self._cache.get(key, default)

    def set(self, key, value, timeout=None):
        self._cache[key] = value
        
    def delete(self, key):
        if key in self._cache:
            del self._cache[key]
            
    def clear(self):
        self._cache = {}

cache = SimpleCache()

def log_performance(func):
    """Decorator to log performance metrics for FastAPI endpoints."""
    @wraps(func)
    def wrapper(*args, **kwargs):
        start_time = time.time()
        result = None
        try:
            result = func(*args, **kwargs)
            return result
        finally:
            end_time = time.time()
            duration_ms = (end_time - start_time) * 1000
            
            endpoint_name = func.__name__
            status_code = getattr(result, 'status_code', 'success') if result else 'error'
            logger.info(f"{endpoint_name}: {duration_ms:.2f}ms (status: {status_code})")
    return wrapper
