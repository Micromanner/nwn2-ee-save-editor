"""
Pydantic models for AbilityManager
Handles ability scores (attributes), modifiers, and derived statistics
"""

from typing import Dict, Any, Optional, List
from pydantic import BaseModel, Field, ConfigDict


class AbilityScore(BaseModel):
    """Individual ability score with all modifiers"""
    base: int = Field(..., ge=1, le=50, description="Base ability score")
    racial_modifier: int = 0
    item_modifier: int = 0
    enhancement_modifier: int = 0
    temporary_modifier: int = 0
    level_up_modifier: int = 0
    total: int = Field(..., description="Total calculated score")
    modifier: int = Field(..., description="Ability modifier (-5 to +20)")


class AbilityScores(BaseModel):
    """All six ability scores"""
    strength: AbilityScore
    dexterity: AbilityScore
    constitution: AbilityScore
    intelligence: AbilityScore
    wisdom: AbilityScore
    charisma: AbilityScore


class AbilityModifiers(BaseModel):
    """Ability modifiers in NWN2 format"""
    Str: int = 0
    Dex: int = 0
    Con: int = 0
    Int: int = 0
    Wis: int = 0
    Cha: int = 0


class DetailedModifiers(BaseModel):
    """Detailed breakdown of all modifier sources"""
    base_modifiers: Dict[str, int]
    racial_modifiers: Dict[str, int]
    item_modifiers: Dict[str, int]
    enhancement_modifiers: Dict[str, int]
    temporary_modifiers: Dict[str, int]
    level_up_modifiers: Dict[str, int]
    total_modifiers: Dict[str, int]


class AttributeDependencies(BaseModel):
    """What game systems depend on each attribute"""
    Str: List[str] = Field(default_factory=list, description="Systems affected by STR")
    Dex: List[str] = Field(default_factory=list, description="Systems affected by DEX")
    Con: List[str] = Field(default_factory=list, description="Systems affected by CON")
    Int: List[str] = Field(default_factory=list, description="Systems affected by INT")
    Wis: List[str] = Field(default_factory=list, description="Systems affected by WIS")
    Cha: List[str] = Field(default_factory=list, description="Systems affected by CHA")


class EncumbranceLimits(BaseModel):
    """Weight carrying capacity based on strength"""
    strength: int = Field(..., description="Character's strength score")
    normal_capacity: int = Field(..., description="Normal carrying capacity (light load)")
    medium_load: int = Field(..., description="Medium load threshold")
    heavy_load: int = Field(..., description="Heavy load threshold") 
    current_weight: int = Field(..., description="Current carried weight")


class HitPoints(BaseModel):
    """Character hit points"""
    current: int = Field(..., description="Current hit points")
    max: int = Field(..., description="Maximum hit points")


class CharacterBiography(BaseModel):
    """Character background and biographical information"""
    name: str = ""
    age: int = 0
    background: str = ""
    experience_points: int = 0


class AttributeState(BaseModel):
    """Complete attribute/ability state from AbilityManager"""
    # Core ability scores
    base_attributes: Dict[str, int] = Field(..., description="Base ability scores without modifiers")
    effective_attributes: Dict[str, int] = Field(..., description="Final ability scores with all modifiers")
    
    # Modifier breakdowns
    attribute_modifiers: Dict[str, int] = Field(..., description="Calculated ability modifiers")
    detailed_modifiers: DetailedModifiers
    
    # Derived statistics
    point_buy_cost: int = Field(..., description="Total point buy cost of current scores")
    derived_stats: Dict[str, Any] = Field(..., description="Derived statistics including hit_points")
    combat_stats: Dict[str, Any] = Field(default_factory=dict, description="Combat statistics including armor_class and initiative")
    saving_throws: Dict[str, Any] = Field(default_factory=dict, description="Saving throw details for fortitude, reflex, will")
    encumbrance_limits: EncumbranceLimits
    saving_throw_modifiers: Dict[str, int]
    skill_modifiers: Dict[int, int] = Field(default_factory=dict, description="Skill ID -> modifier from abilities")
    
    # Dependencies
    attribute_dependencies: AttributeDependencies
    
    
    # Biography
    biography: CharacterBiography
    point_summary: Optional[Dict[str, int]] = Field(None, description="Detailed point summary")


