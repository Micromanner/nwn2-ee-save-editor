"""
Shared Pydantic models used across multiple managers
Base models, common responses, and cross-cutting concerns
"""

from typing import Dict, Any, Optional, List, Union, Literal
from datetime import datetime
from pydantic import BaseModel, Field, ConfigDict


# ============================================================
# Base Response Models
# ============================================================

class BaseResponse(BaseModel):
    """Base response model for all API responses"""
    success: bool = True
    message: Optional[str] = None
    timestamp: datetime = Field(default_factory=datetime.now)


class ErrorResponse(BaseModel):
    """Standard error response"""
    error: str
    error_code: Optional[str] = None
    details: Optional[Dict[str, Any]] = None
    retry_after: Optional[int] = None
    timestamp: datetime = Field(default_factory=datetime.now)


class ValidationError(BaseModel):
    """Validation error detail"""
    field: str
    message: str
    value: Optional[Any] = None
    suggestion: Optional[str] = None


class ValidationResponse(BaseModel):
    """Validation response with errors and warnings"""
    valid: bool
    errors: List[ValidationError] = Field(default_factory=list)
    warnings: List[str] = Field(default_factory=list)
    suggestions: List[str] = Field(default_factory=list)


# ============================================================
# System/Health Models
# ============================================================

class HealthResponse(BaseModel):
    """Service health check response"""
    status: Literal['healthy', 'degraded', 'unhealthy']
    service: str
    uptime: Optional[int] = None
    checks: Dict[str, bool] = Field(default_factory=dict)


class ReadyResponse(BaseModel):
    """Service readiness check response"""
    ready: bool
    message: str
    initialization_progress: int = Field(100, ge=0, le=100)
    components: Dict[str, bool] = Field(default_factory=dict)


class SystemInfo(BaseModel):
    """System information"""
    version: str
    backend: str = "FastAPI"
    python_version: str
    nwn2_path: Optional[str] = None
    working_directory: str
    debug_mode: bool = False


class CacheStatus(BaseModel):
    """Cache status response"""
    status: str = Field(..., description="Cache status (ready, loading, error)")
    cache_size: int = Field(0, description="Number of cached items")
    last_updated: Optional[datetime] = Field(None, description="Last cache update time")
    cache_type: str = Field("general", description="Type of cache")
    memory_usage: Optional[int] = Field(None, description="Memory usage in bytes")
    hit_rate: Optional[float] = Field(None, description="Cache hit rate percentage")


class ConfigResponse(BaseModel):
    """Configuration response"""
    nwn2_install_dir: Optional[str] = Field(None, description="NWN2 installation directory")
    cache_enabled: bool = Field(True, description="Whether caching is enabled")
    debug_mode: bool = Field(False, description="Whether debug mode is active")
    data_paths: Dict[str, str] = Field(default_factory=dict, description="Important data paths")
    feature_flags: Dict[str, bool] = Field(default_factory=dict, description="Feature toggles")


class CacheRebuildResponse(BaseModel):
    """Cache rebuild response"""
    success: bool
    message: str
    rebuild_time: Optional[float] = Field(None, description="Rebuild time in seconds")
    items_cached: int = Field(0, description="Number of items cached")
    errors: List[str] = Field(default_factory=list)


class ConfigUpdateRequest(BaseModel):
    """Configuration update request"""
    config_updates: Dict[str, Any] = Field(..., description="Configuration updates")
    validate_changes: bool = Field(True, description="Validate configuration changes")


class ConfigUpdateResponse(BaseModel):
    """Configuration update response"""
    success: bool
    message: str
    updated_fields: List[str] = Field(default_factory=list)
    validation_errors: List[str] = Field(default_factory=list)
    config: ConfigResponse


class NWN2PathResponse(BaseModel):
    """NWN2 path configuration response"""
    nwn2_install_dir: Optional[str] = None
    nwn2_data_dir: Optional[str] = None
    nwn2_docs_dir: Optional[str] = None
    steam_workshop_dir: Optional[str] = None
    paths_configured: bool = False
    auto_detected: bool = False


class AutoDiscoverResponse(BaseModel):
    """Auto-discovery response"""
    success: bool
    message: str
    paths_found: NWN2PathResponse
    discovery_method: Optional[str] = None


class BackgroundLoadingTriggerResponse(BaseModel):
    """Background loading trigger response"""
    success: bool
    message: str
    loading_started: bool = False
    estimated_time: Optional[float] = None


