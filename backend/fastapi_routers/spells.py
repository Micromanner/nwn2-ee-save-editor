"""
Spells router - Spell management endpoints
Handles spellbooks, memorization, and spell management for all caster types
"""

import logging
from fastapi import APIRouter, Depends, HTTPException, status, Query
from typing import Optional

from fastapi_routers.dependencies import (
    get_character_manager, 
    get_character_session_dep,
    CharacterManagerDep,
    CharacterSessionDep
)
from fastapi_models import (
    SpellsState,
    AvailableSpellsResponse,
    AllSpellsResponse,
    SpellManageRequest,
    SpellManageResponse,
    SpellInfo,
    SpellcastingClass,
    MemorizedSpell,
    SpellSummary,
    SpellSummaryClass,
    MetamagicFeat
)

logger = logging.getLogger(__name__)

router = APIRouter()


@router.get("/characters/{character_id}/spells/state/", response_model=SpellsState)
def get_spells_state(
    character_id: int,
    include_available: bool = Query(False, description="Include available spells (expensive operation)"),
    manager: CharacterManagerDep = Depends(get_character_manager)
):
    """Get current spells and spellbook state for the spells editor"""
    
    try:
        spell_manager = manager.get_manager('spell')
        
        # Get spellcasting classes
        spellcasting_classes = []
        for idx, class_info in enumerate(manager.character_data.get('ClassList', [])):
            class_id = class_info.get('Class', -1)
            class_level = class_info.get('ClassLevel', 0)
            if spell_manager.is_spellcaster(class_id):
                spellcasting_classes.append(SpellcastingClass(
                    index=idx,
                    class_id=class_id,
                    class_name=spell_manager.get_class_name(class_id),
                    class_level=class_level,
                    caster_level=spell_manager.get_caster_level(idx),
                    spell_type='prepared' if spell_manager.is_prepared_caster(class_id) else 'spontaneous'
                ))
        
        # Get spell summary
        summary_data = spell_manager.get_spell_summary()
        
        # Convert to Pydantic models
        caster_classes = [
            SpellSummaryClass(
                id=cls['id'],
                name=cls['name'],
                total_slots=cls['total_slots'],
                max_spell_level=cls['max_spell_level'],
                slots_by_level=cls['slots_by_level']
            )
            for cls in summary_data.get('caster_classes', [])
        ]
        
        metamagic_feats = [
            MetamagicFeat(
                id=feat['id'],
                name=feat['name'],
                level_cost=feat['level_cost']
            )
            for feat in summary_data.get('metamagic_feats', [])
        ]
        
        spell_summary = SpellSummary(
            caster_classes=caster_classes,
            total_spell_levels=summary_data.get('total_spell_levels', 0),
            metamagic_feats=metamagic_feats,
            spell_resistance=summary_data.get('spell_resistance', 0)
        )
        
        # Get memorized spells
        memorized_data = spell_manager.get_all_memorized_spells() if spellcasting_classes else []
        memorized_spells = [
            MemorizedSpell(
                level=spell['level'],
                spell_id=spell['spell_id'],
                class_id=spell['class_id'],
                metamagic=spell.get('metamagic', 0),
                ready=spell.get('ready', True)
            )
            for spell in memorized_data
        ]
        
        # Get available spells by level if requested (expensive operation)
        available_by_level = None
        if include_available and spellcasting_classes:
            available_by_level = {}
            for level in range(10):
                spells_data = spell_manager.get_available_spells(level)
                available_by_level[level] = [
                    SpellInfo(
                        id=spell['id'],
                        name=spell['name'],
                        level=spell['level'],
                        school_id=spell.get('school_id'),
                        school_name=spell.get('school_name'),
                        icon=spell.get('icon'),
                        description=spell.get('description'),
                        range=spell.get('range'),
                        cast_time=spell.get('cast_time'),
                        conjuration_time=spell.get('conjuration_time'),
                        components=spell.get('components'),
                        metamagic=spell.get('metamagic'),
                        target_type=spell.get('target_type'),
                        available_classes=spell.get('available_classes', [])
                    )
                    for spell in spells_data
                ]
        
        return SpellsState(
            spellcasting_classes=spellcasting_classes,
            spell_summary=spell_summary,
            memorized_spells=memorized_spells,
            available_by_level=available_by_level
        )
        
    except Exception as e:
        logger.error(f"Failed to get spells state for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get spells state: {str(e)}"
        )


@router.get("/characters/{character_id}/spells/available/", response_model=AvailableSpellsResponse)
def get_available_spells(
    character_id: int,
    level: int = Query(..., description="Spell level (0-9)"),
    class_id: Optional[int] = Query(None, description="Optional class ID to filter spells"),
    manager: CharacterManagerDep = Depends(get_character_manager)
):
    """Get spells available for learning at a specific level"""
    
    if not 0 <= level <= 9:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Spell level must be between 0 and 9"
        )
    
    try:
        spell_manager = manager.get_manager('spell')
        
        # Get available spells using the implemented method
        available_data = spell_manager.get_available_spells(level, class_id)
        
        available_spells = [
            SpellInfo(
                id=spell['id'],
                name=spell['name'],
                level=spell['level'],
                school_id=spell.get('school_id'),
                school_name=spell.get('school_name'),
                icon=spell.get('icon'),
                description=spell.get('description'),
                range=spell.get('range'),
                cast_time=spell.get('cast_time'),
                conjuration_time=spell.get('conjuration_time'),
                components=spell.get('components'),
                metamagic=spell.get('metamagic'),
                target_type=spell.get('target_type'),
                available_classes=spell.get('available_classes', [])
            )
            for spell in available_data
        ]
        
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


