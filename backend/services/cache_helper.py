"""
Helper functions for caching TDAParser objects using msgpack.
Simplified to only use Rust-based serialization.
"""
from typing import Optional, Union
from pathlib import Path
import logging
from nwn2_rust import TDAParser

logger = logging.getLogger(__name__)


class TDACacheHelper:
    """Helper for caching TDAParser objects using Rust msgpack serialization."""
    
    @staticmethod
    def save_tda(filepath: Union[str, Path], parser: TDAParser) -> None:
        """Save a TDAParser to cache using Rust msgpack serialization."""
        try:
            if hasattr(parser, 'to_msgpack_bytes'):
                cache_data = parser.to_msgpack_bytes()
                Path(filepath).with_suffix('.msgpack').write_bytes(cache_data)
            else:
                logger.warning(f"Parser doesn't support msgpack serialization, not caching")
        except Exception as e:
            logger.error(f"Failed to save TDA cache: {e}")
    
    @staticmethod
    def load_tda(filepath: Union[str, Path]) -> Optional[TDAParser]:
        """Load a TDAParser from msgpack cache."""
        try:
            cache_file = Path(filepath).with_suffix('.msgpack')
            if cache_file.exists():
                cache_data = cache_file.read_bytes()
                if hasattr(TDAParser, 'from_msgpack_bytes'):
                    return TDAParser.from_msgpack_bytes(cache_data)
                else:
                    logger.warning("TDAParser doesn't support msgpack deserialization")
        except Exception as e:
            logger.error(f"Failed to load TDA cache: {e}")
        
        return None
    
    @staticmethod
    def exists(filepath: Union[str, Path]) -> bool:
        """Check if cache file exists."""
        return Path(filepath).with_suffix('.msgpack').exists()