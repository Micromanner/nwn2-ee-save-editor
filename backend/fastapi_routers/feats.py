"""Feats router - Complete feat management endpoints."""

from typing import List, Optional
from fastapi import APIRouter, Depends, HTTPException, status, Query, Body
from loguru import logger

from fastapi_routers.dependencies import (
    get_character_manager,
    get_character_session,
    CharacterManagerDep,
    CharacterSessionDep
)
from fastapi_routers.utils import cache, log_performance

router = APIRouter()


def _invalidate_feat_cache(character_id: int):
    """Invalidate feat cache for a specific character."""
    cache_version_key = f"feat_cache_version:char_{character_id}"
    current_version = cache.get(cache_version_key, 0)
    cache.set(cache_version_key, current_version + 1, timeout=3600)
    logger.debug(f"Invalidated feat cache for character {character_id}")


def _get_feat_cache_key(character_id: int, operation: str, **params):
    """Generate a unique cache key for feat operations."""
    cache_version_key = f"feat_cache_version:char_{character_id}"
    cache_version = cache.get(cache_version_key, 0)
    
    cache_key_parts = [
        f"feat_{operation}",
        f"char_{character_id}",
        f"v_{cache_version}"
    ]
    
    for key, value in params.items():
        if value is not None:
            cache_key_parts.append(f"{key}_{value}")
    
    return ":".join(cache_key_parts)


