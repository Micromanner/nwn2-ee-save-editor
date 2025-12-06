"""
Content router - Campaign, module, quest, and custom content information
"""

from typing import Dict, Any, List
from fastapi import APIRouter, Depends, HTTPException, status
from loguru import logger
from fastapi_routers.dependencies import (
    get_character_manager,
    CharacterManagerDep
)
# from fastapi_models import (...) - moved to lazy loading
router = APIRouter(tags=["content"])


@router.get("/characters/{character_id}/campaign-info")
def get_campaign_info(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get campaign, module, and quest information for a character"""
    from fastapi_models import CustomContentSummary, CampaignInfoResponse
    
    try:
        content_manager = manager.get_manager('content')

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


@router.get("/characters/{character_id}/custom-content")
def get_custom_content(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get detailed custom content information for a character"""
    from fastapi_models import CustomContentSummary
    
    try:
        content_manager = manager.get_manager('content')

        custom_content_data = content_manager.get_custom_content_summary()
        return CustomContentSummary(**custom_content_data)
        
    except Exception as e:
        logger.error(f"Failed to get custom content for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get custom content: {str(e)}"
        )


@router.get("/characters/{character_id}/custom-content/{content_type}")
def get_custom_content_by_type(
    character_id: int,
    content_type: str,
    manager: CharacterManagerDep
):
    """Get custom content filtered by type (feat, spell, class)"""
    from fastapi_models import CustomContentItem
    
    try:
        content_manager = manager.get_manager('content')

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


@router.post("/characters/{character_id}/refresh-content")
def refresh_custom_content(
    character_id: int,
    manager: CharacterManagerDep
):
    """Refresh custom content detection for a character"""
    from fastapi_models import CustomContentSummary

    try:
        content_manager = manager.get_manager('content')

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


@router.get("/characters/{character_id}/companion-influence")
def get_companion_influence(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get companion influence data from globals.xml"""
    from fastapi_models.content_models import CompanionInfluenceResponse, CompanionInfluenceData

    try:
        game_state_manager = manager.get_manager('game_state')
        influence_data = game_state_manager.get_companion_influence()

        companions = {
            comp_id: CompanionInfluenceData(**comp_data)
            for comp_id, comp_data in influence_data.items()
        }

        return CompanionInfluenceResponse(companions=companions)

    except Exception as e:
        logger.error(f"Failed to get companion influence for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get companion influence: {str(e)}"
        )


@router.post("/characters/{character_id}/companion-influence/update")
def update_companion_influence(
    character_id: int,
    manager: CharacterManagerDep,
    request: Dict[str, Any]
):
    """Update companion influence value"""
    from fastapi_models.content_models import UpdateCompanionInfluenceRequest

    try:
        update_request = UpdateCompanionInfluenceRequest(**request)
        game_state_manager = manager.get_manager('game_state')

        success = game_state_manager.update_companion_influence(
            update_request.companion_id,
            update_request.new_influence
        )

        if not success:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Companion not found: {update_request.companion_id}"
            )

        return {
            'success': True,
            'companion_id': update_request.companion_id,
            'new_influence': update_request.new_influence
        }

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to update companion influence for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update companion influence: {str(e)}"
        )


@router.get("/characters/{character_id}/quests/details")
def get_quest_details(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get detailed quest information from globals.xml"""
    from fastapi_models.content_models import QuestDetailsResponse

    try:
        game_state_manager = manager.get_manager('game_state')
        quest_details = game_state_manager.get_quest_details()

        return QuestDetailsResponse(**quest_details)

    except Exception as e:
        logger.error(f"Failed to get quest details for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get quest details: {str(e)}"
        )


@router.post("/characters/{character_id}/quests/variable/update")
def update_quest_variable(
    character_id: int,
    manager: CharacterManagerDep,
    request: Dict[str, Any]
):
    """Update a single quest variable"""
    from fastapi_models.content_models import UpdateQuestVariableRequest

    try:
        update_request = UpdateQuestVariableRequest(**request)
        game_state_manager = manager.get_manager('game_state')

        success = game_state_manager.update_quest_variable(
            update_request.variable_name,
            update_request.value,
            update_request.variable_type
        )

        if not success:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Failed to update variable: {update_request.variable_name}"
            )

        return {
            'success': True,
            'variable_name': update_request.variable_name,
            'value': update_request.value,
            'variable_type': update_request.variable_type
        }

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to update quest variable for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update quest variable: {str(e)}"
        )


