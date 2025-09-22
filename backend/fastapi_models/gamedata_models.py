"""
Pydantic models for gamedata operations
Handles NWN2 game data, paths, configuration, and 2DA/TLK files
"""

from typing import Dict, Any, Optional, List
from datetime import datetime
from pydantic import BaseModel, Field
from pathlib import Path


class NWN2PathInfo(BaseModel):
    """Individual NWN2 path information"""
    path: str = Field(..., description="Full path")
    exists: bool = Field(..., description="Whether path exists")
    readable: bool = Field(True, description="Whether path is readable")
    writable: bool = Field(False, description="Whether path is writable")
    size_bytes: Optional[int] = Field(None, description="Size if it's a file")


class PathInfo(BaseModel):
    """Individual path information"""
    path: Optional[str] = Field(None, description="Full path")
    exists: bool = Field(False, description="Whether path exists")
    auto_detected: bool = Field(False, description="Whether path was auto-detected")


class CustomFolderInfo(BaseModel):
    """Custom folder information"""
    path: str = Field(..., description="Full path")
    exists: bool = Field(False, description="Whether path exists")


class PathConfig(BaseModel):
    """Path configuration"""
    game_folder: PathInfo
    documents_folder: PathInfo
    steam_workshop_folder: PathInfo
    custom_override_folders: List[CustomFolderInfo] = Field(default_factory=list)
    custom_module_folders: List[CustomFolderInfo] = Field(default_factory=list)
    custom_hak_folders: List[CustomFolderInfo] = Field(default_factory=list)


class NWN2PathsResponse(BaseModel):
    """NWN2 installation paths response - matches frontend PathsResponse"""
    paths: PathConfig


class GameDataTableInfo(BaseModel):
    """Information about a 2DA table"""
    table_name: str = Field(..., description="Name of the 2DA table")
    file_path: str = Field(..., description="Path to the 2DA file")
    row_count: int = Field(..., description="Number of rows in table")
    column_count: int = Field(..., description="Number of columns in table")
    columns: List[str] = Field(..., description="Column names")
    
    # Metadata
    source: str = Field(..., description="Source of table (base, expansion, override, hak)")
    priority: int = Field(0, description="Loading priority")
    last_modified: Optional[datetime] = Field(None, description="Last modification time")
    size_bytes: int = Field(0, description="File size")


class GameDataTablesResponse(BaseModel):
    """List of available game data tables"""
    tables: List[GameDataTableInfo]
    total_count: int = Field(..., description="Total number of tables")
    
    # Summary by source
    base_tables: int = Field(0, description="Tables from base game")
    expansion_tables: int = Field(0, description="Tables from expansions")
    override_tables: int = Field(0, description="Tables from override")
    hak_tables: int = Field(0, description="Tables from HAK files")


class GameDataRowRequest(BaseModel):
    """Request for specific rows from a 2DA table"""
    table_name: str = Field(..., description="2DA table name")
    row_ids: Optional[List[int]] = Field(None, description="Specific row IDs to fetch")
    columns: Optional[List[str]] = Field(None, description="Specific columns to include")
    filter_expression: Optional[str] = Field(None, description="Filter expression")
    
    # Pagination
    offset: int = Field(0, ge=0, description="Row offset")
    limit: int = Field(100, ge=1, le=1000, description="Maximum rows to return")


class GameDataRowResponse(BaseModel):
    """Response with 2DA table row data"""
    table_name: str
    rows: List[Dict[str, Any]] = Field(..., description="Row data")
    total_rows: int = Field(..., description="Total rows in table")
    columns: List[str] = Field(..., description="Column names included")
    
    # Pagination info
    offset: int = Field(0, description="Starting row offset")
    limit: int = Field(100, description="Maximum rows returned")
    has_more: bool = Field(False, description="Whether more rows are available")


class TLKInfo(BaseModel):
    """Talk table (TLK) file information"""
    tlk_name: str = Field(..., description="TLK file name")
    file_path: str = Field(..., description="Path to TLK file")
    entry_count: int = Field(..., description="Number of string entries")
    language: Optional[str] = Field(None, description="Language code")
    
    # Metadata
    is_custom: bool = Field(False, description="Custom/mod TLK file")
    priority: int = Field(0, description="Loading priority")
    size_bytes: int = Field(0, description="File size")


class TLKStringRequest(BaseModel):
    """Request for TLK string lookup"""
    string_refs: List[int] = Field(..., description="String reference IDs")
    tlk_file: Optional[str] = Field(None, description="Specific TLK file to search")
    include_metadata: bool = Field(False, description="Include string metadata")


class TLKStringEntry(BaseModel):
    """Individual TLK string entry"""
    string_ref: int = Field(..., description="String reference ID")
    text: str = Field(..., description="String text")
    sound_resref: Optional[str] = Field(None, description="Associated sound file")
    
    # Metadata (if requested)
    tlk_source: Optional[str] = Field(None, description="Source TLK file")
    is_custom: Optional[bool] = Field(None, description="From custom TLK")


