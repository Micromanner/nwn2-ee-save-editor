"""Content router - Campaign, module, quest, and custom content information."""

from typing import Dict, Any, List, Optional
from fastapi import APIRouter, HTTPException, status, Body
from loguru import logger

from fastapi_routers.dependencies import CharacterManagerDep
from fastapi_models import (
    CustomContentSummary, CampaignInfoResponse, CustomContentItem,
    CompanionInfluenceResponse, CompanionInfluenceData, UpdateCompanionInfluenceRequest,
    QuestDetailsResponse, UpdateQuestVariableRequest, BatchUpdateQuestRequest,
    CampaignVariablesResponse, CampaignVariableUpdate, CampaignSettingsResponse, UpdateCampaignSettingsRequest,
    CampaignBackupsResponse, CampaignBackupInfo, RestoreCampaignRequest,
    ModuleVariablesResponse, ModuleVariableUpdate,
    QuestProgressResponse, QuestProgressData, PlotVariablesResponse, PlotVariableData,
    EnrichedQuestsResponse, EnrichedQuestData, UnmappedVariableData, QuestStats,
    DialogueCacheInfo, QuestInfoData, KnownQuestValue
)

router = APIRouter(tags=["content"])


@router.get("/characters/{character_id}/campaign-info")
def get_campaign_info(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get campaign, module, and quest information."""
    try:
        content_manager = manager.get_manager('content')

        campaign_info = content_manager.get_campaign_info()
        custom_content = content_manager.get_custom_content_summary()
        
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
    """Get detailed custom content information."""
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
    """Get custom content filtered by type."""
    try:
        content_manager = manager.get_manager('content')
        content_items = content_manager.get_custom_content_by_type(content_type)
        return [CustomContentItem(**item) for item in content_items]
        
    except ValueError as e:
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail=str(e))
    except Exception as e:
        logger.error(f"Failed to get custom {content_type} content: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get custom content: {str(e)}"
        )


@router.post("/characters/{character_id}/refresh-content")
def refresh_custom_content(
    character_id: int,
    manager: CharacterManagerDep
):
    """Refresh custom content detection for a character."""
    try:
        content_manager = manager.get_manager('content')
        content_manager.refresh_custom_content()

        custom_content_data = content_manager.get_custom_content_summary()
        return {
            'success': True,
            'custom_content': CustomContentSummary(**custom_content_data)
        }

    except Exception as e:
        logger.error(f"Failed to refresh custom content: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to refresh custom content: {str(e)}"
        )


@router.get("/characters/{character_id}/companion-influence")
def get_companion_influence(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get companion influence data from globals.xml."""
    try:
        game_state_manager = manager.get_manager('game_state')
        influence_data = game_state_manager.get_companion_influence()

        companions = {
            comp_id: CompanionInfluenceData(**comp_data)
            for comp_id, comp_data in influence_data.items()
        }

        return CompanionInfluenceResponse(companions=companions)

    except Exception as e:
        logger.error(f"Failed to get companion influence: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get companion influence: {str(e)}"
        )


@router.post("/characters/{character_id}/companion-influence/update")
def update_companion_influence(
    character_id: int,
    manager: CharacterManagerDep,
    request: UpdateCompanionInfluenceRequest = Body(...)
):
    """Update companion influence value."""
    try:
        game_state_manager = manager.get_manager('game_state')

        success = game_state_manager.update_companion_influence(
            request.companion_id,
            request.new_influence
        )

        if not success:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Companion not found: {request.companion_id}"
            )

        return {
            'success': True,
            'companion_id': request.companion_id,
            'new_influence': request.new_influence
        }

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to update companion influence: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update companion influence: {str(e)}"
        )


