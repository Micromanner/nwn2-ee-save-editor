"""
Python Directory Walker - High-performance replacement for Rust DirectoryWalker

Provides optimized directory traversal for NWN2 2DA files with:
- Workshop directory scanning with Steam Workshop structure awareness
- Recursive and non-recursive directory indexing
- Parallel processing for multiple directories
- Memory-efficient file discovery using os.scandir()

This module replaces the Rust-based directory walker with a pure Python implementation
optimized for the specific directory structures used by NWN2 and Steam Workshop.
"""
import os
import time
import logging
import threading
from typing import Dict, List, Optional, Generator, Tuple
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed

logger = logging.getLogger(__name__)


class DirectoryWalkError(Exception):
    """Custom exception for directory walking errors"""
    pass


class PythonDirectoryWalker:
    """
    High-performance directory walker for 2DA files
    
    Optimized replacement for Python's directory traversal and workshop scanning.
    Features:
    - Steam Workshop structure awareness
    - Parallel processing for multiple directories
    - Memory-efficient scanning using generators
    - Comprehensive statistics tracking
    """
    
    def __init__(self, max_workers: Optional[int] = None):
        """
        Initialize the directory walker
        
        Args:
            max_workers: Maximum number of worker threads for parallel processing
        """
        self.max_workers = max_workers or min(8, (os.cpu_count() or 1) + 2)
        self._stats_lock = threading.Lock()
        self._stats: Dict[str, int] = {}
    
    def scan_workshop_directory(self, workshop_dir: Path) -> Dict[str, 'ResourceLocation']:
        """
        Scan workshop directory for 2DA override files
        
        Replicates the Python logic from ResourceManager._scan_workshop_directories
        but with native performance optimizations.
        
        Args:
            workshop_dir: Path to the workshop content directory
            
        Returns:
            Dictionary mapping resource names to ResourceLocation objects
        """
        from .python_resource_scanner import ResourceLocation
        
        start_time = time.time()
        resources = {}
        workshop_items_scanned = 0
        override_dirs_found = 0
        files_found = 0
        
        if not workshop_dir.is_dir():
            logger.warning(f"Workshop directory is not a directory: {workshop_dir}")
            return resources
        
        try:
            # Use os.scandir for better performance than iterdir()
            with os.scandir(workshop_dir) as entries:
                for entry in entries:
                    if not entry.is_dir():
                        continue
                    
                    workshop_items_scanned += 1
                    workshop_item_path = Path(entry.path)
                    
                    # Check for override directory in this workshop item
                    override_dir = workshop_item_path / "override"
                    
                    if override_dir.is_dir():
                        override_dirs_found += 1
                        
                        # Check subdirectories like override/2DA/
                        tda_subdir = override_dir / "2DA"
                        if tda_subdir.is_dir():
                            subdir_files = self._scan_directory_for_2das(tda_subdir, recursive=True)
                            files_found += len(subdir_files)
                            resources.update(subdir_files)
                        
                        # Also check root override directory (non-recursive to avoid duplicates)
                        root_files = self._scan_directory_for_2das(override_dir, recursive=False)
                        files_found += len(root_files)
                        resources.update(root_files)
            
            scan_time_ms = int((time.time() - start_time) * 1000)
            
            # Update statistics
            with self._stats_lock:
                self._stats['last_workshop_scan_time_ms'] = scan_time_ms
                self._stats['last_workshop_items_scanned'] = workshop_items_scanned
                self._stats['last_workshop_override_dirs'] = override_dirs_found
                self._stats['last_workshop_files_found'] = files_found
                
                # Update cumulative stats
                self._stats['total_workshop_scans'] = self._stats.get('total_workshop_scans', 0) + 1
                self._stats['total_workshop_scan_time_ms'] = self._stats.get('total_workshop_scan_time_ms', 0) + scan_time_ms
            
            logger.info(f"Workshop scan completed for {workshop_dir.name}: {files_found} 2DA files from {override_dirs_found} override dirs in {scan_time_ms}ms")
            
        except OSError as e:
            error_msg = f"Error scanning workshop directory {workshop_dir}: {e}"
            logger.error(error_msg)
            raise DirectoryWalkError(error_msg)
        
        return resources
    
    def index_directory(self, directory: Path, recursive: bool = True) -> Dict[str, 'ResourceLocation']:
        """
        Index directory for 2DA files
        
        Args:
            directory: Directory path to index
            recursive: Whether to scan recursively
            
        Returns:
            Dictionary mapping resource names to ResourceLocation objects
        """
        start_time = time.time()
        resources = self._scan_directory_for_2das(directory, recursive)
        scan_time_ms = int((time.time() - start_time) * 1000)
        
        # Update statistics
        with self._stats_lock:
            self._stats['last_dir_index_time_ms'] = scan_time_ms
            self._stats['last_dir_files_found'] = len(resources)
            
            # Update cumulative stats
            self._stats['total_dir_indexes'] = self._stats.get('total_dir_indexes', 0) + 1
            self._stats['total_dir_index_time_ms'] = self._stats.get('total_dir_index_time_ms', 0) + scan_time_ms
        
        logger.info(f"Directory index completed for {directory.name}: {len(resources)} 2DA files in {scan_time_ms}ms")
        
        return resources
    
    def _scan_directory_for_2das(self, directory: Path, recursive: bool) -> Dict[str, 'ResourceLocation']:
        """
        Internal method to scan directory for 2DA files
        
        Args:
            directory: Directory to scan
            recursive: Whether to scan subdirectories
            
        Returns:
            Dictionary of found 2DA files
        """
        from .python_resource_scanner import ResourceLocation
        
        resources = {}
        
        if not directory.is_dir():
            return resources
        
        try:
            # Use generator for memory efficiency
            file_generator = self._walk_directory(directory, recursive)
            
            for file_path in file_generator:
                try:
                    # Check if it's a 2DA file (case-insensitive)
                    if file_path.suffix.lower() == '.2da':
                        file_stat = file_path.stat()
                        
                        # Get the base filename (lowercase for case-insensitive lookup)
                        base_name = file_path.name.lower()
                        
                        resource_location = ResourceLocation(
                            source_type="file",
                            source_path=str(file_path),
                            internal_path=None,
                            size=file_stat.st_size,
                            modified_time=file_stat.st_mtime
                        )
                        
                        resources[base_name] = resource_location
                        
                except (OSError, IOError) as e:
                    logger.warning(f"Could not process file {file_path}: {e}")
                    continue
                    
        except OSError as e:
            error_msg = f"Error scanning directory {directory}: {e}"
            logger.error(error_msg)
            raise DirectoryWalkError(error_msg)
        
        return resources
    
    def _walk_directory(self, directory: Path, recursive: bool) -> Generator[Path, None, None]:
        """
        Memory-efficient directory walker using generators
        
        Args:
            directory: Directory to walk
            recursive: Whether to walk subdirectories
            
        Yields:
            Path objects for files found
        """
        try:
            if recursive:
                # Use os.walk for recursive scanning (most efficient for deep hierarchies)
                for root, _, filenames in os.walk(directory):
                    root_path = Path(root)
                    for filename in filenames:
                        yield root_path / filename
            else:
                # Use os.scandir for non-recursive scanning (most efficient for single level)
                with os.scandir(directory) as entries:
                    for entry in entries:
                        if entry.is_file():
                            yield Path(entry.path)
                            
        except OSError as e:
            logger.error(f"Error walking directory {directory}: {e}")
            raise DirectoryWalkError(f"Directory walk failed: {e}")
    
    def scan_directories_parallel(
        self, 
        directories: List[Path], 
        recursive: bool = True,
        is_workshop: bool = False
    ) -> Dict[str, 'ResourceLocation']:
        """
        Scan multiple directories in parallel
        
        Args:
            directories: List of directory paths to scan
            recursive: Whether to scan recursively
            is_workshop: Whether to use workshop-specific scanning logic
            
        Returns:
            Combined dictionary of all resources found
        """
        if not directories:
            return {}
        
        if len(directories) == 1:
            if is_workshop:
                return self.scan_workshop_directory(directories[0])
            else:
                return self.index_directory(directories[0], recursive)
        
        start_time = time.time()
        combined_resources = {}
        
        try:
            # Process directories in parallel
            with ThreadPoolExecutor(max_workers=self.max_workers) as executor:
                # Submit all tasks
                if is_workshop:
                    future_to_dir = {
                        executor.submit(self.scan_workshop_directory, directory): directory
                        for directory in directories
                    }
                else:
                    future_to_dir = {
                        executor.submit(self.index_directory, directory, recursive): directory
                        for directory in directories
                    }
                
                # Collect results
                for future in as_completed(future_to_dir):
                    directory = future_to_dir[future]
                    try:
                        dir_resources = future.result()
                        combined_resources.update(dir_resources)
                        logger.debug(f"Parallel scan completed for {directory.name}: {len(dir_resources)} resources")
                    except Exception as e:
                        logger.error(f"Failed to scan {directory} in parallel processing: {e}")
            
            total_time_ms = int((time.time() - start_time) * 1000)
            
            # Update parallel processing stats
            with self._stats_lock:
                self._stats['last_parallel_dir_time_ms'] = total_time_ms
                self._stats['last_parallel_dir_count'] = len(directories)
                self._stats['total_parallel_dir_operations'] = self._stats.get('total_parallel_dir_operations', 0) + 1
            
            logger.info(f"Parallel directory scan completed: {len(combined_resources)} resources from {len(directories)} directories in {total_time_ms}ms")
            
        except Exception as e:
            error_msg = f"Parallel directory scanning failed: {e}"
            logger.error(error_msg)
            raise DirectoryWalkError(error_msg)
        
        return combined_resources
    
    def get_directory_stats(self, directory: Path, recursive: bool = True) -> Optional[Dict[str, int]]:
        """
        Get statistics about a directory without full scanning
        
        Args:
            directory: Directory to analyze
            recursive: Whether to count recursively
            
        Returns:
            Dictionary with directory statistics or None if invalid
        """
        if not directory.is_dir():
            return None
        
        try:
            total_files = 0
            total_dirs = 0
            tda_files = 0
            total_size = 0
            
            if recursive:
                for root, dirs, files in os.walk(directory):
                    total_dirs += len(dirs)
                    for filename in files:
                        total_files += 1
                        if filename.lower().endswith('.2da'):
                            tda_files += 1
                        
                        try:
                            file_path = Path(root) / filename
                            total_size += file_path.stat().st_size
                        except (OSError, IOError):
                            continue
            else:
                with os.scandir(directory) as entries:
                    for entry in entries:
                        if entry.is_file():
                            total_files += 1
                            if entry.name.lower().endswith('.2da'):
                                tda_files += 1
                            try:
                                total_size += entry.stat().st_size
                            except (OSError, IOError):
                                continue
                        elif entry.is_dir():
                            total_dirs += 1
            
            return {
                'total_files': total_files,
                'total_directories': total_dirs,
                'tda_files': tda_files,
                'total_size_bytes': total_size
            }
            
        except OSError as e:
            logger.warning(f"Cannot get stats for directory {directory}: {e}")
            return None
    
    def find_2da_files_fast(self, directory: Path, recursive: bool = True) -> List[str]:
        """
        Fast method to just get list of 2DA filenames without full ResourceLocation objects
        
        Args:
            directory: Directory to search
            recursive: Whether to search recursively
            
        Returns:
            List of 2DA filenames found (lowercase)
        """
        tda_files = []
        
        if not directory.is_dir():
            return tda_files
        
        try:
            file_generator = self._walk_directory(directory, recursive)
            
            for file_path in file_generator:
                if file_path.suffix.lower() == '.2da':
                    tda_files.append(file_path.name.lower())
                    
        except DirectoryWalkError:
            logger.warning(f"Error during fast 2DA search in {directory}")
        
        return tda_files
    
    def get_stats(self) -> Dict[str, int]:
        """Get walker performance statistics"""
        with self._stats_lock:
            return self._stats.copy()
    
    def reset_stats(self):
        """Reset performance statistics"""
        with self._stats_lock:
            self._stats.clear()
        logger.debug("Directory walker statistics reset")
    
    def estimate_scan_time(self, directories: List[Path], recursive: bool = True) -> int:
        """
        Estimate scan time based on directory sizes and previous performance
        
        Args:
            directories: List of directories to scan
            recursive: Whether scanning will be recursive
            
        Returns:
            Estimated scan time in milliseconds
        """
        if not directories:
            return 0
        
        # Get average processing rate from stats
        with self._stats_lock:
            if recursive:
                total_time = self._stats.get('total_dir_index_time_ms', 0)
                total_ops = self._stats.get('total_dir_indexes', 0)
            else:
                total_time = self._stats.get('total_workshop_scan_time_ms', 0)
                total_ops = self._stats.get('total_workshop_scans', 0)
        
        if total_ops == 0:
            # Use default estimates based on typical performance
            base_time_per_dir = 50 if recursive else 20  # ms
            return base_time_per_dir * len(directories)
        
        # Use historical average
        avg_time_per_op = total_time / total_ops
        return int(avg_time_per_op * len(directories))
    
    def validate_directories(self, directories: List[Path]) -> Dict[str, bool]:
        """
        Validate multiple directories
        
        Args:
            directories: List of directory paths to validate
            
        Returns:
            Dictionary mapping directory paths to validation results
        """
        results = {}
        
        for directory in directories:
            try:
                results[str(directory)] = directory.exists() and directory.is_dir()
            except OSError:
                results[str(directory)] = False
        
        return results