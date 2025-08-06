"""
Safe caching layer using MessagePack instead of pickle for security.
Eliminates remote code execution vulnerability while maintaining performance.
"""
import msgpack
import logging
from pathlib import Path
from typing import Any, Optional, Union
import os

logger = logging.getLogger(__name__)


class SafeCache:
    """
    Safe caching implementation using MessagePack.
    
    Features:
    - No code execution vulnerability (unlike pickle)
    - Faster than pickle for most data structures
    - Smaller file sizes than JSON
    """
    
    @staticmethod
    def save(filepath: Union[str, Path], data: Any) -> None:
        """
        Save data safely using msgpack.
        
        Args:
            filepath: Path to save file (extension will be changed to .msgpack)
            data: Data to serialize
        """
        filepath = Path(filepath)
        msgpack_file = filepath.with_suffix('.msgpack')
        
        try:
            # Convert data to serializable format
            serializable = SafeCache._make_serializable(data)
            
            # Save with msgpack
            with open(msgpack_file, 'wb') as f:
                packed = msgpack.packb(serializable, use_bin_type=True)
                f.write(packed)
                
            logger.debug(f"Saved cache to {msgpack_file}")
            
        except Exception as e:
            logger.error(f"Failed to save cache to {msgpack_file}: {e}")
            raise
    
    @staticmethod
    def load(filepath: Union[str, Path]) -> Optional[Any]:
        """
        Load data safely using msgpack.
        
        Args:
            filepath: Path to load from (.msgpack extension)
            
        Returns:
            Loaded data or None if file not found
        """
        filepath = Path(filepath)
        msgpack_file = filepath.with_suffix('.msgpack')
        
        if msgpack_file.exists():
            try:
                with open(msgpack_file, 'rb') as f:
                    data = msgpack.unpackb(f.read(), raw=False)
                    return SafeCache._restore_objects(data)
            except Exception as e:
                logger.error(f"Failed to load msgpack cache from {msgpack_file}: {e}")
                raise
        
        return None
    
    @staticmethod
    def exists(filepath: Union[str, Path]) -> bool:
        """Check if cache file exists (msgpack)."""
        filepath = Path(filepath)
        return filepath.with_suffix('.msgpack').exists()
    
    @staticmethod
    def delete(filepath: Union[str, Path]) -> None:
        """Delete cache file (msgpack)."""
        filepath = Path(filepath)
        
        cache_file = filepath.with_suffix('.msgpack')
        if cache_file.exists():
            try:
                cache_file.unlink()
                logger.debug(f"Deleted cache file: {cache_file}")
            except Exception as e:
                logger.error(f"Failed to delete cache file {cache_file}: {e}")
    
    @staticmethod
    def _make_serializable(data: Any) -> Any:
        """
        Convert Python objects to msgpack-serializable format.
        
        Handles:
        - Basic types (str, int, float, bool, None, bytes)
        - Collections (list, tuple, dict)
        - Custom objects (converts to dict with __type__ field)
        """
        if data is None or isinstance(data, (str, int, float, bool, bytes)):
            return data
        
        elif isinstance(data, (list, tuple)):
            return [SafeCache._make_serializable(item) for item in data]
        
        elif isinstance(data, dict):
            return {k: SafeCache._make_serializable(v) for k, v in data.items()}
        
        # Handle objects with __dict__
        elif hasattr(data, '__dict__'):
            obj_dict = {
                '__type__': f"{data.__class__.__module__}.{data.__class__.__name__}",
                '__data__': SafeCache._make_serializable(data.__dict__)
            }
            return obj_dict
        
        # Handle other types by converting to string
        else:
            return str(data)
    
    @staticmethod
    def _restore_objects(data: Any) -> Any:
        """
        Restore objects from msgpack data.
        
        Note: For security, we don't automatically instantiate custom classes.
        Instead, we return dictionaries with type information that can be
        processed by the application layer if needed.
        """
        if isinstance(data, dict):
            # Check if this is a serialized object
            if '__type__' in data and '__data__' in data:
                # For now, just return the data dict
                # Application code can handle instantiation if needed
                return data['__data__']
            else:
                # Regular dict - restore recursively
                return {k: SafeCache._restore_objects(v) for k, v in data.items()}
        
        elif isinstance(data, list):
            return [SafeCache._restore_objects(item) for item in data]
        
        else:
            return data
    
    @staticmethod
    def get_cache_size(cache_dir: Union[str, Path]) -> dict:
        """Get statistics about cache directory size."""
        cache_dir = Path(cache_dir)
        
        stats = {
            'msgpack_count': 0,
            'msgpack_size_mb': 0.0,
            'total_size_mb': 0.0
        }
        
        if not cache_dir.exists():
            return stats
        
        for file in cache_dir.iterdir():
            if file.is_file():
                size_mb = file.stat().st_size / (1024 * 1024)
                
                if file.suffix == '.msgpack':
                    stats['msgpack_count'] += 1
                    stats['msgpack_size_mb'] += size_mb
                    stats['total_size_mb'] += size_mb
        
        return stats