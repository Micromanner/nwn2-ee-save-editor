"""
Logging Configuration for NWN2 Editor
Provides structured logging with Loguru and optional web-based log viewer.
"""
import os
import sys
from pathlib import Path
from datetime import datetime
from loguru import logger

LOG_DIR = Path(__file__).parent.parent / "logs"
LOG_DIR.mkdir(exist_ok=True)

ENABLE_LOG_VIEWER = os.getenv("ENABLE_LOG_VIEWER", "false").lower() == "true"
LOG_VIEWER_PORT = int(os.getenv("LOG_VIEWER_PORT", "9999"))

# Session ID for filtering logs by restart
SESSION_ID = datetime.now().strftime("%Y%m%d_%H%M%S")

def configure_logging():
    """Configure Loguru logging with file rotation and optional filtering"""
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
        LOG_DIR / "app.log",
        rotation="10 MB",
        retention="7 days",
        level="DEBUG",
        format="{time:YYYY-MM-DD HH:mm:ss} | [" + SESSION_ID + "] | {level: <8} | {name}:{line} - {message}"
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
