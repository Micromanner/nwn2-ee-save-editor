"""
Classes router - Class management endpoints
"""

import logging
from typing import Optional, Dict, Any, List
from fastapi import APIRouter, Depends, HTTPException, status, Query

from fastapi_routers.dependencies import (
    get_character_manager, 
    get_character_session,
    CharacterManagerDep,
    CharacterSessionDep
)
# from fastapi_models.class_models import (...) - moved to lazy loading
from character.service_modules.class_categorizer import ClassCategorizer, ClassType
from gamedata.loader import get_game_data_loader

logger = logging.getLogger(__name__)

router = APIRouter()


@router.get("/characters/{character_id}/classes/state")
def get_classes_state(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get current classes state"""
    from fastapi_models.class_models import ClassesState
    
    try:
        class_manager = manager.get_manager('class')
        if not class_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Class manager not available"
            )
        
        # Use manager methods only - no duplicated logic
        class_summary = class_manager.get_class_summary()
        if not class_summary:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Could not get class summary"
            )
        
        attack_bonuses = class_manager.get_attack_bonuses()
        total_saves = class_manager.calculate_total_saves()
        
        # Use manager data directly - simplified structure
        response_data = {
            **class_summary,  # classes, total_level, multiclass, can_multiclass
            'combat_stats': {
                **attack_bonuses,
                **total_saves
            }
        }
        
        return ClassesState(**response_data)
        
    except Exception as e:
        logger.error(f"Failed to get classes state for character {character_id}: {e}")
        logger.error(f"Failed to get classes state for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get classes state: {str(e)}"
        )


@router.post("/characters/{character_id}/classes/change")
def change_class(
    character_id: int,
    char_session: CharacterSessionDep,
    request: dict  # Request body parameter
):
    """
    Change character's class using the unified CharacterManager
    Returns all cascading changes (feats, spells, skills)
    """
    from fastapi_models.class_models import ClassChangeRequest, ClassChangePreview, ClassChangeResult
    session = char_session
    manager = session.character_manager
    
    try:
        class_manager = manager.get_manager('class')
        if not class_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Class manager not available"
            )
        
        # Validate class ID exists
        class_id = request.get('class_id')
        if class_id is None or not hasattr(class_manager.game_data_loader, 'get_by_id') or not class_manager.game_data_loader.get_by_id('classes', class_id):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Invalid class ID: {class_id}"
            )
        
        # Get old class ID if provided (for multiclass characters)
        old_class_id = request.get('old_class_id')
        
        # Use class manager methods - no duplicated logic
        if request.get('preview', False):
            if old_class_id is not None:
                # Multiclass character - change specific class
                changes = class_manager.change_specific_class(
                    old_class_id, class_id, request.get('preserve_level', True)
                )
            else:
                # Single class character - change primary class
                changes = class_manager.change_class(
                    class_id, request.get('preserve_level', True), request.get('cheat_mode', False)
                )
            
            preview_data = {
                'preview': True,
                'class_change': changes or {},
                'has_unsaved_changes': session.has_unsaved_changes()
            }
            return ClassChangePreview(**preview_data)

        # Execute class change using manager
        if old_class_id is not None:
            # Multiclass character - change specific class
            changes = class_manager.change_specific_class(
                old_class_id, class_id, request.get('preserve_level', True)
            )
        else:
            # Single class character - change primary class
            changes = class_manager.change_class(
                class_id, request.get('preserve_level', True), request.get('cheat_mode', False)
            )

        result = {
            'success': True,
            'message': 'Class changed successfully',
            'class_change': changes or {},
            'has_unsaved_changes': session.has_unsaved_changes()
        }

        return ClassChangeResult(**result)

    except Exception as e:
        logger.error(f"Failed to change class for character {character_id}: {e}")
        logger.error(f"Failed to change class for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to change class: {str(e)}"
        )


@router.post("/characters/{character_id}/classes/level-up")
def level_up(
    character_id: int,
    char_session: CharacterSessionDep,
    request: dict  # Request body parameter
):
    """
    Adjust levels in a specific class (add or remove levels)
    Supports level_change parameter for +/- level adjustments
    """
    from fastapi_models.class_models import LevelUpRequest, LevelUpPreview, LevelUpResult
    session = char_session
    manager = session.character_manager
    
    try:
        class_manager = manager.get_manager('class')
        if not class_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Class manager not available"
            )
        
        # Validate class ID exists
        class_id = request.get('class_id')
        if class_id is None or not hasattr(class_manager.game_data_loader, 'get_by_id') or not class_manager.game_data_loader.get_by_id('classes', class_id):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Invalid class ID: {class_id}"
            )
        
        # Get level change (default to +1 for compatibility)
        level_change = request.get('level_change', 1)
        
        if request.get('preview', False):
            # Use manager methods only
            preview_data = {
                'preview': True,
                'level_change': level_change,
                'has_unsaved_changes': session.has_unsaved_changes()
            }
            return LevelUpPreview(**preview_data)
        
        # Use class manager method - no duplicated logic
        changes = class_manager.adjust_class_level(class_id, level_change, request.get('cheat_mode', False))
        
        result = {
            'success': True,
            'message': 'Leveled up successfully',
            'level_changes': changes or {},
            'has_unsaved_changes': session.has_unsaved_changes()
        }
        
        return LevelUpResult(**result)
        
    except Exception as e:
        logger.error(f"Failed to level up character {character_id}: {e}")
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
    """
    Get all classes organized by type and focus for UI selection
    Includes character-specific context for prerequisites
    """
    from fastapi_models.class_models import CategorizedClassesResponse, FocusInfo
    
    try:
        # Get game data loader
        game_data_loader = get_game_data_loader()
        if not game_data_loader:
            raise HTTPException(
                status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
                detail="Game data not available"
            )
        
        # Initialize categorizer
        categorizer = ClassCategorizer(game_data_loader)
        
        # Handle search mode
        if search:
            search_filter = None
            if type_filter == 'base':
                search_filter = ClassType.BASE
            elif type_filter == 'prestige':
                search_filter = ClassType.PRESTIGE
            
            search_results = categorizer.search_classes(search, search_filter)
            
            # Get character context even for search
            character_context = None
            try:
                character_context = _get_character_class_context(manager, categorizer)
            except Exception as e:
                logger.warning(f"Could not get character context: {e}")
            
            return CategorizedClassesResponse(
                categories={},  # Empty categories for search mode
                focus_info={},
                total_classes=0,
                include_unplayable=include_unplayable,
                character_context=character_context,
                search_results=[_serialize_class_info(class_info) for class_info in search_results],
                query=search,
                total_results=len(search_results)
            )
        
        # Get full categorized classes
        categories = categorizer.get_categorized_classes(include_unplayable)
        
        # Apply type filter if specified
        if type_filter in ['base', 'prestige']:
            filtered_categories = {type_filter: categories[type_filter]}
        else:
            filtered_categories = categories
        
        # Serialize the data
        serialized_categories = {}
        for class_type, focus_groups in filtered_categories.items():
            serialized_categories[class_type] = {}
            for focus, class_list in focus_groups.items():
                if class_list:  # Only include non-empty categories
                    serialized_categories[class_type][focus] = [
                        _serialize_class_info(class_info) for class_info in class_list
                    ]
        
        # Get focus display info
        focus_info_raw = categorizer.get_focus_display_info()
        focus_info = {
            focus: FocusInfo(**info) for focus, info in focus_info_raw.items()
        }
        
        # Get character context
        character_context = None
        try:
            character_context = _get_character_class_context(manager, categorizer)
        except Exception as e:
            logger.warning(f"Could not get character context: {e}")
        
        response_data = {
            'categories': serialized_categories,
            'focus_info': focus_info,
            'total_classes': sum(
                len(class_list) 
                for focus_groups in filtered_categories.values() 
                for class_list in focus_groups.values()
            ),
            'include_unplayable': include_unplayable,
            'character_context': character_context
        }
        
        return CategorizedClassesResponse(**response_data)
        
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
    """
    Get all classes organized by type and focus (standalone, no character context needed)
    """
    from fastapi_models.class_models import CategorizedClassesResponse, FocusInfo
    try:
        # Get game data loader
        game_data_loader = get_game_data_loader()
        if not game_data_loader:
            raise HTTPException(
                status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
                detail="Game data not available"
            )
        
        # Initialize categorizer
        categorizer = ClassCategorizer(game_data_loader)
        
        # Handle search mode
        if search:
            search_filter = None
            if type_filter == 'base':
                search_filter = ClassType.BASE
            elif type_filter == 'prestige':
                search_filter = ClassType.PRESTIGE
            
            search_results = categorizer.search_classes(search, search_filter)
            
            return CategorizedClassesResponse(
                categories={},  # Empty categories for search mode
                focus_info={},
                total_classes=0,
                include_unplayable=include_unplayable,
                character_context=None,  # Will be set properly in character-specific endpoint
                search_results=[_serialize_class_info(class_info) for class_info in search_results],
                query=search,
                total_results=len(search_results)
            )
        
        # Get full categorized classes
        categories = categorizer.get_categorized_classes(include_unplayable)
        
        # Apply type filter if specified
        if type_filter in ['base', 'prestige']:
            filtered_categories = {type_filter: categories[type_filter]}
        else:
            filtered_categories = categories
        
        # Serialize the data
        serialized_categories = {}
        for class_type, focus_groups in filtered_categories.items():
            serialized_categories[class_type] = {}
            for focus, class_list in focus_groups.items():
                if class_list:  # Only include non-empty categories
                    serialized_categories[class_type][focus] = [
                        _serialize_class_info(class_info) for class_info in class_list
                    ]
        
        # Get focus display info
        focus_info_raw = categorizer.get_focus_display_info()
        focus_info = {
            focus: FocusInfo(**info) for focus, info in focus_info_raw.items()
        }
        
        response_data = {
            'categories': serialized_categories,
            'focus_info': focus_info,
            'total_classes': sum(
                len(class_list) 
                for focus_groups in filtered_categories.values() 
                for class_list in focus_groups.values()
            ),
            'include_unplayable': include_unplayable
        }
        
        return CategorizedClassesResponse(**response_data)
        
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
    request: dict  # Request body parameter
):
    """Add a new class to character (for multiclassing)"""
    session = char_session
    manager = session.character_manager
    
    try:
        class_manager = manager.get_manager('class')
        if not class_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Class manager not available"
            )
        
        # Validate class ID exists
        class_id = request.get('class_id')
        if class_id is None or not hasattr(class_manager.game_data_loader, 'get_by_id') or not class_manager.game_data_loader.get_by_id('classes', class_id):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Invalid class ID: {class_id}"
            )
        
        # Use class manager method to add class level (handles new classes too)
        changes = class_manager.add_class_level(class_id, request.get('cheat_mode', False))
        
        return {
            'success': True,
            'message': 'Class added successfully',
            'changes': changes or {},
            'has_unsaved_changes': session.has_unsaved_changes()
        }
        
    except Exception as e:
        logger.error(f"Failed to add class {class_id} to character {character_id}: {e}")
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
    """Remove a class from multiclass character"""
    session = char_session
    manager = session.character_manager
    
    try:
        class_manager = manager.get_manager('class')
        if not class_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Class manager not available"
            )
        
        # Use manager method directly
        result = class_manager.remove_class(class_id)
        
        return {
            'success': True,
            'message': 'Class removed successfully',
            'changes': result,
            'has_unsaved_changes': session.has_unsaved_changes()
        }
        
    except Exception as e:
        logger.error(f"Failed to remove class {class_id} from character {character_id}: {e}")
        logger.error(f"Failed to remove class for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to remove class: {str(e)}"
        )


@router.get("/characters/{character_id}/classes/validate")
def validate_classes(
    character_id: int,
    manager: CharacterManagerDep
):
    """Validate current class configuration"""
    
    try:
        class_manager = manager.get_manager('class')
        if not class_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Class manager not available"
            )
        
        # Use manager method directly
        is_valid, errors = class_manager.validate()
        
        return {
            'valid': is_valid,
            'errors': errors
        }
        
    except Exception as e:
        logger.error(f"Failed to validate classes for character {character_id}: {e}")
        logger.error(f"Failed to validate classes for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to validate classes: {str(e)}"
        )


@router.get("/characters/{character_id}/classes/progression/{class_id}")
def get_class_progression_summary(
    character_id: int,
    class_id: int,
    manager: CharacterManagerDep,
    max_level: int = Query(20, description="Maximum level to show")
):
    """Get detailed class progression summary"""
    
    try:
        class_manager = manager.get_manager('class')
        if not class_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Class manager not available"
            )
        
        # Use manager method directly
        progression = class_manager.get_class_progression_summary(class_id, max_level)
        
        return progression
        
    except Exception as e:
        logger.error(f"Failed to get class progression for class {class_id}: {e}")
        logger.error(f"Failed to get class progression summary for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get class progression summary: {str(e)}"
        )


@router.get("/characters/{character_id}/classes/prestige-options")
def get_prestige_options(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get available prestige class options for character"""
    
    try:
        class_manager = manager.get_manager('class')
        if not class_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Class manager not available"
            )
        
        # Use manager method directly
        prestige_options = class_manager.get_prestige_class_options()
        
        return {
            'prestige_classes': prestige_options
        }
        
    except Exception as e:
        logger.error(f"Failed to get prestige options for character {character_id}: {e}")
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
    """Check if character has levels in a specific class by name"""
    
    try:
        class_manager = manager.get_manager('class')
        if not class_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Class manager not available"
            )
        
        # Use manager method directly
        has_class = class_manager.has_class_by_name(class_name)
        class_level = class_manager.get_class_level_by_name(class_name) if has_class else 0
        
        return {
            'has_class': has_class,
            'class_level': class_level,
            'class_name': class_name
        }
        
    except Exception as e:
        logger.error(f"Failed to check class {class_name} for character {character_id}: {e}")
        logger.error(f"Failed to check class by name for character {character_id}: {e}")
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
    """Get level information for a class including max level and remaining levels"""
    
    try:
        class_manager = manager.get_manager('class')
        if not class_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Class manager not available"
            )
        
        # Use manager method directly
        level_info = class_manager.get_class_level_info(class_id)
        
        return level_info
        
    except Exception as e:
        logger.error(f"Failed to get level info for class {class_id} for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get class level info: {str(e)}"
        )


