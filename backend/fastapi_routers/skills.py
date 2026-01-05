"""Skills router - All skill-related endpoints."""

from typing import Dict, Any, List
from fastapi import APIRouter, Depends, HTTPException, status, Body
from loguru import logger

from fastapi_routers.dependencies import (
    get_character_session,
    CharacterSessionDep
)
from fastapi_models import (
    SkillSummary, AllSkillsResponse, SkillUpdateRequest, SkillUpdateResponse,
    SkillChange, SkillBatchUpdateRequest, SkillBatchUpdateResponse,
    SkillResetResponse, SkillCheckRequest, SkillCheckResponse,
    SkillPrerequisites, SkillBuild, SkillBuildImportResponse
)

router = APIRouter()


@router.get("/characters/{character_id}/skills/state")
def get_skills_state(
    character_id: int,
    char_session: CharacterSessionDep
):
    """Get current skills state."""
    try:
        session = char_session
        manager = session.character_manager
        skill_manager = manager.get_manager('skill')
        
        summary = skill_manager.get_skill_summary()
        return SkillSummary(**summary)
        
    except Exception as e:
        logger.error(f"Failed to get skills state for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get skills state: {str(e)}"
        )


@router.get("/characters/{character_id}/skills/all")
def get_all_skills(
    character_id: int,
    char_session: CharacterSessionDep
):
    """Get complete list of all skills."""
    try:
        session = char_session
        manager = session.character_manager
        skill_manager = manager.get_manager('skill')
        
        all_skills = skill_manager.get_all_skills()
        return AllSkillsResponse(skills=all_skills)
        
    except Exception as e:
        logger.error(f"Failed to get all skills for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get all skills: {str(e)}"
        )


@router.post("/characters/{character_id}/skills/update")
def update_skills(
    character_id: int,
    char_session: CharacterSessionDep,
    request: SkillUpdateRequest = Body(...)
):
    """Update skill ranks for character."""
    try:
        session = char_session
        manager = session.character_manager
        skill_manager = manager.get_manager('skill')
        
        change_dicts, validation_errors = skill_manager.update_skills(request.skills)
        
        changes = [SkillChange(**c) for c in change_dicts]
        
        summary = skill_manager.get_skill_summary()
        points_remaining = skill_manager.get_unspent_points()
        
        return SkillUpdateResponse(
            changes=changes,
            skill_summary=SkillSummary(**summary),
            points_remaining=points_remaining,
            validation_errors=validation_errors,
            has_unsaved_changes=session.has_unsaved_changes()
        )
        
    except Exception as e:
        logger.error(f"Failed to update skills for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update skills: {str(e)}"
        )


