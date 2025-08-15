"""Skills router - All skill-related endpoints
Handles skill points, ranks, modifiers, and skill checks
"""

import logging
from typing import Dict, Any
from fastapi import APIRouter, Depends, HTTPException, status

from fastapi_routers.dependencies import (
    get_character_manager, 
    get_character_session_dep,
    CharacterManagerDep,
    CharacterSessionDep
)
from fastapi_models import (
    SkillInfo,
    SkillPoints,
    SkillSummary,
    SkillUpdateRequest,
    SkillUpdateResponse,
    SkillChange,
    SkillBatchUpdateRequest,
    SkillBatchUpdateResponse,
    SkillResetRequest,
    SkillResetResponse,
    SkillCheckRequest,
    SkillCheckResponse,
    SkillPrerequisites,
    SkillBuild,
    SkillBuildImportRequest,
    SkillBuildImportResponse,
    AllSkillsResponse
)

logger = logging.getLogger(__name__)

router = APIRouter()


@router.get("/characters/{character_id}/skills/state/", response_model=SkillSummary)
def get_skills_state(
    character_id: int,
    manager: CharacterManagerDep = Depends(get_character_manager)
):
    """Get current skills state for the skills editor"""
    try:
        skill_manager = manager.get_manager('skill')
        
        # Use actual manager method that exists
        summary = skill_manager.get_skill_summary()
        
        return SkillSummary(**summary)
        
    except Exception as e:
        logger.error(f"Failed to get skills state for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get skills state: {str(e)}"
        )


@router.get("/characters/{character_id}/skills/all/", response_model=AllSkillsResponse)
def get_all_skills(
    character_id: int,
    manager: CharacterManagerDep = Depends(get_character_manager)
):
    """Get complete list of all skills with current ranks and modifiers"""
    try:
        skill_manager = manager.get_manager('skill')
        
        # Use actual manager method
        all_skills = skill_manager.get_all_skills()
        
        return AllSkillsResponse(skills=all_skills)
        
    except Exception as e:
        logger.error(f"Failed to get all skills for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get all skills: {str(e)}"
        )


