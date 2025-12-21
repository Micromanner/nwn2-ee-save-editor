"""
Rust adapter for resource scanning.

GFF parsing is now handled directly by nwn2_rust:
- nwn2_rust.GffParser(path) or nwn2_rust.GffParser.from_bytes(data)
- nwn2_rust.GffWriter(file_type, file_version)
"""
from typing import Dict, List
from pathlib import Path


class RustScannerAdapter:
    """Adapter to make RustResourceScanner compatible with Python scanner interfaces"""

    def __init__(self, rust_scanner):
        self.rust_scanner = rust_scanner

    def index_zips_parallel(self, zip_paths: List[Path]) -> Dict:
        return self.rust_scanner.scan_zip_files(zip_paths)

    def index_zip(self, zip_path: Path) -> Dict:
        return self.rust_scanner.scan_zip_files([zip_path])

    def scan_directories_parallel(self, directories: List[Path], recursive: bool = True) -> Dict:
        results = {}
        for directory in directories:
            dir_results = self.rust_scanner.index_directory(directory, recursive=recursive)
            results.update(dir_results)
        return results

    def index_directory(self, directory_path: Path, recursive: bool = True) -> Dict:
        return self.rust_scanner.index_directory(directory_path, recursive=recursive)

    def scan_zip_files(self, zip_paths: List) -> Dict:
        return self.rust_scanner.scan_zip_files(zip_paths)

    def scan_workshop_directories(self, workshop_dirs: List) -> Dict:
        return self.rust_scanner.scan_workshop_directories(workshop_dirs)

    def comprehensive_scan(self, base_path: Path, **kwargs) -> Dict:
        return self.rust_scanner.comprehensive_scan(base_path, **kwargs)

    def get_performance_stats(self) -> Dict:
        return self.rust_scanner.get_performance_stats()