@router.post("/characters/{character_id}/skills/batch")
def batch_update_skills(
    character_id: int,
    char_session: CharacterSessionDep,
    request: SkillBatchUpdateRequest = Body(...)
):
    """Batch update multiple skills."""
    try:
        session = char_session
        manager = session.character_manager
        skill_manager = manager.get_manager('skill')
        
        skills_dict = {int(k): v for k, v in request.skills.items()}
        results = skill_manager.batch_set_skills(skills_dict)
        
        changes = []
        for result in results:
            if result['success']:
                skill_info = skill_manager.get_skill_info(result['skill_id'])
                skill_name = skill_info['name'] if skill_info else f"Skill {result['skill_id']}"
                changes.append(SkillChange(
                    skill_id=result['skill_id'],
                    skill_name=skill_name,
                    old_rank=0,  
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


@router.post("/characters/{character_id}/skills/reset")
def reset_skills(
    character_id: int,
    char_session: CharacterSessionDep,
    request: Dict[str, Any] = Body(...)
):
    """Reset all skills to 0 and refund points."""
    try:
        session = char_session
        manager = session.character_manager
        skill_manager = manager.get_manager('skill')
        
        spent_before = skill_manager._calculate_spent_skill_points()
        skill_manager.reset_all_skills()
        available_after = skill_manager._calculate_available_skill_points()
        
        return SkillResetResponse(
            message="All skills reset successfully",
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


@router.post("/characters/{character_id}/skills/{skill_id}/check")
def skill_check(
    character_id: int,
    skill_id: int,
    char_session: CharacterSessionDep,
    request: SkillCheckRequest = Body(...)
):
    """Simulate a skill check (d20 + modifiers)."""
    try:
        session = char_session
        manager = session.character_manager
        skill_manager = manager.get_manager('skill')
        
        skill_data = skill_manager.game_rules_service.get_by_id('skills', skill_id)
        if not skill_data:
            raise HTTPException(status_code=400, detail=f"Skill {skill_id} does not exist")
        
        result = skill_manager.roll_skill_check(
            skill_id,
            take_10=request.take_10,
            take_20=request.take_20,
            circumstance_bonus=request.circumstance_bonus
        )
        
        dc = request.dc
        final_total = result['total']
        
        return SkillCheckResponse(
            skill_id=skill_id,
            skill_name=result['skill_name'],
            roll=result['roll'],
            modifier=result['modifier'],
            circumstance=result['circumstance'],
            total=final_total,
            dc=dc,
            success=final_total >= dc,
            critical_success=result.get('critical', False),
            critical_failure=result.get('fumble', False),
            breakdown=result.get('breakdown', {}),
            margin=final_total - dc
        )
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to perform skill check for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to perform skill check: {str(e)}"
        )


@router.get("/characters/{character_id}/skills/{skill_id}/prerequisites")
def skill_prerequisites(
    character_id: int,
    skill_id: int,
    char_session: CharacterSessionDep
):
    """Get prerequisites for a specific skill."""
    try:
        session = char_session
        manager = session.character_manager
        skill_manager = manager.get_manager('skill')
        
        skill_data = skill_manager.game_rules_service.get_by_id('skills', skill_id)
        if not skill_data:
            raise HTTPException(status_code=400, detail=f"Skill {skill_id} does not exist")
        
        prerequisites = skill_manager.get_skill_prerequisites(skill_id)
        return SkillPrerequisites(**prerequisites)
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to get skill prerequisites for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get skill prerequisites: {str(e)}"
        )


@router.get("/characters/{character_id}/skills/export")
def export_build(
    character_id: int,
    char_session: CharacterSessionDep
):
    """Export current skill build."""
    try:
        session = char_session
        manager = session.character_manager
        skill_manager = manager.get_manager('skill')
        
        build = skill_manager.export_skill_build()
        return SkillBuild(**build)
        
    except Exception as e:
        logger.error(f"Failed to export skill build for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to export skill build: {str(e)}"
        )


@router.post("/characters/{character_id}/skills/import")
def import_build(
    character_id: int,
    char_session: CharacterSessionDep,
    request: Dict[str, Any] = Body(...)
):
    """Import a skill build."""
    try:
        session = char_session
        manager = session.character_manager
        skill_manager = manager.get_manager('skill')
        
        build_dict = {'skills': request['skills']}
        if request.get('character_level'):
            build_dict['character_level'] = request['character_level']
        if request.get('total_skill_points'):
            build_dict['total_skill_points'] = request['total_skill_points']
            
        success = skill_manager.import_skill_build(build_dict)
        
        if success:
            summary = skill_manager.get_skill_summary()
            imported_count = len(request['skills'])
            
            return SkillBuildImportResponse(
                message="Skill build imported successfully",
                summary=SkillSummary(**summary),
                imported_count=imported_count,
                validation_errors=[],
                saved=False
            )
        else:
             raise HTTPException(status_code=400, detail="Failed to import skill build")
             
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to import skill build for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to import skill build: {str(e)}"
        )


@router.get("/debug/skills_table")
def debug_skills_table():
    """Debug endpoint to check if skills table loads."""
    try:
        from services.fastapi.shared_services import get_shared_game_rules_service
        rules_service = get_shared_game_rules_service()
        skills_table = rules_service.get_table('skills')
        
        return {
            "table_type": str(type(skills_table)),
            "table_length": len(skills_table) if skills_table else 0,
            "table_exists": skills_table is not None,
            "first_skill": skills_table[0] if skills_table and len(skills_table) > 0 else None
        }
    except Exception as e:
        logger.error(f"Failed to debug skills table: {e}")
        return {"error": str(e), "table_exists": False}