class TLKStringResponse(BaseModel):
    """Response with TLK string data"""
    strings: List[TLKStringEntry]
    found_count: int = Field(..., description="Number of strings found")
    missing_refs: List[int] = Field(default_factory=list, description="String refs not found")


class HAKInfo(BaseModel):
    """HAK file information"""
    hak_name: str = Field(..., description="HAK file name")
    file_path: str = Field(..., description="Path to HAK file")
    resource_count: int = Field(..., description="Number of resources in HAK")
    
    # Resources by type
    resource_types: Dict[str, int] = Field(default_factory=dict, description="Count by resource type")
    
    # Metadata
    is_loaded: bool = Field(False, description="Whether HAK is currently loaded")
    priority: int = Field(0, description="Loading priority")
    size_bytes: int = Field(0, description="File size")
    created: Optional[datetime] = Field(None, description="Creation time")


class HAKListResponse(BaseModel):
    """List of available HAK files"""
    haks: List[HAKInfo]
    total_count: int = Field(..., description="Total number of HAK files")
    loaded_count: int = Field(0, description="Number of loaded HAK files")


class ModuleInfo(BaseModel):
    """Module (.mod) file information"""
    module_name: str = Field(..., description="Module file name")
    display_name: str = Field(..., description="Module display name")
    description: Optional[str] = Field(None, description="Module description")
    
    # Module properties
    min_level: int = Field(1, description="Minimum character level")
    max_level: int = Field(40, description="Maximum character level")
    required_haks: List[str] = Field(default_factory=list, description="Required HAK files")
    
    # File info
    file_path: str = Field(..., description="Path to module file")
    size_bytes: int = Field(0, description="File size")
    last_modified: Optional[datetime] = Field(None, description="Last modification time")
    
    # Content flags
    is_expansion: bool = Field(False, description="Official expansion module")
    is_premium: bool = Field(False, description="Premium module")
    is_custom: bool = Field(False, description="Custom/user module")


class ModuleListResponse(BaseModel):
    """List of available modules"""
    modules: List[ModuleInfo]
    total_count: int = Field(..., description="Total number of modules")
    
    # Summary
    expansion_count: int = Field(0, description="Official expansion modules")
    premium_count: int = Field(0, description="Premium modules")
    custom_count: int = Field(0, description="Custom modules")


class GameDataCacheInfo(BaseModel):
    """Game data cache information"""
    cache_type: str = Field(..., description="Type of cache (2da, tlk, icons, etc.)")
    status: str = Field(..., description="Cache status (ready, loading, error)")
    
    # Cache statistics
    entry_count: int = Field(0, description="Number of cached entries")
    memory_usage_bytes: int = Field(0, description="Memory usage")
    hit_rate: float = Field(0.0, description="Cache hit rate percentage")
    
    # Timing
    last_updated: Optional[datetime] = Field(None, description="Last cache update")
    load_time_ms: Optional[int] = Field(None, description="Time to load cache")


class GameDataCacheResponse(BaseModel):
    """Complete game data cache status"""
    caches: List[GameDataCacheInfo]
    total_memory_usage: int = Field(0, description="Total memory usage across all caches")
    overall_status: str = Field(..., description="Overall cache system status")
    
    # Actions
    can_refresh: bool = Field(True, description="Can refresh caches")
    can_clear: bool = Field(True, description="Can clear caches")


class GameDataRefreshRequest(BaseModel):
    """Request to refresh game data"""
    cache_types: List[str] = Field(default_factory=list, description="Specific caches to refresh")
    force_reload: bool = Field(False, description="Force reload even if current")
    clear_first: bool = Field(False, description="Clear caches before reloading")


class GameDataRefreshResponse(BaseModel):
    """Response after refreshing game data"""
    success: bool
    message: str
    
    # Refresh results
    refreshed_caches: List[str] = Field(..., description="Caches that were refreshed")
    refresh_time_ms: int = Field(0, description="Total refresh time")
    errors: List[str] = Field(default_factory=list, description="Any errors encountered")
    
    # New cache status
    cache_status: GameDataCacheResponse


class GameDataConfigResponse(BaseModel):
    """NWN2 gamedata configuration response"""
    nwn2_install_path: str = Field(..., description="NWN2 installation directory")
    nwn2_user_path: str = Field(..., description="NWN2 user directory")
    saves_path: str = Field(..., description="Savegames directory")
    data_path: str = Field(..., description="Game data directory")
    dialog_tlk_path: str = Field(..., description="Main dialog.tlk file path")


# Simple response models that relay data from existing business logic
class GameDataTableResponse(BaseModel):
    """Generic response for any table data - delegates to GameDataViewSet"""
    table_name: str
    data: List[Dict[str, Any]]
    count: int


class GameDataTablesResponse(BaseModel):
    """List of available tables - delegates to GameDataViewSet"""
    tables: List[Dict[str, Any]]
    total_tables: int


class GameDataSchemaResponse(BaseModel):
    """Table schema information - delegates to GameDataViewSet"""
    table_name: str
    row_count: int
    columns: List[Dict[str, Any]]


class GameDataModulesResponse(BaseModel):
    """Modules list - delegates to GameDataViewSet"""
    modules: List[Dict[str, Any]]
    count: int