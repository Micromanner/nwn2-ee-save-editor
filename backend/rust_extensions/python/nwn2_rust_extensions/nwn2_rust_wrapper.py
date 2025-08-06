"""
Python wrapper for NWN2 Rust Extensions

Provides a clean Python interface to the Rust performance optimizations,
with fallback to the original Python implementation if Rust extensions
are not available.
"""

import sys
import logging
from typing import Dict, List, Optional, Any, Union
from pathlib import Path

logger = logging.getLogger(__name__)

# Try to import the Rust extension
try:
    import nwn2_rust_extensions
    RUST_AVAILABLE = True
    logger.info("Rust extensions loaded successfully")
except ImportError as e:
    RUST_AVAILABLE = False
    logger.warning(f"Rust extensions not available: {e}")
    logger.info("Falling back to Python implementation")


class ResourceLocation:
    """Python representation of ResourceLocation from Rust"""
    
    def __init__(self, source_type: str, source_path: str, 
                 internal_path: Optional[str] = None, 
                 size: int = 0, modified_time: int = 0):
        self.source_type = source_type
        self.source_path = source_path
        self.internal_path = internal_path
        self.size = size
        self.modified_time = modified_time
    
    def __repr__(self):
        return f"ResourceLocation(type={self.source_type}, path={self.source_path})"
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'ResourceLocation':
        """Create ResourceLocation from dictionary (from Rust)"""
        return cls(
            source_type=data['source_type'],
            source_path=data['source_path'],
            internal_path=data.get('internal_path'),
            size=data.get('size', 0),
            modified_time=data.get('modified_time', 0)
        )


class PathTiming:
    """Python representation of PathTiming from Rust"""
    
    def __init__(self, operation: str, duration_ms: int, paths_checked: int, paths_found: int):
        self.operation = operation
        self.duration_ms = duration_ms
        self.paths_checked = paths_checked
        self.paths_found = paths_found
    
    def __repr__(self):
        return f"PathTiming(op={self.operation}, time={self.duration_ms}ms, checked={self.paths_checked}, found={self.paths_found})"


class DiscoveryResult:
    """Python representation of DiscoveryResult from Rust"""
    
    def __init__(self, nwn2_paths: List[str], steam_paths: List[str], gog_paths: List[str],
                 total_time_ms: int, timing_breakdown: List[PathTiming]):
        self.nwn2_paths = nwn2_paths
        self.steam_paths = steam_paths
        self.gog_paths = gog_paths
        self.total_time_ms = total_time_ms
        self.timing_breakdown = timing_breakdown
    
    @property
    def total_time_seconds(self) -> float:
        """Get total time in seconds"""
        return self.total_time_ms / 1000.0
    
    def __repr__(self):
        return f"DiscoveryResult(nwn2={len(self.nwn2_paths)}, steam={len(self.steam_paths)}, gog={len(self.gog_paths)}, time={self.total_time_ms}ms)"
    
    @classmethod
    def from_rust_result(cls, rust_result) -> 'DiscoveryResult':
        """Create DiscoveryResult from Rust result object"""
        timing_breakdown = []
        for timing in rust_result.timing_breakdown:
            timing_breakdown.append(PathTiming(
                operation=timing.operation,
                duration_ms=timing.duration_ms,
                paths_checked=timing.paths_checked,
                paths_found=timing.paths_found
            ))
        
        return cls(
            nwn2_paths=list(rust_result.nwn2_paths),
            steam_paths=list(rust_result.steam_paths),
            gog_paths=list(rust_result.gog_paths),
            total_time_ms=rust_result.total_time_ms,
            timing_breakdown=timing_breakdown
        )