@router.post("/characters/{character_id}/skills/update/", response_model=SkillUpdateResponse)
def update_skills(
    character_id: int,
    skill_update: SkillUpdateRequest,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Update skill ranks for character (in-memory only - use savegame endpoints to save)"""
    try:
        character, session = char_session
        manager = session.character_manager
        skill_manager = manager.get_manager('skill')
        
        # Use actual manager method that exists
        changes = []
        validation_errors = []
        
        for skill_id_str, new_rank in skill_update.skills.items():
            try:
                skill_id = int(skill_id_str)
            except (ValueError, TypeError):
                validation_errors.append(f"Invalid skill ID: {skill_id_str}")
                continue
                
            # Validate skill exists
            skill_data = skill_manager.game_data_loader.get_by_id('skills', skill_id)
            if not skill_data:
                validation_errors.append(f"Skill {skill_id} does not exist")
                continue
                
            # Validate rank is non-negative
            if new_rank < 0:
                validation_errors.append(f"Cannot set negative ranks ({new_rank}) for skill {skill_id}")
                continue
            
            old_rank = skill_manager.get_skill_ranks(skill_id)
            success = skill_manager.set_skill_rank(skill_id, new_rank)
            
            if success:
                skill_info = skill_manager.get_skill_info(skill_id)
                skill_name = skill_info['name'] if skill_info else f'Skill {skill_id}'
                changes.append(SkillChange(
                    skill_id=skill_id,
                    skill_name=skill_name,
                    old_rank=old_rank,
                    new_rank=new_rank,
                    points_spent=skill_manager.calculate_skill_cost(skill_id, new_rank),
                    new_total_modifier=skill_manager.calculate_skill_modifier(skill_id)
                ))
            else:
                validation_errors.append(f"Failed to set skill {skill_id} to {new_rank} ranks")
        
        # Get updated summary
        summary = skill_manager.get_skill_summary()
        points_remaining = skill_manager.get_unspent_points()
        
        return SkillUpdateResponse(
            changes=changes,
            skill_summary=SkillSummary(**summary),
            points_remaining=points_remaining,
            validation_errors=validation_errors,
            has_unsaved_changes=True
        )
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to update skills for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update skills: {str(e)}"
        )


@router.post("/characters/{character_id}/skills/batch/", response_model=SkillBatchUpdateResponse)
def batch_update_skills(
    character_id: int,
    batch_update: SkillBatchUpdateRequest,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Batch update multiple skills at once"""
    try:
        character, session = char_session
        manager = session.character_manager
        skill_manager = manager.get_manager('skill')
        
        # Use actual manager method that exists
        skills_dict = {int(k): v for k, v in batch_update.skills.items()}
        results = skill_manager.batch_set_skills(skills_dict)
        
        # Convert to expected format
        changes = []
        for result in results:
            if result['success']:
                skill_info = skill_manager.get_skill_info(result['skill_id'])
                skill_name = skill_info['name'] if skill_info else f"Skill {result['skill_id']}"
                changes.append(SkillChange(
                    skill_id=result['skill_id'],
                    skill_name=skill_name,
                    old_rank=0,  # batch_set_skills doesn't track old ranks
                    new_rank=result['ranks'],
                    points_spent=skill_manager.calculate_skill_cost(result['skill_id'], result['ranks']),
                    new_total_modifier=skill_manager.calculate_skill_modifier(result['skill_id'])
                ))
        
        summary = skill_manager.get_skill_summary()
        
        return SkillBatchUpdateResponse(
            results=changes,
            summary=SkillSummary(**summary),
            total_changes=len(changes),
            points_refunded=0,
            saved=False
        )
        
    except Exception as e:
        logger.error(f"Failed to batch update skills for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to batch update skills: {str(e)}"
        )


@router.post("/characters/{character_id}/skills/reset/", response_model=SkillResetResponse)
def reset_skills(
    character_id: int,
    reset_request: SkillResetRequest,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Reset all skills to 0 and refund points (ignores preserve_class_skills and refund_percentage - manager doesn't support these)"""
    try:
        character, session = char_session
        manager = session.character_manager
        skill_manager = manager.get_manager('skill')
        
        # Calculate points before reset
        spent_before = skill_manager._calculate_spent_skill_points()
        
        # Note: preserve_class_skills and refund_percentage are ignored since 
        # manager's reset_all_skills() method doesn't support these parameters
        skill_manager.reset_all_skills()
        
        # Calculate results
        available_after = skill_manager._calculate_available_skill_points()
        
        return SkillResetResponse(
            message="All skills reset successfully (preserved class skills and refund percentage not supported)",
            points_refunded=spent_before,
            available_points=available_after,
            skills_reset=len([s for s in skill_manager.gff.get('SkillList', []) if isinstance(s, dict)]),
            saved=False
        )
        
    except Exception as e:
        logger.error(f"Failed to reset skills for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to reset skills: {str(e)}"
        )


@router.post("/characters/{character_id}/skills/{skill_id}/check/", response_model=SkillCheckResponse)
def skill_check(
    character_id: int,
    skill_id: int,
    check_request: SkillCheckRequest,
    manager: CharacterManagerDep = Depends(get_character_manager)
):
    """Simulate a skill check (d20 + modifiers)"""
    try:
        skill_manager = manager.get_manager('skill')
        
        # Validate skill exists
        skill_data = skill_manager.game_data_loader.get_by_id('skills', skill_id)
        if not skill_data:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Skill {skill_id} does not exist"
            )
        
        # Handle take_10/take_20 since manager method doesn't support it
        if check_request.take_20:
            roll = 20
            critical = False
            fumble = False
        elif check_request.take_10:
            roll = 10
            critical = False
            fumble = False
        else:
            # Use actual manager method (only takes skill_id)
            result = skill_manager.roll_skill_check(skill_id)
            roll = result['roll']
            critical = result['critical']
            fumble = result['fumble']
        
        # Calculate modifier manually since we might not have used the manager method
        modifier = skill_manager.calculate_skill_modifier(skill_id)
        breakdown = skill_manager._get_modifier_breakdown(skill_id)
        
        # Get skill info
        from gamedata.dynamic_loader.field_mapping_utility import field_mapper
        skill_name = field_mapper.get_field_value(skill_data, 'label', f'Skill {skill_id}')
        
        # Add additional fields from request
        dc = check_request.dc
        circumstance = check_request.circumstance_bonus
        final_total = roll + modifier + circumstance
        
        return SkillCheckResponse(
            skill_id=skill_id,
            skill_name=skill_name,
            roll=roll,
            modifier=modifier,
            circumstance=circumstance,
            total=final_total,
            dc=dc,
            success=final_total >= dc,
            critical_success=critical,
            critical_failure=fumble,
            breakdown=breakdown,
            margin=final_total - dc
        )
        
    except ValueError:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Invalid skill ID"
        )
    except Exception as e:
        logger.error(f"Failed to perform skill check for character {character_id}, skill {skill_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to perform skill check: {str(e)}"
        )


@router.get("/characters/{character_id}/skills/{skill_id}/prerequisites/", response_model=SkillPrerequisites)
def skill_prerequisites(
    character_id: int,
    skill_id: int,
    manager: CharacterManagerDep = Depends(get_character_manager)
):
    """Get prerequisites for a specific skill"""
    try:
        skill_manager = manager.get_manager('skill')
        
        # Validate skill exists
        skill_data = skill_manager.game_data_loader.get_by_id('skills', skill_id)
        if not skill_data:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Skill {skill_id} does not exist"
            )
        
        # Use skill manager method - no duplicated logic
        prerequisites = skill_manager.get_skill_prerequisites(skill_id)
        
        return SkillPrerequisites(**prerequisites)
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to get skill prerequisites for character {character_id}, skill {skill_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get skill prerequisites: {str(e)}"
        )


