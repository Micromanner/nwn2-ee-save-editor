"""
Pydantic models for SaveManager - Only models that match actual SaveManager output
"""

from typing import Dict, Any, Optional, List
from pydantic import BaseModel, Field


# Models matching SaveManager.get_save_summary() output structure
class SaveDetails(BaseModel):
    """Single save details from SaveManager"""
    total: int = Field(..., description="Total save bonus")
    base: int = Field(..., description="Base save from class")
    ability: int = Field(..., description="Ability modifier")
    feat: int = Field(..., description="Feat bonuses")
    racial: int = Field(..., description="Racial bonuses")
    resistance: int = Field(..., description="Resistance bonuses")
    temporary: int = Field(..., description="Temporary modifiers")
    breakdown: str = Field(..., description="Formatted breakdown string")


class SaveSummaryResponse(BaseModel):
    """Response from SaveManager.get_save_summary()"""
    fortitude: SaveDetails
    reflex: SaveDetails
    will: SaveDetails
    conditions: List[str] = Field(default_factory=list, description="Save conditions like Evasion")
    immunities: List[str] = Field(default_factory=list, description="Immunities")


# Model for SaveManager.calculate_saving_throws() - different from summary (no conditions/immunities)
class SaveBreakdownResponse(BaseModel):
    """Response from SaveManager.calculate_saving_throws()"""
    fortitude: SaveDetails
    reflex: SaveDetails
    will: SaveDetails


# Model for individual save totals - matches manual construction in router
class SaveTotalsResponse(BaseModel):
    """Response from individual save calculation methods - router constructs manually"""
    fortitude: int
    reflex: int
    will: int


# Model for SaveManager.check_save() output
class SaveCheckResponse(BaseModel):
    """Response from SaveManager.check_save()"""
    success: Optional[bool] = Field(None, description="Success if rolled, None if probability")
    total_bonus: int = Field(..., description="Character's total save bonus")
    dc: int = Field(..., description="Difficulty class")
    roll_needed: int = Field(..., description="Minimum d20 roll needed")
    success_chance: Optional[int] = Field(None, description="Success percentage (for non-take_20)")
    auto_success: bool = Field(False, description="Automatic success")
    auto_fail: bool = Field(False, description="Automatic failure")


# Request models for SaveManager methods
class SaveCheckRequest(BaseModel):
    """Request for SaveManager.check_save()"""
    save_type: str = Field(..., description="'fortitude', 'reflex', or 'will'")
    dc: int = Field(..., description="Difficulty class")
    modifier: int = Field(0, description="Additional modifier")
    take_20: bool = Field(False, description="Take 20 instead of rolling")


class TemporaryModifierRequest(BaseModel):
    """Request for SaveManager temporary modifier methods"""
    save_type: str = Field(..., description="'fortitude', 'reflex', or 'will'")
    modifier: int = Field(..., description="Modifier amount")
    duration: float = Field(0, description="Duration in seconds (0 = until removed)")


class MiscSaveBonusRequest(BaseModel):
    """Request for SaveManager.set_misc_save_bonus()"""
    save_type: str = Field(..., description="'fortitude', 'reflex', or 'will'")
    value: int = Field(..., description="Bonus value to set")


# Model for SaveManager.set_misc_save_bonus() output - matches actual manager return
class MiscSaveBonusResponse(BaseModel):
    """Response from SaveManager.set_misc_save_bonus()"""
    save_type: str
    gff_field: str
    old_value: int
    new_value: int
    new_saves: Dict[str, int] = Field(..., description="New save totals: {fortitude: int, reflex: int, will: int}")


# Model for SaveManager.get_racial_saves() output
class RacialSavesResponse(BaseModel):
    """Response from SaveManager.get_racial_saves()"""
    fortitude: int = Field(0, description="Racial fortitude bonus")
    reflex: int = Field(0, description="Racial reflex bonus") 
    will: int = Field(0, description="Racial will bonus")


# Simple success response models
class TemporaryModifierResponse(BaseModel):
    """Response for temporary modifier operations"""
    success: bool = Field(True)
    message: str = Field(..., description="Operation result message")
    save_type: str = Field(..., description="Save type affected")
    modifier: int = Field(..., description="Modifier amount")
    duration: Optional[float] = Field(None, description="Duration if adding")


class ClearModifiersResponse(BaseModel):
    """Response for clearing all temporary modifiers"""
    success: bool = Field(True)
    message: str = Field(..., description="Operation result message")


# Aliases for backward compatibility if needed
SavesState = SaveSummaryResponse
SaveState = SaveSummaryResponse
SaveBreakdown = SaveBreakdownResponse