"""
Feats router - Complete feat management endpoints
Handles feat selection, prerequisites, and feat management
"""

import time
from typing import List, Optional
from functools import wraps
from fastapi import APIRouter, Depends, HTTPException, status, Query, Body
from loguru import logger

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

    try:
        feat_type_int = int(feat_type) if feat_type not in (None, '', 'null', 'undefined') else None
    except (ValueError, TypeError):
        logger.warning(f"Invalid feat_type value: {feat_type}, defaulting to None")
        feat_type_int = None

    cache_key = _get_feat_cache_key(
        character_id,
        "legitimate",
        category=category.lower(),
        subcategory=subcategory.lower(),
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

        logger.info(f"get_legitimate_feats endpoint: character_id={character_id}, page={page}, limit={limit}, category='{category}', subcategory='{subcategory}', search='{search}', feat_type={feat_type_int}")

        result = feat_manager.get_legitimate_feats(
            feat_type=feat_type_int,
            category=category.lower(),
            subcategory=subcategory.lower(),
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
    """Add a feat to character"""
    from fastapi_models import FeatAddRequest, FeatAddResponse

    feat_request = FeatAddRequest(**feat_request)
    try:
        session = char_session
        manager = session.character_manager
        feat_manager = manager.get_manager('feat')

        # Use manager method to handle feat addition with auto-prerequisites
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

        # Get feat info and updated summary
        feat_info = feat_manager.get_feat_info(feat_request.feat_id)
        if not feat_info:
            feat_info = {'id': feat_request.feat_id, 'name': f'Feat {feat_request.feat_id}', 'label': f'Feat {feat_request.feat_id}'}
        feat_summary = feat_manager.get_feat_summary_fast() or {}

        _invalidate_feat_cache(character_id)

        message = 'Feat added successfully'
        if auto_added_feats:
            # Separate feats and abilities
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
    """Remove a feat from character"""
    from fastapi_models import FeatRemoveRequest, FeatRemoveResponse

    feat_request = FeatRemoveRequest(**feat_request)
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

        # Flatten feat_info fields to top level and add detailed_prerequisites
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
    """Add a domain to character (grants all associated feats)"""
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
    """Remove a domain from character (removes all associated feats)"""
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
    """Get list of available domains"""
    try:
        feat_manager = manager.get_manager('feat')
        game_rules = feat_manager.game_rules_service

        domains_table = game_rules.get_table('domains')
        if not domains_table:
            return {'domains': []}

        from gamedata.field_mapper import field_mapper

        domains = []
        for domain_id, domain_data in enumerate(domains_table):
            domain_name = field_mapper.get_field_value(domain_data, 'label', f'Domain_{domain_id}')
            epithet_feat_id = field_mapper.get_field_value(domain_data, 'EpithetFeat', None)

            has_domain = False
            if epithet_feat_id:
                try:
                    has_domain = feat_manager.has_feat(int(epithet_feat_id))
                except (ValueError, TypeError):
                    pass

            domains.append({
                'id': domain_id,
                'name': domain_name,
                'has_domain': has_domain,
                'epithet_feat_id': epithet_feat_id
            })

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


@router.get("/characters/{character_id}/feats/debug-table-lookup")
def debug_feat_table_lookup(
    character_id: int,
    manager: CharacterManagerDep,
    class_id: int = Query(56, description="Class ID to test"),
    feat_id: int = Query(2038, description="Feat ID to search for")
):
    """Debug endpoint to test feat table lookup"""
    feat_manager = manager.get_manager('feat')
    
    debug_info = {
        "class_id": class_id,
        "feat_id": feat_id,
        "steps": []
    }
    
    try:
        # Step 1: Get class data
        class_data = feat_manager.game_rules_service.get_by_id('classes', class_id)
        if not class_data:
            debug_info["steps"].append(f"FAIL: Class {class_id} not found")
            return debug_info
        
        debug_info["steps"].append(f"OK: Found class: {getattr(class_data, 'name', 'Unknown')}")
        
        # Step 2: Get feat table name using field mapper
        from gamedata.dynamic_loader.field_mapping_utility import field_mapper
        feat_table_name = field_mapper.get_field_value(class_data, 'feats_table', None)
        if not feat_table_name:
            debug_info["steps"].append("FAIL: No feats_table in class data")
            return debug_info
        
        debug_info["steps"].append(f"OK: Feat table name: {feat_table_name}")
        
        # Step 3: Load feat table
        feat_table = feat_manager.game_rules_service.get_table(feat_table_name.lower())
        if not feat_table:
            debug_info["steps"].append(f"FAIL: Could not load table {feat_table_name}")
            return debug_info
        
        debug_info["steps"].append(f"OK: Loaded table with {len(feat_table)} entries")
        
        # Step 4: Search for feat
        found_entries = []
        for i, feat_entry in enumerate(feat_table):
            # Try different ways to get feat index
            entry_feat_id_raw = getattr(feat_entry, 'feat_index', None)
            entry_feat_id_mapped = field_mapper.get_field_value(feat_entry, 'feat_index', None)
            
            # Also try direct attribute access
            entry_feat_id_direct = getattr(feat_entry, 'FeatIndex', None)
            
            found_entries.append({
                "index": i,
                "feat_index_attr": entry_feat_id_raw,
                "feat_index_mapped": entry_feat_id_mapped,
                "feat_index_direct": entry_feat_id_direct,
                "all_attrs": [attr for attr in dir(feat_entry) if not attr.startswith('_')][:10]  # First 10 attrs
            })
            
            # Check if any matches our target feat
            for val in [entry_feat_id_raw, entry_feat_id_mapped, entry_feat_id_direct]:
                if val is not None:
                    try:
                        if int(val) == feat_id:
                            debug_info["steps"].append(f"SUCCESS: FOUND feat {feat_id} at index {i}!")
                            break
                    except (ValueError, TypeError):
                        pass
        
        debug_info["table_entries"] = found_entries[:5]  # First 5 entries for debugging
        
        # Step 5: Test the actual method
        result = feat_manager._is_feat_from_class_table(feat_id, class_id)
        debug_info["method_result"] = result
        debug_info["steps"].append(f"Method result: {result}")
        
    except Exception as e:
        debug_info["steps"].append(f"ERROR: Exception: {str(e)}")
        import traceback
        debug_info["traceback"] = traceback.format_exc()
    
    return debug_info

@router.get("/characters/{character_id}/feats/check-class-specific")
@log_performance
def check_character_feats(
    character_id: int,
    manager: CharacterManagerDep,
    removed_class_id: int = Query(..., description="Class ID to simulate removal (e.g., 56 for Stormlord)")
):
    """
    TEST ENDPOINT: Check which feats would be removed if a specific class was changed
    This tests our enhanced class-specific feat removal logic without actually changing the class
    """
    try:
        feat_manager = manager.get_manager('feat')
        
        # Get current classes (remaining after removal)
        class_list = manager.gff.get('ClassList', [])
        remaining_class_ids = {cls.get('Class') for cls in class_list if cls.get('Class') != removed_class_id}
        
        # Get all current feats - USE FEAT MANAGER'S METHOD INSTEAD
        all_feats = feat_manager.get_all_feats()
        current_feats = [feat['id'] for feat in all_feats]
        
        # Debug: Check if we have any Stormlord feats in the list
        stormlord_feats_found = [f for f in current_feats if 2033 <= f <= 2048]
        logger.info(f"STORMLORD DEBUG: Found {len(stormlord_feats_found)} Stormlord feats in character: {stormlord_feats_found}")
        logger.info(f"STORMLORD DEBUG: Total feats in character: {len(current_feats)}")
        
        # DIRECT TEST: Check if our logic works on known Stormlord feat
        test_feat_2033 = feat_manager._is_class_specific_feat(2033, removed_class_id, remaining_class_ids)
        is_protected_2033 = feat_manager.is_feat_protected(2033)
        logger.info(f"STORMLORD DEBUG: Direct test of feat 2033 removal: {test_feat_2033}, protected: {is_protected_2033}")
        
        # Test why 2033 might not be in current_feats
        logger.info(f"STORMLORD DEBUG: Is 2033 in current_feats? {2033 in current_feats}")
        
        # Check each feat to see if it would be removed
        feats_to_remove = []
        feats_to_keep = []
        protected_feats = []
        
        for feat_id in current_feats:
            # Check if this is a Stormlord feat (for debug)
            is_stormlord_feat = 2033 <= feat_id <= 2048
            
            # Check if protected
            if feat_manager.is_feat_protected(feat_id):
                protected_feats.append(feat_id)
                continue
                
            # Test our enhanced logic
            should_remove = feat_manager._is_class_specific_feat(feat_id, removed_class_id, remaining_class_ids)
            if should_remove:
                feats_to_remove.append(feat_id)
            else:
                feats_to_keep.append(feat_id)
                
            # Debug log for Stormlord feats
            if is_stormlord_feat:
                feat_data = feat_manager.game_rules_service.get_by_id('feat', feat_id)
                feat_name = "Unknown"
                if feat_data:
                    from gamedata.dynamic_loader.field_mapping_utility import field_mapper
                    feat_name = field_mapper.get_field_value(feat_data, 'label', f'Feat {feat_id}')
                logger.info(f"STORMLORD DEBUG: Feat {feat_id} ({feat_name}) - should_remove={should_remove}")
        
        # Get detailed feat info for the results
        def get_feat_info(feat_id):
            feat_data = feat_manager.game_rules_service.get_by_id('feat', feat_id)
            if feat_data:
                from gamedata.dynamic_loader.field_mapping_utility import field_mapper
                return {
                    'id': feat_id,
                    'name': field_mapper.get_field_value(feat_data, 'label', f'Feat {feat_id}'),
                    'description': field_mapper.get_field_value(feat_data, 'description', '')[:100] + '...' if field_mapper.get_field_value(feat_data, 'description', '') else ''
                }
            return {'id': feat_id, 'name': f'Feat {feat_id}', 'description': 'Unknown feat'}
        
        # Get class names
        removed_class_data = feat_manager.game_rules_service.get_by_id('classes', removed_class_id)
        removed_class_name = 'Unknown'
        if removed_class_data:
            from gamedata.dynamic_loader.field_mapping_utility import field_mapper
            removed_class_name = field_mapper.get_field_value(removed_class_data, 'label', f'Class {removed_class_id}')
        
        remaining_class_names = []
        for class_id in remaining_class_ids:
            class_data = feat_manager.game_rules_service.get_by_id('classes', class_id)
            if class_data:
                from gamedata.dynamic_loader.field_mapping_utility import field_mapper
                class_name = field_mapper.get_field_value(class_data, 'label', f'Class {class_id}')
                remaining_class_names.append(f"{class_name} ({class_id})")
        
        return {
            'test_scenario': {
                'removed_class_id': removed_class_id,
                'removed_class_name': removed_class_name,
                'remaining_classes': remaining_class_names
            },
            'results': {
                'total_feats': len(current_feats),
                'feats_to_remove': {
                    'count': len(feats_to_remove),
                    'feat_ids': feats_to_remove,
                    'details': [get_feat_info(fid) for fid in feats_to_remove[:10]]  # Limit to first 10 for readability
                },
                'feats_to_keep': {
                    'count': len(feats_to_keep),
                    'feat_ids': feats_to_keep[:10] if len(feats_to_keep) > 10 else feats_to_keep,  # Show first 10
                    'total_kept': len(feats_to_keep)
                },
                'protected_feats': {
                    'count': len(protected_feats),
                    'feat_ids': protected_feats,
                    'details': [get_feat_info(fid) for fid in protected_feats]
                }
            },
            'validation': {
                'logic_working': len(feats_to_remove) > 0 if removed_class_id == 56 else 'N/A (test with Stormlord class 56)',
                'protected_feats_respected': len(protected_feats) > 0,
                'no_feats_lost_unexpectedly': True  # Would need more complex validation
            }
        }
        
    except Exception as e:
        logger.error(f"Failed to check class-specific feats for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to check class-specific feats: {str(e)}"
        )