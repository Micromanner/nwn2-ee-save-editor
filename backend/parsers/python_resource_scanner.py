"""
Python Resource Scanner - High-performance replacement for Rust ResourceScanner

Provides optimized scanning of NWN2 game files including:
- ZIP file indexing for 2DA resources
- Workshop directory traversal
- Override directory scanning
- Comprehensive resource location tracking

This module replaces the Rust-based resource scanner with a pure Python implementation
that maintains the same performance characteristics while being easier to maintain
and debug within the existing Django codebase.
"""
import os
import time
import logging
from typing import Dict, List, Optional, Union, Any, Tuple
from pathlib import Path
from dataclasses import dataclass, field
from concurrent.futures import ThreadPoolExecutor, as_completed
import threading

logger = logging.getLogger(__name__)


@dataclass
class ResourceLocation:
    """Represents the location and metadata of a game resource file"""
    source_type: str  # "zip", "file", "workshop"
    source_path: str
    internal_path: Optional[str] = None  # For ZIP files
    size: int = 0
    modified_time: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for API serialization"""
        return {
            'source_type': self.source_type,
            'source_path': self.source_path,
            'internal_path': self.internal_path,
            'size': self.size,
            'modified_time': self.modified_time
        }


@dataclass
class ScanResults:
    """Results from a comprehensive resource scan"""
    scan_time_ms: int
    resources_found: int
    zip_files_scanned: int
    directories_scanned: int
    workshop_items_found: int
    resource_locations: Dict[str, ResourceLocation] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for API serialization"""
        return {
            'scan_time_ms': self.scan_time_ms,
            'resources_found': self.resources_found,
            'zip_files_scanned': self.zip_files_scanned,
            'directories_scanned': self.directories_scanned,
            'workshop_items_found': self.workshop_items_found,
            'resource_locations': {k: v.to_dict() for k, v in self.resource_locations.items()}
        }


class ResourceScanError(Exception):
    """Custom exception for resource scanning errors"""
    pass