@router.get("/characters/{character_id}/quests/details")
def get_quest_details(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get detailed quest information from globals.xml."""
    try:
        game_state_manager = manager.get_manager('game_state')
        quest_details = game_state_manager.get_quest_details()

        return QuestDetailsResponse(**quest_details)

    except Exception as e:
        logger.error(f"Failed to get quest details: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get quest details: {str(e)}"
        )


@router.post("/characters/{character_id}/quests/variable/update")
def update_quest_variable(
    character_id: int,
    manager: CharacterManagerDep,
    request: UpdateQuestVariableRequest = Body(...)
):
    """Update a single quest variable."""
    try:
        game_state_manager = manager.get_manager('game_state')

        success = game_state_manager.update_quest_variable(
            request.variable_name,
            request.value,
            request.variable_type
        )

        if not success:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Failed to update variable: {request.variable_name}"
            )

        return {
            'success': True,
            'variable_name': request.variable_name,
            'value': request.value,
            'variable_type': request.variable_type
        }

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to update quest variable: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update quest variable: {str(e)}"
        )


@router.post("/characters/{character_id}/quests/batch-update")
def batch_update_quests(
    character_id: int,
    manager: CharacterManagerDep,
    request: BatchUpdateQuestRequest = Body(...)
):
    """Update multiple quest variables at once."""
    try:
        game_state_manager = manager.get_manager('game_state')

        updates_list = [
            {
                'variable_name': update.variable_name,
                'value': update.value,
                'variable_type': update.variable_type
            }
            for update in request.updates
        ]

        success = game_state_manager.batch_update_quests(updates_list)

        if not success:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Some variables failed to update"
            )

        return {
            'success': True,
            'total_updates': len(request.updates)
        }

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to batch update quests: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to batch update quests: {str(e)}"
        )


@router.get("/characters/{character_id}/campaign/variables")
def get_campaign_variables(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get all campaign variables from globals.xml."""
    try:
        game_state_manager = manager.get_manager('game_state')
        variables = game_state_manager.get_all_campaign_variables()

        return CampaignVariablesResponse(**variables)

    except Exception as e:
        logger.error(f"Failed to get campaign variables: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get campaign variables: {str(e)}"
        )


@router.post("/characters/{character_id}/campaign/variable/update")
def update_campaign_variable(
    character_id: int,
    manager: CharacterManagerDep,
    request: CampaignVariableUpdate = Body(...)
):
    """Update a single campaign variable."""
    try:
        game_state_manager = manager.get_manager('game_state')

        success = game_state_manager.update_campaign_variable(
            request.variable_name,
            request.value,
            request.variable_type
        )

        if not success:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Failed to update variable: {request.variable_name}"
            )

        return {
            'success': True,
            'variable_name': request.variable_name,
            'value': request.value,
            'variable_type': request.variable_type
        }

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to update campaign variable: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update campaign variable: {str(e)}"
        )


@router.get("/characters/{character_id}/campaign/settings")
def get_campaign_settings(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get campaign settings from campaign.cam file."""
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
        logger.error(f"Failed to get campaign settings: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get campaign settings: {str(e)}"
        )


@router.post("/characters/{character_id}/campaign/settings")
def update_campaign_settings(
    character_id: int,
    manager: CharacterManagerDep,
    request: UpdateCampaignSettingsRequest = Body(...)
):
    """Update campaign settings in campaign.cam file (affects ALL saves)."""
    try:
        content_manager = manager.get_manager('content')
        settings_dict = request.model_dump(exclude_none=True)

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
        logger.error(f"Failed to update campaign settings: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update campaign settings: {str(e)}"
        )


@router.get("/characters/{character_id}/campaign/backups")
def list_campaign_backups(
    character_id: int,
    manager: CharacterManagerDep
):
    """List all available campaign.cam backups."""
    try:
        content_manager = manager.get_manager('content')
        backups = content_manager.list_campaign_backups()

        campaign_name = None
        campaign_guid = None
        settings = content_manager.get_campaign_settings()
        if settings:
            campaign_name = settings.get('campaign_name')
            campaign_guid = settings.get('campaign_guid')

        return CampaignBackupsResponse(
            backups=[CampaignBackupInfo(**b) for b in backups],
            campaign_name=campaign_name,
            campaign_guid=campaign_guid
        )

    except Exception as e:
        logger.error(f"Failed to list campaign backups: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to list campaign backups: {str(e)}"
        )


@router.post("/characters/{character_id}/campaign/restore")
def restore_campaign_from_backup(
    character_id: int,
    manager: CharacterManagerDep,
    request: RestoreCampaignRequest = Body(...)
):
    """Restore campaign.cam from a backup file."""
    try:
        content_manager = manager.get_manager('content')
        success = content_manager.restore_campaign_from_backup_file(request.backup_path)

        if not success:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Failed to restore campaign from backup"
            )

        return {
            'success': True,
            'restored_from': request.backup_path
        }

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to restore campaign: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to restore campaign: {str(e)}"
        )


@router.get("/characters/{character_id}/module/variables")
def get_module_variables(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get all module variables."""
    try:
        content_manager = manager.get_manager('content')
        variables = content_manager.get_module_variables()

        return ModuleVariablesResponse(**variables)

    except Exception as e:
        logger.error(f"Failed to get module variables: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get module variables: {str(e)}"
        )


@router.post("/characters/{character_id}/module/variable/update")
def update_module_variable(
    character_id: int,
    manager: CharacterManagerDep,
    request: ModuleVariableUpdate = Body(...)
):
    """Update a single module variable in VarTable."""
    try:
        content_manager = manager.get_manager('content')

        success = content_manager.update_module_variable(
            request.variable_name,
            request.value,
            request.variable_type,
            request.module_id
        )

        if not success:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Failed to update variable: {request.variable_name}"
            )

        return {
            'success': True,
            'variable_name': request.variable_name,
            'value': request.value,
            'variable_type': request.variable_type,
            'module_id': request.module_id
        }

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to update module variable: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update module variable: {str(e)}"
        )


