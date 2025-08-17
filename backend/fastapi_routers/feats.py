"""
Feats router - Complete feat management endpoints
Handles feat selection, prerequisites, and feat management
"""

import logging
import time
from typing import List, Optional
from functools import wraps
from fastapi import APIRouter, Depends, HTTPException, status, Query
# Simplified caching - can be replaced with proper caching later
class SimpleCache:
    def __init__(self):
        self._cache = {}
    
    def get(self, key, default=None):
        return self._cache.get(key, default)
    
    def set(self, key, value, timeout=None):
        self._cache[key] = value

# Simple in-memory cache replacement for Django cache
cache = SimpleCache()

from fastapi_routers.dependencies import (
    get_character_manager, 
    get_character_session,
    CharacterManagerDep,
    CharacterSessionDep
)
# from fastapi_models import (...) - moved to lazy loading

logger = logging.getLogger(__name__)

router = APIRouter()


def _invalidate_feat_cache(character_id: int):
    """Helper function to invalidate feat cache - no duplicated logic"""
    cache_version_key = f"feat_cache_version:char_{character_id}"
    current_version = cache.get(cache_version_key, 0)
    cache.set(cache_version_key, current_version + 1, timeout=3600)
    logger.debug(f"Invalidated feat cache for character {character_id}")


def _get_feat_cache_key(character_id: int, operation: str, **params):
    """Helper function to generate feat cache keys - no duplicated logic"""
    cache_version_key = f"feat_cache_version:char_{character_id}"
    cache_version = cache.get(cache_version_key, 0)
    
    cache_key_parts = [
        f"feat_{operation}",
        f"char_{character_id}",
        f"v_{cache_version}"
    ]
    
    # Add parameters to cache key
    for key, value in params.items():
        if value is not None:
            cache_key_parts.append(f"{key}_{value}")
    
    return ":".join(cache_key_parts)


def log_performance(func):
    """Decorator to log performance metrics for FastAPI endpoints"""
    @wraps(func)
    def wrapper(*args, **kwargs):
        start_time = time.time()
        result = None
        try:
            result = func(*args, **kwargs)  # Remove await since functions are sync
            return result
        finally:
            end_time = time.time()
            duration_ms = (end_time - start_time) * 1000
            
            # Log performance data
            endpoint_name = func.__name__
            status_code = getattr(result, 'status_code', 'success') if result else 'error'
            logger.info(f"{endpoint_name}: {duration_ms:.2f}ms (status: {status_code})")
    return wrapper