@router.post("/characters/{character_id}/quests/batch-update")
def batch_update_quests(
    character_id: int,
    manager: CharacterManagerDep,
    request: Dict[str, Any]
):
    """Update multiple quest variables at once"""
    from fastapi_models.content_models import BatchUpdateQuestRequest

    try:
        batch_request = BatchUpdateQuestRequest(**request)
        game_state_manager = manager.get_manager('game_state')

        # Convert Pydantic models to dicts for manager
        updates_list = [
            {
                'variable_name': update.variable_name,
                'value': update.value,
                'variable_type': update.variable_type
            }
            for update in batch_request.updates
        ]

        success = game_state_manager.batch_update_quests(updates_list)

        if not success:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Some variables failed to update"
            )

        return {
            'success': True,
            'total_updates': len(batch_request.updates)
        }

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to batch update quests for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to batch update quests: {str(e)}"
        )


@router.get("/characters/{character_id}/campaign/variables")
def get_campaign_variables(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get all campaign variables from globals.xml"""
    from fastapi_models.content_models import CampaignVariablesResponse

    try:
        game_state_manager = manager.get_manager('game_state')
        variables = game_state_manager.get_all_campaign_variables()

        return CampaignVariablesResponse(**variables)

    except Exception as e:
        logger.error(f"Failed to get campaign variables for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get campaign variables: {str(e)}"
        )


@router.post("/characters/{character_id}/campaign/variable/update")
def update_campaign_variable(
    character_id: int,
    manager: CharacterManagerDep,
    request: Dict[str, Any]
):
    """Update a single campaign variable"""
    from fastapi_models.content_models import CampaignVariableUpdate

    try:
        update_request = CampaignVariableUpdate(**request)
        game_state_manager = manager.get_manager('game_state')

        success = game_state_manager.update_campaign_variable(
            update_request.variable_name,
            update_request.value,
            update_request.variable_type
        )

        if not success:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Failed to update variable: {update_request.variable_name}"
            )

        return {
            'success': True,
            'variable_name': update_request.variable_name,
            'value': update_request.value,
            'variable_type': update_request.variable_type
        }

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to update campaign variable for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update campaign variable: {str(e)}"
        )


@router.get("/characters/{character_id}/campaign/settings")
def get_campaign_settings(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get campaign settings from campaign.cam file"""
    from fastapi_models.content_models import CampaignSettingsResponse

    try:
        content_manager = manager.get_manager('content')
        settings = content_manager.get_campaign_settings()

        if not settings:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail="Campaign settings file not found. Make sure you have loaded a save file."
            )

        return CampaignSettingsResponse(**settings)

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to get campaign settings for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get campaign settings: {str(e)}"
        )


@router.post("/characters/{character_id}/campaign/settings")
def update_campaign_settings(
    character_id: int,
    manager: CharacterManagerDep,
    request: Dict[str, Any]
):
    """
    Update campaign settings in campaign.cam file

    WARNING: This affects ALL saves using this campaign!
    Changes are written to the game installation directory.
    """
    from fastapi_models.content_models import UpdateCampaignSettingsRequest

    try:
        update_request = UpdateCampaignSettingsRequest(**request)
        content_manager = manager.get_manager('content')

        # Convert Pydantic model to dict, excluding None values
        settings_dict = update_request.model_dump(exclude_none=True)

        if not settings_dict:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="No settings provided to update"
            )

        success = content_manager.update_campaign_settings(settings_dict)

        if not success:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Failed to update campaign settings"
            )

        return {
            'success': True,
            'updated_fields': list(settings_dict.keys()),
            'warning': 'These changes affect ALL saves using this campaign'
        }

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to update campaign settings for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update campaign settings: {str(e)}"
        )


@router.get("/characters/{character_id}/module/variables")
def get_module_variables(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get all module variables from VarTable in module.ifo"""
    from fastapi_models.content_models import ModuleVariablesResponse

    try:
        content_manager = manager.get_manager('content')
        variables = content_manager.get_module_variables()

        return ModuleVariablesResponse(**variables)

    except Exception as e:
        logger.error(f"Failed to get module variables for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get module variables: {str(e)}"
        )


@router.post("/characters/{character_id}/module/variable/update")
def update_module_variable(
    character_id: int,
    manager: CharacterManagerDep,
    request: Dict[str, Any]
):
    """
    Update a single module variable in VarTable.

    If module_id is provided, updates the variable in that module's .z file.
    Otherwise, updates the current module's standalone module.ifo file.
    """
    from fastapi_models.content_models import ModuleVariableUpdate

    try:
        update_request = ModuleVariableUpdate(**request)
        content_manager = manager.get_manager('content')

        success = content_manager.update_module_variable(
            update_request.variable_name,
            update_request.value,
            update_request.variable_type,
            update_request.module_id
        )

        if not success:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Failed to update variable: {update_request.variable_name}"
            )

        return {
            'success': True,
            'variable_name': update_request.variable_name,
            'value': update_request.value,
            'variable_type': update_request.variable_type,
            'module_id': update_request.module_id
        }

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to update module variable for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update module variable: {str(e)}"
        )


@router.get("/characters/{character_id}/modules")
def get_all_modules(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get list of all available modules from save"""
    try:
        content_manager = manager.get_manager('content')
        modules = content_manager.get_all_available_modules()

        return {
            'modules': modules,
            'current_module': content_manager.current_module_name
        }

    except Exception as e:
        logger.error(f"Failed to get modules for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get modules: {str(e)}"
        )


@router.get("/characters/{character_id}/modules/{module_id}")
def get_module_by_id(
    character_id: int,
    module_id: str,
    manager: CharacterManagerDep
):
    """Get specific module info and variables"""
    try:
        content_manager = manager.get_manager('content')
        module_data = content_manager.get_module_by_id(module_id)

        if not module_data:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Module not found: {module_id}"
            )

        return module_data

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to get module {module_id} for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get module: {str(e)}"
        )


