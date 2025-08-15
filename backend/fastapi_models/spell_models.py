"""
Pydantic models for SpellManager
Handles spellcasting, spell books, metamagic, and spell preparation
"""

from typing import Dict, Any, Optional, List, Literal, Union
from pydantic import BaseModel, Field


class SpellInfo(BaseModel):
    """Complete spell information from SpellManager.get_available_spells()"""
    id: int = Field(..., description="Spell ID from spells.2da")
    name: str = Field(..., description="Spell name")
    description: Optional[str] = None
    icon: Optional[str] = None
    
    # Spell properties
    school_id: Optional[int] = Field(None, description="School of magic ID")
    school_name: Optional[str] = Field(None, description="School name (Evocation, etc.)")
    level: int = Field(..., ge=0, le=9, description="Spell level")
    
    # Casting properties
    cast_time: Optional[str] = Field(None, description="Casting time")
    range: Optional[str] = Field(None, description="Spell range")
    conjuration_time: Optional[str] = Field(None, description="Conjuration time")
    components: Optional[str] = Field(None, description="V, S, M components")
    
    # Target information
    target_type: Optional[str] = Field(None, description="Target type")
    metamagic: Optional[str] = Field(None, description="Metamagic options")
    
    # Class availability
    available_classes: List[str] = Field(default_factory=list, description="Classes that can cast this spell")


class SpellSchool(BaseModel):
    """School of magic information"""
    id: int
    name: str
    description: Optional[str] = None
    opposition_schools: List[int] = Field(default_factory=list, description="Opposing school IDs")
    specialist_bonus: bool = Field(False, description="Character gets specialist bonus")


class SpellcastingClass(BaseModel):
    """Spellcasting class information from manager iteration"""
    index: int = Field(..., description="Class list index")
    class_id: int = Field(..., description="Class ID")
    class_name: str = Field(..., description="Class name")
    class_level: int = Field(..., description="Levels in this class")
    caster_level: int = Field(..., description="Effective caster level")
    spell_type: Literal['prepared', 'spontaneous'] = Field(..., description="Spellcasting type")


class MetamagicFeat(BaseModel):
    """Metamagic feat information"""
    id: int = Field(..., description="Feat ID")
    name: str = Field(..., description="Feat name")
    level_cost: int = Field(..., description="Spell level increase")
    description: Optional[str] = None
    
    # Application rules
    can_apply_to_cantrips: bool = Field(False, description="Can apply to 0-level spells")
    can_apply_to_spontaneous: bool = Field(True, description="Can apply to spontaneous spells")
    stacks: bool = Field(True, description="Can stack with other metamagic")


class MemorizedSpell(BaseModel):
    """Individual memorized spell entry from get_all_memorized_spells()"""
    level: int = Field(..., ge=0, le=9, description="Spell level")
    spell_id: int = Field(..., description="Memorized spell ID")
    class_id: int = Field(..., description="Spellcasting class ID")
    metamagic: int = Field(0, description="Metamagic bitfield")
    ready: bool = Field(True, description="Spell is ready to cast")




class SpellSummaryClass(BaseModel):
    """Simplified spellcasting class for summary"""
    id: int
    name: str
    total_slots: int
    max_spell_level: int
    slots_by_level: Dict[int, int]


class SpellSummary(BaseModel):
    """High-level spellcasting summary from get_spell_summary()"""
    # Data provided by manager
    caster_classes: List[SpellSummaryClass] = Field(default_factory=list)
    total_spell_levels: int = Field(0, description="Total spell levels")
    metamagic_feats: List[MetamagicFeat] = Field(default_factory=list)
    spell_resistance: int = Field(0, description="Spell resistance")




# Removed complex spell functionality models not implemented in spell_manager:
# - SpellLearnRequest/Response (replaced by simple add/remove)
# - SpellForgetRequest/Response (replaced by simple add/remove)  
# - SpellMemorizeRequest/Response (manager has prepare_spell but different interface)
# - SpellUnmemorizeRequest/Response (manager has clear_memorized_spells)
# - SpellSearchRequest/Response (manager only has get_available_spells)
# - SpellBookExport/Import (not implemented in manager)
# - SpellCastSimulation/Result (not implemented in manager)
# - SpellValidation (manager has basic validate() method)
# - SpellUpdateRequest/Response (complex batch operations not implemented)


# Router-specific models
class SpellsState(BaseModel):
    """Complete spell state for spells editor"""
    spellcasting_classes: List[SpellcastingClass] = Field(default_factory=list)
    spell_summary: SpellSummary
    memorized_spells: List[MemorizedSpell] = Field(default_factory=list)
    available_by_level: Optional[Dict[int, List[SpellInfo]]] = None


class AvailableSpellsResponse(BaseModel):
    """Response for available spells endpoint"""
    spell_level: int
    class_id: Optional[int]
    available_spells: List[SpellInfo]
    total: int


class AllSpellsResponse(BaseModel):
    """Response for all spells endpoint"""
    spells: List[SpellInfo]
    count: int
    total_by_level: Dict[int, int]


class SpellManageRequest(BaseModel):
    """Request to manage spells (add/remove)"""
    action: Literal['add', 'remove'] = Field(..., description="Action to perform")
    spell_id: int = Field(..., description="Spell ID")
    class_index: int = Field(..., description="Spellcasting class index")
    spell_level: Optional[int] = Field(None, description="Override spell level")


class SpellManageResponse(BaseModel):
    """Response after managing spells"""
    message: str
    spell_summary: SpellSummary
    has_unsaved_changes: bool = True