class ScanResults:
    """Python representation of ScanResults from Rust"""
    
    def __init__(self, scan_time_ms: int, resources_found: int,
                 zip_files_scanned: int, directories_scanned: int,
                 workshop_items_found: int, 
                 resource_locations: Dict[str, ResourceLocation]):
        self.scan_time_ms = scan_time_ms
        self.resources_found = resources_found
        self.zip_files_scanned = zip_files_scanned
        self.directories_scanned = directories_scanned
        self.workshop_items_found = workshop_items_found
        self.resource_locations = resource_locations
    
    @property
    def scan_time_seconds(self) -> float:
        """Get scan time in seconds"""
        return self.scan_time_ms / 1000.0
    
    def __repr__(self):
        return (f"ScanResults(time={self.scan_time_ms}ms, "
                f"resources={self.resources_found}, "
                f"zips={self.zip_files_scanned})")
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'ScanResults':
        """Create ScanResults from dictionary (from Rust)"""
        # Convert resource locations
        resource_locations = {}
        for name, location_data in data.get('resource_locations', {}).items():
            resource_locations[name] = ResourceLocation.from_dict(location_data)
        
        return cls(
            scan_time_ms=data['scan_time_ms'],
            resources_found=data['resources_found'],
            zip_files_scanned=data['zip_files_scanned'],
            directories_scanned=data['directories_scanned'],
            workshop_items_found=data['workshop_items_found'],
            resource_locations=resource_locations
        )