class PythonResourceScanner:
    """
    High-performance resource scanner for NWN2 game files
    
    Replaces the Rust ResourceScanner with optimized Python implementation.
    Features:
    - Parallel processing for improved performance
    - Comprehensive statistics tracking
    - Memory-efficient scanning with generators
    - Thread-safe operations
    """
    
    def __init__(self, max_workers: Optional[int] = None):
        """
        Initialize the resource scanner
        
        Args:
            max_workers: Maximum number of worker threads for parallel processing
        """
        self.max_workers = max_workers or min(32, (os.cpu_count() or 1) + 4)
        self._stats_lock = threading.Lock()
        self._stats: Dict[str, int] = {}
        
        # Import components lazily to avoid circular imports
        self._zip_indexer = None
        self._directory_walker = None
    
    @property
    def zip_indexer(self):
        """Lazy-load ZIP indexer to avoid circular imports"""
        if self._zip_indexer is None:
            from .python_zip_indexer import PythonZipIndexer
            self._zip_indexer = PythonZipIndexer()
        return self._zip_indexer
    
    @property
    def directory_walker(self):
        """Lazy-load directory walker to avoid circular imports"""
        if self._directory_walker is None:
            from .python_directory_walker import PythonDirectoryWalker
            self._directory_walker = PythonDirectoryWalker()
        return self._directory_walker
    
    def scan_zip_files(self, zip_paths: List[str]) -> Dict[str, ResourceLocation]:
        """
        Scan ZIP files for 2DA resources
        
        Args:
            zip_paths: List of ZIP file paths to scan
            
        Returns:
            Dictionary mapping resource names to ResourceLocation objects
        """
        if not zip_paths:
            return {}
        
        start_time = time.time()
        results = {}
        
        try:
            # Filter existing paths
            existing_paths = [Path(p) for p in zip_paths if Path(p).exists()]
            
            if not existing_paths:
                logger.warning("No valid ZIP paths found for scanning")
                return {}
            
            # Use parallel processing for multiple ZIP files
            if len(existing_paths) > 1:
                results = self.zip_indexer.index_zips_parallel(existing_paths)
            else:
                results = self.zip_indexer.index_zip(existing_paths[0])
            
            # Update statistics
            scan_time_ms = int((time.time() - start_time) * 1000)
            with self._stats_lock:
                self._stats['last_zip_scan_time_ms'] = scan_time_ms
                self._stats['last_zip_files_scanned'] = len(existing_paths)
                self._stats['last_zip_resources_found'] = len(results)
            
            logger.info(f"Scanned {len(existing_paths)} ZIP files in {scan_time_ms}ms, found {len(results)} resources")
            
        except Exception as e:
            logger.error(f"Error scanning ZIP files: {e}")
            raise ResourceScanError(f"ZIP scanning failed: {e}")
        
        return results
    
    def scan_workshop_directories(self, workshop_dirs: List[str]) -> Dict[str, ResourceLocation]:
        """
        Scan workshop directories for override files
        
        Args:
            workshop_dirs: List of workshop directory paths to scan
            
        Returns:
            Dictionary mapping resource names to ResourceLocation objects
        """
        if not workshop_dirs:
            return {}
        
        start_time = time.time()
        results = {}
        
        try:
            # Filter existing directories
            existing_dirs = [Path(d) for d in workshop_dirs if Path(d).exists() and Path(d).is_dir()]
            
            if not existing_dirs:
                logger.warning("No valid workshop directories found for scanning")
                return {}
            
            # Use parallel processing for multiple directories
            if len(existing_dirs) > 1:
                results = self.directory_walker.scan_directories_parallel(existing_dirs, is_workshop=True)
            else:
                results = self.directory_walker.scan_workshop_directory(existing_dirs[0])
            
            # Update statistics
            scan_time_ms = int((time.time() - start_time) * 1000)
            with self._stats_lock:
                self._stats['last_workshop_scan_time_ms'] = scan_time_ms
                self._stats['last_workshop_dirs_scanned'] = len(existing_dirs)
                self._stats['last_workshop_resources_found'] = len(results)
            
            logger.info(f"Scanned {len(existing_dirs)} workshop directories in {scan_time_ms}ms, found {len(results)} resources")
            
        except Exception as e:
            logger.error(f"Error scanning workshop directories: {e}")
            raise ResourceScanError(f"Workshop scanning failed: {e}")
        
        return results
    
    def index_directory(self, directory_path: str, recursive: bool = True) -> Dict[str, ResourceLocation]:
        """
        Index directory for 2DA files
        
        Args:
            directory_path: Directory path to index
            recursive: Whether to scan recursively
            
        Returns:
            Dictionary mapping resource names to ResourceLocation objects
        """
        dir_path = Path(directory_path)
        
        if not dir_path.exists() or not dir_path.is_dir():
            logger.warning(f"Directory does not exist or is not a directory: {directory_path}")
            return {}
        
        start_time = time.time()
        
        try:
            results = self.directory_walker.index_directory(dir_path, recursive)
            
            # Update statistics
            scan_time_ms = int((time.time() - start_time) * 1000)
            with self._stats_lock:
                self._stats['last_dir_index_time_ms'] = scan_time_ms
                self._stats['last_dir_resources_found'] = len(results)
            
            logger.info(f"Indexed directory {directory_path} in {scan_time_ms}ms, found {len(results)} resources")
            
        except Exception as e:
            logger.error(f"Error indexing directory {directory_path}: {e}")
            raise ResourceScanError(f"Directory indexing failed: {e}")
        
        return results
    
    def comprehensive_scan(
        self,
        nwn2_data_dir: str,
        workshop_dirs: List[str],
        custom_override_dirs: List[str],
        enhanced_data_dir: Optional[str] = None
    ) -> ScanResults:
        """
        Comprehensive resource scan
        
        Performs all scanning operations and returns combined results with timing info.
        
        Args:
            nwn2_data_dir: NWN2 data directory path
            workshop_dirs: List of workshop directories
            custom_override_dirs: List of custom override directories
            enhanced_data_dir: Enhanced edition data directory (optional)
            
        Returns:
            ScanResults object with timing and resource information
        """
        start_time = time.time()
        all_resources = {}
        zip_files_scanned = 0
        directories_scanned = 0
        workshop_items_found = 0
        
        try:
            # 1. Scan ZIP files in data directories
            zip_files = ["2da.zip", "2da_x1.zip", "2da_x2.zip"]
            zip_paths = []
            
            # Base NWN2 data directory
            data_dir = Path(nwn2_data_dir)
            if data_dir.exists():
                for zip_name in zip_files:
                    zip_path = data_dir / zip_name
                    if zip_path.exists():
                        zip_paths.append(str(zip_path))
            
            # Enhanced edition data directory
            if enhanced_data_dir:
                enhanced_path = Path(enhanced_data_dir)
                if enhanced_path.exists():
                    for zip_name in zip_files:
                        zip_path = enhanced_path / zip_name
                        if zip_path.exists():
                            zip_paths.append(str(zip_path))
            
            # Scan ZIP files
            if zip_paths:
                zip_resources = self.scan_zip_files(zip_paths)
                zip_files_scanned = len(zip_paths)
                all_resources.update(zip_resources)
                logger.info(f"ZIP scan completed: {len(zip_resources)} resources from {zip_files_scanned} files")
            
            # 2. Scan workshop directories
            if workshop_dirs:
                workshop_resources = self.scan_workshop_directories(workshop_dirs)
                workshop_items_found = len(workshop_resources)
                all_resources.update(workshop_resources)
                logger.info(f"Workshop scan completed: {workshop_items_found} resources")
            
            # 3. Scan custom override directories
            for override_dir in custom_override_dirs:
                try:
                    override_resources = self.index_directory(override_dir, recursive=True)
                    directories_scanned += 1
                    all_resources.update(override_resources)
                    logger.info(f"Override directory scan completed: {len(override_resources)} resources from {override_dir}")
                except Exception as e:
                    logger.warning(f"Failed to scan override directory {override_dir}: {e}")
            
            scan_time_ms = int((time.time() - start_time) * 1000)
            
            results = ScanResults(
                scan_time_ms=scan_time_ms,
                resources_found=len(all_resources),
                zip_files_scanned=zip_files_scanned,
                directories_scanned=directories_scanned,
                workshop_items_found=workshop_items_found,
                resource_locations=all_resources
            )
            
            logger.info(f"Comprehensive scan completed in {scan_time_ms}ms: {len(all_resources)} total resources")
            
            # Update global statistics  
            with self._stats_lock:
                self._stats['last_comprehensive_scan_time_ms'] = scan_time_ms
                self._stats['last_comprehensive_resources_found'] = len(all_resources)
                self._stats['total_comprehensive_scans'] = self._stats.get('total_comprehensive_scans', 0) + 1
            
            return results
            
        except Exception as e:
            logger.error(f"Comprehensive scan failed: {e}")
            raise ResourceScanError(f"Comprehensive scan failed: {e}")
    
    def get_performance_stats(self) -> Dict[str, int]:
        """Get performance statistics from the last scan"""
        with self._stats_lock:
            stats = self._stats.copy()
        
        # Add stats from sub-components
        if self._zip_indexer:
            stats.update(self.zip_indexer.get_stats())
        if self._directory_walker:
            stats.update(self.directory_walker.get_stats())
        
        return stats
    
    def reset_stats(self):
        """Reset all performance statistics"""
        with self._stats_lock:
            self._stats.clear()
        
        if self._zip_indexer:
            self.zip_indexer.reset_stats()
        if self._directory_walker:
            self.directory_walker.reset_stats()
        
        logger.info("Performance statistics reset")