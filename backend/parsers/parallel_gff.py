"""
Parallel GFF parsing utilities for improved performance

Uses multiprocessing to parse multiple GFF files simultaneously,
providing significant speedup for save game loading.
"""
import tempfile
import os
from concurrent.futures import ProcessPoolExecutor, as_completed
from typing import List, Tuple, Dict, Any
from .gff import GFFParser


def parse_gff_data(filename: str, data_bytes: bytes) -> Dict[str, Any]:
    """
    Parse GFF data from bytes in a separate process
    
    Args:
        filename: Name of the file (for error reporting)
        data_bytes: Raw GFF file data
        
    Returns:
        Dict containing parsed data and metadata
    """
    try:
        # Create temporary file from data
        with tempfile.NamedTemporaryFile(suffix='.gff', delete=False) as tmp:
            tmp.write(data_bytes)
            temp_path = tmp.name
        
        try:
            # Parse the GFF file
            parser = GFFParser()
            result = parser.read(temp_path)
            
            return {
                'filename': filename,
                'success': True,
                'data': parser.top_level_struct.to_dict(),
                'file_type': parser.file_type,
                'error': None
            }
        finally:
            os.unlink(temp_path)
            
    except Exception as e:
        return {
            'filename': filename,
            'success': False,
            'data': None,
            'file_type': 'unknown',
            'error': str(e)
        }


def parse_gff_files_parallel(gff_files: List[Tuple[str, bytes]], max_workers: int = 4) -> Dict[str, Dict[str, Any]]:
    """
    Parse multiple GFF files in parallel using multiprocessing
    
    Args:
        gff_files: List of (filename, data_bytes) tuples
        max_workers: Number of processes to use (default: 4)
        
    Returns:
        Dict mapping filename to parsed data or error info
        
    Example:
        gff_files = [
            ('player.bic', bic_data),
            ('playerlist.ifo', ifo_data),
            ('companion.ros', ros_data)
        ]
        results = parse_gff_files_parallel(gff_files)
        player_data = results['player.bic']['data']
    """
    results = {}
    
    if not gff_files:
        return results
    
    # For single files, don't use multiprocessing (overhead not worth it)
    if len(gff_files) == 1:
        filename, data_bytes = gff_files[0]
        result = parse_gff_data(filename, data_bytes)
        results[filename] = result
        return results
    
    # Use multiprocessing for multiple files
    with ProcessPoolExecutor(max_workers=max_workers) as executor:
        # Submit all parsing tasks
        future_to_filename = {
            executor.submit(parse_gff_data, filename, data): filename
            for filename, data in gff_files
        }
        
        # Collect results as they complete
        for future in as_completed(future_to_filename):
            filename = future_to_filename[future]
            try:
                result = future.result()
                results[filename] = result
            except Exception as e:
                results[filename] = {
                    'filename': filename,
                    'success': False,
                    'data': None,
                    'file_type': 'unknown',
                    'error': str(e)
                }
    
    return results


def extract_and_parse_save_gff_files(save_handler, max_workers: int = 4) -> Dict[str, Dict[str, Any]]:
    """
    Extract and parse all GFF files from a save game in parallel
    
    Args:
        save_handler: SaveGameHandler instance
        max_workers: Number of processes to use
        
    Returns:
        Dict mapping filename to parsed GFF data
        
    Example:
        from parsers.savegame_handler import SaveGameHandler
        handler = SaveGameHandler('/path/to/save')
        results = extract_and_parse_save_gff_files(handler)
        
        player_data = results['playerlist.ifo']['data']
        if 'player.bic' in results:
            bic_data = results['player.bic']['data']
    """
    gff_files = []
    
    # Extract playerlist.ifo (always present)
    try:
        player_data = save_handler.extract_player_data()
        gff_files.append(('playerlist.ifo', player_data))
    except Exception as e:
        # If we can't get player data, this save is unusable
        raise ValueError(f"Could not extract playerlist.ifo: {e}")
    
    # Extract player.bic if present
    try:
        bic_data = save_handler.extract_player_bic()
        if bic_data:
            gff_files.append(('player.bic', bic_data))
    except Exception:
        pass  # player.bic is optional
    
    # Extract companion .ros files
    try:
        file_list = save_handler.list_files()
        companion_files = [f for f in file_list if f.endswith('.ros')]
        
        for filename in companion_files:
            try:
                file_data = save_handler.extract_file(filename)
                if file_data:
                    gff_files.append((filename, file_data))
            except Exception:
                continue  # Skip failed companion files
    except Exception:
        pass  # If we can't list files, just continue with what we have
    
    # Parse all files in parallel
    return parse_gff_files_parallel(gff_files, max_workers)