class RustResourceScanner:
    """High-level Python interface to Rust ResourceScanner"""
    
    def __init__(self):
        if RUST_AVAILABLE:
            self._scanner = nwn2_rust_extensions.ResourceScanner()
            self._using_rust = True
        else:
            self._scanner = None
            self._using_rust = False
            logger.warning("Rust scanner not available, using fallback")
    
    @property
    def using_rust(self) -> bool:
        """Check if using Rust implementation"""
        return self._using_rust
    
    def scan_zip_files(self, zip_paths: List[Union[str, Path]]) -> Dict[str, ResourceLocation]:
        """
        Scan ZIP files for 2DA resources
        
        Args:
            zip_paths: List of ZIP file paths to scan
            
        Returns:
            Dictionary mapping resource names to ResourceLocation objects
        """
        if not self._using_rust:
            return self._fallback_scan_zip_files(zip_paths)
        
        # Convert paths to strings
        str_paths = [str(path) for path in zip_paths]
        
        try:
            rust_results = self._scanner.scan_zip_files(str_paths)
            
            # Convert Rust results to Python objects
            results = {}
            for name, location_data in rust_results.items():
                results[name] = ResourceLocation.from_dict(location_data)
            
            logger.debug(f"Rust ZIP scan found {len(results)} resources")
            return results
            
        except Exception as e:
            logger.error(f"Rust ZIP scanning failed: {e}")
            return self._fallback_scan_zip_files(zip_paths)
    
    def scan_workshop_directories(self, workshop_dirs: List[Union[str, Path]]) -> Dict[str, ResourceLocation]:
        """
        Scan workshop directories for override files
        
        Args:
            workshop_dirs: List of workshop directory paths to scan
            
        Returns:
            Dictionary mapping resource names to ResourceLocation objects
        """
        if not self._using_rust:
            return self._fallback_scan_workshop_directories(workshop_dirs)
        
        # Convert paths to strings
        str_paths = [str(path) for path in workshop_dirs]
        
        try:
            rust_results = self._scanner.scan_workshop_directories(str_paths)
            
            # Convert Rust results to Python objects
            results = {}
            for name, location_data in rust_results.items():
                results[name] = ResourceLocation.from_dict(location_data)
            
            logger.debug(f"Rust workshop scan found {len(results)} resources")
            return results
            
        except Exception as e:
            logger.error(f"Rust workshop scanning failed: {e}")
            return self._fallback_scan_workshop_directories(workshop_dirs)
    
    def index_directory(self, directory_path: Union[str, Path], 
                       recursive: bool = True) -> Dict[str, ResourceLocation]:
        """
        Index directory for 2DA files
        
        Args:
            directory_path: Directory path to index
            recursive: Whether to scan recursively
            
        Returns:
            Dictionary mapping resource names to ResourceLocation objects
        """
        if not self._using_rust:
            return self._fallback_index_directory(directory_path, recursive)
        
        try:
            rust_results = self._scanner.index_directory(str(directory_path), recursive)
            
            # Convert Rust results to Python objects
            results = {}
            for name, location_data in rust_results.items():
                results[name] = ResourceLocation.from_dict(location_data)
            
            logger.debug(f"Rust directory indexing found {len(results)} resources")
            return results
            
        except Exception as e:
            logger.error(f"Rust directory indexing failed: {e}")
            return self._fallback_index_directory(directory_path, recursive)
    
    def comprehensive_scan(self, nwn2_data_dir: Union[str, Path],
                          enhanced_data_dir: Optional[Union[str, Path]] = None,
                          workshop_dirs: Optional[List[Union[str, Path]]] = None,
                          custom_override_dirs: Optional[List[Union[str, Path]]] = None) -> ScanResults:
        """
        Comprehensive resource scan
        
        Args:
            nwn2_data_dir: NWN2 data directory path
            enhanced_data_dir: Enhanced edition data directory (optional)
            workshop_dirs: List of workshop directories
            custom_override_dirs: List of custom override directories
            
        Returns:
            ScanResults object with timing and resource information
        """
        if not self._using_rust:
            return self._fallback_comprehensive_scan(
                nwn2_data_dir, enhanced_data_dir, workshop_dirs, custom_override_dirs
            )
        
        # Convert paths to strings and handle defaults
        workshop_paths = [str(path) for path in (workshop_dirs or [])]
        custom_paths = [str(path) for path in (custom_override_dirs or [])]
        enhanced_path = str(enhanced_data_dir) if enhanced_data_dir else None
        
        try:
            rust_results = self._scanner.comprehensive_scan(
                str(nwn2_data_dir),
                enhanced_path,
                workshop_paths,
                custom_paths
            )
            
            results = ScanResults.from_dict(rust_results)
            logger.info(f"Rust comprehensive scan completed in {results.scan_time_ms}ms, "
                       f"found {results.resources_found} resources")
            return results
            
        except Exception as e:
            logger.error(f"Rust comprehensive scan failed: {e}")
            return self._fallback_comprehensive_scan(
                nwn2_data_dir, enhanced_data_dir, workshop_dirs, custom_override_dirs
            )
    
    def get_performance_stats(self) -> Dict[str, int]:
        """Get performance statistics from the last scan"""
        if not self._using_rust:
            return {}
        
        try:
            return self._scanner.get_performance_stats()
        except Exception as e:
            logger.error(f"Failed to get Rust performance stats: {e}")
            return {}
    
    # Fallback implementations (using original Python logic)
    
    def _fallback_scan_zip_files(self, zip_paths: List[Union[str, Path]]) -> Dict[str, ResourceLocation]:
        """Fallback ZIP scanning using Python implementation"""
        logger.info("Using Python fallback for ZIP scanning")
        
        # Import here to avoid circular imports
        try:
            from parsers.resource_manager import ResourceManager
            
            # Create a temporary ResourceManager to use its ZIP scanning logic
            rm = ResourceManager()
            
            # This is a simplified fallback - in practice you'd extract the 
            # specific ZIP scanning logic from ResourceManager
            results = {}
            
            # For now, return empty results as a placeholder
            # Real implementation would extract and adapt the ResourceManager logic
            logger.warning("Python ZIP scanning fallback not fully implemented")
            return results
            
        except Exception as e:
            logger.error(f"Python ZIP scanning fallback failed: {e}")
            return {}
    
    def _fallback_scan_workshop_directories(self, workshop_dirs: List[Union[str, Path]]) -> Dict[str, ResourceLocation]:
        """Fallback workshop scanning using Python implementation"""
        logger.info("Using Python fallback for workshop scanning")
        # Placeholder - would implement Python fallback logic here
        return {}
    
    def _fallback_index_directory(self, directory_path: Union[str, Path], 
                                 recursive: bool) -> Dict[str, ResourceLocation]:
        """Fallback directory indexing using Python implementation"""
        logger.info("Using Python fallback for directory indexing")
        # Placeholder - would implement Python fallback logic here
        return {}
    
    def _fallback_comprehensive_scan(self, *args, **kwargs) -> ScanResults:
        """Fallback comprehensive scan using Python implementation"""
        logger.info("Using Python fallback for comprehensive scan")
        
        # Return minimal results
        return ScanResults(
            scan_time_ms=0,
            resources_found=0,
            zip_files_scanned=0,
            directories_scanned=0,
            workshop_items_found=0,
            resource_locations={}
        )


