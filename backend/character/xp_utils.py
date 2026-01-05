"""XP and level calculation utilities."""

from typing import List, Optional
from loguru import logger

_xp_table_cache: Optional[List[int]] = None


def get_xp_table(rules_service) -> List[int]:
    """Load XP thresholds from exptable.2da."""
    global _xp_table_cache

    if _xp_table_cache is not None:
        return _xp_table_cache

    exptable = rules_service.get_table('exptable')
    if not exptable:
        raise ValueError("exptable.2da not found - required for XP calculations")

    _xp_table_cache = []
    for row in exptable:
        xp_val = getattr(row, 'XP', None)
        if xp_val is None:
            continue
        xp_str = str(xp_val).strip()
        if xp_str in ('****', '0xFFFFFFFF', '-1', ''):
            continue
        try:
            _xp_table_cache.append(int(xp_str))
        except (ValueError, TypeError):
            continue

    if not _xp_table_cache:
        raise ValueError("exptable.2da contains no valid XP values")

    logger.debug(f"Loaded {len(_xp_table_cache)} XP thresholds from exptable.2da")
    return _xp_table_cache


def xp_to_level(xp: int, rules_service) -> int:
    """Convert XP amount to character level."""
    xp_table = get_xp_table(rules_service)
    for level, threshold in enumerate(xp_table, start=1):
        if xp < threshold:
            return max(1, level - 1)
    return len(xp_table)


def level_to_xp(level: int, rules_service) -> int:
    """Get minimum XP required for a level."""
    xp_table = get_xp_table(rules_service)
    if level < 1:
        return 0
    if level > len(xp_table):
        level = len(xp_table)
    return xp_table[level - 1]


def clear_xp_cache():
    """Clear the XP table cache (for testing or mod reload)."""
    global _xp_table_cache
    _xp_table_cache = None