class BackgroundLoadingStatusResponse(BaseModel):
    """Background loading status response"""
    status: Literal['idle', 'loading', 'completed', 'error']
    progress: int = Field(0, ge=0, le=100)
    current_task: Optional[str] = None
    estimated_remaining: Optional[float] = None
    errors: List[str] = Field(default_factory=list)


class InitializationStatusResponse(BaseModel):
    """System initialization status response"""
    status: Literal['initializing', 'ready', 'error']
    progress: int = Field(0, ge=0, le=100)
    components: Dict[str, bool] = Field(default_factory=dict)
    errors: List[str] = Field(default_factory=list)
    startup_time: Optional[float] = None


# ============================================================
# Session Models
# ============================================================

class SessionInfo(BaseModel):
    """Session information"""
    session_id: str
    character_id: Union[int, str]
    character_name: Optional[str] = None
    character_file: str
    started_at: datetime
    last_activity: datetime
    has_unsaved_changes: bool = False
    changes_count: int = 0


class SessionStatus(BaseModel):
    """Session status response"""
    active: bool
    session: Optional[SessionInfo] = None
    total_sessions: int = 0


class ActiveSessionsList(BaseModel):
    """List of active character sessions"""
    sessions: List[SessionInfo]
    count: int


# ============================================================
# Change Tracking Models
# ============================================================

class ChangeRecord(BaseModel):
    """Individual change record"""
    field: str
    old_value: Any
    new_value: Any
    timestamp: datetime = Field(default_factory=datetime.now)
    manager: Optional[str] = None
    validated: bool = True


class CascadingEffect(BaseModel):
    """Cascading effect from a change"""
    type: str
    description: str
    affected_field: str
    old_value: Any
    new_value: Any
    manager: str


class ChangeResult(BaseModel):
    """Result of a change operation"""
    success: bool = True
    changes: List[ChangeRecord] = Field(default_factory=list)
    cascading_effects: List[CascadingEffect] = Field(default_factory=list)
    validation_errors: List[str] = Field(default_factory=list)
    has_unsaved_changes: bool = True


# ============================================================
# Pagination Models
# ============================================================

class PaginationParams(BaseModel):
    """Pagination request parameters"""
    page: int = Field(1, ge=1, description="Page number")
    limit: int = Field(50, ge=1, le=500, description="Items per page")
    sort_by: Optional[str] = None
    sort_order: Literal['asc', 'desc'] = 'asc'


class PaginationInfo(BaseModel):
    """Pagination metadata"""
    page: int
    limit: int
    total: int
    total_pages: int
    has_next: bool
    has_prev: bool
    start_idx: int
    end_idx: int


class PaginatedResponse(BaseModel):
    """Base paginated response"""
    items: List[Any]
    pagination: PaginationInfo


# ============================================================
# File/Resource Models
# ============================================================

class FileInfo(BaseModel):
    """File information"""
    path: str
    name: str
    size: int
    modified: datetime
    file_type: str
    readable: bool = True
    writable: bool = True


class ResourceInfo(BaseModel):
    """Game resource information"""
    resource_type: str
    resource_name: str
    source: str  # "base", "expansion", "override", "hak", etc.
    priority: int
    file_info: Optional[FileInfo] = None


# ============================================================
# Custom Content Models
# ============================================================

class CustomContentItem(BaseModel):
    """Individual custom content item"""
    type: str  # "feat", "spell", "item", "class", etc.
    id: int
    name: Optional[str] = None
    source: Optional[str] = None  # HAK file or override
    protected: bool = False
    reason: Optional[str] = None


class CustomContentSummary(BaseModel):
    """Summary of custom content"""
    total: int
    by_type: Dict[str, int] = Field(default_factory=dict)
    by_source: Dict[str, int] = Field(default_factory=dict)
    protected_count: int = 0
    items: List[CustomContentItem] = Field(default_factory=list)


# ============================================================
# GFF/Data Models
# ============================================================

class GFFFieldInfo(BaseModel):
    """GFF field structure information"""
    path: str
    label: str
    type: str
    type_id: int
    value: Optional[Any] = None
    
    # For structs and lists
    fields: Optional[List['GFFFieldInfo']] = None
    count: Optional[int] = None
    
    model_config = ConfigDict(arbitrary_types_allowed=True)


class RawDataRequest(BaseModel):
    """Request for raw GFF data"""
    path: Optional[str] = Field(None, description="Specific path in GFF structure")
    include_metadata: bool = Field(False, description="Include field metadata")
    max_depth: int = Field(10, ge=1, le=20, description="Maximum nesting depth")