@router.get("/characters/{character_id}/modules")
def get_all_modules(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get list of all available modules from save."""
    try:
        content_manager = manager.get_manager('content')
        modules = content_manager.get_all_available_modules()

        return {
            'modules': modules,
            'current_module': content_manager.current_module_name
        }

    except Exception as e:
        logger.error(f"Failed to get modules: {e}")
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
    """Get specific module info and variables."""
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
        logger.error(f"Failed to get module {module_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get module: {str(e)}"
        )


@router.get("/characters/{character_id}/quests/progress")
def get_quest_progress(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get quest progress with enriched data from module.jrl."""
    try:
        game_state_manager = manager.get_manager('game_state')
        quest_progress = game_state_manager.get_quest_progress()

        quests = [QuestProgressData(**quest) for quest in quest_progress]

        return QuestProgressResponse(
            quests=quests,
            total_count=len(quests)
        )

    except Exception as e:
        logger.error(f"Failed to get quest progress: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get quest progress: {str(e)}"
        )


@router.get("/characters/{character_id}/quests/plot-variables")
def get_plot_variables(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get all plot variables categorized by quest definition status."""
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
        logger.error(f"Failed to get plot variables: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get plot variables: {str(e)}"
        )


@router.get("/characters/{character_id}/quests/enriched")
def get_enriched_quests(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get enriched quest data with dialogue-based variable mappings."""
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
        logger.error(f"Failed to get enriched quests: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get enriched quests: {str(e)}"
        )


@router.get("/characters/{character_id}/available-deities")
def get_available_deities(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get list of available deities from game data."""
    try:
        content_manager = manager.get_manager('content')
        deities = content_manager.get_available_deities()
        
        return {
            'deities': deities,
            'total': len(deities)
        }
        
    except Exception as e:
        logger.error(f"Failed to get available deities: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get available deities: {str(e)}"
        )


@router.get("/characters/{character_id}/deity")
def get_deity(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get character's current deity."""
    try:
        content_manager = manager.get_manager('content')
        return {
            'deity': content_manager.get_deity()
        }
        
    except Exception as e:
        logger.error(f"Failed to get deity for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get deity: {str(e)}"
        )


@router.post("/characters/{character_id}/deity")
def set_deity(
    character_id: int,
    manager: CharacterManagerDep,
    request: Dict[str, Any] = Body(...)
):
    """Set character's deity."""
    try:
        deity_name = request.get('deity', '')
        
        content_manager = manager.get_manager('content')
        success = content_manager.set_deity(deity_name)
        
        if not success:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Failed to set deity"
            )
        
        return {
            'success': True,
            'deity': deity_name
        }
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to set deity: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to set deity: {str(e)}"
        )


@router.get("/characters/{character_id}/biography")
def get_biography(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get character's biography."""
    try:
        content_manager = manager.get_manager('content')
        return {
            'biography': content_manager.get_biography()
        }
        
    except Exception as e:
        logger.error(f"Failed to get biography: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get biography: {str(e)}"
        )


@router.post("/characters/{character_id}/biography")
def set_biography(
    character_id: int,
    manager: CharacterManagerDep,
    request: Dict[str, Any] = Body(...)
):
    """Set character's biography."""
    try:
        biography_text = request.get('biography', '')
        
        content_manager = manager.get_manager('content')
        success = content_manager.set_biography(biography_text)
        
        if not success:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Failed to set biography"
            )
        
        return {
            'success': True,
            'biography_length': len(biography_text)
        }
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to set biography: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to set biography: {str(e)}"
        )