@router.get("/characters/{character_id}/quests/progress")
def get_quest_progress(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get quest progress with enriched data from module.jrl"""
    from fastapi_models.content_models import QuestProgressResponse, QuestProgressData

    try:
        game_state_manager = manager.get_manager('game_state')
        quest_progress = game_state_manager.get_quest_progress()

        quests = [QuestProgressData(**quest) for quest in quest_progress]

        return QuestProgressResponse(
            quests=quests,
            total_count=len(quests)
        )

    except Exception as e:
        logger.error(f"Failed to get quest progress for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get quest progress: {str(e)}"
        )


@router.get("/characters/{character_id}/quests/plot-variables")
def get_plot_variables(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get all plot variables categorized by quest definition status"""
    from fastapi_models.content_models import PlotVariablesResponse, PlotVariableData

    try:
        game_state_manager = manager.get_manager('game_state')
        plot_vars = game_state_manager.get_all_plot_variables()

        quest_variables = [PlotVariableData(**var) for var in plot_vars['quest_variables']]
        unknown_variables = [PlotVariableData(**var) for var in plot_vars['unknown_variables']]

        return PlotVariablesResponse(
            quest_variables=quest_variables,
            unknown_variables=unknown_variables,
            total_count=plot_vars['total_count']
        )

    except Exception as e:
        logger.error(f"Failed to get plot variables for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get plot variables: {str(e)}"
        )


@router.get("/characters/{character_id}/quests/enriched")
def get_enriched_quests(
    character_id: int,
    manager: CharacterManagerDep
):
    """
    Get enriched quest data with dialogue-based variable mappings

    Returns quests with high-confidence variable-to-quest mappings
    from dialogue file analysis, plus unmapped quest-like variables.
    """
    from fastapi_models.content_models import (
        EnrichedQuestsResponse,
        EnrichedQuestData,
        UnmappedVariableData,
        QuestStats,
        DialogueCacheInfo,
        QuestInfoData,
        KnownQuestValue,
    )

    try:
        game_state_manager = manager.get_manager('game_state')
        enriched_data = game_state_manager.get_enriched_quests()

        quests = []
        for q in enriched_data.get('quests', []):
            quest_info = None
            if q.get('quest_info'):
                quest_info = QuestInfoData(**q['quest_info'])

            known_values = [
                KnownQuestValue(**kv)
                for kv in q.get('known_values', [])
            ]

            quests.append(EnrichedQuestData(
                variable_name=q['variable_name'],
                current_value=q['current_value'],
                variable_type=q.get('variable_type', 'int'),
                quest_info=quest_info,
                known_values=known_values,
                confidence=q.get('confidence', 'low'),
                source=q.get('source', 'unknown'),
                is_completed=q.get('is_completed', False),
                is_active=q.get('is_active', False),
            ))

        unmapped = [
            UnmappedVariableData(**uv)
            for uv in enriched_data.get('unmapped_variables', [])
        ]

        stats_data = enriched_data.get('stats', {})
        stats = QuestStats(
            total=stats_data.get('total', 0),
            completed=stats_data.get('completed', 0),
            active=stats_data.get('active', 0),
            unmapped=stats_data.get('unmapped', 0),
        )

        cache_data = enriched_data.get('cache_info', {})
        cache_info = DialogueCacheInfo(
            cached=cache_data.get('cached', False),
            version=cache_data.get('version'),
            generated_at=cache_data.get('generated_at'),
            dialogue_count=cache_data.get('dialogue_count', 0),
            mapping_count=cache_data.get('mapping_count', 0),
            campaign_name=cache_data.get('campaign_name', ''),
        )

        return EnrichedQuestsResponse(
            quests=quests,
            unmapped_variables=unmapped,
            stats=stats,
            cache_info=cache_info,
        )

    except Exception as e:
        logger.error(f"Failed to get enriched quests for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get enriched quests: {str(e)}"
        )