class AttributeChangeRequest(BaseModel):
    """Request to change ability scores"""
    attributes: Dict[str, int] = Field(..., description="Ability name -> new value")
    should_validate: bool = Field(True, description="Whether to validate changes")


class AttributeSetRequest(BaseModel):
    """Request to set a single ability score"""
    attribute: str = Field(..., description="Ability name (Str, Dex, Con, Int, Wis, Cha)")
    value: int = Field(..., ge=1, le=50, description="New ability score value")
    should_validate: bool = Field(True, description="Whether to validate the change")


class AttributeChangeResponse(BaseModel):
    """Response after changing ability scores"""
    success: bool = True
    attribute_changes: List[Dict[str, Any]] = Field(..., description="List of applied attribute changes")
    cascading_effects: List[Dict[str, Any]] = Field(default_factory=list, description="Secondary effects from changes")  
    saved: bool = Field(False, description="Whether changes were saved to disk")
    has_unsaved_changes: bool = True
    validation_errors: List[str] = Field(default_factory=list)
    warnings: List[str] = Field(default_factory=list)


class PointBuyRequest(BaseModel):
    """Point buy character creation request"""
    attributes: Dict[str, int] = Field(..., description="Target ability scores")
    total_points: int = Field(32, description="Total points available")
    racial_modifiers: Optional[Dict[str, int]] = None


class PointBuyResponse(BaseModel):
    """Point buy validation response"""
    valid: bool
    points_used: int
    points_remaining: int
    cost_breakdown: Dict[str, int] = Field(..., description="Cost per ability")
    errors: List[str] = Field(default_factory=list)


class AttributeRollRequest(BaseModel):
    """Request for rolling ability scores"""
    method: str = Field("4d6_drop_lowest", description="Rolling method")
    allow_reroll: bool = Field(True, description="Allow rerolling scores below minimum")
    minimum_total: int = Field(70, description="Minimum total of all scores")


class AttributeRollResponse(BaseModel):
    """Response with rolled ability scores"""
    rolls: Dict[str, List[int]] = Field(..., description="Individual die rolls per ability")
    final_scores: Dict[str, int] = Field(..., description="Final rolled scores")
    total: int = Field(..., description="Sum of all scores")
    point_buy_equivalent: int = Field(..., description="Equivalent point buy cost")


class AttributeSummary(BaseModel):
    """Summary of ability scores for display"""
    base_attributes: Dict[str, int]
    effective_attributes: Dict[str, int]
    modifiers: Dict[str, int]
    racial_modifiers: Dict[str, int]
    item_modifiers: Dict[str, int]
    enhancement_modifiers: Dict[str, int]
    temporary_modifiers: Dict[str, int]
    point_buy_cost: int
    
    # Character info
    character_name: Optional[str] = None
    character_age: Optional[int] = None
    character_background: Optional[str] = None
    experience_points: Optional[int] = None


class AttributeValidation(BaseModel):
    """Validation result for ability scores"""
    valid: bool
    errors: List[str] = Field(default_factory=list)
    warnings: List[str] = Field(default_factory=list)
    suggestions: List[str] = Field(default_factory=list)


class AttributeModifiersResponse(BaseModel):
    """Response with detailed breakdown of all attribute modifiers"""
    base_modifiers: Dict[str, int] = Field(..., description="Base ability modifiers from scores")
    racial_modifiers: Dict[str, int] = Field(..., description="Racial attribute modifiers")
    enhancement_modifiers: Dict[str, int] = Field(..., description="Enhancement bonuses to abilities")
    item_modifiers: Dict[str, int] = Field(..., description="Item bonuses to abilities")
    temporary_modifiers: Dict[str, int] = Field(..., description="Temporary bonuses to abilities")
    total_modifiers: Dict[str, int] = Field(..., description="Final total modifiers from all sources")