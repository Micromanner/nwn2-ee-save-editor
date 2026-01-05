"""Spells router - Spell management endpoints."""

from fastapi import APIRouter, Depends, HTTPException, status, Query, Body
from typing import Optional, List, Dict, Any
from loguru import logger

from fastapi_routers.dependencies import (
    get_character_manager,
    get_character_session,
    CharacterManagerDep,
    CharacterSessionDep
)
from fastapi_models import SpellManageRequest

router = APIRouter()


@router.get("/characters/{character_id}/spells/state")
def get_spells_state(
    character_id: int,
    manager: CharacterManagerDep,
    include_available: bool = Query(False, description="Include available spells (expensive operation)")
):
    """Get current spells and spellbook state."""
    try:
        from fastapi_models import SpellsState
        
        spell_manager = manager.get_manager('spell')
        
        state_summary = spell_manager.get_spells_state_summary(include_available=include_available)
        
        return SpellsState(**state_summary)
        
    except Exception as e:
        logger.error(f"Failed to get spells state for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get spells state: {str(e)}"
        )


@router.get("/characters/{character_id}/spells/available")
def get_available_spells(
    character_id: int,
    manager: CharacterManagerDep,
    level: int = Query(..., description="Spell level (0-9)"),
    class_id: Optional[int] = Query(None, description="Optional class ID to filter spells")
):
    """Get spells available for learning at a specific level."""
    if not 0 <= level <= 9:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Spell level must be between 0 and 9"
        )
    
    try:
        from fastapi_models import AvailableSpellsResponse, SpellInfo
        
        spell_manager = manager.get_manager('spell')
        available_data = spell_manager.get_available_spells(level, class_id)
        
        available_spells = [SpellInfo(**spell) for spell in available_data]
        
        return AvailableSpellsResponse(
            spell_level=level,
            class_id=class_id,
            available_spells=available_spells,
            total=len(available_spells)
        )
        
    except Exception as e:
        logger.error(f"Failed to get available spells for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get available spells: {str(e)}"
        )


@router.get("/characters/{character_id}/spells/all")
def get_all_spells(
    character_id: int,
    manager: CharacterManagerDep,
    level: Optional[int] = Query(None, description="Filter by spell level (0-9)"),
    school: Optional[str] = Query(None, description="Filter by school name"),
    search: Optional[str] = Query(None, description="Search spell names")
):
    """Get all legitimate spells (filtered)."""
    try:
        from fastapi_models import AllSpellsResponse, SpellInfo
        
        spell_manager = manager.get_manager('spell')
        
        if level is not None:
            if not 0 <= level <= 9:
                raise HTTPException(status_code=400, detail="Spell level must be between 0 and 9")
            all_spells = spell_manager.get_available_spells(level)
            total_by_level = {level: len(all_spells)}
        else:
            all_spells = []
            total_by_level = {}
            for spell_level in range(10):
                level_spells = spell_manager.get_available_spells(spell_level)
                total_by_level[spell_level] = len(level_spells)
                all_spells.extend(level_spells)
        
        filtered_spells = all_spells
        if school and school.lower() != 'all':
            filtered_spells = [s for s in filtered_spells if s.get('school_name', '').lower() == school.lower()]
        
        if search and search.strip():
            search_term = search.strip().lower()
            filtered_spells = [s for s in filtered_spells if search_term in s.get('name', '').lower()]
        
        if (school and school.lower() != 'all') or (search and search.strip()):
            if level is not None:
                total_by_level = {level: len(filtered_spells)}
            else:
                total_by_level = {}
                for spell_level in range(10):
                    count = len([s for s in filtered_spells if s.get('level') == spell_level])
                    total_by_level[spell_level] = count
        
        spells = [SpellInfo(**spell) for spell in filtered_spells]
        
        return AllSpellsResponse(
            spells=spells,
            count=len(spells),
            total_by_level=total_by_level
        )
        
    except Exception as e:
        logger.error(f"Failed to get all spells for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get all spells: {str(e)}"
        )


@router.get("/characters/{character_id}/spells/legitimate")
def get_legitimate_spells(
    character_id: int,
    manager: CharacterManagerDep,
    levels: Optional[str] = Query(None, description="Comma-separated spell levels (0-9)"),
    schools: Optional[str] = Query(None, description="Comma-separated school names"),
    page: int = Query(1, ge=1, description="Page number"),
    limit: int = Query(50, ge=1, le=200, description="Results per page"),
    search: Optional[str] = Query(None, description="Search term for spell name/description"),
    class_id: Optional[int] = Query(None, description="Filter spells available to this class")
):
    """Get legitimate spells with pagination."""
    try:
        from fastapi_models import LegitimateSpellsResponse, SpellInfo, SpellPaginationInfo

        spell_manager = manager.get_manager('spell')

        # Parse comma-separated levels
        level_list = None
        if levels:
            try:
                level_list = [int(l.strip()) for l in levels.split(',') if l.strip()]
                if any(l < 0 or l > 9 for l in level_list):
                    raise ValueError("Levels must be between 0 and 9")
            except ValueError as e:
                raise HTTPException(status_code=400, detail=f"Invalid levels parameter: {str(e)}")

        school_list = [s.strip() for s in schools.split(',') if s.strip()] if schools else None
        search_term = search.strip() if search else None

        result = spell_manager.get_legitimate_spells(
            levels=level_list,
            schools=school_list,
            search=search_term,
            page=page,
            limit=limit,
            class_id=class_id
        )

        spells = [SpellInfo(**spell) for spell in result['spells']]
        pagination = SpellPaginationInfo(**result['pagination'])

        return LegitimateSpellsResponse(spells=spells, pagination=pagination)

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to get legitimate spells for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get legitimate spells: {str(e)}"
        )


@router.post("/characters/{character_id}/spells/manage")
def manage_spells(
    character_id: int,
    spell_request: SpellManageRequest,
    char_session: CharacterSessionDep
):
    """Add or remove spells from character's spellbook."""
    try:
        from fastapi_models import SpellManageResponse, SpellSummary
        
        session = char_session
        manager = session.character_manager
        spell_manager = manager.get_manager('spell')
        
        success, message = spell_manager.manage_spell(
            action=spell_request.action,
            spell_id=spell_request.spell_id,
            class_index=spell_request.class_index,
            spell_level=spell_request.spell_level
        )
        
        if not success:
            raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail=message)

        summary_data = spell_manager.get_spell_summary()
        spell_summary = SpellSummary(**summary_data)
        
        return SpellManageResponse(
            message=message,
            spell_summary=spell_summary,
            has_unsaved_changes=session.has_unsaved_changes()
        )
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to manage spells for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to manage spells: {str(e)}"
        )
