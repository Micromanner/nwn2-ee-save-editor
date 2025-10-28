"""
Pydantic models for ClassManager
Handles character classes, multiclassing, and level progression
"""

from typing import Dict, Any, Optional, List, Literal
from pydantic import BaseModel, Field


class ClassInfo(BaseModel):
    """Complete class information - simplified to match ClassCategorizer output"""
    id: int = Field(..., description="Class ID from classes.2da")
    name: str = Field(..., description="Class name")
    label: str = Field(..., description="Display label")
    description: Optional[str] = None
    
    # Class properties - match ClassCategorizer ClassInfo
    type: str = Field(..., description="base, prestige, or npc")
    focus: str = Field(..., description="combat, arcane_caster, divine_caster, etc.")
    max_level: int = Field(..., description="Maximum level")
    hit_die: int = Field(..., description="Hit die size")
    skill_points: int = Field(..., description="Skill points per level")
    
    # Simplified progression - just BAB type
    bab_progression: str = Field(..., description="good, average, or poor")
    
    # Spellcasting
    is_spellcaster: bool = Field(False, description="Has spellcasting")
    has_arcane: bool = Field(False, description="Casts arcane spells")
    has_divine: bool = Field(False, description="Casts divine spells")
    
    # Requirements
    primary_ability: Optional[str] = Field(None, description="Primary ability score")
    alignment_restricted: bool = Field(False, description="Has alignment restrictions")
    prerequisites: Optional[Dict[str, Any]] = Field(None, description="Class prerequisites")
    
    # Special properties
    is_custom: bool = Field(False, description="Custom/modded class")
    is_available: bool = Field(True, description="Character can take this class")
    
    # Router expects parsed_description field
    parsed_description: Optional[Dict[str, Any]] = Field(None, description="Parsed class description")


class ClassLevel(BaseModel):
    """Individual class level information - matches manager's get_class_summary() output"""
    id: int = Field(..., description="Class ID")
    name: str = Field(..., description="Class name")
    level: int = Field(..., description="Levels in this class")
    
    model_config = {"populate_by_name": True}


class ClassFeature(BaseModel):
    """Class feature/ability gained at specific levels"""
    feature_name: str = Field(..., description="Feature name")
    level_gained: int = Field(..., description="Level when gained")
    description: Optional[str] = None
    feature_type: str = Field("ability", description="Type of feature")
    
    # Mechanical effects
    feat_gained: Optional[int] = Field(None, description="Feat ID if grants feat")
    spell_level_gained: Optional[int] = Field(None, description="Spell level unlocked")
    special_ability: Optional[str] = Field(None, description="Special ability description")


class MulticlassInfo(BaseModel):
    """Multiclassing status - simplified to match manager data"""
    is_multiclass: bool = Field(False, description="Character has multiple classes")
    can_multiclass: bool = Field(True, description="Can take additional classes")


class CombatProgression(BaseModel):
    """Combat statistics - matches manager's get_attack_bonuses() and calculate_total_saves()"""
    # From get_attack_bonuses()
    base_attack_bonus: int = Field(..., description="Total BAB")
    melee_attack_bonus: int = Field(..., description="Melee attack bonus")
    ranged_attack_bonus: int = Field(..., description="Ranged attack bonus")
    touch_attack_bonus: int = Field(..., description="Touch attack bonus")
    multiple_attacks: List[int] = Field(..., description="Multiple attack sequence")
    has_weapon_finesse: bool = Field(False, description="Has Weapon Finesse feat")
    
    # From calculate_total_saves()
    fortitude: int = Field(..., description="Total Fortitude save")
    reflex: int = Field(..., description="Total Reflex save")
    will: int = Field(..., description="Total Will save")
    base_fortitude: int = Field(..., description="Base Fortitude save")
    base_reflex: int = Field(..., description="Base Reflex save")
    base_will: int = Field(..., description="Base Will save")


class ClassSummary(BaseModel):
    """Complete class summary - matches manager's get_class_summary() exactly"""
    classes: List[ClassLevel]
    total_level: int
    multiclass: bool = Field(False, description="Character has multiple classes")
    can_multiclass: bool = Field(True, description="Can take additional classes")


class ClassState(BaseModel):
    """Complete class state from ClassManager"""
    summary: ClassSummary
    all_classes: List[ClassInfo] = Field(..., description="All available classes")
    prestige_options: List[ClassInfo] = Field(default_factory=list, description="Available prestige classes")
    class_features: Dict[str, List[ClassFeature]] = Field(default_factory=dict, description="Features by class")


