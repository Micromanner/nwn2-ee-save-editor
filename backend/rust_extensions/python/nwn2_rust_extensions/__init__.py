"""
NWN2 Rust Extensions Python Module

This module provides Python wrappers for the Rust performance extensions,
including path discovery and resource scanning functionality.
"""

from .nwn2_rust_wrapper import (
    ResourceLocation,
    PathTiming,
    DiscoveryResult,
    ScanResults,
    RustResourceScanner,
    discover_nwn2_paths,
    profile_path_discovery,
    create_resource_scanner,
    RUST_AVAILABLE
)

# Import the compiled Rust module directly for new classes
try:
    from . import nwn2_rust_extensions as _rust_module
    # Expose new ZIP reader classes
    ZipContentReader = _rust_module.ZipContentReader
    ZipReadRequest = _rust_module.ZipReadRequest
    ZipReadResult = _rust_module.ZipReadResult
    # Expose cache classes
    CacheBuilder = _rust_module.CacheBuilder
    CacheManager = _rust_module.CacheManager
    # Expose PrerequisiteGraph
    PrerequisiteGraph = _rust_module.PrerequisiteGraph
except (ImportError, AttributeError):
    # Fallback if not available
    ZipContentReader = None
    ZipReadRequest = None
    ZipReadResult = None
    CacheBuilder = None
    CacheManager = None
    PrerequisiteGraph = None

__all__ = [
    'ResourceLocation',
    'PathTiming', 
    'DiscoveryResult',
    'ScanResults',
    'RustResourceScanner',
    'discover_nwn2_paths',
    'profile_path_discovery',
    'create_resource_scanner',
    'RUST_AVAILABLE',
    'ZipContentReader',
    'ZipReadRequest',
    'ZipReadResult',
    'CacheBuilder',
    'CacheManager',
    'PrerequisiteGraph'
]