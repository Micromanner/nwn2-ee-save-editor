"""
Level-Up Models - Pydantic models for level-up workflow
"""

from typing import Dict, List, Optional, Any
from pydantic import BaseModel, Field


class FeatSlotInfo(BaseModel):
    general: int = Field(0, description="General feat slots available (levels 1, 3, 6, 9...)")
    bonus: int = Field(0, description="Bonus feat slots (class-granted)")


class LevelUpRequirements(BaseModel):
    can_level_up: bool = Field(description="Whether character can level up")
    current_level: int = Field(description="Current total character level")
    new_level: int = Field(description="Level after level-up")
    class_id: int = Field(description="Class ID to level up in")
    class_name: str = Field(description="Human-readable class name")
    new_class_level: int = Field(description="New level in this specific class")
    hp_gain: int = Field(description="Maximum HP gain (hit die + CON modifier)")
    skill_points: int = Field(description="Skill points to allocate")
    feat_slots: FeatSlotInfo = Field(default_factory=FeatSlotInfo)
    has_ability_increase: bool = Field(description="Whether this level grants +1 ability (every 4 levels)")
    spell_slots_gained: Dict[int, int] = Field(default_factory=dict, description="Spell slots gained by level")
    available_feats: List[Dict[str, Any]] = Field(default_factory=list, description="Feats available to select")
    available_spells: Dict[int, List[Dict[str, Any]]] = Field(default_factory=dict, description="Spells available by level")
    class_skills: List[int] = Field(default_factory=list, description="Class skill IDs (1 point = 1 rank)")
    cross_class_skills: List[int] = Field(default_factory=list, description="Cross-class skill IDs (2 points = 1 rank)")
    is_spellcaster: bool = Field(False, description="Whether this class is a spellcaster")
    current_abilities: Dict[str, int] = Field(default_factory=dict, description="Current ability scores")


class SpellSelection(BaseModel):
    spell_level: int = Field(description="Spell level (0-9)")
    spell_id: int = Field(description="Spell ID from spells.2da")


class LevelUpSelections(BaseModel):
    class_id: int = Field(description="Class ID to level up in")
    ability_increase: Optional[str] = Field(None, description="Ability to increase: Str, Dex, Con, Int, Wis, Cha")
    feats: List[int] = Field(default_factory=list, description="Feat IDs to add")
    skills: Dict[int, int] = Field(default_factory=dict, description="skill_id -> points_spent (not ranks)")
    spells: List[SpellSelection] = Field(default_factory=list, description="Spells to learn")


class LevelUpPreviewResponse(BaseModel):
    valid: bool = Field(description="Whether all selections are valid")
    errors: List[str] = Field(default_factory=list, description="Validation errors")
    warnings: List[str] = Field(default_factory=list, description="Non-blocking warnings")
    hp_gained: int = Field(description="HP that will be gained")
    new_total_level: int = Field(description="New total character level")
    new_class_level: int = Field(description="New level in this class")
    stats_preview: Dict[str, Any] = Field(default_factory=dict, description="Preview of stat changes")
    feats_to_gain: List[Dict[str, Any]] = Field(default_factory=list, description="Feats that will be added")
    skills_to_update: Dict[int, int] = Field(default_factory=dict, description="Skills to update")
    spells_to_learn: List[Dict[str, Any]] = Field(default_factory=list, description="Spells to learn")


class LevelUpApplyResponse(BaseModel):
    success: bool = Field(description="Whether level-up was applied successfully")
    message: str = Field(description="Status message")
    new_total_level: int = Field(description="New total character level")
    new_class_level: int = Field(description="New level in this class")
    changes: Dict[str, Any] = Field(default_factory=dict, description="All changes made")
    has_unsaved_changes: bool = Field(True, description="Whether save file has unsaved changes")
    updated_state: Dict[str, Any] = Field(default_factory=dict, description="Updated character state")