class ClassChangeRequest(BaseModel):
    """Request to change character classes"""
    action: Literal['change_primary', 'add_level', 'remove_class'] = Field(..., description="Type of change")
    class_id: int = Field(..., description="Class to modify")
    
    # For class changes
    preserve_level: bool = Field(True, description="Keep total level when changing primary class")
    
    # For level changes  
    levels: int = Field(1, ge=1, description="Number of levels to add/remove")
    
    # Options
    cheat_mode: bool = Field(False, description="Bypass prerequisites and restrictions")
    preview: bool = Field(False, description="Only preview changes without applying them")


class ClassChangeResponse(BaseModel):
    """Response after changing classes"""
    success: bool
    message: str
    
    # Changes made
    class_changes: Dict[str, Any] = Field(..., description="Class modifications")
    stats_updated: Dict[str, Any] = Field(default_factory=dict, description="Derived stat changes")
    feats_changed: Dict[str, List[int]] = Field(default_factory=dict, description="Feat additions/removals")
    spells_changed: Dict[str, Any] = Field(default_factory=dict, description="Spell changes")
    skills_reset: bool = Field(False, description="Skills were reset")
    
    # Updated summary
    class_summary: ClassSummary
    has_unsaved_changes: bool = True


class PrestigeClassOption(BaseModel):
    """Available prestige class information"""
    class_info: ClassInfo
    meets_prerequisites: bool
    missing_requirements: List[str] = Field(default_factory=list)
    
    # Recommendation
    recommended: bool = Field(False, description="Good fit for character")
    recommendation_reason: Optional[str] = None


class ClassProgressionPreview(BaseModel):
    """Preview of class progression"""
    class_id: int
    class_name: str
    progression_levels: List[Dict[str, Any]] = Field(..., description="Features gained per level")
    
    # Summary
    total_levels_previewed: int
    key_features: List[str] = Field(default_factory=list, description="Major features")
    spellcasting_progression: Optional[Dict[str, Any]] = None


class ClassValidationRequest(BaseModel):
    """Request to validate class changes"""
    proposed_classes: List[Dict[str, int]] = Field(..., description="Proposed class/level pairs")
    check_prerequisites: bool = Field(True, description="Check class prerequisites")
    check_multiclass_penalty: bool = Field(True, description="Calculate multiclass penalties")


class ClassValidationResponse(BaseModel):
    """Class change validation result"""
    valid: bool
    errors: List[str] = Field(default_factory=list)
    warnings: List[str] = Field(default_factory=list)
    
    # Penalty analysis
    multiclass_penalty: float = Field(0.0, description="XP penalty percentage")
    experience_penalty: int = Field(0, description="Total XP penalty")
    
    # Recommendations
    suggestions: List[str] = Field(default_factory=list)
    alternative_builds: List[Dict[str, Any]] = Field(default_factory=list)


class ClassSearchRequest(BaseModel):
    """Request to search for classes"""
    query: Optional[str] = Field(None, description="Search term")
    class_type: Optional[str] = Field(None, description="base, prestige, or npc")
    focus: Optional[str] = Field(None, description="combat, arcane_caster, etc.")
    
    # Filters
    spellcaster_only: bool = Field(False, description="Only spellcasting classes")
    available_only: bool = Field(False, description="Only classes character can take")
    prestige_only: bool = Field(False, description="Only prestige classes")
    
    # Requirements
    min_bab: Optional[int] = Field(None, description="Minimum BAB requirement")
    alignment_match: bool = Field(False, description="Match character's alignment")
    
    # Pagination
    page: int = Field(1, ge=1)
    limit: int = Field(50, ge=1, le=200)


class ClassSearchResponse(BaseModel):
    """Class search results"""
    classes: List[ClassInfo]
    total: int
    page: int
    pages: int
    
    # Search metadata
    filters_applied: Dict[str, Any] = Field(default_factory=dict)
    prestige_classes_found: int = Field(0, description="Number of prestige classes in results")


class ClassBuildExport(BaseModel):
    """Class build for export/sharing"""
    character_level: int
    class_progression: List[Dict[str, Any]] = Field(..., description="Level-by-level class choices")
    
    # Build metadata
    build_name: Optional[str] = None
    build_type: Optional[str] = None  # "pure", "multiclass", "gish", etc.
    notes: Optional[str] = None
    
    # Calculated stats
    final_bab: int
    final_saves: Dict[str, int]
    spellcasting_summary: Optional[Dict[str, Any]] = None


