"""Logging configuration for NWN2 Editor."""
import os
import sys
from pathlib import Path
from datetime import datetime
from loguru import logger

from utils.paths import get_writable_dir


LOG_DIR = get_writable_dir("logs")

ENABLE_LOG_VIEWER = os.getenv("ENABLE_LOG_VIEWER", "false").lower() == "true"
LOG_VIEWER_PORT = int(os.getenv("LOG_VIEWER_PORT", "9999"))


SESSION_ID = datetime.now().strftime("%Y%m%d_%H%M%S")

def configure_logging():
    """Configure Loguru logging."""
    logger.remove()

    log_filter = os.getenv("LOG_FILTER", "")
    log_level = os.getenv("LOG_LEVEL", "INFO").upper()

    if log_filter:
        logger.add(
            sys.stderr,
            level="DEBUG",
            format="<green>{time:HH:mm:ss}</green> | <level>{level: <8}</level> | <cyan>{name}</cyan>:{line} - <level>{message}</level>",
            filter=lambda record: log_filter in record["name"]
        )
    else:
        logger.add(
            sys.stderr,
            level=log_level,
            format="<green>{time:HH:mm:ss}</green> | <level>{level: <8}</level> | <cyan>{name}</cyan>:{line} - <level>{message}</level>"
        )

    logger.add(
        LOG_DIR / f"app_{SESSION_ID}.log",
        rotation="5 MB",
        retention=5,
        level="DEBUG",
        format="{time:YYYY-MM-DD HH:mm:ss} | {level: <8} | {name}:{line} - {message}"
    )

    logger.add(
        LOG_DIR / "error.log",
        rotation="10 MB",
        retention="14 days",
        level="ERROR",
        format="{time:YYYY-MM-DD HH:mm:ss} | {level: <8} | {name}:{line} - {message}"
    )

    return logger

configure_logging()
