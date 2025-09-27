"""
Pydantic models for savegame operations
Handles save game import, export, backup, and file management
"""

from typing import Dict, Any, Optional, List
from datetime import datetime
from pydantic import BaseModel, Field


class SavegameImportRequest(BaseModel):
    """Request to import a save game"""
    save_path: str = Field(..., description="Full path to save game directory")
    character_name: Optional[str] = Field(None, description="Expected character name")
    validate_save: bool = Field(True, description="Validate save game integrity")
    create_backup: bool = Field(True, description="Create backup before importing")


class SavegameImportResponse(BaseModel):
    """Response after importing save game"""
    success: bool
    message: str
    character_id: str = Field(..., description="Imported character ID")
    character_name: str = Field(..., description="Character name")
    save_path: str = Field(..., description="Save game path")
    
    # Import details
    files_imported: int = Field(0, description="Number of files imported")
    backup_created: bool = Field(False, description="Whether backup was created")
    validation_warnings: List[str] = Field(default_factory=list)


class CompanionInfo(BaseModel):
    """Companion character information in save"""
    name: str = Field(..., description="Companion name")
    file_name: str = Field(..., description="ROS file name")
    tag: str = Field(..., description="Companion tag")
    level: Optional[int] = Field(None, description="Companion level")
    class_name: Optional[str] = Field(None, description="Primary class")
    is_active: bool = Field(False, description="Currently in party")
    influence: Optional[int] = Field(None, description="Influence with PC")


class SavegameCompanionsResponse(BaseModel):
    """List of companions in save game"""
    companions: List[CompanionInfo]
    count: int = Field(..., description="Number of companions")
    active_companions: int = Field(0, description="Companions in party")


class BackupInfo(BaseModel):
    """Backup file information"""
    path: str = Field(..., description="Backup directory path")
    folder_name: str = Field(..., description="Backup folder name")
    timestamp: str = Field(..., description="Backup creation timestamp (ISO format)")
    display_name: str = Field(..., description="Display name from savename.txt")
    size_bytes: int = Field(0, description="Backup size in bytes")
    original_save: str = Field(..., description="Original save folder name")


class SavegameInfoResponse(BaseModel):
    """Complete save game information"""
    save_directory: str = Field(..., description="Save game directory")
    character_name: str = Field(..., description="Main character name")
    
    # File information
    original_save_exists: bool = Field(..., description="Original save files exist")
    files_in_save: List[str] = Field(..., description="All files in save directory")
    save_size_bytes: int = Field(0, description="Total save size")
    
    # Game information
    module_name: Optional[str] = Field(None, description="Current module")
    area_name: Optional[str] = Field(None, description="Current area")
    campaign: Optional[str] = Field(None, description="Campaign name")
    character_level: Optional[int] = Field(None, description="Character level")
    play_time: Optional[int] = Field(None, description="Play time in seconds")
    
    # Companions and backups
    companions: List[CompanionInfo] = Field(default_factory=list)
    backups: List[BackupInfo] = Field(default_factory=list)
    
    # Save metadata
    last_modified: Optional[datetime] = Field(None, description="When save was last modified")
    created: Optional[datetime] = Field(None, description="When save was created")


class SavegameUpdateRequest(BaseModel):
    """Request to update save game files"""
    sync_current_state: bool = Field(False, description="Sync current character state to save")
    updates: Dict[str, Any] = Field(default_factory=dict, description="Specific file updates")
    create_backup: bool = Field(True, description="Create backup before updating")
    validate_after_update: bool = Field(True, description="Validate save after changes")


class SavegameUpdateResponse(BaseModel):
    """Response after updating save game"""
    success: bool
    message: str
    
    # Update details
    changes: Dict[str, Any] = Field(..., description="Changes made to save files")
    files_updated: List[str] = Field(default_factory=list, description="Files that were modified")
    backup_created: bool = Field(False, description="Whether backup was created")
    backup_path: Optional[str] = Field(None, description="Path to created backup")
    
    # Validation
    validation_passed: bool = Field(True, description="Whether save passed validation")
    validation_warnings: List[str] = Field(default_factory=list)


class SavegameRestoreRequest(BaseModel):
    """Request to restore save from backup"""
    backup_path: str = Field(..., description="Full path to backup directory")
    confirm_restore: bool = Field(False, description="Confirmation for destructive operation")
    create_pre_restore_backup: bool = Field(True, description="Backup current state before restore")