@router.get("/characters/{character_id}/spells/all/", response_model=AllSpellsResponse)
def get_all_spells(
    character_id: int,
    manager: CharacterManagerDep = Depends(get_character_manager)
):
    """Get all legitimate spells (filtered) for spell browsing"""
    
    try:
        spell_manager = manager.get_manager('spell')
        
        # Get all spells by level - let manager handle the data
        all_spells = []
        total_by_level = {}
        
        for level in range(10):  # Levels 0-9
            level_spells = spell_manager.get_available_spells(level)
            total_by_level[level] = len(level_spells)
            all_spells.extend(level_spells)
        
        # Use spell data as-is from manager (no manual deduplication)
        spells = [
            SpellInfo(
                id=spell['id'],
                name=spell['name'],
                level=spell['level'],
                school_id=spell.get('school_id'),
                school_name=spell.get('school_name'),
                icon=spell.get('icon'),
                description=spell.get('description'),
                range=spell.get('range'),
                cast_time=spell.get('cast_time'),
                conjuration_time=spell.get('conjuration_time'),
                components=spell.get('components'),
                metamagic=spell.get('metamagic'),
                target_type=spell.get('target_type'),
                available_classes=spell.get('available_classes', [])
            )
            for spell in all_spells
        ]
        
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


@router.post("/characters/{character_id}/spells/manage/", response_model=SpellManageResponse)
def manage_spells(
    character_id: int,
    spell_request: SpellManageRequest,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Add or remove spells from character's spellbook"""
    character, session = char_session
    
    try:
        manager = session.character_manager
        spell_manager = manager.get_manager('spell')
        
        # Validate spell ID exists
        if not spell_manager.rules_service.get_by_id('spells', spell_request.spell_id):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Invalid spell ID: {spell_request.spell_id}"
            )
        
        # Get class ID from index
        class_list = manager.character_data.get('ClassList', [])
        if spell_request.class_index >= len(class_list):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Invalid class index: {spell_request.class_index}"
            )
        class_id = class_list[spell_request.class_index].get('Class', -1)
        
        # Validate class is a spellcaster
        if not spell_manager.is_spellcaster(class_id):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Selected class cannot cast spells"
            )
        
        if spell_request.action == 'add':
            # Determine spell level if not provided
            spell_level = spell_request.spell_level
            if spell_level is None:
                spell_level = spell_manager.get_spell_level_for_class(spell_request.spell_id, class_id)
                if spell_level is None:
                    raise HTTPException(
                        status_code=status.HTTP_400_BAD_REQUEST,
                        detail="Could not determine spell level for this class"
                    )
            
            # Add the spell
            added = spell_manager.add_known_spell(class_id, spell_level, spell_request.spell_id)
            if not added:
                raise HTTPException(
                    status_code=status.HTTP_409_CONFLICT,
                    detail="Spell is already known"
                )
            message = "Spell added successfully"
            
        elif spell_request.action == 'remove':
            # Determine spell level if not provided
            spell_level = spell_request.spell_level
            if spell_level is None:
                spell_level = spell_manager.get_spell_level_for_class(spell_request.spell_id, class_id)
                if spell_level is None:
                    raise HTTPException(
                        status_code=status.HTTP_400_BAD_REQUEST,
                        detail="Could not determine spell level for this class"
                    )
            
            # Remove the spell
            removed = spell_manager.remove_known_spell(class_id, spell_level, spell_request.spell_id)
            if not removed:
                raise HTTPException(
                    status_code=status.HTTP_404_NOT_FOUND,
                    detail="Spell not found in known spell list"
                )
            message = "Spell removed successfully"
            
        else:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Unsupported action: {spell_request.action}"
            )
        
        # Get updated spell summary
        summary_data = spell_manager.get_spell_summary()
        
        # Convert to Pydantic models
        caster_classes = [
            SpellSummaryClass(
                id=cls['id'],
                name=cls['name'],
                total_slots=cls['total_slots'],
                max_spell_level=cls['max_spell_level'],
                slots_by_level=cls['slots_by_level']
            )
            for cls in summary_data.get('caster_classes', [])
        ]
        
        metamagic_feats = [
            MetamagicFeat(
                id=feat['id'],
                name=feat['name'],
                level_cost=feat['level_cost']
            )
            for feat in summary_data.get('metamagic_feats', [])
        ]
        
        spell_summary = SpellSummary(
            caster_classes=caster_classes,
            total_spell_levels=summary_data.get('total_spell_levels', 0),
            metamagic_feats=metamagic_feats,
            spell_resistance=summary_data.get('spell_resistance', 0)
        )
        
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