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
    compare_performance,
    RUST_AVAILABLE
)

__all__ = [
    'ResourceLocation',
    'PathTiming', 
    'DiscoveryResult',
    'ScanResults',
    'RustResourceScanner',
    'discover_nwn2_paths',
    'profile_path_discovery',
    'create_resource_scanner',
    'compare_performance',
    'RUST_AVAILABLE'
]