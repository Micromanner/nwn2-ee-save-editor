"""Classes router - Class management endpoints."""

from typing import Optional, Dict, Any, List
from fastapi import APIRouter, Depends, HTTPException, status, Query, Body
from loguru import logger

from fastapi_routers.dependencies import (
    get_character_manager,
    get_character_session,
    CharacterManagerDep,
    CharacterSessionDep
)
from fastapi_models.class_models import (
    ClassesState, ClassChangeRequest, ClassChangePreview, ClassChangeResult,
    LevelUpRequest, LevelUpPreview, LevelUpResult, CategorizedClassesResponse,
    FocusInfo, ClassFeaturesResponse, ClassAddRequest
)

router = APIRouter()


@router.get("/characters/{character_id}/classes/state")
def get_classes_state(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get current classes state."""
    try:
        class_manager = manager.get_manager('class')
        if not class_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Class manager not available"
            )
        
        class_summary = class_manager.get_class_summary()
        attack_bonuses = class_manager.get_attack_bonuses()
        total_saves = class_manager.calculate_total_saves()
        
        response_data = {
            **class_summary,
            'combat_stats': {
                **attack_bonuses,
                **total_saves
            },
            'xp_progress': class_manager.get_xp_progress()
        }
        
        return ClassesState(**response_data)
        
    except Exception as e:
        logger.error(f"Failed to get classes state for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get classes state: {str(e)}"
        )


@router.post("/characters/{character_id}/classes/change")
def change_class(
    character_id: int,
    char_session: CharacterSessionDep,
    request: ClassChangeRequest = Body(...)
):
    """Change character's class."""
    session = char_session
    manager = session.character_manager
    
    try:
        class_manager = manager.get_manager('class')
        if not class_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Class manager not available"
            )
        
        if not class_manager.rules_service.get_by_id('classes', request.class_id):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Invalid class ID: {request.class_id}"
            )

        if request.preview:
            changes = {}
            if request.old_class_id is not None:
                changes = class_manager.change_specific_class(
                    request.old_class_id, request.class_id, request.preserve_level
                )
            
            return ClassChangePreview(
                preview=True,
                class_change=changes or {},
                has_unsaved_changes=session.has_unsaved_changes()
            )

        if request.old_class_id is not None:
            changes = class_manager.change_specific_class(
                request.old_class_id, request.class_id, request.preserve_level
            )
        else:
            changes = class_manager.change_class(
                request.class_id, request.preserve_level
            )

        updated_state = manager.get_level_up_state()

        return ClassChangeResult(
            success=True,
            message='Class changed successfully',
            class_change=changes or {},
            has_unsaved_changes=session.has_unsaved_changes(),
            updated_state=updated_state
        )

    except Exception as e:
        logger.error(f"Failed to change class for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to change class: {str(e)}"
        )


@router.post("/characters/{character_id}/classes/level-up")
def level_up(
    character_id: int,
    char_session: CharacterSessionDep,
    request: LevelUpRequest = Body(...)
):
    """Adjust levels in a specific class."""
    session = char_session
    manager = session.character_manager
    
    try:
        class_manager = manager.get_manager('class')
        if not class_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Class manager not available"
            )
        
        if not class_manager.rules_service.get_by_id('classes', request.class_id):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Invalid class ID: {request.class_id}"
            )
        
        if request.preview:
            return LevelUpPreview(
                preview=True,
                level_change=request.level_change,
                has_unsaved_changes=session.has_unsaved_changes()
            )
        
        changes = class_manager.adjust_class_level(request.class_id, request.level_change)
        updated_state = manager.get_level_up_state()

        return LevelUpResult(
            success=True,
            message='Leveled up successfully',
            level_changes=changes or {},
            has_unsaved_changes=session.has_unsaved_changes(),
            updated_state=updated_state
        )
        
    except Exception as e:
        logger.error(f"Failed to level up character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to level up: {str(e)}"
        )


@router.get("/characters/{character_id}/classes/categorized")
def get_categorized_classes_for_character(
    character_id: int,
    manager: CharacterManagerDep,
    search: Optional[str] = Query(None, description="Filter classes by name"),
    type_filter: Optional[str] = Query(None, alias="type", description="Filter by 'base' or 'prestige'"),
    include_unplayable: bool = Query(False, description="Include NPC classes")
):
    """Get all classes organized by type and focus for UI selection."""
    try:
        class_manager = manager.get_manager('class')
        
        result = class_manager.get_categorized_classes(search, type_filter, include_unplayable)
        
        character_context = _get_character_class_context(manager)
        result['character_context'] = character_context
        
        return CategorizedClassesResponse(**result)
        
    except Exception as e:
        logger.error(f"Error getting categorized classes: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to categorize classes: {str(e)}"
        )


