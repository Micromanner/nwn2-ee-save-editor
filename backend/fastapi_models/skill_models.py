"""
Pydantic models for SkillManager
Handles skill ranks, modifiers, and skill checks
"""

from typing import Dict, Any, Optional, List, Literal
from pydantic import BaseModel, Field


class SkillInfo(BaseModel):
    """Individual skill information - matches SkillManager.get_skill_info() output"""
    id: int = Field(..., description="Skill ID from skills.2da")
    label: str = Field(..., description="Skill label")
    name: str = Field(..., description="Skill name")
    key_ability: str = Field(..., description="Key ability (STR, DEX, etc.)")
    armor_check: bool = Field(False, description="Subject to armor check penalty")
    is_class_skill: bool = Field(..., description="Is this a class skill")
    current_ranks: int = Field(0, ge=0, description="Current skill ranks")
    max_ranks: int = Field(..., description="Maximum possible ranks")
    total_modifier: int = Field(..., description="Total skill modifier")


class SkillPoints(BaseModel):
    """Skill point allocation information - simplified to match manager"""
    available_points: int = Field(0, description="Available skill points")
    spent_points: int = Field(0, description="Total spent skill points")
    total_available: int = Field(..., description="Total skill points earned")
    overspent: int = Field(0, description="Points overspent")


class SkillSummary(BaseModel):
    """Complete skill summary from SkillManager - matches get_skill_summary() output"""
    # Point allocation
    available_points: int
    total_available: int
    spent_points: int
    overspent: int = 0
    
    # Rank statistics
    total_ranks: int
    skills_with_ranks: int
    
    # Skill lists
    class_skills: List[SkillInfo]
    cross_class_skills: List[SkillInfo]
    
    # Optional error field from manager
    error: Optional[str] = None


class SkillUpdateRequest(BaseModel):
    """Request to update skill ranks"""
    skills: Dict[str, int] = Field(..., description="skill_id -> new_rank mapping")
    should_validate: bool = Field(True, description="Validate point spending")


class SkillChange(BaseModel):
    """Individual skill change record"""
    skill_id: int
    skill_name: str
    old_rank: int
    new_rank: int
    points_spent: int = Field(..., description="Points used for this skill")
    new_total_modifier: int


class SkillUpdateResponse(BaseModel):
    """Response after updating skills"""
    changes: List[SkillChange]
    skill_summary: SkillSummary
    points_remaining: int
    validation_errors: List[str] = Field(default_factory=list)
    has_unsaved_changes: bool = True


class SkillBatchUpdateRequest(BaseModel):
    """Batch skill update request"""
    skills: Dict[str, int] = Field(..., description="skill_id -> new_rank")
    redistribute: bool = Field(False, description="Reset and redistribute all points")


class SkillBatchUpdateResponse(BaseModel):
    """Batch skill update response"""
    results: List[SkillChange]
    summary: SkillSummary
    total_changes: int
    points_refunded: int = 0
    saved: bool = False


class SkillResetRequest(BaseModel):
    """Request to reset skill points"""
    preserve_class_skills: bool = Field(False, description="Keep ranks in class skills")
    refund_percentage: int = Field(100, ge=0, le=100, description="Percentage of points to refund")


class SkillResetResponse(BaseModel):
    """Response after resetting skills"""
    message: str
    points_refunded: int
    available_points: int
    skills_reset: int
    saved: bool = False


class SkillCheckRequest(BaseModel):
    """Request for skill check simulation"""
    skill_id: int
    dc: int = Field(15, description="Difficulty class")
    take_10: bool = Field(False, description="Take 10 on the check")
    take_20: bool = Field(False, description="Take 20 on the check")
    circumstance_bonus: int = Field(0, description="Circumstance modifier")


class SkillCheckResponse(BaseModel):
    """Skill check simulation result"""
    skill_id: int
    skill_name: str
    roll: int = Field(..., description="d20 roll (or 10/20)")
    modifier: int = Field(..., description="Total skill modifier")
    circumstance: int = Field(0, description="Circumstance modifier")
    total: int = Field(..., description="Final result")
    dc: int = Field(..., description="Difficulty class")
    success: bool
    critical_success: bool = Field(False, description="Natural 20")
    critical_failure: bool = Field(False, description="Natural 1")
    
    # Breakdown
    breakdown: Dict[str, int] = Field(..., description="Modifier breakdown")
    margin: int = Field(..., description="Success/failure margin")


class SkillPrerequisites(BaseModel):
    """Skill prerequisites and requirements - matches SkillManager.get_skill_prerequisites() output"""
    skill_id: int
    requirements: List[Dict[str, Any]] = Field(default_factory=list)


class SkillBuild(BaseModel):
    """Skill build for export/import"""
    character_level: int
    total_skill_points: int
    skills: Dict[str, Dict[str, Any]] = Field(..., description="skill_name -> skill_info")
    
    # Metadata
    class_skills: List[str] = Field(default_factory=list)
    skill_focus: List[str] = Field(default_factory=list)
    notes: Optional[str] = None


class SkillBuildImportRequest(BaseModel):
    """Request to import skill build"""
    skills: Dict[str, Dict[str, Any]] = Field(..., description="skill_name -> skill_info")
    character_level: Optional[int] = None
    total_skill_points: Optional[int] = None
    replace_existing: bool = Field(False, description="Replace current skills")
    should_validate: bool = Field(True, description="Validate the build")


class SkillBuildImportResponse(BaseModel):
    """Response after importing skill build"""
    message: str
    summary: SkillSummary
    imported_count: int
    validation_errors: List[str] = Field(default_factory=list)
    saved: bool = False


class AllSkillsResponse(BaseModel):
    """Complete list of all skills - matches SkillManager.get_all_skills() output"""
    skills: List[Dict[str, Any]] = Field(..., description="List of all skills with current state")


