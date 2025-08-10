"""
Adapter classes to make Rust implementations compatible with Python interface
"""
from typing import Dict, List, Optional
from pathlib import Path
import logging

logger = logging.getLogger(__name__)


class RustScannerAdapter:
    """Adapter to make RustResourceScanner compatible with Python scanner interfaces"""
    
    def __init__(self, rust_scanner):
        self.rust_scanner = rust_scanner
        
    # Methods for PythonZipIndexer compatibility
    def index_zips_parallel(self, zip_paths: List[Path]) -> Dict:
        """Adapter for index_zips_parallel -> scan_zip_files"""
        return self.rust_scanner.scan_zip_files(zip_paths)
    
    # Methods for PythonDirectoryWalker compatibility  
    def scan_directories_parallel(self, directories: List[Path], recursive: bool = True) -> Dict:
        """Adapter for scan_directories_parallel -> multiple index_directory calls"""
        results = {}
        for directory in directories:
            dir_results = self.rust_scanner.index_directory(directory, recursive=recursive)
            results.update(dir_results)
        return results
    
    def index_directory(self, directory_path: Path, recursive: bool = True) -> Dict:
        """Direct pass-through to Rust index_directory"""
        return self.rust_scanner.index_directory(directory_path, recursive=recursive)
    
    # Methods for PythonResourceScanner compatibility
    def scan_zip_files(self, zip_paths: List) -> Dict:
        """Direct pass-through to Rust scan_zip_files"""
        return self.rust_scanner.scan_zip_files(zip_paths)
    
    def scan_workshop_directories(self, workshop_dirs: List) -> Dict:
        """Direct pass-through to Rust scan_workshop_directories"""  
        return self.rust_scanner.scan_workshop_directories(workshop_dirs)
    
    def comprehensive_scan(self, base_path: Path, **kwargs) -> Dict:
        """Direct pass-through to Rust comprehensive_scan"""
        return self.rust_scanner.comprehensive_scan(base_path, **kwargs)
    
    def get_performance_stats(self) -> Dict:
        """Direct pass-through to Rust get_performance_stats"""
        return self.rust_scanner.get_performance_stats()