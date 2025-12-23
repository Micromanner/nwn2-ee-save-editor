"""
Spells router - Spell management endpoints
Handles spellbooks, memorization, and spell management for all caster types
"""

from fastapi import APIRouter, Depends, HTTPException, status, Query
from typing import Optional
from loguru import logger

from fastapi_routers.dependencies import (
    get_character_manager,
    get_character_session,
    CharacterManagerDep,
    CharacterSessionDep
)
# from fastapi_models import (...) - moved to lazy loading

router = APIRouter()


@router.get("/characters/{character_id}/spells/state")
def get_spells_state(
    character_id: int,
    manager: CharacterManagerDep,
    include_available: bool = Query(False, description="Include available spells (expensive operation)")
):
    """Get current spells and spellbook state for the spells editor"""
    try:
        # Lazy imports for performance
        from fastapi_models import (
            SpellsState, SpellcastingClass, SpellSummaryClass, MetamagicFeat,
            SpellSummary, MemorizedSpell, SpellInfo
        )
        
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
        
        # Get memorized spells (basic info - frontend enriches from legitimate spells data)
        memorized_data = spell_manager.get_all_memorized_spells() if spellcasting_classes else []

        # Get spell names for basic display
        memorized_spells = []
        for spell in memorized_data:
            spell_details = spell_manager.get_spell_details(spell['spell_id'])
            memorized_spells.append(MemorizedSpell(
                level=spell['level'],
                spell_id=spell['spell_id'],
                name=spell_details['name'],
                icon=spell_details['icon'],
                school_name=spell_details.get('school_name'),
                description=spell_details.get('description'),
                class_id=spell['class_id'],
                metamagic=spell.get('metamagic', 0),
                ready=spell.get('ready', True)
            ))
        
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
                        available_metamagic=spell.get('available_metamagic'),
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


@router.get("/characters/{character_id}/spells/available")
def get_available_spells(
    character_id: int,
    manager: CharacterManagerDep,
    level: int = Query(..., description="Spell level (0-9)"),
    class_id: Optional[int] = Query(None, description="Optional class ID to filter spells")
):
    """Get spells available for learning at a specific level"""
    if not 0 <= level <= 9:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Spell level must be between 0 and 9"
        )
    
    try:
        # Lazy imports for performance
        from fastapi_models import AvailableSpellsResponse, SpellInfo
        
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
                available_metamagic=spell.get('available_metamagic'),
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


@router.get("/characters/{character_id}/spells/all")
def get_all_spells(
    character_id: int,
    manager: CharacterManagerDep,
    level: Optional[int] = Query(None, description="Filter by spell level (0-9)"),
    school: Optional[str] = Query(None, description="Filter by school name"),
    search: Optional[str] = Query(None, description="Search spell names")
):
    """Get all legitimate spells (filtered) for spell browsing"""
    try:
        # Lazy imports for performance
        from fastapi_models import AllSpellsResponse, SpellInfo
        
        spell_manager = manager.get_manager('spell')
        
        # Handle filtering - if level is specified, get only that level
        if level is not None:
            if not 0 <= level <= 9:
                raise HTTPException(
                    status_code=status.HTTP_400_BAD_REQUEST,
                    detail="Spell level must be between 0 and 9"
                )
            all_spells = spell_manager.get_available_spells(level)
            total_by_level = {level: len(all_spells)}
        else:
            # Get all spells by level - let manager handle the data
            all_spells = []
            total_by_level = {}
            
            for spell_level in range(10):  # Levels 0-9
                level_spells = spell_manager.get_available_spells(spell_level)
                total_by_level[spell_level] = len(level_spells)
                all_spells.extend(level_spells)
        
        # Apply additional filtering (school, search)
        filtered_spells = all_spells
        
        if school and school.lower() != 'all':
            filtered_spells = [
                spell for spell in filtered_spells 
                if spell.get('school_name', '').lower() == school.lower()
            ]
        
        if search and search.strip():
            search_term = search.strip().lower()
            filtered_spells = [
                spell for spell in filtered_spells 
                if search_term in spell.get('name', '').lower()
            ]
        
        # Recalculate total_by_level for filtered results if filters were applied
        if (school and school.lower() != 'all') or (search and search.strip()):
            if level is not None:
                # Single level requested - just count filtered spells
                total_by_level = {level: len(filtered_spells)}
            else:
                # Multiple levels - recalculate counts by level
                total_by_level = {}
                for spell_level in range(10):
                    count = len([s for s in filtered_spells if s.get('level') == spell_level])
                    total_by_level[spell_level] = count
        
        # Convert to Pydantic models
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
                available_metamagic=spell.get('available_metamagic'),
                target_type=spell.get('target_type'),
                available_classes=spell.get('available_classes', [])
            )
            for spell in filtered_spells
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
    """Get legitimate spells with pagination (mirrors feats endpoint)"""
    try:
        from fastapi_models import LegitimateSpellsResponse, SpellInfo, SpellPaginationInfo

        spell_manager = manager.get_manager('spell')

        level_list = None
        if levels:
            try:
                level_list = [int(l.strip()) for l in levels.split(',') if l.strip()]
                if any(l < 0 or l > 9 for l in level_list):
                    raise ValueError("Levels must be between 0 and 9")
            except ValueError as e:
                raise HTTPException(
                    status_code=status.HTTP_400_BAD_REQUEST,
                    detail=f"Invalid levels parameter: {str(e)}"
                )

        school_list = None
        if schools:
            school_list = [s.strip() for s in schools.split(',') if s.strip()]

        search_term = search.strip() if search else None

        result = spell_manager.get_legitimate_spells(
            levels=level_list,
            schools=school_list,
            search=search_term,
            page=page,
            limit=limit,
            class_id=class_id
        )

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
                available_metamagic=spell.get('available_metamagic'),
                target_type=spell.get('target_type'),
                available_classes=spell.get('available_classes', [])
            )
            for spell in result['spells']
        ]

        pagination = SpellPaginationInfo(
            page=result['pagination']['page'],
            limit=result['pagination']['limit'],
            total=result['pagination']['total'],
            pages=result['pagination']['pages'],
            has_next=result['pagination']['has_next'],
            has_previous=result['pagination']['has_previous']
        )

        return LegitimateSpellsResponse(
            spells=spells,
            pagination=pagination
        )

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
    spell_request,  # Type removed for lazy loading
    char_session: CharacterSessionDep
):
    """Add or remove spells from character's spellbook"""
    try:
        # Lazy imports for performance
        from fastapi_models import (
            SpellManageRequest, SpellManageResponse, SpellSummaryClass,
            MetamagicFeat, SpellSummary
        )
        
        session = char_session
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