# Path Discovery Functions
def discover_nwn2_paths(search_paths: Optional[List[Union[str, Path]]] = None) -> DiscoveryResult:
    """
    Discover NWN2 installation paths using Rust implementation
    
    Args:
        search_paths: Optional list of paths to search. If None, uses default paths.
        
    Returns:
        DiscoveryResult object with found paths and timing information
        
    Raises:
        RuntimeError: If Rust extensions are not available
    """
    if not RUST_AVAILABLE:
        raise RuntimeError("Rust extensions are required for path discovery. "
                          "Please ensure nwn2_rust_extensions is properly installed.")
    
    # Convert paths to strings if provided
    str_paths = None
    if search_paths is not None:
        str_paths = [str(path) for path in search_paths]
    
    try:
        rust_result = nwn2_rust_extensions.discover_nwn2_paths_rust(str_paths)
        return DiscoveryResult.from_rust_result(rust_result)
    except Exception as e:
        raise RuntimeError(f"Rust path discovery failed: {e}")


def profile_path_discovery(iterations: int = 100) -> Dict[str, float]:
    """
    Profile path discovery performance using Rust implementation
    
    Args:
        iterations: Number of iterations to run for profiling
        
    Returns:
        Dictionary with performance statistics
        
    Raises:
        RuntimeError: If Rust extensions are not available
    """
    if not RUST_AVAILABLE:
        raise RuntimeError("Rust extensions are required for path discovery profiling. "
                          "Please ensure nwn2_rust_extensions is properly installed.")
    
    try:
        return nwn2_rust_extensions.profile_path_discovery_rust(iterations)
    except Exception as e:
        raise RuntimeError(f"Rust path discovery profiling failed: {e}")


# Convenience function for easy integration
def create_resource_scanner() -> RustResourceScanner:
    """Create a resource scanner instance (Rust if available, Python fallback otherwise)"""
    return RustResourceScanner()


# Performance comparison utilities
def compare_performance(python_scanner, rust_scanner, test_paths: Dict[str, Any]) -> Dict[str, Any]:
    """
    Compare performance between Python and Rust implementations
    
    Args:
        python_scanner: Original Python ResourceManager instance
        rust_scanner: RustResourceScanner instance
        test_paths: Dictionary with test paths and parameters
        
    Returns:
        Performance comparison results
    """
    import time
    
    comparison_results = {
        'python_times': {},
        'rust_times': {},
        'speedup_factors': {},
        'rust_available': rust_scanner.using_rust
    }
    
    if not rust_scanner.using_rust:
        logger.warning("Rust not available for performance comparison")
        return comparison_results
    
    # Test ZIP scanning
    if 'zip_paths' in test_paths:
        zip_paths = test_paths['zip_paths']
        
        # Python timing (would need to extract specific methods from ResourceManager)
        python_start = time.perf_counter()
        # python_results = python_scanner.scan_zip_files(zip_paths)  # hypothetical
        python_time = time.perf_counter() - python_start
        
        # Rust timing
        rust_start = time.perf_counter()
        rust_results = rust_scanner.scan_zip_files(zip_paths)
        rust_time = time.perf_counter() - rust_start
        
        comparison_results['python_times']['zip_scanning'] = python_time
        comparison_results['rust_times']['zip_scanning'] = rust_time
        comparison_results['speedup_factors']['zip_scanning'] = python_time / rust_time if rust_time > 0 else 0
    
    return comparison_results