@router.get("/characters/{character_id}/feats/state")
@log_performance
def get_feats_state(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get current feats state (no expensive available_feats calculation)"""
    from fastapi_models import FeatState
    try:
        feat_manager = manager.get_manager('feat')
        
        # Use existing manager methods to build feat state (fast version - no expensive operations)
        feat_summary = feat_manager.get_feat_summary_fast()
        all_feats = feat_manager.get_all_feats()
        # Skip expensive operations for state endpoint
        available_feats = []
        legitimate_feats = []
        feat_chains = {}
        
        feat_state = {
            'summary': feat_summary,
            'all_feats': all_feats,
            'available_feats': available_feats,
            'legitimate_feats': legitimate_feats,
            'feat_chains': feat_chains,
            'recommended_feats': []
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
    """Get feats available for selection based on current state"""
    from fastapi_models import AvailableFeatsResponse
    try:
        feat_manager = manager.get_manager('feat')
        
        # Use feat manager method - no duplicated logic
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
    feat_type: Optional[int] = Query(None, description="Filter by feat type"),
    category: str = Query("", description="Filter by category"),
    subcategory: str = Query("", description="Filter by subcategory"),
    page: int = Query(1, ge=1, description="Page number"),
    limit: int = Query(50, ge=1, le=200, description="Items per page"),
    search: str = Query("", description="Search term")
):
    """Get legitimate feats with pagination and smart filtering (no validation for performance)"""
    from fastapi_models import LegitimateFeatsResponse
    # Use helper function to generate cache key - no duplicated logic
    cache_key = _get_feat_cache_key(
        character_id, 
        "legitimate",
        category=category.lower(),
        subcategory=subcategory.lower(),
        feat_type=feat_type,
        page=page,
        limit=limit
    )
    
    # Try to get from cache first
    cached_response = cache.get(cache_key)
    if cached_response is not None:
        logger.debug(f"Returning cached feat response for key: {cache_key}")
        return LegitimateFeatsResponse(**cached_response)
    
    try:
        feat_manager = manager.get_manager('feat')
        
        # Use existing manager method and apply pagination manually
        if category or subcategory:
            all_feats = feat_manager.get_legitimate_feats_by_category(
                category=category.lower(),
                subcategory=subcategory.lower(),
                feat_type=feat_type
            )
        else:
            all_feats = feat_manager.get_legitimate_feats(feat_type=feat_type)
        
        # Apply search filter if provided
        if search.strip():
            search_lower = search.strip().lower()
            all_feats = [f for f in all_feats if search_lower in f.get('name', '').lower() or search_lower in f.get('label', '').lower()]
        
        # Apply pagination
        total = len(all_feats)
        start_idx = (page - 1) * limit
        end_idx = start_idx + limit
        feats = all_feats[start_idx:end_idx]
        pages = (total + limit - 1) // limit
        
        response_data = {
            'feats': feats,
            'total': total,
            'page': page,
            'pages': pages,
            'limit': limit
        }
        
        # Cache the response for 5 minutes
        cache.set(cache_key, response_data, timeout=300)
        logger.debug(f"Cached feat response for key: {cache_key}")
        
        return LegitimateFeatsResponse(**response_data)
        
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
    feat_request,  # Type removed for lazy loading
    char_session: CharacterSessionDep
):
    """Add a feat to character"""
    from fastapi_models import FeatAddRequest, FeatAddResponse
    try:
        session = char_session
        manager = session.character_manager
        feat_manager = manager.get_manager('feat')
        
        # Check if feat can be added (only if not ignoring prerequisites)
        if not feat_request.ignore_prerequisites:
            can_take, reason = feat_manager.can_take_feat(feat_request.feat_id)
            if not can_take:
                raise HTTPException(
                    status_code=status.HTTP_400_BAD_REQUEST,
                    detail=f'Cannot add feat: {reason}'
                )
        
        result = feat_manager.add_feat(feat_request.feat_id)
        if not result:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail='Failed to add feat'
            )
        
        # Get feat info and updated summary
        feat_info = feat_manager.get_feat_info(feat_request.feat_id)
        if not feat_info:
            feat_info = {'id': feat_request.feat_id, 'name': f'Feat {feat_request.feat_id}', 'label': f'Feat {feat_request.feat_id}'}
        feat_summary = feat_manager.get_feat_summary_fast() or {}
        
        # Use helper function to invalidate cache - no duplicated logic
        _invalidate_feat_cache(character_id)
        
        return FeatAddResponse(
            message='Feat added successfully',
            feat_info=feat_info,
            feat_summary=feat_summary,
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
    feat_request,  # Type removed for lazy loading
    char_session: CharacterSessionDep
):
    """Remove a feat from character"""
    from fastapi_models import FeatRemoveRequest, FeatRemoveResponse
    try:
        session = char_session
        manager = session.character_manager
        feat_manager = manager.get_manager('feat')
        
        # Check if feat is protected (unless force removal)
        if not feat_request.force and feat_manager.is_feat_protected(feat_request.feat_id):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail='Cannot remove this feat (granted by class/race or required by other feats)'
            )
        
        # Get feat info before removal
        removed_feat = feat_manager.get_feat_info(feat_request.feat_id)
        if not removed_feat:
            removed_feat = {'id': feat_request.feat_id, 'name': f'Feat {feat_request.feat_id}', 'label': f'Feat {feat_request.feat_id}'}
        
        result = feat_manager.remove_feat(feat_request.feat_id)
        if not result:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail='Failed to remove feat'
            )
        
        # Get updated feat summary
        feat_summary = feat_manager.get_feat_summary_fast() or {}
        
        # Use helper function to invalidate cache - no duplicated logic
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
    """Get prerequisites for a specific feat"""
    from fastapi_models import FeatPrerequisites
    try:
        feat_manager = manager.get_manager('feat')
        
        can_take, missing_reqs = feat_manager.get_feat_prerequisites_info(feat_id)
        feat_info = feat_manager.get_feat_info(feat_id)
        
        # Get actual prerequisites from feat info
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
    """Check if character meets prerequisites for a feat"""
    from fastapi_models import FeatValidationResponse
    try:
        feat_manager = manager.get_manager('feat')
        
        can_take, reason = feat_manager.can_take_feat(feat_id)
        has_feat = feat_manager.has_feat(feat_id)
        feat_info = feat_manager.get_feat_info(feat_id) or {}
        feat_name = feat_info.get('name', f'Feat {feat_id}')
        
        # Get prerequisites structure
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
    """Get detailed information about a specific feat including description and prerequisites"""
    from fastapi_models import FeatDetails
    try:
        feat_manager = manager.get_manager('feat')
        
        # Get detailed feat information
        feat_info = feat_manager.get_feat_info(feat_id)
        if not feat_info:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f'Feat {feat_id} not found'
            )
        
        # Get detailed prerequisites
        detailed_prereqs = feat_manager.get_detailed_prerequisites(feat_id) or {}
        
        # Combine information
        detailed_feat = {
            **feat_info,
            'detailed_prerequisites': detailed_prereqs
        }
        
        return FeatDetails(**detailed_feat)
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to get feat details for character {character_id}, feat {feat_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get feat details: {str(e)}"
        )


@router.get("/characters/{character_id}/feats/{feat_id}/validate")
@log_performance
def validate_feat(
    character_id: int,
    feat_id: int,
    manager: CharacterManagerDep
):
    """
    Validate if character can take a specific feat (on-demand validation).
    This is the performance-optimized replacement for checking prerequisites
    during list loading. Call this when user hovers/clicks on a feat.
    """
    from fastapi_models import FeatValidationResponse, FeatPrerequisites
    try:
        feat_manager = manager.get_manager('feat')
        
        # Check if character already has the feat
        if feat_manager.has_feat(feat_id):
            return FeatValidationResponse(
                feat_id=feat_id,
                feat_name="",  # Will be filled by manager
                can_take=False,
                has_feat=True,
                prerequisites=FeatPrerequisites(),
                missing_requirements=['Already has this feat']
            )
        
        # Use helper function to generate cache key - no duplicated logic
        cache_key = _get_feat_cache_key(character_id, "validate", feat_id=feat_id)
        
        # Try to get cached validation result
        cached_result = cache.get(cache_key)
        if cached_result is not None:
            logger.debug(f"Returning cached validation for feat {feat_id}")
            return FeatValidationResponse(**cached_result)
        
        # Perform validation
        can_take, reason = feat_manager.can_take_feat(feat_id)
        
        # Get feat info for name and prerequisites
        feat_info = feat_manager.get_feat_info(feat_id) or {}
        feat_name = feat_info.get('name', f'Feat {feat_id}')
        prereqs = feat_info.get('prerequisites', {})
        
        # Build response
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
        
        # Cache the result for 5 minutes
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
    """Get feats organized by category (fast display version, no validation)"""
    from fastapi_models import FeatsByCategoryResponse
    try:
        feat_manager = manager.get_manager('feat')
        
        # Use existing manager method to get categories
        categories = feat_manager.get_feat_categories_fast() or {}
        total_feats = sum(len(feats) for feats in categories.values() if feats)
        
        # Get character level for context
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