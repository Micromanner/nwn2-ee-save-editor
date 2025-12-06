"""
Pydantic models for ContentManager
Handles campaigns, modules, areas, quests, and custom content detection
"""

from __future__ import annotations
from typing import Dict, Any, Optional, List, Union
from datetime import datetime
from pydantic import BaseModel, Field


# Simplified module info matching ContentManager output
class ModuleInfo(BaseModel):
    """Basic module information from ContentManager"""
    module_name: str = Field("", description="Module display name")
    area_name: str = Field("", description="Current area name")
    campaign: str = Field("", description="Campaign name")
    entry_area: str = Field("", description="Entry area")
    module_description: str = Field("", description="Module description")


# Simplified campaign info matching ContentManager quest data
class CampaignInfo(BaseModel):
    """Basic campaign information from ContentManager"""
    total_quests: int = Field(0, description="Total quest count")
    completed_quests: int = Field(0, description="Completed quest count")
    active_quests: int = Field(0, description="Active quest count")
    quest_completion_rate: float = Field(0.0, description="Quest completion percentage")


class CampaignInfoResponse(BaseModel):
    """Combined campaign and module info response matching ContentManager.get_campaign_info()"""
    # Module info fields
    module_name: str = Field("", description="Module display name")
    area_name: str = Field("", description="Current area name")
    campaign: str = Field("", description="Campaign name")
    entry_area: str = Field("", description="Entry area")
    module_description: str = Field("", description="Module description")
    current_module: Optional[str] = Field(None, description="Current module from save")
    
    # Campaign data fields
    total_quests: int = Field(0, description="Total quest count")
    completed_quests: int = Field(0, description="Completed quest count")
    active_quests: int = Field(0, description="Active quest count")
    quest_completion_rate: float = Field(0.0, description="Quest completion percentage")
    quest_details: Optional[Dict[str, Any]] = Field(None, description="Detailed quest data")
    
    # Custom content will be added separately in router
    custom_content: Optional[CustomContentSummary] = Field(None, description="Custom content summary")








class CustomContentItem(BaseModel):
    """Individual custom content item matching ContentManager output"""
    type: str = Field(..., description="Type of custom content")
    id: int = Field(..., description="Content identifier")
    name: str = Field(..., description="Content name")
    source: str = Field(..., description="Content source")
    protected: bool = Field(False, description="Protected from removal")
    
    # Optional fields for some content types
    level: Optional[int] = Field(None, description="Level (for spells/classes)")
    index: Optional[int] = Field(None, description="Index in character data")


class CustomContentSummary(BaseModel):
    """Summary of all custom content matching ContentManager output"""
    total_count: int = Field(0, description="Total custom content items")
    by_type: Dict[str, int] = Field(default_factory=dict, description="Count by content type")
    items: List[Dict[str, Any]] = Field(default_factory=list, description="Individual item details")






class CompanionInfluenceData(BaseModel):
    """Companion influence information from globals.xml"""
    name: str = Field(..., description="Companion display name")
    influence: Optional[int] = Field(None, description="Current influence value")
    recruitment: str = Field(..., description="Recruitment status: not_recruited, met, recruited")
    source: str = Field(..., description="Data source: explicit or discovered")


class CompanionInfluenceResponse(BaseModel):
    """Response containing all companion influence data"""
    companions: Dict[str, CompanionInfluenceData] = Field(
        default_factory=dict,
        description="Map of companion_id to influence data"
    )


class UpdateCompanionInfluenceRequest(BaseModel):
    """Request to update companion influence"""
    companion_id: str = Field(..., description="Companion identifier (e.g., 'neeshka', 'khelgar')")
    new_influence: int = Field(..., description="New influence value")


class QuestVariable(BaseModel):
    """Individual quest variable from globals.xml"""
    name: str = Field(..., description="Variable name in globals.xml")
    value: Union[int, str, float] = Field(..., description="Variable value")
    type: str = Field(..., description="Variable type: int, string, float")
    category: Optional[str] = Field(None, description="Quest category: completed, active, unknown")


class QuestGroup(BaseModel):
    """Group of related quest variables with common prefix"""
    prefix: str = Field(..., description="Common quest prefix (e.g., 'n2_a1_')")
    name: str = Field(..., description="Quest group display name")
    variables: List[QuestVariable] = Field(default_factory=list, description="Quest variables in group")
    completed_count: int = Field(0, description="Number of completed quest variables")
    active_count: int = Field(0, description="Number of active quest variables")
    total_count: int = Field(0, description="Total quest variables in group")


class QuestDetailsResponse(BaseModel):
    """Detailed quest information from globals.xml"""
    groups: List[QuestGroup] = Field(default_factory=list, description="Quest groups by prefix")
    total_quests: int = Field(0, description="Total quest variables")
    completed_quests: int = Field(0, description="Completed quest variables")
    active_quests: int = Field(0, description="Active quest variables")
    unknown_quests: int = Field(0, description="Unknown quest variables")
    completion_rate: float = Field(0.0, description="Quest completion percentage")


