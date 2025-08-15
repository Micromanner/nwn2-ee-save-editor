"""
Content router - Campaign, module, quest, and custom content information
"""

import logging
from typing import Dict, Any, List
from fastapi import APIRouter, Depends, HTTPException, status
from fastapi_routers.dependencies import (
    get_character_manager,
    CharacterManagerDep
)
from fastapi_models import (
    CustomContentSummary,
    CustomContentItem,
    CampaignInfoResponse
)

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/api", tags=["content"])


@router.get("/characters/{character_id}/campaign-info/", response_model=CampaignInfoResponse)
def get_campaign_info(
    character_id: int,
    manager: CharacterManagerDep = Depends(get_character_manager)
):
    """Get campaign, module, and quest information for a character"""
    
    try:
        content_manager = manager.get_manager('content')
        
        # Use content manager methods - no duplicated logic
        campaign_info = content_manager.get_campaign_info()
        custom_content = content_manager.get_custom_content_summary()
        
        # Build response using manager data
        result = campaign_info.copy()
        result['custom_content'] = CustomContentSummary(**custom_content)
        
        return CampaignInfoResponse(**result)
        
    except Exception as e:
        logger.error(f"Failed to get campaign info for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get campaign info: {str(e)}"
        )


@router.get("/characters/{character_id}/custom-content/", response_model=CustomContentSummary)
def get_custom_content(
    character_id: int,
    manager: CharacterManagerDep = Depends(get_character_manager)
):
    """Get detailed custom content information for a character"""
    
    try:
        content_manager = manager.get_manager('content')
        
        # Use content manager method - no duplicated logic
        custom_content_data = content_manager.get_custom_content_summary()
        return CustomContentSummary(**custom_content_data)
        
    except Exception as e:
        logger.error(f"Failed to get custom content for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get custom content: {str(e)}"
        )


@router.get("/characters/{character_id}/custom-content/{content_type}/", response_model=List[CustomContentItem])
def get_custom_content_by_type(
    character_id: int,
    content_type: str,
    manager: CharacterManagerDep = Depends(get_character_manager)
):
    """Get custom content filtered by type (feat, spell, class)"""
    
    try:
        content_manager = manager.get_manager('content')
        
        # Use content manager method - no duplicated logic
        content_items = content_manager.get_custom_content_by_type(content_type)
        return [CustomContentItem(**item) for item in content_items]
        
    except ValueError as e:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=str(e)
        )
    except Exception as e:
        logger.error(f"Failed to get custom {content_type} content for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get custom content: {str(e)}"
        )


@router.post("/characters/{character_id}/refresh-content/", response_model=Dict[str, Any])
def refresh_custom_content(
    character_id: int,
    manager: CharacterManagerDep = Depends(get_character_manager)
):
    """Refresh custom content detection for a character"""
    
    try:
        content_manager = manager.get_manager('content')
        
        # Use content manager method - no duplicated logic
        content_manager.refresh_custom_content()
        
        custom_content_data = content_manager.get_custom_content_summary()
        return {
            'success': True,
            'custom_content': CustomContentSummary(**custom_content_data)
        }
        
    except Exception as e:
        logger.error(f"Failed to refresh custom content for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to refresh custom content: {str(e)}"
        )