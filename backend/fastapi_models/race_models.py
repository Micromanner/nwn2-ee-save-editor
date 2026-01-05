"""Pydantic models for RaceManager."""

from typing import Dict, Any, Optional, List, Literal
from pydantic import BaseModel, Field


class CurrentRace(BaseModel):
    """Character's current race information - matches get_racial_properties() output."""
    race_id: int
    race_name: str
    subrace: str = ""
    size: int
    size_name: str
    base_speed: int
    ability_modifiers: Dict[str, int] = Field(default_factory=dict)
    racial_feats: List[int] = Field(default_factory=list)
    favored_class: Optional[int] = None


class RaceSummary(BaseModel):
    """Summary of racial properties with formatted strings - matches get_race_summary() output."""
    race_id: int
    race_name: str
    subrace: str = ""
    size: int
    size_name: str
    base_speed: int
    ability_modifiers: Dict[str, int] = Field(default_factory=dict)
    racial_feats: List[int] = Field(default_factory=list)
    favored_class: Optional[int] = None
    ability_modifier_string: str = Field("None", description="Formatted modifier string like 'STR +2, DEX -1'")


class RaceChangeRequest(BaseModel):
    """Request to change character race."""
    race_id: int = Field(..., description="New race ID")
    subrace: str = Field("", description="New subrace string")
    preserve_feats: bool = Field(True, description="Keep existing racial feats if possible")


class RaceChangeResponse(BaseModel):
    """Response after changing race - matches change_race() return structure."""
    success: bool
    old_race: Dict[str, Any]  # Contains id, name, subrace
    new_race: Dict[str, Any]  # Contains id, name, subrace  
    ability_changes: List[Dict[str, Any]] = Field(default_factory=list)
    size_change: Optional[Dict[str, Any]] = None
    speed_change: Optional[Dict[str, Any]] = None
    feat_changes: Dict[str, List[Dict[str, Any]]] = Field(default_factory=dict)  # removed, added
    has_unsaved_changes: bool = True


class RaceValidationRequest(BaseModel):
    """Request to validate race selection."""
    race_id: int
    subrace_id: Optional[int] = None
    character_level: Optional[int] = None


class RaceValidationResponse(BaseModel):
    """Race validation result - matches validate_race_change() return structure."""
    valid: bool
    errors: List[str] = Field(default_factory=list)


class SubraceInfo(BaseModel):
    """Information about a subrace."""
    id: int
    name: str
    label: str
    base_race: int


class AvailableSubracesResponse(BaseModel):
    """Available subraces for a race - matches get_available_subraces() return structure."""
    race_id: int
    subraces: List[SubraceInfo] = Field(default_factory=list)


class SubraceValidationResponse(BaseModel):
    """Subrace validation result - matches validate_subrace() return structure."""
    race_id: int
    subrace: str
    valid: bool
    errors: List[str] = Field(default_factory=list)