@router.get("/classes/features/{class_id}")
def get_class_features(
    class_id: int,
    max_level: int = Query(20, description="Maximum level to show progression for"),
    include_spells: bool = Query(True, description="Include spell progression tables"),
    include_proficiencies: bool = Query(True, description="Include weapon/armor proficiencies"),
    character_id: Optional[int] = Query(None, description="Character ID for personalized data")
):
    """
    Get detailed class features and progression for a specific class
    """
    from fastapi_models.class_models import ClassFeaturesResponse
    try:
        # Get game data loader
        game_data_loader = get_game_data_loader()
        if not game_data_loader:
            raise HTTPException(
                status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
                detail="Game data not available"
            )
        
        # Get class data
        class_data = game_data_loader.get_by_id('classes', class_id)
        if not class_data:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Class with ID {class_id} not found"
            )
        
        # Use ClassCategorizer for class features - no duplicated logic
        categorizer = ClassCategorizer(game_data_loader)
        class_info = categorizer._create_simple_class_info(class_data, class_id)
        
        progression_data = {
            'class_id': class_id,
            'class_name': class_info.name if class_info else 'Unknown Class',
            'basic_info': {
                'hit_die': class_info.hit_die if class_info else 8,
                'skill_points_per_level': class_info.skill_points if class_info else 2,
                'is_spellcaster': class_info.is_spellcaster if class_info else False,
                'spell_type': 'arcane' if class_info and class_info.has_arcane else ('divine' if class_info and class_info.has_divine else 'none')
            },
            'description': class_info.parsed_description.__dict__ if class_info and class_info.parsed_description else {},
            'max_level_shown': max_level
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


# Helper functions

def _serialize_class_info(class_info):  # Return type removed for lazy loading
    """Serialize ClassInfo object for API response"""
    from fastapi_models.class_models import ClassInfo
    # Serialize parsed description if available
    parsed_desc = None
    if class_info.parsed_description:
        parsed_desc = {
            'title': class_info.parsed_description.title,
            'class_type': class_info.parsed_description.class_type,
            'summary': class_info.parsed_description.summary,
            'restrictions': class_info.parsed_description.restrictions,
            'requirements': class_info.parsed_description.requirements,
            'features': class_info.parsed_description.features,
            'abilities': class_info.parsed_description.abilities,
            'html': class_info.parsed_description.raw_html
        }
    
    return ClassInfo(
        id=class_info.id,
        name=class_info.name,
        label=class_info.label,
        type=class_info.class_type.value,
        focus=class_info.focus.value,
        max_level=class_info.max_level,
        hit_die=class_info.hit_die,
        skill_points=class_info.skill_points,
        is_spellcaster=class_info.is_spellcaster,
        has_arcane=class_info.has_arcane,
        has_divine=class_info.has_divine,
        primary_ability=class_info.primary_ability,
        bab_progression=class_info.bab_progression,
        alignment_restricted=class_info.alignment_restricted,
        description=class_info.description,
        parsed_description=parsed_desc,
        prerequisites=class_info.prerequisites
    )


def _get_character_class_context(manager, categorizer) -> Optional[Dict[str, Any]]:
    """Get character-specific class context using manager methods only"""
    context = {}
    
    try:
        # Use class manager methods - no duplicated logic
        class_manager = manager.get_manager('class')
        class_summary = class_manager.get_class_summary()
        context['current_classes'] = class_summary
        
        # Use manager methods for prestige requirements if available
        if hasattr(class_manager, 'get_prestige_class_options'):
            prestige_options = class_manager.get_prestige_class_options()
            context['prestige_requirements'] = prestige_options
        
        context['can_multiclass'] = class_summary.get('can_multiclass', True)
        context['multiclass_slots_used'] = len(class_summary.get('classes', []))
        
    except Exception as e:
        logger.warning(f"Error getting character class context: {e}")
        context['error'] = str(e)
    
    return context