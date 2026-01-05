"""Background import preloader for FastAPI startup optimization."""

import asyncio
import time
from typing import List, Dict, Any
import sys
from loguru import logger

HEAVY_IMPORTS = [
    # FastAPI models (highest priority - used by all routers)
    {
        'module': 'fastapi_models',
        'description': 'FastAPI Pydantic models',
        'estimated_time': 0.18
    },
    
    # Parser modules (used by savegame and other heavy operations)
    {
        'module': 'services.core.savegame_handler',
        'description': 'Save game handler',
        'estimated_time': 0.42
    },
    
    # Manager modules (used by character operations)
    {
        'module': 'character.character_manager',
        'description': 'Character manager',
        'estimated_time': 0.15
    },
    
    # Game data modules (used by character operations)
    {
        'module': 'gamedata.dynamic_loader.dynamic_game_data_loader',
        'description': 'Game data loader',
        'estimated_time': 0.12
    },
    
    {
        'module': 'character.factory', 
        'description': 'Character factory',
        'estimated_time': 0.08
    }
]

_preload_status = {
    'started': False,
    'completed': False,
    'progress': {},
    'errors': {}
}


def _import_module_safely(module_info: Dict[str, Any]) -> Dict[str, Any]:
    """Safely import a module and return status."""
    module_name = module_info['module']
    start_time = time.time()
    
    try:
        # Use __import__ to load the module
        __import__(module_name)
        load_time = time.time() - start_time
        
        logger.info(f"Preloaded {module_name} in {load_time:.3f}s")
        
        return {
            'module': module_name,
            'success': True,
            'load_time': load_time,
            'description': module_info['description']
        }
        
    except Exception as e:
        load_time = time.time() - start_time
        logger.warning(f"Failed to preload {module_name}: {e}")
        
        return {
            'module': module_name,
            'success': False,
            'load_time': load_time,
            'error': str(e),
            'description': module_info['description']
        }


async def preload_heavy_imports() -> Dict[str, Any]:
    """Preload heavy imports in background thread pool."""
    global _preload_status
    
    if _preload_status['started']:
        logger.info("Import preloading already started")
        return _preload_status
    
    _preload_status['started'] = True
    logger.info(f"Starting background preload of {len(HEAVY_IMPORTS)} heavy modules...")
    
    total_start = time.time()
    results = []
    
    # Run imports in background thread pool (to avoid blocking event loop)
    tasks = []
    for module_info in HEAVY_IMPORTS:
        task = asyncio.to_thread(_import_module_safely, module_info)
        tasks.append(task)
    
    # Execute all imports concurrently
    try:
        results = await asyncio.gather(*tasks, return_exceptions=True)
    except Exception as e:
        logger.error(f"Error during concurrent preloading: {e}")
        results = []
    
    # Process results
    successful = 0
    failed = 0
    total_time = time.time() - total_start
    
    for result in results:
        if isinstance(result, Exception):
            logger.error(f"Preload task failed: {result}")
            failed += 1
        elif result.get('success'):
            successful += 1
            _preload_status['progress'][result['module']] = result
        else:
            failed += 1
            _preload_status['errors'][result['module']] = result
    
    _preload_status['completed'] = True
    
    final_status = {
        'preload_completed': True,
        'total_time': total_time,
        'modules_loaded': successful,
        'modules_failed': failed,
        'modules_total': len(HEAVY_IMPORTS),
        'success_rate': (successful / len(HEAVY_IMPORTS)) * 100,
        'results': results
    }
    
    logger.info(
        f"Preloading completed in {total_time:.3f}s: "
        f"{successful}/{len(HEAVY_IMPORTS)} modules loaded "
        f"({final_status['success_rate']:.1f}% success rate)"
    )
    
    return final_status


def get_preload_status() -> Dict[str, Any]:
    """Get current preloading status."""
    return _preload_status.copy()