class ClassBuildImportRequest(BaseModel):
    """Request to import class build"""
    build_data: ClassBuildExport
    apply_to_character: bool = Field(False, description="Apply build to current character")
    validate_build: bool = Field(True, description="Validate build before applying")


class ClassBuildImportResponse(BaseModel):
    """Response after importing class build"""
    success: bool
    message: str
    
    # Build analysis
    build_valid: bool
    validation_errors: List[str] = Field(default_factory=list)
    
    # If applied
    changes_made: Optional[List[Dict[str, Any]]] = None
    class_summary: Optional[ClassSummary] = None
    has_unsaved_changes: bool = False


class ClassesState(BaseModel):
    """Complete state from ClassManager for the frontend"""
    # From get_class_summary()
    classes: List[ClassLevel]
    total_level: int
    multiclass: bool = Field(False, description="Character has multiple classes")
    can_multiclass: bool = Field(True, description="Can take additional classes")
    
    # From get_attack_bonuses() and calculate_total_saves() - combined
    combat_stats: Dict[str, Any] = Field(default_factory=dict)


class LevelUpRequest(BaseModel):
    """Request to level up a character in a specific class"""
    class_id: int = Field(..., description="Class to level up in")
    cheat_mode: bool = Field(False, description="Bypass level restrictions")
    preview: bool = Field(False, description="Only preview the changes")


class ClassChangeResult(BaseModel):
    """Result after changing a character's class"""
    success: bool
    message: str
    class_change: Dict[str, Any] = Field(default_factory=dict)
    has_unsaved_changes: bool = True
    updated_state: Dict[str, Any] = Field(default_factory=dict, description="Updated character state after change")


class ClassChangePreview(BaseModel):
    """Preview of class change effects"""
    preview: bool = True
    class_change: Dict[str, Any] = Field(default_factory=dict)
    has_unsaved_changes: bool = True


class LevelUpResult(BaseModel):
    """Result after leveling up a character"""
    success: bool
    message: str
    level_changes: Dict[str, Any] = Field(default_factory=dict)
    has_unsaved_changes: bool = True
    updated_state: Dict[str, Any] = Field(default_factory=dict, description="Updated character state after level change")


class LevelUpPreview(BaseModel):
    """Preview of level up effects"""
    preview: bool = True
    level_change: int
    has_unsaved_changes: bool = True


class FocusInfo(BaseModel):
    """Information about a class focus category"""
    name: str = Field(..., description="Focus display name")
    description: str = Field(..., description="Focus description")
    color: Optional[str] = Field(None, description="UI color for this focus")
    icon: Optional[str] = Field(None, description="UI icon for this focus")


class SearchClassesResult(BaseModel):
    """Search results for classes"""
    search_results: List[ClassInfo] = Field(default_factory=list)
    query: Optional[str] = None
    total_results: int = 0


class CategorizedClassesResponse(BaseModel):
    """Response with classes organized by type and focus"""
    categories: Dict[str, Dict[str, List[ClassInfo]]] = Field(default_factory=dict)
    focus_info: Dict[str, FocusInfo] = Field(default_factory=dict)
    total_classes: int = 0
    include_unplayable: bool = False
    character_context: Optional[Dict[str, Any]] = None
    
    # Search mode fields (optional)
    search_results: Optional[List[ClassInfo]] = None
    query: Optional[str] = None
    total_results: Optional[int] = None


class ClassFeaturesRequest(BaseModel):
    """Request for detailed class features"""
    class_id: int = Field(..., description="Class to get features for")
    max_level: int = Field(20, description="Maximum level to show")
    include_spells: bool = Field(True, description="Include spell progression")
    include_proficiencies: bool = Field(True, description="Include weapon/armor proficiencies")
    character_id: Optional[int] = Field(None, description="Character context")


class ClassFeaturesResponse(BaseModel):
    """Response with detailed class features and progression"""
    class_id: int
    class_name: str
    basic_info: Dict[str, Any] = Field(default_factory=dict)
    description: Dict[str, Any] = Field(default_factory=dict)
    max_level_shown: int = 20
    
    # Feature progression (optional, can be added later)
    features_by_level: Optional[Dict[int, List[ClassFeature]]] = None
    spell_progression: Optional[Dict[str, Any]] = None
    proficiencies: Optional[List[str]] = None