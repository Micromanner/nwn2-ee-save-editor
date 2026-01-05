"""Helper functions for caching TDAParser objects using msgpack."""
from typing import Union
from pathlib import Path
from loguru import logger
from nwn2_rust import TDAParser


class TDACacheHelper:
    """Helper for caching TDAParser objects using Rust msgpack serialization."""

    @staticmethod
    def save_tda(filepath: Union[str, Path], parser: TDAParser) -> None:
        """Save a TDAParser to cache using Rust msgpack serialization."""
        cache_data = parser.to_msgpack_bytes()
        Path(filepath).with_suffix('.msgpack').write_bytes(cache_data)
        logger.debug(f"Saved TDA cache to {filepath}")

    @staticmethod
    def load_tda(filepath: Union[str, Path]) -> TDAParser:
        """Load a TDAParser from msgpack cache."""
        cache_file = Path(filepath).with_suffix('.msgpack')
        if not cache_file.exists():
            raise FileNotFoundError(f"Cache file not found: {cache_file}")
        cache_data = cache_file.read_bytes()
        return TDAParser.from_msgpack_bytes(cache_data)

    @staticmethod
    def exists(filepath: Union[str, Path]) -> bool:
        """Check if cache file exists."""
        return Path(filepath).with_suffix('.msgpack').exists()
