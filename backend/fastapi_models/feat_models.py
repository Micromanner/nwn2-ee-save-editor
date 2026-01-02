"""
Pydantic models for FeatManager
Handles character feats and prerequisites
"""

from typing import Dict, Any, Optional, List
from pydantic import BaseModel, Field, ConfigDict


class FeatPrerequisites(BaseModel):
    """Feat prerequisite requirements"""
    abilities: Dict[str, int] = Field(default_factory=dict, description="Required ability scores")
    feats: List[int] = Field(default_factory=list, description="Required feat IDs")
    class_: int = Field(-1, description="Required class (-1 for any)", alias="class")
    level: int = Field(0, description="Required character level")
    bab: int = Field(0, description="Required BAB")
    spell_level: int = Field(0, description="Required spell level")

    model_config = ConfigDict(populate_by_name=True)


class FeatState(BaseModel):
    """Complete feat state from FeatManager"""
    summary: Dict[str, Any]
    all_feats: List[Dict[str, Any]] = Field(..., description="All character feats with details")
    available_feats: List[Dict[str, Any]] = Field(..., description="Feats that can be taken")
    legitimate_feats: List[Dict[str, Any]] = Field(..., description="Feats meeting prerequisites")
    recommended_feats: List[Dict[str, Any]] = Field(default_factory=list)
    point_summary: Dict[str, int] = Field(default_factory=dict, description="Feat slot summary: total_slots, total_feats, available")

    model_config = ConfigDict(extra='allow')


class FeatAddRequest(BaseModel):
    """Request to add a feat"""
    feat_id: int = Field(..., description="Feat ID to add")
    ignore_prerequisites: bool = Field(False, description="Bypass prerequisite checks")


class FeatAddResponse(BaseModel):
    """Response after adding a feat"""
    message: str
    feat_info: Dict[str, Any]
    feat_summary: Dict[str, Any]
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
    removed_feat: Dict[str, Any]
    feat_summary: Dict[str, Any]
    cascading_effects: List[Dict[str, Any]] = Field(default_factory=list)
    has_unsaved_changes: bool = True

    model_config = ConfigDict(extra='allow')


class FeatValidationResponse(BaseModel):
    """Feat prerequisite validation result"""
    feat_id: int
    feat_name: str
    can_take: bool
    has_feat: bool
    prerequisites: FeatPrerequisites
    missing_requirements: List[str] = Field(default_factory=list)


class AvailableFeatsResponse(BaseModel):
    """Response with available feats for selection"""
    available_feats: List[Dict[str, Any]] = Field(..., description="Feats that can be selected")
    total: int = Field(..., description="Total number of available feats")

    model_config = ConfigDict(extra='allow')


class PaginationMetadata(BaseModel):
    """Pagination metadata"""
    page: int = Field(..., description="Current page number")
    limit: int = Field(..., description="Items per page")
    total: int = Field(..., description="Total number of items")
    pages: int = Field(..., description="Total number of pages")
    has_next: bool = Field(..., description="Whether there is a next page")
    has_previous: bool = Field(..., description="Whether there is a previous page")


class LegitimateFeatsResponse(BaseModel):
    """Response with legitimate feats (paginated)"""
    feats: List[Dict[str, Any]] = Field(..., description="Legitimate feats for current page")
    pagination: PaginationMetadata = Field(..., description="Pagination metadata")

    model_config = ConfigDict(extra='allow')


class FeatsByCategoryResponse(BaseModel):
    """Response with feats organized by category"""
    categories: Dict[str, List[Dict[str, Any]]] = Field(..., description="Feats organized by category")
    total_feats: int = Field(..., description="Total number of feats across all categories")
    character_level: int = Field(..., description="Character level for context")

    model_config = ConfigDict(extra='allow')
