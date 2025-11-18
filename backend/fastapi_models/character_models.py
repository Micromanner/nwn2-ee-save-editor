"""
Pydantic models for CharacterManager
Top-level character management, transactions, and orchestration
"""

from typing import Dict, Any, Optional, List, Union
from datetime import datetime
from pydantic import BaseModel, Field, ConfigDict


class CharacterInfo(BaseModel):
    """Basic character information"""
    id: Union[int, str] = Field(..., description="Character ID (file path or numeric)")
    file_path: str
    file_name: str
    file_type: str = Field(..., description="bic, ros, or ifo")
    
    # Basic info
    first_name: str = ""
    last_name: str = ""
    full_name: str = ""
    
    # Character details
    level: int = 1
    experience: int = 0
    race_name: str = ""
    alignment: Dict[str, int] = Field(default_factory=dict)
    alignment_string: str = ""
    
    # File metadata
    is_savegame: bool = False
    is_companion: bool = False
    created_at: Optional[datetime] = None
    modified_at: Optional[datetime] = None
    file_size: Optional[int] = None


class CharacterSummary(BaseModel):
    """High-level character summary from get_character_summary()"""
    id: Optional[Union[int, str]] = None  # Character ID for frontend compatibility
    name: str
    level: int
    race: str
    alignment: Dict[str, int]
    classes: Dict[str, Any]  # Changed to match actual return from get_class_summary()
    abilities: Optional[Dict[str, int]] = None  # Made optional since not always populated
    gold: int = 0

    # Additional info from content manager
    campaign_name: Optional[str] = None
    module_name: Optional[str] = None
    area_name: Optional[str] = None
    quest_details: Optional[Dict[str, Any]] = None

    # Custom content
    custom_content_count: int = 0


class ManagerStatus(BaseModel):
    """Status of an individual manager"""
    name: str
    loaded: bool
    initialized: bool
    has_errors: bool = False
    error_message: Optional[str] = None
    data_count: Optional[int] = None
    last_accessed: Optional[datetime] = None


class ManagersStatus(BaseModel):
    """Status of all registered managers from get_manager_status()"""
    total_managers: int
    loaded_managers: int
    managers: Dict[str, ManagerStatus]
    initialization_time: Optional[float] = None


class Transaction(BaseModel):
    """Character modification transaction"""
    id: str
    timestamp: float
    changes: List[Dict[str, Any]]
    committed: bool = False
    rolled_back: bool = False
    duration: Optional[float] = None


class TransactionHistory(BaseModel):
    """Transaction history from get_transaction_history()"""
    transactions: List[Transaction]
    active_transaction: Optional[Transaction] = None
    total_transactions: int
    committed_count: int
    rolled_back_count: int


class ValidationResult(BaseModel):
    """Character validation result"""
    valid: bool
    errors: List[str] = Field(default_factory=list)
    warnings: List[str] = Field(default_factory=list)
    
    # Validation by manager
    manager_errors: Dict[str, List[str]] = Field(default_factory=dict)
    corruption_risks: List[str] = Field(default_factory=list)


class BatchUpdateRequest(BaseModel):
    """Request for batch character updates"""
    updates: List[Dict[str, Any]] = Field(..., description="List of update operations")
    should_validate: bool = Field(True, description="Validate before applying")
    use_transaction: bool = Field(True, description="Wrap in transaction")


class BatchUpdateResult(BaseModel):
    """Result of batch update operation"""
    total: int
    successful: int
    failed: int
    results: List[Dict[str, Any]]
    transaction_id: Optional[str] = None
    validation_errors: List[str] = Field(default_factory=list)
    has_unsaved_changes: bool = True


class CharacterState(BaseModel):
    """Complete character state aggregation"""
    # Core info
    info: CharacterInfo
    summary: CharacterSummary
    
    # Manager states (populated on demand) - match actual manager return types
    abilities: Optional[Dict[str, Any]] = None
    combat: Optional[Dict[str, Any]] = None
    skills: Optional[List[Dict[str, Any]]] = None  # Skills return a list
    feats: Optional[List[Dict[str, Any]]] = None   # Feats return a list
    spells: Optional[Dict[str, Any]] = None
    inventory: Optional[Dict[str, Any]] = None
    saves: Optional[Dict[str, Any]] = None
    classes: Optional[Dict[str, Any]] = None
    race: Optional[Dict[str, Any]] = None
    content: Optional[Dict[str, Any]] = None
    
    # Metadata
    custom_content: Dict[str, Any] = Field(default_factory=dict)
    manager_status: Optional[Dict[str, Dict[str, Any]]] = None
    has_unsaved_changes: bool = False


