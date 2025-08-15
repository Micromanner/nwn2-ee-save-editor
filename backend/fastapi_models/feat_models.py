"""
Pydantic models for FeatManager
Handles character feats, prerequisites, and feat chains
"""

from typing import Dict, Any, Optional, List, Literal
from pydantic import BaseModel, Field, ConfigDict


class FeatPrerequisites(BaseModel):
    """Feat prerequisite requirements"""
    abilities: Dict[str, int] = Field(default_factory=dict, description="Required ability scores")
    feats: List[int] = Field(default_factory=list, description="Required feat IDs")
    class_: int = Field(-1, description="Required class (-1 for any)", alias="class")
    level: int = Field(0, description="Required character level")
    bab: int = Field(0, description="Required BAB")
    spell_level: int = Field(0, description="Required spell level")
    
    # Additional prerequisites
    skills: Dict[int, int] = Field(default_factory=dict, description="Required skill ranks")
    race: List[int] = Field(default_factory=list, description="Required race IDs")
    alignment: Optional[str] = Field(None, description="Required alignment")
    deity: Optional[str] = Field(None, description="Required deity")
    
    # Special requirements
    or_prerequisites: List['FeatPrerequisites'] = Field(default_factory=list, description="Alternative prerequisites")
    custom_requirement: Optional[str] = Field(None, description="Custom requirement description")
    
    model_config = ConfigDict(populate_by_name=True)


class DetailedPrerequisite(BaseModel):
    """Detailed prerequisite check result"""
    type: Literal['ability', 'feat', 'class', 'level', 'bab', 'skill', 'race', 'alignment', 'custom']
    description: str
    met: bool
    current_value: Optional[Any] = None
    required_value: Optional[Any] = None
    
    # For feat prerequisites
    feat_id: Optional[int] = None
    feat_name: Optional[str] = None
    
    # For class prerequisites  
    class_id: Optional[int] = None
    class_name: Optional[str] = None
    
    # For skill prerequisites
    skill_id: Optional[int] = None
    skill_name: Optional[str] = None


class FeatInfo(BaseModel):
    """Complete feat information"""
    id: int = Field(..., description="Feat ID from feat.2da")
    name: str = Field(..., description="Feat name")
    label: str = Field(..., description="Display label")
    description: Optional[str] = None
    icon: Optional[str] = None
    
    # Categorization - match Manager's 'type' field
    category: Optional[str] = None
    subcategory: Optional[str] = None
    type: Optional[int] = None
    
    # Feat properties - match Manager output
    protected: bool = Field(False, description="Cannot be removed")
    custom: bool = Field(False, description="Custom/mod feat")
    has_feat: bool = Field(False, description="Character has this feat")
    
    # Prerequisites
    prerequisites: FeatPrerequisites = Field(default_factory=FeatPrerequisites)
    
    # Character state - optional validation fields
    can_take: Optional[bool] = Field(None, description="Character meets prerequisites")
    missing_requirements: Optional[List[str]] = Field(None, description="Unmet prerequisites")
    source: Optional[str] = Field(None, description="How feat was acquired")


class FeatChain(BaseModel):
    """Feat progression chain"""
    chain_name: str
    chain_id: str
    feats: List[FeatInfo]
    current_progress: int = Field(0, description="Number of feats in chain acquired")
    next_available: Optional[FeatInfo] = None
    completed: bool = False


class FeatSlots(BaseModel):
    """Available feat slots by type"""
    available: int = Field(0, description="Total available slots")
    used: int = Field(0, description="Total used slots")
    
    # Slot breakdown
    general: int = Field(0, description="General feat slots")
    fighter: int = Field(0, description="Fighter bonus feat slots")
    wizard: int = Field(0, description="Wizard bonus feat slots")
    ranger: int = Field(0, description="Ranger combat style slots")
    monk: int = Field(0, description="Monk bonus feat slots")
    rogue: int = Field(0, description="Rogue special ability slots")
    epic: int = Field(0, description="Epic feat slots")
    
    # Custom slots from mods
    custom: Dict[str, int] = Field(default_factory=dict, description="Custom feat slots")