@router.get("/classes/categorized")
def get_categorized_classes_standalone(
    search: Optional[str] = Query(None, description="Filter classes by name"),
    type_filter: Optional[str] = Query(None, alias="type", description="Filter by 'base' or 'prestige'"),
    include_unplayable: bool = Query(False, description="Include NPC classes")
):
    """Get all classes organized by type."""
    from services.gamedata.class_categorizer import ClassCategorizer, ClassType
    from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader

    try:
        game_data_loader = get_dynamic_game_data_loader()
        categorizer = ClassCategorizer(game_data_loader)
        
        if search:
            search_filter = None
            if type_filter == 'base':
                search_filter = ClassType.BASE
            elif type_filter == 'prestige':
                search_filter = ClassType.PRESTIGE
            
            search_results = categorizer.search_classes(search, search_filter)
            
            return CategorizedClassesResponse(
                categories={}, 
                focus_info={}, 
                total_classes=0, 
                include_unplayable=include_unplayable,
                search_results=[c.to_dict() for c in search_results],
                query=search,
                total_results=len(search_results)
            )
        
        categories = categorizer.get_categorized_classes(include_unplayable)
        if type_filter in ['base', 'prestige']:
            categories = {type_filter: categories[type_filter]}
            
        serialized_categories = {}
        for c_type, focus_groups in categories.items():
            serialized_categories[c_type] = {
                focus: [c.to_dict() for c in c_list]
                for focus, c_list in focus_groups.items() if c_list
            }
            
        focus_info = categorizer.get_focus_display_info()
        
        return CategorizedClassesResponse(
            categories=serialized_categories,
            focus_info={k: FocusInfo(**v) for k, v in focus_info.items()},
            total_classes=sum(len(l) for g in categories.values() for l in g.values()),
            include_unplayable=include_unplayable
        )

    except Exception as e:
        logger.error(f"Error getting categorized classes: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to categorize classes: {str(e)}"
        )


@router.post("/characters/{character_id}/classes/add")
def add_class(
    character_id: int,
    char_session: CharacterSessionDep,
    request: ClassAddRequest = Body(...)
):
    """Add a new class to character."""
    session = char_session
    manager = session.character_manager
    
    try:
        class_manager = manager.get_manager('class')
        if not class_manager.rules_service.get_by_id('classes', request.class_id):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Invalid class ID: {request.class_id}"
            )

        changes = class_manager.add_class_level(request.class_id)
        updated_state = manager.get_level_up_state()

        return {
            'success': True,
            'message': 'Class added successfully',
            'changes': changes or {},
            'has_unsaved_changes': session.has_unsaved_changes(),
            'updated_state': updated_state
        }
        
    except Exception as e:
        logger.error(f"Failed to add class {request.class_id} to character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to add class: {str(e)}"
        )


@router.post("/characters/{character_id}/classes/remove/{class_id}")
def remove_class(
    character_id: int,
    class_id: int,
    char_session: CharacterSessionDep
):
    """Remove a class from multiclass character."""
    session = char_session
    manager = session.character_manager
    
    try:
        class_manager = manager.get_manager('class')
        result = class_manager.remove_class(class_id)
        
        return {
            'success': True,
            'message': 'Class removed successfully',
            'changes': result,
            'has_unsaved_changes': session.has_unsaved_changes()
        }
        
    except Exception as e:
        logger.error(f"Failed to remove class {class_id} from character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to remove class: {str(e)}"
        )


@router.get("/characters/{character_id}/classes/validate")
def validate_classes(
    character_id: int,
    manager: CharacterManagerDep
):
    """Validate current class configuration."""
    try:
        class_manager = manager.get_manager('class')
        is_valid, errors = class_manager.validate()
        
        return {
            'valid': is_valid,
            'errors': errors
        }
        
    except Exception as e:
        logger.error(f"Failed to validate classes for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to validate classes: {str(e)}"
        )