class RawDataResponse(BaseModel):
    """Raw GFF data response"""
    path: str = Field(..., description="GFF path that was accessed")
    value: Any = Field(..., description="Value at the specified path")
    field_type: str = Field(..., description="GFF field type")
    raw_data: Dict[str, Any] = Field(..., description="Raw data structure")


class FieldStructureResponse(BaseModel):
    """GFF field structure analysis response"""
    path: str = Field(..., description="Path that was analyzed")
    structure: Dict[str, Any] = Field(..., description="Analyzed structure")
    total_fields: int = Field(..., description="Total number of fields found")
    max_depth_analyzed: int = Field(..., description="Maximum depth analyzed")


class RawFieldUpdateRequest(BaseModel):
    """Request to update a raw GFF field"""
    path: str = Field(..., description="GFF path to the field")
    value: Any = Field(..., description="New value for the field")


class RawFieldUpdateResponse(BaseModel):
    """Response after updating a raw field"""
    success: bool = True
    path: str = Field(..., description="Path that was updated")
    old_value: Any = Field(..., description="Previous value")
    new_value: Any = Field(..., description="New value")
    message: Optional[str] = Field(None, description="Additional information")


# ============================================================
# Batch Operation Models
# ============================================================

class BatchOperation(BaseModel):
    """Single operation in a batch"""
    operation_id: str
    operation_type: str
    target: str
    params: Dict[str, Any]
    should_validate: bool = True


class BatchOperationResult(BaseModel):
    """Result of a single batch operation"""
    operation_id: str
    success: bool
    result: Optional[Any] = None
    error: Optional[str] = None
    duration_ms: Optional[int] = None


class BatchRequest(BaseModel):
    """Batch operation request"""
    operations: List[BatchOperation]
    stop_on_error: bool = Field(False, description="Stop if any operation fails")
    use_transaction: bool = Field(True, description="Wrap in transaction")


class BatchResponse(BaseModel):
    """Batch operation response"""
    total: int
    successful: int
    failed: int
    results: List[BatchOperationResult]
    transaction_id: Optional[str] = None
    total_duration_ms: Optional[int] = None


# ============================================================
# Export/Import Models
# ============================================================

class ExportOptions(BaseModel):
    """Options for data export"""
    format: Literal['json', 'gff', 'xml'] = 'json'
    include_metadata: bool = True
    include_custom_content: bool = True
    compress: bool = False
    
    # Selective export
    sections: List[str] = Field(default_factory=list, description="Specific sections to export")
    exclude_sections: List[str] = Field(default_factory=list, description="Sections to exclude")


class ImportOptions(BaseModel):
    """Options for data import"""
    should_validate: bool = True
    merge: bool = Field(False, description="Merge with existing data")
    overwrite: bool = Field(True, description="Overwrite existing data")
    create_backup: bool = True
    
    # Selective import
    sections: List[str] = Field(default_factory=list, description="Specific sections to import")
    ignore_sections: List[str] = Field(default_factory=list, description="Sections to ignore")


# ============================================================
# Alignment Models
# ============================================================

class AlignmentResponse(BaseModel):
    """Character alignment response"""
    lawChaos: int = Field(..., ge=0, le=100, description="Law-Chaos axis (0=Chaotic, 50=Neutral, 100=Lawful)")
    goodEvil: int = Field(..., ge=0, le=100, description="Good-Evil axis (0=Evil, 50=Neutral, 100=Good)")
    alignment_string: str = Field(..., description="Human readable alignment (e.g., 'Lawful Good')")
    has_unsaved_changes: Optional[bool] = None


class AlignmentUpdateRequest(BaseModel):
    """Request to update character alignment"""
    lawChaos: Optional[int] = Field(None, ge=0, le=100, description="Law-Chaos axis value")
    goodEvil: Optional[int] = Field(None, ge=0, le=100, description="Good-Evil axis value")


class AlignmentShiftRequest(BaseModel):
    """Request to shift alignment by relative amounts"""
    lawChaosShift: int = Field(0, description="Shift toward Law(+) or Chaos(-)")
    goodEvilShift: int = Field(0, description="Shift toward Good(+) or Evil(-)")


class AlignmentShiftResponse(AlignmentResponse):
    """Response after shifting alignment"""
    shifted: Dict[str, int] = Field(..., description="Applied shift amounts")


# Rebuild models that have forward references
GFFFieldInfo.model_rebuild()