@router.get("/characters/{character_id}/feats/state")
@log_performance
def get_feats_state(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get current feats state."""
    from fastapi_models import FeatState
    try:
        feat_manager = manager.get_manager('feat')
        
        feat_summary = feat_manager.get_feat_summary_fast()
        all_feats = feat_manager.get_all_feats()
        point_summary = feat_manager.get_feat_points_summary()

        feat_state = {
            'summary': feat_summary,
            'all_feats': all_feats,
            'available_feats': [],
            'legitimate_feats': [],
            'recommended_feats': [],
            'point_summary': point_summary
        }
        
        return FeatState(**feat_state)
        
    except Exception as e:
        logger.error(f"Failed to get feats state for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get feats state: {str(e)}"
        )


@router.get("/characters/{character_id}/feats/available")
def get_available_feats(
    character_id: int,
    manager: CharacterManagerDep,
    feat_type: Optional[int] = Query(None, description="Filter by feat type")
):
    """Get feats available for selection based on current state."""
    from fastapi_models import AvailableFeatsResponse
    try:
        feat_manager = manager.get_manager('feat')
        available = feat_manager.get_available_feats(feat_type=feat_type)
        
        return AvailableFeatsResponse(
            available_feats=available,
            total=len(available)
        )
        
    except Exception as e:
        logger.error(f"Failed to get available feats for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get available feats: {str(e)}"
        )


@router.get("/characters/{character_id}/feats/legitimate")
@log_performance
def get_legitimate_feats(
    character_id: int,
    manager: CharacterManagerDep,
    feat_type: Optional[int] = Query(None, description="Filter by feat type (bitflag)"),
    page: int = Query(1, ge=1, description="Page number"),
    limit: int = Query(50, ge=1, le=200, description="Items per page"),
    search: str = Query("", description="Search term")
):
    """Get legitimate feats with pagination and smart filtering."""
    from fastapi_models import LegitimateFeatsResponse

    try:
        feat_type_int = int(feat_type) if feat_type not in (None, '', 'null', 'undefined') else None
    except (ValueError, TypeError):
        logger.warning(f"Invalid feat_type value: {feat_type}, defaulting to None")
        feat_type_int = None

    cache_key = _get_feat_cache_key(
        character_id,
        "legitimate",
        feat_type=feat_type_int,
        page=page,
        limit=limit,
        search=search.lower()
    )

    cached_response = cache.get(cache_key)
    if cached_response is not None:
        logger.info(f"Returning cached feat response for key: {cache_key}, {len(cached_response.get('feats', []))} feats")
        return LegitimateFeatsResponse(**cached_response)

    try:
        feat_manager = manager.get_manager('feat')

        logger.info(f"get_legitimate_feats endpoint: character_id={character_id}, page={page}, limit={limit}, search='{search}', feat_type={feat_type_int}")

        result = feat_manager.get_legitimate_feats(
            feat_type=feat_type_int,
            search=search.strip() if search.strip() else None,
            page=page,
            limit=limit
        )

        logger.info(f"get_legitimate_feats: Received {len(result['feats'])} feats, total={result['pagination']['total']}, page={result['pagination']['page']}")

        cache.set(cache_key, result, timeout=300)
        logger.debug(f"Cached feat response for key: {cache_key}")

        return LegitimateFeatsResponse(**result)

    except Exception as e:
        logger.error(f"Failed to get legitimate feats for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get legitimate feats: {str(e)}"
        )


@router.post("/characters/{character_id}/feats/add")
@log_performance
def add_feat(
    character_id: int,
    char_session: CharacterSessionDep,
    feat_request: dict = Body(...)
):
    """Add a feat to character."""
    from fastapi_models import FeatAddRequest, FeatAddResponse

    feat_request = FeatAddRequest(**feat_request)
    try:
        session = char_session
        manager = session.character_manager
        feat_manager = manager.get_manager('feat')

        auto_add_prereqs = not feat_request.ignore_prerequisites
        success, auto_added_feats = feat_manager.add_feat_with_prerequisites(
            feat_request.feat_id,
            auto_add_prerequisites=auto_add_prereqs
        )

        if not success:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail='Failed to add feat'
            )

        feat_info = feat_manager.get_feat_info(feat_request.feat_id)
        if not feat_info:
            feat_info = {'id': feat_request.feat_id, 'name': f'Feat {feat_request.feat_id}', 'label': f'Feat {feat_request.feat_id}'}
        feat_summary = feat_manager.get_feat_summary_fast() or {}

        _invalidate_feat_cache(character_id)

        message = 'Feat added successfully'
        if auto_added_feats:
            added_feats = [f for f in auto_added_feats if f.get('type') == 'feat']
            increased_abilities = [f for f in auto_added_feats if f.get('type') == 'ability']

            changes = []
            if increased_abilities:
                ability_changes = ', '.join([f['label'] for f in increased_abilities])
                changes.append(f"abilities: {ability_changes}")
            if added_feats:
                feat_names = ', '.join([f['label'] or f['name'] for f in added_feats])
                changes.append(f"feats: {feat_names}")

            if changes:
                message = f"Feat added successfully (also added {'; '.join(changes)})"

        return FeatAddResponse(
            message=message,
            feat_info=feat_info,
            feat_summary=feat_summary,
            cascading_effects=auto_added_feats,
            has_unsaved_changes=getattr(session, 'has_unsaved_changes', lambda: True)()
        )
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to add feat for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to add feat: {str(e)}"
        )


@router.post("/characters/{character_id}/feats/remove")
def remove_feat(
    character_id: int,
    char_session: CharacterSessionDep,
    feat_request: dict = Body(...)
):
    """Remove a feat from character."""
    from fastapi_models import FeatRemoveRequest, FeatRemoveResponse

    feat_request = FeatRemoveRequest(**feat_request)
    try:
        session = char_session
        manager = session.character_manager
        feat_manager = manager.get_manager('feat')
        
        if not feat_request.force and feat_manager.is_feat_protected(feat_request.feat_id):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail='Cannot remove this feat (granted by class/race or required by other feats)'
            )
        
        removed_feat = feat_manager.get_feat_info(feat_request.feat_id)
        if not removed_feat:
            removed_feat = {'id': feat_request.feat_id, 'name': f'Feat {feat_request.feat_id}', 'label': f'Feat {feat_request.feat_id}'}
        
        result = feat_manager.remove_feat(feat_request.feat_id)
        if not result:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail='Failed to remove feat'
            )
        
        feat_summary = feat_manager.get_feat_summary_fast() or {}
        
        _invalidate_feat_cache(character_id)
        
        return FeatRemoveResponse(
            message='Feat removed successfully',
            removed_feat=removed_feat,
            feat_summary=feat_summary,
            has_unsaved_changes=getattr(session, 'has_unsaved_changes', lambda: True)()
        )
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to remove feat for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to remove feat: {str(e)}"
        )


@router.get("/characters/{character_id}/feats/{feat_id}/prerequisites")
def get_feat_prerequisites(
    character_id: int,
    feat_id: int,
    manager: CharacterManagerDep
):
    """Get prerequisites for a specific feat."""
    from fastapi_models import FeatPrerequisites
    try:
        feat_manager = manager.get_manager('feat')
        
        can_take, missing_reqs = feat_manager.get_feat_prerequisites_info(feat_id)
        feat_info = feat_manager.get_feat_info(feat_id)
        
        prereqs = feat_info.get('prerequisites', {}) if feat_info else {}
        if not feat_info:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f'Feat {feat_id} not found'
            )
        
        return FeatPrerequisites(
            abilities=prereqs.get('abilities', {}),
            feats=prereqs.get('feats', []),
            class_=prereqs.get('class', -1),
            level=prereqs.get('level', 0),
            bab=prereqs.get('bab', 0),
            spell_level=prereqs.get('spell_level', 0)
        )
        
    except Exception as e:
        logger.error(f"Failed to get feat prerequisites for character {character_id}, feat {feat_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get feat prerequisites: {str(e)}"
        )


@router.get("/characters/{character_id}/feats/{feat_id}/check")
def check_feat_prerequisites(
    character_id: int,
    feat_id: int,
    manager: CharacterManagerDep
):
    """Check if character meets prerequisites for a feat."""
    from fastapi_models import FeatValidationResponse, FeatPrerequisites
    try:
        feat_manager = manager.get_manager('feat')
        
        can_take, reason = feat_manager.can_take_feat(feat_id)
        has_feat = feat_manager.has_feat(feat_id)
        feat_info = feat_manager.get_feat_info(feat_id) or {}
        feat_name = feat_info.get('name', f'Feat {feat_id}')
        
        prereqs = feat_info.get('prerequisites', {})
        
        return FeatValidationResponse(
            feat_id=feat_id,
            feat_name=feat_name,
            can_take=can_take,
            has_feat=has_feat,
            prerequisites=FeatPrerequisites(
                abilities=prereqs.get('abilities', {}),
                feats=prereqs.get('feats', []),
                class_=prereqs.get('class', -1),
                level=prereqs.get('level', 0),
                bab=prereqs.get('bab', 0),
                spell_level=prereqs.get('spell_level', 0)
            ),
            missing_requirements=[reason] if not can_take else []
        )
        
    except Exception as e:
        logger.error(f"Failed to check feat prerequisites for character {character_id}, feat {feat_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to check feat prerequisites: {str(e)}"
        )


@router.get("/characters/{character_id}/feats/{feat_id}/details")
def get_feat_details(
    character_id: int,
    feat_id: int,
    manager: CharacterManagerDep
):
    """Get detailed information about a specific feat."""
    try:
        feat_manager = manager.get_manager('feat')
        
        feat_info = feat_manager.get_feat_info(feat_id)
        if not feat_info:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f'Feat {feat_id} not found'
            )
        
        detailed_prereqs = feat_manager.get_detailed_prerequisites(feat_id) or {}

        detailed_feat = {
            **feat_info,
            'detailed_prerequisites': detailed_prereqs
        }

        return detailed_feat
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to get feat details for character {character_id}, feat {feat_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get feat details: {str(e)}"
        )


@router.post("/characters/{character_id}/domains/add")
def add_domain(
    character_id: int,
    char_session: CharacterSessionDep,
    domain_request: dict = Body(...)
):
    """Add a domain to character."""
    try:
        domain_id = domain_request.get('domain_id')
        if domain_id is None:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail='domain_id is required'
            )

        session = char_session
        manager = session.character_manager
        feat_manager = manager.get_manager('feat')

        result = feat_manager.add_domain(domain_id)

        _invalidate_feat_cache(character_id)

        return {
            'message': f"Domain '{result['domain_name']}' added successfully",
            'domain_info': result,
            'has_unsaved_changes': getattr(session, 'has_unsaved_changes', lambda: True)()
        }

    except ValueError as e:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=str(e)
        )
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to add domain for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to add domain: {str(e)}"
        )


@router.delete("/characters/{character_id}/domains/{domain_id}")
def remove_domain(
    character_id: int,
    domain_id: int,
    char_session: CharacterSessionDep
):
    """Remove a domain from character."""
    try:
        session = char_session
        manager = session.character_manager
        feat_manager = manager.get_manager('feat')

        result = feat_manager.remove_domain(domain_id)

        _invalidate_feat_cache(character_id)

        return {
            'message': f"Domain '{result['domain_name']}' removed successfully",
            'domain_info': result,
            'has_unsaved_changes': getattr(session, 'has_unsaved_changes', lambda: True)()
        }

    except ValueError as e:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=str(e)
        )
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to remove domain for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to remove domain: {str(e)}"
        )


@router.get("/characters/{character_id}/domains/available")
def get_available_domains(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get list of available domains."""
    try:
        feat_manager = manager.get_manager('feat')
        domains = feat_manager.get_available_domains()
        
        return {'domains': domains}

    except Exception as e:
        logger.error(f"Failed to get available domains for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get available domains: {str(e)}"
        )


@router.get("/characters/{character_id}/feats/{feat_id}/validate")
@log_performance
def validate_feat(
    character_id: int,
    feat_id: int,
    manager: CharacterManagerDep
):
    """Validate if character can take a specific feat."""
    from fastapi_models import FeatValidationResponse, FeatPrerequisites
    try:
        feat_manager = manager.get_manager('feat')
        
        if feat_manager.has_feat(feat_id):
            return FeatValidationResponse(
                feat_id=feat_id,
                feat_name="",
                can_take=False,
                has_feat=True,
                prerequisites=FeatPrerequisites(),
                missing_requirements=['Already has this feat']
            )
        
        cache_key = _get_feat_cache_key(character_id, "validate", feat_id=feat_id)
        
        cached_result = cache.get(cache_key)
        if cached_result is not None:
            logger.debug(f"Returning cached validation for feat {feat_id}")
            return FeatValidationResponse(**cached_result)
        
        can_take, reason = feat_manager.can_take_feat(feat_id)
        
        feat_info = feat_manager.get_feat_info(feat_id) or {}
        feat_name = feat_info.get('name', f'Feat {feat_id}')
        prereqs = feat_info.get('prerequisites', {})
        
        validation_result = {
            'feat_id': feat_id,
            'feat_name': feat_name,
            'can_take': can_take,
            'has_feat': feat_manager.has_feat(feat_id),
            'prerequisites': {
                'abilities': prereqs.get('abilities', {}),
                'feats': prereqs.get('feats', []),
                'class_': prereqs.get('class', -1),
                'level': prereqs.get('level', 0),
                'bab': prereqs.get('bab', 0),
                'spell_level': prereqs.get('spell_level', 0)
            },
            'missing_requirements': [reason] if not can_take else []
        }
        
        cache.set(cache_key, validation_result, timeout=300)
        
        return FeatValidationResponse(**validation_result)
        
    except Exception as e:
        logger.error(f"Failed to validate feat for character {character_id}, feat {feat_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to validate feat: {str(e)}"
        )


@router.get("/characters/{character_id}/feats/by-category")
@log_performance
def get_feats_by_category(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get feats organized by category."""
    from fastapi_models import FeatsByCategoryResponse
    try:
        feat_manager = manager.get_manager('feat')
        
        categories = feat_manager.get_feat_categories_fast() or {}
        total_feats = sum(len(feats) for feats in categories.values() if feats)
        
        class_list = manager.gff.get('ClassList', [])
        character_level = sum(c.get('ClassLevel', 0) for c in class_list)
        
        categories_response = {
            'categories': categories,
            'total_feats': total_feats,
            'character_level': character_level
        }
        
        return FeatsByCategoryResponse(**categories_response)
        
    except Exception as e:
        logger.error(f"Failed to get feats by category for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get feats by category: {str(e)}"
        )