class SavegameBackupsResponse(BaseModel):
    """Response listing available backups"""
    backups: List[BackupInfo] = Field(..., description="List of available backups")
    count: int = Field(..., description="Number of backups found")


class SavegameRestoreResponse(BaseModel):
    """Response after restoring save from backup"""
    success: bool = Field(..., description="Whether restore was successful")
    restored_from: str = Field(..., description="Backup path restored from")
    files_restored: List[str] = Field(..., description="Files that were restored")
    pre_restore_backup: Optional[str] = Field(None, description="Pre-restore backup path")
    restore_timestamp: str = Field(..., description="When restore completed")
    backups_cleaned_up: int = Field(0, description="Number of old backups cleaned up")


class SavegameExportRequest(BaseModel):
    """Request to export save game"""
    export_path: str = Field(..., description="Target export directory")
    export_format: str = Field("nwn2", description="Export format (nwn2, portable)")
    include_companions: bool = Field(True, description="Include companion files")
    compress: bool = Field(False, description="Compress exported files")
    character_name_override: Optional[str] = Field(None, description="Override character name")


class SavegameExportResponse(BaseModel):
    """Response after exporting save game"""
    success: bool
    message: str
    export_path: str = Field(..., description="Final export path")
    files_exported: List[str] = Field(..., description="Files included in export")
    export_size_bytes: int = Field(0, description="Size of exported data")


class SavegameValidationRequest(BaseModel):
    """Request to validate save game integrity"""
    deep_validation: bool = Field(False, description="Perform deep GFF structure validation")
    check_companions: bool = Field(True, description="Validate companion files")
    check_module_compatibility: bool = Field(True, description="Check module compatibility")
    repair_if_possible: bool = Field(False, description="Attempt automatic repairs")


class SavegameValidationResponse(BaseModel):
    """Save game validation results"""
    valid: bool = Field(..., description="Whether save is valid")
    
    # Validation results
    errors: List[str] = Field(default_factory=list, description="Critical errors found")
    warnings: List[str] = Field(default_factory=list, description="Non-critical issues")
    info: List[str] = Field(default_factory=list, description="Informational notes")
    
    # Specific checks
    gff_structure_valid: bool = Field(True, description="GFF files are structurally sound")
    companions_valid: bool = Field(True, description="Companion files are valid")
    module_compatible: bool = Field(True, description="Compatible with current module")
    custom_content_detected: bool = Field(False, description="Custom content detected")
    
    # Repair results (if requested)
    repairs_attempted: List[str] = Field(default_factory=list)
    repairs_successful: List[str] = Field(default_factory=list)
    repairs_failed: List[str] = Field(default_factory=list)


class SavegameBrowseRequest(BaseModel):
    """Request to browse save game directories"""
    base_path: Optional[str] = Field(None, description="Base directory to search")
    include_subdirectories: bool = Field(True, description="Search subdirectories")
    filter_by_character: Optional[str] = Field(None, description="Filter by character name")
    sort_by: str = Field("modified", description="Sort by: name, created, modified, size")
    sort_order: str = Field("desc", description="Sort order: asc, desc")


class SavegameListItem(BaseModel):
    """Individual save game in browse results"""
    save_directory: str = Field(..., description="Save directory path")
    character_name: str = Field(..., description="Character name")
    character_level: Optional[int] = Field(None, description="Character level")
    
    # File info
    last_modified: datetime = Field(..., description="Last modification time")
    created: Optional[datetime] = Field(None, description="Creation time")
    size_bytes: int = Field(0, description="Total size")
    file_count: int = Field(0, description="Number of files")
    
    # Game info
    module_name: Optional[str] = Field(None, description="Current module")
    campaign: Optional[str] = Field(None, description="Campaign name")
    play_time: Optional[int] = Field(None, description="Play time in seconds")
    
    # Status
    has_companions: bool = Field(False, description="Has companion files")
    has_custom_content: bool = Field(False, description="Contains custom content")
    is_valid: bool = Field(True, description="Save appears valid")


class SavegameBrowseResponse(BaseModel):
    """Browse save games response"""
    saves: List[SavegameListItem] = Field(..., description="Found save games")
    total_count: int = Field(..., description="Total number of saves found")
    
    # Search info
    base_path: str = Field(..., description="Base path searched")
    search_time_ms: int = Field(0, description="Search time in milliseconds")
    filters_applied: Dict[str, Any] = Field(default_factory=dict)