class FeatCategories(BaseModel):
    """Feat category organization - flexible to match Manager's get_feat_categories_fast"""
    model_config = ConfigDict(extra='allow')  # Allow additional categories
    
    # Common categories that may appear
    General: Optional[List[FeatInfo]] = Field(None)
    Combat: Optional[List[FeatInfo]] = Field(None)
    Class: Optional[List[FeatInfo]] = Field(None)
    Epic: Optional[List[FeatInfo]] = Field(None)
    Metamagic: Optional[List[FeatInfo]] = Field(None)
    Divine: Optional[List[FeatInfo]] = Field(None)


class CurrentFeats(BaseModel):
    """Character's current feats organized by source - matches Manager's get_feat_summary_fast output"""
    total: int = Field(0, description="Total number of feats")
    
    # Match Manager's get_feat_summary_fast structure
    protected: List[FeatInfo] = Field(default_factory=list, description="Cannot be removed")
    class_feats: List[FeatInfo] = Field(default_factory=list, description="From class levels")
    general_feats: List[FeatInfo] = Field(default_factory=list, description="General feats")
    custom_feats: List[FeatInfo] = Field(default_factory=list, description="Custom/mod feats")


class FeatSummary(BaseModel):
    """Complete feat summary from FeatManager"""
    current_feats: CurrentFeats
    feat_slots: FeatSlots
    categories: FeatCategories
    feat_chains: List[FeatChain] = Field(default_factory=list)
    
    # Statistics
    total_feats: int
    epic_feats: int = 0
    custom_feats: int = 0
    
    # Validation
    has_invalid_feats: bool = False
    invalid_feats: List[int] = Field(default_factory=list)


class FeatState(BaseModel):
    """Complete feat state from FeatManager"""
    summary: Dict[str, Any]
    all_feats: List[Dict[str, Any]] = Field(..., description="All character feats with details")
    available_feats: List[Dict[str, Any]] = Field(..., description="Feats that can be taken")
    legitimate_feats: List[Dict[str, Any]] = Field(..., description="Feats meeting prerequisites")
    
    # Feat chains as dict like Manager returns
    feat_chains: Dict[str, Any]  # Manager returns dict
    recommended_feats: List[Dict[str, Any]] = Field(default_factory=list)
    
    model_config = ConfigDict(extra='allow')


class FeatAddRequest(BaseModel):
    """Request to add a feat"""
    feat_id: int = Field(..., description="Feat ID to add")
    ignore_prerequisites: bool = Field(False, description="Bypass prerequisite checks")
    feat_type: Optional[str] = Field(None, description="Feat slot type to use")


class FeatAddResponse(BaseModel):
    """Response after adding a feat"""
    message: str
    feat_info: Dict[str, Any]  # Manager returns dict
    feat_summary: Dict[str, Any]  # Manager returns dict
    cascading_effects: List[Dict[str, Any]] = Field(default_factory=list)
    has_unsaved_changes: bool = True
    
    model_config = ConfigDict(extra='allow')


class FeatRemoveRequest(BaseModel):
    """Request to remove a feat"""
    feat_id: int = Field(..., description="Feat ID to remove")
    force: bool = Field(False, description="Force removal even if protected")


class FeatRemoveResponse(BaseModel):
    """Response after removing a feat"""
    message: str
    removed_feat: Dict[str, Any]  # Manager returns dict
    feat_summary: Dict[str, Any]  # Manager returns dict
    cascading_effects: List[Dict[str, Any]] = Field(default_factory=list)
    dependent_feats_removed: List[int] = Field(default_factory=list)
    has_unsaved_changes: bool = True
    
    model_config = ConfigDict(extra='allow')


class FeatValidationRequest(BaseModel):
    """Request to validate feat prerequisites"""
    feat_id: int
    detailed: bool = Field(False, description="Include detailed prerequisite breakdown")


class FeatValidationResponse(BaseModel):
    """Feat prerequisite validation result"""
    feat_id: int
    feat_name: str
    can_take: bool
    has_feat: bool
    
    # Prerequisites
    prerequisites: FeatPrerequisites
    detailed_prerequisites: List[DetailedPrerequisite] = Field(default_factory=list)
    
    # Missing requirements
    missing_requirements: List[str] = Field(default_factory=list)
    met_requirements: List[str] = Field(default_factory=list)
    
    # Suggestions
    suggestions: List[str] = Field(default_factory=list)