class UpdateQuestVariableRequest(BaseModel):
    """Request to update a single quest variable"""
    variable_name: str = Field(..., description="Quest variable name in globals.xml")
    value: Union[int, str, float] = Field(..., description="New value for quest variable")
    variable_type: str = Field("int", description="Variable type: int, string, float")


class BatchUpdateQuestRequest(BaseModel):
    """Request to update multiple quest variables at once"""
    updates: List[UpdateQuestVariableRequest] = Field(..., description="List of quest variable updates")


class ModulePropertyUpdate(BaseModel):
    """Request to update module properties in module.ifo"""
    module_name: Optional[str] = Field(None, description="Module display name")
    module_description: Optional[str] = Field(None, description="Module description")
    entry_area: Optional[str] = Field(None, description="Entry area name")
    creator_id: Optional[str] = Field(None, description="Creator identifier")


class CampaignVariableUpdate(BaseModel):
    """Request to update campaign-specific variables in globals.xml"""
    variable_name: str = Field(..., description="Campaign variable name")
    value: Union[int, str, float] = Field(..., description="New value")
    variable_type: str = Field("int", description="Variable type: int, string, float")


class CampaignVariablesResponse(BaseModel):
    """Response containing campaign variables from globals.xml"""
    integers: Dict[str, int] = Field(default_factory=dict, description="Integer variables")
    strings: Dict[str, str] = Field(default_factory=dict, description="String variables")
    floats: Dict[str, float] = Field(default_factory=dict, description="Float variables")
    total_count: int = Field(0, description="Total variable count")


class ModuleVariablesResponse(BaseModel):
    """Response containing module variables from VarTable in module.ifo"""
    integers: Dict[str, int] = Field(default_factory=dict, description="Integer variables")
    strings: Dict[str, str] = Field(default_factory=dict, description="String variables")
    floats: Dict[str, float] = Field(default_factory=dict, description="Float variables")
    total_count: int = Field(0, description="Total variable count")


class ModuleVariableUpdate(BaseModel):
    """Request to update module-specific variables in VarTable"""
    variable_name: str = Field(..., description="Module variable name")
    value: Union[int, str, float] = Field(..., description="New value")
    variable_type: str = Field("int", description="Variable type: int, string, float")
    module_id: Optional[str] = Field(None, description="Target module ID (None for current module)")


class CampaignSettingsResponse(BaseModel):
    """Campaign settings from campaign.cam file"""
    campaign_file_path: str = Field(..., description="Path to campaign.cam file")
    guid: str = Field(..., description="Campaign GUID")
    display_name: str = Field("", description="Campaign display name")
    description: str = Field("", description="Campaign description")
    level_cap: int = Field(20, description="Maximum character level")
    xp_cap: int = Field(0, description="Maximum experience points (0 = unlimited)")
    companion_xp_weight: float = Field(0.0, description="Companion XP weight (party size penalty)")
    henchman_xp_weight: float = Field(0.0, description="Henchman XP weight")
    attack_neutrals: int = Field(0, description="Can attack neutral creatures (0=no, 1=yes)")
    auto_xp_award: int = Field(1, description="Auto award XP (0=no, 1=yes)")
    journal_sync: int = Field(1, description="Sync journal entries (0=no, 1=yes)")
    no_char_changing: int = Field(0, description="Lock character changes (0=no, 1=yes)")
    use_personal_reputation: int = Field(0, description="Use personal reputation (0=no, 1=yes)")
    start_module: str = Field("", description="Starting module name")
    module_names: List[str] = Field(default_factory=list, description="List of modules in campaign")


class QuestProgressData(BaseModel):
    """Quest progress information with enriched definitions from module.jrl"""
    variable: str = Field(..., description="Quest variable name in globals.xml")
    category: str = Field(..., description="Quest category from module.jrl")
    name: str = Field(..., description="Quest name/description from module.jrl")
    description: Optional[str] = Field(None, description="Human-readable description for parsed variables")
    current_stage: int = Field(..., description="Current quest stage from globals.xml")
    is_completed: bool = Field(False, description="Whether quest is completed")
    xp: int = Field(0, description="XP reward for quest")
    source: str = Field("unknown", description="Source of quest definition (campaign, module, parsed, unknown)")
    type_hint: Optional[str] = Field(None, description="Variable type hint (boolean, progression, state)")


class QuestProgressResponse(BaseModel):
    """Response containing all quest progress data"""
    quests: List[QuestProgressData] = Field(default_factory=list, description="List of quests with progress")
    total_count: int = Field(0, description="Total number of quests")