@router.get("/characters/{character_id}/skills/export/", response_model=SkillBuild)
def export_build(
    character_id: int,
    manager: CharacterManagerDep = Depends(get_character_manager)
):
    """Export current skill build for saving/sharing"""
    try:
        skill_manager = manager.get_manager('skill')
        
        # Use skill manager method - no duplicated logic
        build = skill_manager.export_skill_build()
        
        return SkillBuild(**build)
        
    except Exception as e:
        logger.error(f"Failed to export skill build for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to export skill build: {str(e)}"
        )


@router.post("/characters/{character_id}/skills/import/", response_model=SkillBuildImportResponse)
def import_build(
    character_id: int,
    build_data: SkillBuildImportRequest,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Import a skill build (ignores replace_existing and should_validate - manager doesn't support these)"""
    try:
        character, session = char_session
        manager = session.character_manager
        skill_manager = manager.get_manager('skill')
        
        # Build the data in the format the manager expects
        build_dict = {
            'skills': build_data.skills
        }
        
        # Add optional fields if provided
        if build_data.character_level is not None:
            build_dict['character_level'] = build_data.character_level
        if build_data.total_skill_points is not None:
            build_dict['total_skill_points'] = build_data.total_skill_points
            
        # Note: replace_existing and should_validate are ignored since 
        # manager's import_skill_build() doesn't support these parameters
        
        success = skill_manager.import_skill_build(build_dict)
        
        if success:
            summary = skill_manager.get_skill_summary()
            imported_count = len([s for s in build_data.skills.keys()])
            
            return SkillBuildImportResponse(
                message="Skill build imported successfully (replace_existing and validation options not supported)",
                summary=SkillSummary(**summary),
                imported_count=imported_count,
                validation_errors=[],
                saved=False
            )
        else:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Failed to import skill build"
            )
            
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to import skill build for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to import skill build: {str(e)}"
        )