class FeatSearchRequest(BaseModel):
    """Request to search for feats"""
    query: Optional[str] = Field(None, description="Search term")
    category: Optional[str] = None
    subcategory: Optional[str] = None
    prerequisites_met: bool = Field(False, description="Only show feats with met prerequisites")
    include_epic: bool = Field(True, description="Include epic feats")
    include_custom: bool = Field(True, description="Include custom feats")
    
    # Pagination
    page: int = Field(1, ge=1)
    limit: int = Field(50, ge=1, le=200)


class FeatSearchResponse(BaseModel):
    """Feat search results"""
    feats: List[FeatInfo]
    total: int
    page: int
    pages: int
    
    # Search metadata
    query: Optional[str] = None
    filters_applied: Dict[str, Any] = Field(default_factory=dict)


class FeatBuild(BaseModel):
    """Feat build for planning/export"""
    character_level: int
    feats_by_level: Dict[int, List[int]] = Field(..., description="Level -> feat IDs")
    feat_order: List[int] = Field(..., description="Order feats were taken")
    
    # Build metadata
    build_name: Optional[str] = None
    build_type: Optional[str] = None  # "melee", "caster", "archer", etc.
    notes: Optional[str] = None
    
    # Validation
    valid: bool = True
    validation_errors: List[str] = Field(default_factory=list)


class FeatRespecRequest(BaseModel):
    """Request to respec feats"""
    keep_racial: bool = Field(True, description="Keep racial feats")
    keep_class: bool = Field(True, description="Keep class-granted feats")
    new_feats: List[int] = Field(default_factory=list, description="New feat selection")


class FeatRespecResponse(BaseModel):
    """Response after feat respec"""
    message: str
    removed_count: int
    added_count: int
    feat_summary: FeatSummary
    validation_errors: List[str] = Field(default_factory=list)
    has_unsaved_changes: bool = True


class FeatUpdateRequest(BaseModel):
    """General feat configuration update request"""
    feat_changes: Dict[str, Any] = Field(..., description="Feat modifications")
    add_feats: List[int] = Field(default_factory=list, description="Feat IDs to add")
    remove_feats: List[int] = Field(default_factory=list, description="Feat IDs to remove")
    ignore_prerequisites: bool = Field(False, description="Bypass prerequisite checks")
    validate_changes: bool = Field(True, description="Validate all changes")


class FeatUpdateResponse(BaseModel):
    """General feat update response"""
    success: bool
    message: str
    changes_applied: List[Dict[str, Any]] = Field(..., description="List of changes made")
    feats_added: List[int] = Field(default_factory=list)
    feats_removed: List[int] = Field(default_factory=list)
    validation_errors: List[str] = Field(default_factory=list)
    feat_summary: FeatSummary
    has_unsaved_changes: bool = True


class AvailableFeatsResponse(BaseModel):
    """Response with available feats for selection"""
    available_feats: List[Dict[str, Any]] = Field(..., description="Feats that can be selected")
    total: int = Field(..., description="Total number of available feats")
    
    model_config = ConfigDict(extra='allow')


class LegitimateFeatsResponse(BaseModel):
    """Response with legitimate feats (paginated)"""
    feats: List[Dict[str, Any]] = Field(..., description="Legitimate feats for current page")
    total: int = Field(..., description="Total number of legitimate feats")
    page: int = Field(..., description="Current page number")
    pages: int = Field(..., description="Total number of pages")
    limit: int = Field(..., description="Items per page")
    
    model_config = ConfigDict(extra='allow')


class FeatDetails(BaseModel):
    """Detailed information about a specific feat"""
    feat_info: Dict[str, Any] = Field(..., description="Basic feat information")
    detailed_prerequisites: Dict[str, Any] = Field(default_factory=dict, description="Manager returns dict")
    feat_chains: List[str] = Field(default_factory=list, description="Feat chains this belongs to")
    synergies: List[str] = Field(default_factory=list, description="Feats that work well with this")
    conflicts: List[str] = Field(default_factory=list, description="Incompatible feats")
    
    model_config = ConfigDict(extra='allow')


class FeatsByCategoryResponse(BaseModel):
    """Response with feats organized by category"""
    categories: Dict[str, List[Dict[str, Any]]] = Field(..., description="Feats organized by category")
    total_feats: int = Field(..., description="Total number of feats across all categories")
    character_level: int = Field(..., description="Character level for context")
    
    model_config = ConfigDict(extra='allow')