"""
Helper functions for caching TDAParser objects safely using msgpack.
"""
from typing import Any, Optional, Union
from pathlib import Path
from gamedata.cache.safe_cache import SafeCache
# Use the imported TDAParser (now Rust-based) from __init__.py
from . import TDAParser


class TDACacheHelper:
    """Helper for safely caching TDAParser objects."""
    
    @staticmethod
    def save_tda(filepath: Union[str, Path], parser: TDAParser) -> None:
        """Save a TDAParser to cache."""
        try:
            # Try Rust parser serialization first
            if hasattr(parser, 'to_msgpack_bytes'):
                cache_data = parser.to_msgpack_bytes()
                Path(filepath).with_suffix('.msgpack').write_bytes(cache_data)
            else:
                # Fallback to SafeCache for compatibility
                SafeCache.save(filepath, parser)
        except Exception:
            # Fallback to SafeCache for any issues
            SafeCache.save(filepath, parser)
    
    @staticmethod
    def load_tda(filepath: Union[str, Path]) -> Optional[TDAParser]:
        """Load a TDAParser from cache, reconstructing the object."""
        try:
            # Try loading Rust parser serialization first
            cache_file = Path(filepath).with_suffix('.msgpack')
            if cache_file.exists():
                cache_data = cache_file.read_bytes()
                if hasattr(TDAParser, 'from_msgpack_bytes'):
                    return TDAParser.from_msgpack_bytes(cache_data)
        except Exception:
            pass
        
        # Fallback to SafeCache
        data = SafeCache.load(filepath)
        if data is None:
            return None
        
        # If it's already a TDAParser (legacy cache), return as-is
        if isinstance(data, TDAParser):
            return data
        
        # Otherwise reconstruct from dict
        if isinstance(data, dict):
            try:
                parser = TDAParser()
                # Restore attributes from the dict
                for key, value in data.items():
                    setattr(parser, key, value)
                return parser
            except Exception:
                return None
        
        return None
    
    @staticmethod
    def exists(filepath: Union[str, Path]) -> bool:
        """Check if cache file exists."""
        # Check both Rust serialization and SafeCache formats
        rust_cache = Path(filepath).with_suffix('.msgpack')
        return rust_cache.exists() or SafeCache.exists(filepath)