@router.get("/characters/{character_id}/classes/prestige-options")
def get_prestige_options(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get available prestige class options."""
    try:
        class_manager = manager.get_manager('class')
        return {
            'prestige_classes': class_manager.get_prestige_class_options()
        }
        
    except Exception as e:
        logger.error(f"Failed to get prestige options for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get prestige options: {str(e)}"
        )


@router.get("/characters/{character_id}/classes/has-class/{class_name}")
def has_class_by_name(
    character_id: int,
    class_name: str,
    manager: CharacterManagerDep
):
    """Check if character has levels in a specific class."""
    try:
        class_manager = manager.get_manager('class')
        has_class = class_manager.has_class_by_name(class_name)
        class_level = class_manager.get_class_level_by_name(class_name) if has_class else 0
        
        return {
            'has_class': has_class,
            'class_level': class_level,
            'class_name': class_name
        }
        
    except Exception as e:
        logger.error(f"Failed to check class {class_name} for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to check class by name: {str(e)}"
        )


@router.get("/characters/{character_id}/classes/level-info/{class_id}")
def get_class_level_info(
    character_id: int,
    class_id: int,
    manager: CharacterManagerDep
):
    """Get level information for a class."""
    try:
        class_manager = manager.get_manager('class')
        return class_manager.get_class_level_info(class_id)
        
    except Exception as e:
        logger.error(f"Failed to get level info for class {class_id} for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get class level info: {str(e)}"
        )


@router.get("/characters/{character_id}/classes/history")
def get_level_history(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get character level up history."""
    try:
        class_manager = manager.get_manager('class')
        return {'history': class_manager.get_level_history()}

    except Exception as e:
        logger.error(f"Failed to get level history for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get level history: {str(e)}"
        )


@router.get("/characters/{character_id}/classes/experience")
def get_experience(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get character experience points and level progress."""
    try:
        class_manager = manager.get_manager('class')
        return class_manager.get_xp_progress()

    except Exception as e:
        logger.error(f"Failed to get experience for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get experience: {str(e)}"
        )


@router.post("/characters/{character_id}/classes/experience")
def set_experience(
    character_id: int,
    char_session: CharacterSessionDep,
    request: dict = Body(...)
):
    """Set character experience points."""
    session = char_session
    manager = session.character_manager

    try:
        class_manager = manager.get_manager('class')
        xp = request.get('xp')
        if xp is None:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Missing 'xp' in request body"
            )

        result = class_manager.set_experience(xp)
        result['has_unsaved_changes'] = session.has_unsaved_changes()

        return result

    except ValueError as e:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=str(e)
        )
    except Exception as e:
        logger.error(f"Failed to set experience for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to set experience: {str(e)}"
        )


@router.get("/classes/features/{class_id}")
def get_class_features(
    class_id: int,
    max_level: int = Query(20, description="Maximum level to show progression for"),
    include_spells: bool = Query(True, description="Include spell progression tables"),
    include_proficiencies: bool = Query(True, description="Include weapon/armor proficiencies"),
    character_id: Optional[int] = Query(None, description="Character ID for personalized data")
):
    """Get detailed class features and progression."""
    from services.gamedata.class_categorizer import ClassCategorizer
    from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader

    try:
        game_data_loader = get_dynamic_game_data_loader()
        if not game_data_loader:
            raise HTTPException(status_code=503, detail="Game data unavailable")
        
        class_data = game_data_loader.get_by_id('classes', class_id)
        if not class_data:
            raise HTTPException(status_code=404, detail=f"Class {class_id} not found")
            
        categorizer = ClassCategorizer(game_data_loader)
        class_info = categorizer.get_class_info(class_id)
        
        progression_data = {
            'class_id': class_id,
            'class_name': class_info.name if class_info else 'Unknown Class',
            'basic_info': {
                'hit_die': class_info.hit_die if class_info else 8,
                'skill_points_per_level': class_info.skill_points if class_info else 2,
                'is_spellcaster': class_info.is_spellcaster if class_info else False,
                'spell_type': 'arcane' if class_info and class_info.has_arcane else ('divine' if class_info and class_info.has_divine else 'none')
            },
            'description': {},
            'max_level_shown': max_level
        }
        
        if class_info and class_info.parsed_description:
            progression_data['description'] = {
                'title': getattr(class_info.parsed_description, 'title', ''),
                'class_type': getattr(class_info.parsed_description, 'class_type', ''),
                'summary': getattr(class_info.parsed_description, 'summary', ''),
                'features': getattr(class_info.parsed_description, 'features', '')
            }
        
        return ClassFeaturesResponse(**progression_data)
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Error getting class features for class {class_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get class features: {str(e)}"
        )


def _get_character_class_context(manager) -> Optional[Dict[str, Any]]:
    """Get character-specific class context."""
    context = {}
    try:
        class_manager = manager.get_manager('class')
        class_summary = class_manager.get_class_summary()
        context['current_classes'] = class_summary

        if hasattr(class_manager, 'get_prestige_class_options'):
            context['prestige_requirements'] = class_manager.get_prestige_class_options()

        context['can_multiclass'] = class_summary.get('can_multiclass', True)
        context['multiclass_slots_used'] = len(class_summary.get('classes', []))
    except Exception as e:
        logger.warning(f"Error getting character class context: {e}")
        context['error'] = str(e)

    return context