class CharacterExport(BaseModel):
    """Complete character export data"""
    version: str = "1.0"
    export_date: datetime
    
    # Character data
    character_data: Dict[str, Any] = Field(..., description="Complete GFF data")
    
    # Optional metadata
    summary: Optional[CharacterSummary] = None
    custom_content: Optional[Dict[str, Any]] = None
    module_info: Optional[Dict[str, Any]] = None
    
    # Export options
    include_inventory: bool = True
    include_spells: bool = True
    include_journal: bool = True


class CharacterImportRequest(BaseModel):
    """Request to import character data"""
    file_path: str = Field(..., description="Path to character file")
    import_options: Dict[str, bool] = Field(default_factory=dict)
    should_validate: bool = Field(True, description="Validate on import")


class CharacterImportResponse(BaseModel):
    """Response after importing character"""
    success: bool
    character_id: Union[int, str]
    character_info: CharacterInfo
    validation_result: Optional[ValidationResult] = None
    warnings: List[str] = Field(default_factory=list)


class CharacterSaveRequest(BaseModel):
    """Request to save character changes"""
    sync_current_state: bool = Field(False, description="Sync in-memory state first")
    create_backup: bool = Field(True, description="Create backup before saving")
    should_validate: bool = Field(True, description="Validate before saving")


class CharacterSaveResponse(BaseModel):
    """Response after saving character"""
    success: bool
    save_path: str
    backup_path: Optional[str] = None
    validation_result: Optional[ValidationResult] = None
    changes_saved: int
    file_size: int


class CharacterBackup(BaseModel):
    """Character backup information"""
    backup_path: str
    original_path: str
    created_at: datetime
    file_size: int
    backup_reason: Optional[str] = None


class CharacterBackupList(BaseModel):
    """List of character backups"""
    backups: List[CharacterBackup]
    total: int
    total_size: int


class CharacterComparisonRequest(BaseModel):
    """Request to compare two characters"""
    character_id_1: Union[int, str]
    character_id_2: Union[int, str]
    compare_sections: List[str] = Field(default_factory=list, description="Specific sections to compare")


class CharacterComparisonResult(BaseModel):
    """Character comparison result"""
    character_1: CharacterInfo
    character_2: CharacterInfo
    differences: Dict[str, Dict[str, Any]] = Field(..., description="Section -> differences")
    similarity_score: float = Field(..., ge=0, le=1, description="Overall similarity 0-1")


class CharacterCloneRequest(BaseModel):
    """Request to clone a character"""
    source_character_id: Union[int, str]
    new_name: str
    new_file_path: Optional[str] = None
    
    # Clone options
    reset_experience: bool = False
    reset_level: bool = False
    reset_inventory: bool = False
    reset_journal: bool = False


class CharacterCloneResponse(BaseModel):
    """Response after cloning character"""
    success: bool
    new_character_id: Union[int, str]
    new_character_info: CharacterInfo
    clone_path: str


class CharacterEventData(BaseModel):
    """Event data for character changes"""
    event_type: str
    timestamp: float
    manager: str
    data: Dict[str, Any]
    transaction_id: Optional[str] = None


class CharacterEvents(BaseModel):
    """Character event history"""
    events: List[CharacterEventData]
    total_events: int
    managers_involved: List[str]


class CharacterDebugInfo(BaseModel):
    """Debug information for troubleshooting"""
    character_id: Union[int, str]
    file_path: str
    file_size: int
    gff_version: str
    gff_type: str
    field_count: int
    struct_count: int
    list_count: int
    
    # Manager info
    managers_loaded: List[str]
    manager_errors: Dict[str, str] = Field(default_factory=dict)
    
    # Memory usage
    memory_usage: Optional[int] = None
    load_time: Optional[float] = None
    
    # Custom content
    custom_content_detected: bool
    custom_content_count: int
    unknown_fields: List[str] = Field(default_factory=list)