class PlotVariableData(BaseModel):
    """Plot variable from globals.xml with quest definition status"""
    name: str = Field(..., description="Variable name")
    display_name: Optional[str] = Field(None, description="Human-readable display name from parsing")
    description: Optional[str] = Field(None, description="Human-readable description for parsed variables")
    value: Union[int, str, float] = Field(..., description="Variable value")
    type: str = Field(..., description="Variable type: int, string, float")
    has_definition: bool = Field(False, description="Whether variable has quest definition")
    category: Optional[str] = Field(None, description="Quest category if has definition")
    quest_text: Optional[str] = Field(None, description="Quest text if has definition")
    type_hint: Optional[str] = Field(None, description="Variable type hint (boolean, progression, state)")


class PlotVariablesResponse(BaseModel):
    """Response containing all plot variables categorized by quest definition status"""
    quest_variables: List[PlotVariableData] = Field(
        default_factory=list,
        description="Variables with quest definitions"
    )
    unknown_variables: List[PlotVariableData] = Field(
        default_factory=list,
        description="Quest-like variables without definitions"
    )
    total_count: int = Field(0, description="Total plot variables found")


class UpdateCampaignSettingsRequest(BaseModel):
    """Request to update campaign settings"""
    level_cap: Optional[int] = Field(None, description="Maximum character level", ge=1, le=40)
    xp_cap: Optional[int] = Field(None, description="Maximum experience points", ge=0)
    companion_xp_weight: Optional[float] = Field(None, description="Companion XP weight", ge=0.0, le=1.0)
    henchman_xp_weight: Optional[float] = Field(None, description="Henchman XP weight", ge=0.0, le=1.0)
    attack_neutrals: Optional[int] = Field(None, description="Can attack neutrals", ge=0, le=1)
    auto_xp_award: Optional[int] = Field(None, description="Auto award XP", ge=0, le=1)
    journal_sync: Optional[int] = Field(None, description="Sync journal", ge=0, le=1)
    no_char_changing: Optional[int] = Field(None, description="Lock character changes", ge=0, le=1)
    use_personal_reputation: Optional[int] = Field(None, description="Use personal reputation", ge=0, le=1)


class KnownQuestValue(BaseModel):
    """Known value for a quest variable with description"""
    value: int = Field(..., description="Variable value")
    description: str = Field(..., description="Description of what this value means")
    is_completed: bool = Field(False, description="Whether this value represents quest completion")


class QuestInfoData(BaseModel):
    """Quest definition information from module.jrl or dialogue mapping"""
    category: str = Field(..., description="Quest category tag")
    category_name: str = Field(..., description="Human-readable category name")
    entry_id: int = Field(0, description="Journal entry ID")
    quest_name: str = Field(..., description="Quest display name")
    current_stage_text: str = Field("", description="Current stage description")
    xp: int = Field(0, description="XP reward")


class EnrichedQuestData(BaseModel):
    """Enriched quest data with dialogue mapping information"""
    variable_name: str = Field(..., description="Variable name in globals.xml")
    current_value: int = Field(..., description="Current variable value")
    variable_type: str = Field("int", description="Variable type")
    quest_info: Optional[QuestInfoData] = Field(None, description="Quest definition info")
    known_values: List[KnownQuestValue] = Field(default_factory=list, description="Known values with descriptions")
    confidence: str = Field("low", description="Mapping confidence: high, medium, low")
    source: str = Field("unknown", description="Data source: campaign, module")
    is_completed: bool = Field(False, description="Whether quest is completed")
    is_active: bool = Field(False, description="Whether quest is active")


class UnmappedVariableData(BaseModel):
    """Quest-like variable without quest mapping"""
    variable_name: str = Field(..., description="Variable name")
    display_name: str = Field(..., description="Human-readable display name")
    current_value: Union[int, str, float] = Field(..., description="Current value")
    variable_type: str = Field("int", description="Variable type")
    category: str = Field("General", description="Parsed category")


class QuestStats(BaseModel):
    """Quest statistics summary"""
    total: int = Field(0, description="Total mapped quests")
    completed: int = Field(0, description="Completed quests")
    active: int = Field(0, description="Active quests")
    unmapped: int = Field(0, description="Unmapped variables")


class DialogueCacheInfo(BaseModel):
    """Dialogue mapping cache information"""
    cached: bool = Field(False, description="Whether cache exists")
    version: Optional[str] = Field(None, description="Cache version")
    generated_at: Optional[str] = Field(None, description="Cache generation timestamp")
    dialogue_count: int = Field(0, description="Number of dialogue files parsed")
    mapping_count: int = Field(0, description="Number of mappings found")
    campaign_name: str = Field("", description="Campaign name")


class EnrichedQuestsResponse(BaseModel):
    """Response containing enriched quest data with dialogue mappings"""
    quests: List[EnrichedQuestData] = Field(default_factory=list, description="Enriched quest data")
    unmapped_variables: List[UnmappedVariableData] = Field(default_factory=list, description="Variables without mappings")
    stats: QuestStats = Field(default_factory=QuestStats, description="Quest statistics")
    cache_info: DialogueCacheInfo = Field(default_factory=DialogueCacheInfo, description="Cache information")