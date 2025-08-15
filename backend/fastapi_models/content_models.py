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






# All quest/variable/companion/validation/export functionality has been removed
# as ContentManager doesn't implement these features