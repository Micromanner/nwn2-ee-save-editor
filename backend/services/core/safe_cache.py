"""Safe caching using MessagePack instead of pickle for security."""
import msgpack
from loguru import logger
from pathlib import Path
from typing import Any, Union


class SafeCache:
    """Safe caching implementation using MessagePack."""

    @staticmethod
    def save(filepath: Union[str, Path], data: Any) -> None:
        """Save data safely using msgpack."""
        filepath = Path(filepath)
        msgpack_file = filepath.with_suffix('.msgpack')
        serializable = SafeCache._make_serializable(data)
        with open(msgpack_file, 'wb') as f:
            packed = msgpack.packb(serializable, use_bin_type=True)
            f.write(packed)
        logger.debug(f"Saved cache to {msgpack_file}")

    @staticmethod
    def load(filepath: Union[str, Path]) -> Any:
        """Load data safely using msgpack."""
        filepath = Path(filepath)
        msgpack_file = filepath.with_suffix('.msgpack')
        if not msgpack_file.exists():
            raise FileNotFoundError(f"Cache file not found: {msgpack_file}")
        with open(msgpack_file, 'rb') as f:
            data = msgpack.unpackb(f.read(), raw=False)
            return SafeCache._restore_objects(data)

    @staticmethod
    def exists(filepath: Union[str, Path]) -> bool:
        """Check if cache file exists."""
        filepath = Path(filepath)
        return filepath.with_suffix('.msgpack').exists()

    @staticmethod
    def delete(filepath: Union[str, Path]) -> None:
        """Delete cache file."""
        filepath = Path(filepath)
        cache_file = filepath.with_suffix('.msgpack')
        if cache_file.exists():
            cache_file.unlink()
            logger.debug(f"Deleted cache file: {cache_file}")

    @staticmethod
    def _make_serializable(data: Any) -> Any:
        """Convert Python objects to msgpack-serializable format."""
        if data is None or isinstance(data, (str, int, float, bool, bytes)):
            return data
        elif isinstance(data, (list, tuple)):
            return [SafeCache._make_serializable(item) for item in data]
        elif isinstance(data, dict):
            return {k: SafeCache._make_serializable(v) for k, v in data.items()}
        elif hasattr(data, '__dict__'):
            obj_dict = {
                '__type__': f"{data.__class__.__module__}.{data.__class__.__name__}",
                '__data__': SafeCache._make_serializable(data.__dict__)
            }
            return obj_dict
        else:
            return str(data)

    @staticmethod
    def _restore_objects(data: Any) -> Any:
        """Restore objects from msgpack data (type info discarded for security)."""
        if isinstance(data, dict):
            if '__type__' in data and '__data__' in data:
                logger.debug(f"Discarding type info during cache restore: {data['__type__']}")
                return data['__data__']
            else:
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
