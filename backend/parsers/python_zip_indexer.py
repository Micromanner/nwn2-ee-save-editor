"""
Python ZIP Indexer - High-performance replacement for Rust ZipIndexer

Provides optimized ZIP file scanning for NWN2 2DA resources with:
- Parallel processing for multiple ZIP files
- Case-insensitive file detection
- Comprehensive statistics tracking
- Memory-efficient processing

This module replaces the Rust-based ZIP indexer with a pure Python implementation
that maintains high performance while being easier to integrate and debug.
"""
import os
import time
import zipfile
import logging
import threading
from typing import Dict, List, Optional, Union
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed

logger = logging.getLogger(__name__)


class ZipIndexError(Exception):
    """Custom exception for ZIP indexing errors"""
    pass


class PythonZipIndexer:
    """
    High-performance ZIP file indexer for 2DA resources
    
    Optimized replacement for Python's ZIP scanning logic in ResourceManager.
    Features:
    - Parallel processing for multiple ZIP files
    - Comprehensive statistics tracking
    - Memory-efficient file processing
    - Thread-safe operations
    """
    
    def __init__(self, max_workers: Optional[int] = None):
        """
        Initialize the ZIP indexer
        
        Args:
            max_workers: Maximum number of worker threads for parallel processing
        """
        self.max_workers = max_workers or min(8, (os.cpu_count() or 1) + 2)
        self._stats_lock = threading.Lock()
        self._stats: Dict[str, int] = {}
    
    def index_zip(self, zip_path: Path) -> Dict[str, 'ResourceLocation']:
        """
        Index a single ZIP file for 2DA resources
        
        Args:
            zip_path: Path to the ZIP file
            
        Returns:
            Dictionary mapping resource names (lowercase) to ResourceLocation objects
        """
        from .python_resource_scanner import ResourceLocation
        
        start_time = time.time()
        resources = {}
        
        if not zip_path.exists():
            logger.warning(f"ZIP file does not exist: {zip_path}")
            return resources
        
        try:
            # Get ZIP file metadata
            zip_stat = zip_path.stat()
            zip_size = zip_stat.st_size
            zip_modified = zip_stat.st_mtime
            
            files_processed = 0
            tda_files_found = 0
            
            # Open and process ZIP file
            with zipfile.ZipFile(zip_path, 'r') as archive:
                for file_info in archive.filelist:
                    files_processed += 1
                    file_name = file_info.filename
                    
                    # Check if it's a 2DA file (case-insensitive)
                    if file_name.lower().endswith('.2da'):
                        tda_files_found += 1
                        
                        # Extract the base filename (without path)
                        base_name = Path(file_name).name.lower()
                        
                        resource_location = ResourceLocation(
                            source_type="zip",
                            source_path=str(zip_path),
                            internal_path=file_name,
                            size=file_info.file_size,
                            modified_time=zip_modified
                        )
                        
                        # Store with lowercase name for case-insensitive lookup
                        # Later ZIP files will override earlier ones (X2 > X1 > base)
                        resources[base_name] = resource_location
                        
                        # Log expansion overrides for debugging
                        if 'x1' in zip_path.name.lower() or 'x2' in zip_path.name.lower():
                            logger.debug(f"Expansion override: {base_name} from {zip_path.name}:{file_name}")
            
            index_time_ms = int((time.time() - start_time) * 1000)
            
            # Update statistics
            with self._stats_lock:
                self._stats['last_zip_index_time_ms'] = index_time_ms
                self._stats['last_zip_size_bytes'] = zip_size
                self._stats['last_zip_files_processed'] = files_processed
                self._stats['last_zip_2da_files_found'] = tda_files_found
                
                # Update cumulative stats
                self._stats['total_zips_indexed'] = self._stats.get('total_zips_indexed', 0) + 1
                self._stats['total_zip_index_time_ms'] = self._stats.get('total_zip_index_time_ms', 0) + index_time_ms
                self._stats['total_2da_files_indexed'] = self._stats.get('total_2da_files_indexed', 0) + tda_files_found
            
            logger.debug(f"Indexed {zip_path.name}: {tda_files_found} 2DA files from {files_processed} total files in {index_time_ms}ms")
            
        except zipfile.BadZipFile as e:
            error_msg = f"Bad ZIP file {zip_path}: {e}"
            logger.error(error_msg)
            raise ZipIndexError(error_msg)
        except Exception as e:
            error_msg = f"Error indexing ZIP {zip_path}: {e}"
            logger.error(error_msg)
            raise ZipIndexError(error_msg)
        
        return resources
    
    def index_zips_parallel(self, zip_paths: List[Path]) -> Dict[str, 'ResourceLocation']:
        """
        Index multiple ZIP files in parallel
        
        Args:
            zip_paths: List of ZIP file paths
            
        Returns:
            Combined dictionary of all resources found
        """
        if not zip_paths:
            return {}
        
        if len(zip_paths) == 1:
            return self.index_zip(zip_paths[0])
        
        start_time = time.time()
        combined_resources = {}
        
        try:
            # Process ZIP files in parallel
            with ThreadPoolExecutor(max_workers=self.max_workers) as executor:
                # Submit all tasks
                future_to_path = {
                    executor.submit(self.index_zip, zip_path): zip_path 
                    for zip_path in zip_paths
                }
                
                # Collect results in order to maintain override precedence
                results = {}
                for future in as_completed(future_to_path):
                    zip_path = future_to_path[future]
                    try:
                        zip_resources = future.result()
                        results[zip_path] = zip_resources
                        logger.debug(f"Parallel indexing completed for {zip_path.name}: {len(zip_resources)} resources")
                    except Exception as e:
                        logger.error(f"Failed to index {zip_path} in parallel processing: {e}")
                        results[zip_path] = {}
                
                # Apply resources in original order to maintain override precedence
                # (base game → X1 → X2, so later files override earlier ones)
                for zip_path in zip_paths:
                    if zip_path in results:
                        zip_resources = results[zip_path]
                        combined_resources.update(zip_resources)
                        if 'x1' in zip_path.name.lower() or 'x2' in zip_path.name.lower():
                            # Count expansion overrides
                            expansion_overrides = len([name for name in zip_resources.keys() if name in combined_resources])
                            logger.info(f"Applied {expansion_overrides} expansion overrides from {zip_path.name}")
            
            total_time_ms = int((time.time() - start_time) * 1000)
            
            # Update parallel processing stats
            with self._stats_lock:
                self._stats['last_parallel_zip_time_ms'] = total_time_ms
                self._stats['last_parallel_zip_count'] = len(zip_paths)
                self._stats['total_parallel_operations'] = self._stats.get('total_parallel_operations', 0) + 1
            
            logger.info(f"Parallel ZIP indexing completed: {len(combined_resources)} resources from {len(zip_paths)} files in {total_time_ms}ms")
            
        except Exception as e:
            error_msg = f"Parallel ZIP indexing failed: {e}"
            logger.error(error_msg)
            raise ZipIndexError(error_msg)
        
        return combined_resources
    
    def _index_zip_worker(self, zip_path: Path) -> Dict[str, 'ResourceLocation']:
        """
        Worker method for parallel processing
        
        Args:
            zip_path: Path to ZIP file to index
            
        Returns:
            Dictionary of resources found in this ZIP
        """
        try:
            return self.index_zip(zip_path)
        except Exception as e:
            logger.error(f"Worker failed to index {zip_path}: {e}")
            return {}
    
    def get_stats(self) -> Dict[str, int]:
        """Get indexer performance statistics"""
        with self._stats_lock:
            return self._stats.copy()
    
    def reset_stats(self):
        """Reset performance statistics"""
        with self._stats_lock:
            self._stats.clear()
        logger.debug("ZIP indexer statistics reset")
    
    def get_zip_info(self, zip_path: Path) -> Optional[Dict[str, Union[int, str]]]:
        """
        Get basic information about a ZIP file without full indexing
        
        Args:
            zip_path: Path to ZIP file
            
        Returns:
            Dictionary with ZIP file information or None if invalid
        """
        if not zip_path.exists():
            return None
        
        try:
            zip_stat = zip_path.stat()
            
            with zipfile.ZipFile(zip_path, 'r') as archive:
                total_files = len(archive.filelist)
                tda_files = sum(1 for f in archive.filelist if f.filename.lower().endswith('.2da'))
            
            return {
                'path': str(zip_path),
                'size_bytes': zip_stat.st_size,
                'modified_time': zip_stat.st_mtime,
                'total_files': total_files,
                'tda_files': tda_files
            }
            
        except (zipfile.BadZipFile, OSError) as e:
            logger.warning(f"Cannot get info for ZIP {zip_path}: {e}")
            return None
    
    def validate_zip_files(self, zip_paths: List[Path]) -> Dict[str, bool]:
        """
        Validate multiple ZIP files
        
        Args:
            zip_paths: List of ZIP file paths to validate
            
        Returns:
            Dictionary mapping file paths to validation results
        """
        results = {}
        
        for zip_path in zip_paths:
            try:
                if not zip_path.exists():
                    results[str(zip_path)] = False
                    continue
                
                with zipfile.ZipFile(zip_path, 'r') as archive:
                    # Try to read the file list - this will fail if ZIP is corrupt
                    _ = archive.filelist
                    results[str(zip_path)] = True
                    
            except (zipfile.BadZipFile, OSError):
                results[str(zip_path)] = False
        
        return results
    
    def estimate_scan_time(self, zip_paths: List[Path]) -> int:
        """
        Estimate scan time based on file sizes and previous performance
        
        Args:
            zip_paths: List of ZIP file paths
            
        Returns:
            Estimated scan time in milliseconds
        """
        if not zip_paths:
            return 0
        
        # Get average processing rate from stats
        with self._stats_lock:
            total_time = self._stats.get('total_zip_index_time_ms', 0)
            total_zips = self._stats.get('total_zips_indexed', 0)
        
        if total_zips == 0:
            # Use default estimate: ~2ms per MB for ZIP processing
            total_size = sum(p.stat().st_size for p in zip_paths if p.exists())
            return int((total_size / 1024 / 1024) * 2)
        
        # Use historical average
        avg_time_per_zip = total_time / total_zips
        return int(avg_time_per_zip * len(zip_paths))
    
    def get_compression_stats(self, zip_path: Path) -> Optional[Dict[str, float]]:
        """
        Get compression statistics for a ZIP file
        
        Args:
            zip_path: Path to ZIP file
            
        Returns:
            Dictionary with compression statistics or None if invalid
        """
        if not zip_path.exists():
            return None
        
        try:
            with zipfile.ZipFile(zip_path, 'r') as archive:
                total_compressed = 0
                total_uncompressed = 0
                
                for file_info in archive.filelist:
                    if file_info.filename.lower().endswith('.2da'):
                        total_compressed += file_info.compress_size
                        total_uncompressed += file_info.file_size
                
                if total_uncompressed == 0:
                    return None
                
                compression_ratio = (total_uncompressed - total_compressed) / total_uncompressed
                
                return {
                    'compressed_size': total_compressed,
                    'uncompressed_size': total_uncompressed,
                    'compression_ratio': compression_ratio,
                    'space_saved': total_uncompressed - total_compressed
                }
                
        except (zipfile.BadZipFile, OSError) as e:
            logger.warning(f"Cannot get compression stats for {zip_path}: {e}")
            return None