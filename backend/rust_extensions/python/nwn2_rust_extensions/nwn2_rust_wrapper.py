"""
Python wrapper for NWN2 Rust Extensions

Provides a clean Python interface to the Rust performance optimizations.
"""

import sys
import logging
from typing import Dict, List, Optional, Any, Union
from pathlib import Path

logger = logging.getLogger(__name__)

# Import the compiled Rust module directly (not the Python package)
from . import nwn2_rust_extensions

# We always require Rust extensions now
RUST_AVAILABLE = True


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
        return f"PathTiming({self.operation}: {self.duration_ms}ms, found {self.paths_found}/{self.paths_checked})"
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'PathTiming':
        """Create PathTiming from dictionary (from Rust)"""
        return cls(
            operation=data['operation'],
            duration_ms=data['duration_ms'],
            paths_checked=data['paths_checked'],
            paths_found=data['paths_found']
        )


class DiscoveryResult:
    """Python representation of DiscoveryResult from Rust"""
    
    def __init__(self, nwn2_paths: List[str], is_steam: bool, is_gog: bool,
                 steam_workshop_path: Optional[str], timings: List[PathTiming]):
        self.nwn2_paths = nwn2_paths
        self.is_steam = is_steam
        self.is_gog = is_gog
        self.steam_workshop_path = steam_workshop_path
        self.timings = timings
    
    def __repr__(self):
        platform = "Steam" if self.is_steam else "GOG" if self.is_gog else "Unknown"
        return f"DiscoveryResult({len(self.nwn2_paths)} paths, {platform})"
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'DiscoveryResult':
        """Create DiscoveryResult from dictionary (from Rust)"""
        timings = [PathTiming.from_dict(t) for t in data.get('timings', [])]
        return cls(
            nwn2_paths=data['nwn2_paths'],
            is_steam=data['is_steam'],
            is_gog=data['is_gog'],
            steam_workshop_path=data.get('steam_workshop_path'),
            timings=timings
        )


class ScanResults:
    """Results from comprehensive resource scanning"""
    
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
    
    def __repr__(self):
        return (f"ScanResults({self.resources_found} resources, "
                f"{self.scan_time_ms}ms)")
    
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
        """Initialize the Rust scanner (required)"""
        self._scanner = nwn2_rust_extensions.ResourceScanner()
    
    def scan_zip_files(self, zip_paths: List[Union[str, Path]]) -> Dict[str, ResourceLocation]:
        """
        Scan ZIP files for 2DA resources
        
        Args:
            zip_paths: List of ZIP file paths to scan
            
        Returns:
            Dictionary mapping resource names to ResourceLocation objects
        """
        # Convert paths to strings
        str_paths = [str(path) for path in zip_paths]
        
        rust_results = self._scanner.scan_zip_files(str_paths)
        
        # The Rust module returns native ResourceLocation objects directly
        logger.debug(f"Rust ZIP scan found {len(rust_results)} resources")
        return rust_results
    
    def scan_workshop_directories(self, workshop_dirs: List[Union[str, Path]]) -> Dict[str, ResourceLocation]:
        """
        Scan workshop directories for override files
        
        Args:
            workshop_dirs: List of workshop directory paths to scan
            
        Returns:
            Dictionary mapping resource names to ResourceLocation objects
        """
        # Convert paths to strings
        str_paths = [str(path) for path in workshop_dirs]
        
        rust_results = self._scanner.scan_workshop_directories(str_paths)
        
        # The Rust module returns native ResourceLocation objects directly
        logger.debug(f"Rust workshop scan found {len(rust_results)} resources")
        return rust_results
    
    def index_directory(self, directory_path: Union[str, Path], 
                       recursive: bool = True) -> Dict[str, ResourceLocation]:
        """
        Index a directory for resources
        
        Args:
            directory_path: Directory path to index
            recursive: Whether to scan subdirectories
            
        Returns:
            Dictionary mapping resource names to ResourceLocation objects
        """
        rust_results = self._scanner.index_directory(str(directory_path), recursive)
        
        # The Rust module returns native ResourceLocation objects directly
        logger.debug(f"Rust directory indexing found {len(rust_results)} resources")
        return rust_results
    
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
            ScanResults object with detailed information
        """
        # Convert paths to strings
        str_data_dir = str(nwn2_data_dir)
        str_enhanced_dir = str(enhanced_data_dir) if enhanced_data_dir else None
        str_workshop_dirs = [str(d) for d in workshop_dirs] if workshop_dirs else []
        str_custom_dirs = [str(d) for d in custom_override_dirs] if custom_override_dirs else []
        
        rust_results = self._scanner.comprehensive_scan(
            str_data_dir,
            str_enhanced_dir,
            str_workshop_dirs,
            str_custom_dirs
        )
        
        # Convert to Python ScanResults
        return ScanResults.from_dict(rust_results)
    
    def get_performance_stats(self) -> Dict[str, Any]:
        """
        Get performance statistics from scanner
        
        Returns:
            Dictionary with performance metrics
        """
        return self._scanner.get_performance_stats()


def discover_nwn2_paths(search_paths: Optional[List[Union[str, Path]]] = None) -> DiscoveryResult:
    """
    Discover NWN2 installation paths using Rust implementation
    
    Args:
        search_paths: Optional list of paths to search
        
    Returns:
        DiscoveryResult with found paths and metadata
    """
    str_paths = [str(p) for p in search_paths] if search_paths else None
    rust_result = nwn2_rust_extensions.discover_nwn2_paths_rust(str_paths)
    return DiscoveryResult.from_dict(rust_result)


def profile_path_discovery(search_paths: Optional[List[Union[str, Path]]] = None) -> DiscoveryResult:
    """
    Profile NWN2 path discovery with detailed timing
    
    Args:
        search_paths: Optional list of paths to search
        
    Returns:
        DiscoveryResult with detailed timing information
    """
    str_paths = [str(p) for p in search_paths] if search_paths else None
    rust_result = nwn2_rust_extensions.profile_path_discovery_rust(str_paths)
    return DiscoveryResult.from_dict(rust_result)


def create_resource_scanner() -> RustResourceScanner:
    """Create a new RustResourceScanner instance"""
    return RustResourceScanner()