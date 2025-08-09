"""
Feat ViewSet - All feat-related endpoints
Handles feat selection, prerequisites, and feat management
"""

from rest_framework import status
from rest_framework.decorators import action
from rest_framework.response import Response
from django.core.cache import cache
import logging
import hashlib
import time
from functools import wraps

from .base_character_view import BaseCharacterViewSet

logger = logging.getLogger(__name__)


def log_performance(func):
    """Decorator to log performance metrics for API endpoints"""
    @wraps(func)
    def wrapper(*args, **kwargs):
        start_time = time.time()
        result = None
        try:
            result = func(*args, **kwargs)
            return result
        finally:
            end_time = time.time()
            duration_ms = (end_time - start_time) * 1000
            
            # Log performance data
            endpoint_name = func.__name__
            status_code = result.status_code if result else 'error'
            logger.info(f"{endpoint_name}: {duration_ms:.2f}ms (status: {status_code})")
    return wrapper


class FeatViewSet(BaseCharacterViewSet):
    """
    ViewSet for feat-related operations
    All endpoints are nested under /api/characters/{id}/feats/
    """
    @action(detail=False, methods=['get'], url_path='state')
    @log_performance
    def feats_state(self, request, character_pk=None):
        """Get current feats state (no expensive available_feats calculation)"""
        
        try:
            character, manager = self._get_character_manager(character_pk)
            feat_manager = manager.get_manager('feat')
            
            state = {
                'current_feats': feat_manager.get_feat_summary_fast(),  # Use FAST method for display
                # Removed available_feats - use /feats/legitimate/ endpoint instead
                'feat_slots': {
                    'available': feat_manager.get_bonus_feats_available(),
                    'used': len(feat_manager.get_all_feats())
                },
                'categories': {
                    'general': 0,
                    'combat': 1,
                    'metamagic': 2,
                    'item_creation': 3,
                    'divine': 4,
                    'epic': 5
                }
            }
            
            return Response(state, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "feats_state")
    
    @action(detail=False, methods=['get'], url_path='available')
    def available_feats(self, request, character_pk=None):
        """Get feats available for selection based on current state"""
        feat_type = request.query_params.get('type', None)
        
        try:
            character, manager = self._get_character_manager(character_pk)
            feat_manager = manager.get_manager('feat')
            
            # Get available feats
            available = feat_manager.get_available_feats(
                feat_type=int(feat_type) if feat_type else None
            )
            
            return Response({
                'available_feats': available,
                'total': len(available)
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "available_feats")
    
    @action(detail=False, methods=['post'], url_path='add')
    @log_performance
    def add_feat(self, request, character_pk=None):
        """Add a feat to character"""
        feat_id = request.data.get('feat_id')
        
        if feat_id is None:
            return Response(
                {'error': 'feat_id is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            feat_manager = manager.get_manager('feat')
            
            # Check if feat can be added
            available = feat_manager.get_available_feats()
            if not any(f['id'] == feat_id for f in available):
                return Response(
                    {'error': 'Feat not available or prerequisites not met'},
                    status=status.HTTP_400_BAD_REQUEST
                )
            
            result = feat_manager.add_feat(feat_id)
            
            # Get updated feat list
            feat_summary = feat_manager.get_feat_summary()
            
            # Invalidate feat cache for this character by incrementing a version key
            cache_version_key = f"feat_cache_version:char_{character_pk}"
            current_version = cache.get(cache_version_key, 0)
            cache.set(cache_version_key, current_version + 1, timeout=3600)  # Version key expires in 1 hour
            logger.debug(f"Invalidated feat cache for character {character_pk} after adding feat")
            
            return Response({
                'message': 'Feat added successfully',
                'feat_summary': feat_summary,
                'result': result,
                'has_unsaved_changes': session.has_unsaved_changes()
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "add_feat")
    
    @action(detail=False, methods=['post'], url_path='remove')
    def remove_feat(self, request, character_pk=None):
        """Remove a feat from character"""
        feat_id = request.data.get('feat_id')
        
        if feat_id is None:
            return Response(
                {'error': 'feat_id is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            feat_manager = manager.get_manager('feat')
            
            # Check if feat is protected
            if feat_manager.is_feat_protected(feat_id):
                return Response(
                    {'error': 'Cannot remove this feat (granted by class/race or required by other feats)'},
                    status=status.HTTP_400_BAD_REQUEST
                )
            
            result = feat_manager.remove_feat(feat_id)
            
            # Get updated feat list
            feat_summary = feat_manager.get_feat_summary()
            
            # Invalidate feat cache for this character by incrementing a version key
            cache_version_key = f"feat_cache_version:char_{character_pk}"
            current_version = cache.get(cache_version_key, 0)
            cache.set(cache_version_key, current_version + 1, timeout=3600)  # Version key expires in 1 hour
            logger.debug(f"Invalidated feat cache for character {character_pk} after removing feat")
            
            return Response({
                'message': 'Feat removed successfully',
                'feat_summary': feat_summary,
                'result': result,
                'has_unsaved_changes': session.has_unsaved_changes()
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "remove_feat")
    
    @action(detail=True, methods=['get'], url_path='prerequisites')
    def feat_prerequisites(self, request, character_pk=None, pk=None):
        """Get prerequisites for a specific feat"""
        try:
            character, manager = self._get_character_manager(character_pk)
            feat_manager = manager.get_manager('feat')
            
            feat_id = int(pk)
            valid, prereq_errors = feat_manager.validate_feat_prerequisites(feat_id)
            prerequisites = {
                'feat_id': feat_id,
                'valid': valid,
                'missing_prerequisites': prereq_errors
            }
            
            return Response(prerequisites, status=status.HTTP_200_OK)
            
        except ValueError:
            return Response(
                {'error': 'Invalid feat ID'},
                status=status.HTTP_400_BAD_REQUEST
            )
        except Exception as e:
            return self._handle_character_error(character_pk, e, "feat_prerequisites")
    
    @action(detail=True, methods=['get'], url_path='check')
    def check_prerequisites(self, request, character_pk=None, pk=None):
        """Check if character meets prerequisites for a feat"""
        try:
            character, manager = self._get_character_manager(character_pk)
            feat_manager = manager.get_manager('feat')
            
            feat_id = int(pk)
            can_take, reason = feat_manager.can_take_feat(feat_id)
            check_result = {
                'feat_id': feat_id,
                'can_take': can_take,
                'reason': reason
            }
            
            return Response(check_result, status=status.HTTP_200_OK)
            
        except ValueError:
            return Response(
                {'error': 'Invalid feat ID'},
                status=status.HTTP_400_BAD_REQUEST
            )
        except Exception as e:
            return self._handle_character_error(character_pk, e, "check_prerequisites")
    
    @action(detail=False, methods=['get'], url_path='by-category')
    @log_performance
    def feats_by_category(self, request, character_pk=None):
        """Get feats organized by category (fast display version, no validation)"""
        try:
            character, manager = self._get_character_manager(character_pk)
            feat_manager = manager.get_manager('feat')
            
            # Use fast method that skips validation
            categorized_feats = feat_manager.get_feat_categories_fast()
            
            return Response(categorized_feats, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "feats_by_category")
    
    @action(detail=False, methods=['get'], url_path='legitimate')
    @log_performance
    def legitimate_feats(self, request, character_pk=None):
        """Get legitimate feats with pagination and smart filtering (no validation for performance)"""
        feat_type = request.query_params.get('type', None)
        category = request.query_params.get('category', '').lower()
        subcategory = request.query_params.get('subcategory', '').lower()
        # DEPRECATED: only_available parameter - always act as false for performance
        # Validation should be done on-demand via /feats/validate/{id} endpoint
        only_available = False  # Always skip validation for performance
        page = int(request.query_params.get('page', 1))
        limit = int(request.query_params.get('limit', 50))
        search = request.query_params.get('search', '').strip()
        
        # Get cache version for this character (to invalidate on feat changes)
        cache_version_key = f"feat_cache_version:char_{character_pk}"
        cache_version = cache.get(cache_version_key, 0)
        
        # Generate cache key based on character and query parameters
        # Note: Removed only_available from cache key since it's always false now
        cache_key_parts = [
            f"feat_legitimate",
            f"char_{character_pk}",
            f"v_{cache_version}",  # Include version for easy invalidation
            f"cat_{category}",
            f"sub_{subcategory}",
            f"type_{feat_type}",
            f"page_{page}",
            f"limit_{limit}",
        ]
        
        # Create a stable cache key
        cache_key = ":".join(filter(None, cache_key_parts))
        
        # Try to get from cache first
        cached_response = cache.get(cache_key)
        if cached_response is not None:
            logger.debug(f"Returning cached feat response for key: {cache_key}")
            return Response(cached_response, status=status.HTTP_200_OK)
        
        try:
            character, manager = self._get_character_manager(character_pk)
            feat_manager = manager.get_manager('feat')
            
            # Get feats based on category and availability
            if only_available:
                # Get only feats the character can actually take
                all_feats = feat_manager.get_available_feats_by_category(
                    category=category,
                    subcategory=subcategory,
                    feat_type=int(feat_type) if feat_type else None
                )
            else:
                # Get all legitimate feats in category (no prereq checking)
                all_feats = feat_manager.get_legitimate_feats_by_category(
                    category=category,
                    subcategory=subcategory,
                    feat_type=int(feat_type) if feat_type else None
                )
            
            # Apply search filter if provided
            if search:
                search_lower = search.lower()
                all_feats = [
                    feat for feat in all_feats 
                    if search_lower in feat.get('label', '').lower() or 
                       search_lower in feat.get('name', '').lower()
                ]
            
            # Calculate pagination
            total = len(all_feats)
            start_idx = (page - 1) * limit
            end_idx = start_idx + limit
            paginated_feats = all_feats[start_idx:end_idx]
            
            has_next = end_idx < total
            has_prev = page > 1
            total_pages = (total + limit - 1) // limit  # Ceiling division
            
            response_data = {
                'legitimate_feats': paginated_feats,
                'pagination': {
                    'page': page,
                    'limit': limit,
                    'total': total,
                    'total_pages': total_pages,
                    'has_next': has_next,
                    'has_prev': has_prev,
                    'start_idx': start_idx + 1 if paginated_feats else 0,
                    'end_idx': min(end_idx, total)
                },
                'search': search,
                'category': category,
                'subcategory': subcategory,
                # Removed only_available from response - always false now
            }
            
            # Cache the response for 5 minutes (300 seconds)
            # Cache will be invalidated when character changes (feat added/removed)
            cache.set(cache_key, response_data, timeout=300)
            logger.debug(f"Cached feat response for key: {cache_key}")
            
            return Response(response_data, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "legitimate_feats")
    
    @action(detail=True, methods=['get'], url_path='details')
    def feat_details(self, request, character_pk=None, pk=None):
        """Get detailed information about a specific feat including description and prerequisites"""
        try:
            character, manager = self._get_character_manager(character_pk)
            feat_manager = manager.get_manager('feat')
            
            feat_id = int(pk)
            
            # Get detailed feat information
            feat_info = feat_manager.get_feat_info(feat_id)
            if not feat_info:
                return Response(
                    {'error': f'Feat {feat_id} not found'},
                    status=status.HTTP_404_NOT_FOUND
                )
            
            # Get detailed prerequisites
            detailed_prereqs = feat_manager.get_detailed_prerequisites(feat_id)
            
            # Combine information
            detailed_feat = {
                **feat_info,
                'detailed_prerequisites': detailed_prereqs
            }
            
            return Response(detailed_feat, status=status.HTTP_200_OK)
            
        except ValueError:
            return Response(
                {'error': 'Invalid feat ID'},
                status=status.HTTP_400_BAD_REQUEST
            )
        except Exception as e:
            return self._handle_character_error(character_pk, e, "feat_details")
    
    @action(detail=True, methods=['get'], url_path='validate')
    @log_performance
    def validate_feat(self, request, character_pk=None, pk=None):
        """
        Validate if character can take a specific feat (on-demand validation).
        This is the performance-optimized replacement for checking prerequisites
        during list loading. Call this when user hovers/clicks on a feat.
        """
        try:
            character, manager = self._get_character_manager(character_pk)
            feat_manager = manager.get_manager('feat')
            
            feat_id = int(pk)
            
            # Check if character already has the feat
            if feat_manager.has_feat(feat_id):
                return Response({
                    'feat_id': feat_id,
                    'can_take': False,
                    'reason': 'Already has this feat',
                    'has_feat': True,
                    'missing_requirements': []
                }, status=status.HTTP_200_OK)
            
            # Get cache version for this character
            cache_version_key = f"feat_cache_version:char_{character_pk}"
            cache_version = cache.get(cache_version_key, 0)
            
            # Create cache key for this validation
            cache_key = f"feat_validate:char_{character_pk}:v_{cache_version}:feat_{feat_id}"
            
            # Try to get cached validation result
            cached_result = cache.get(cache_key)
            if cached_result is not None:
                logger.debug(f"Returning cached validation for feat {feat_id}")
                return Response(cached_result, status=status.HTTP_200_OK)
            
            # Perform validation
            can_take, missing_reqs = feat_manager.validate_feat_prerequisites(feat_id)
            
            # Build response
            validation_result = {
                'feat_id': feat_id,
                'can_take': can_take,
                'reason': 'Meets all requirements' if can_take else 'Missing prerequisites',
                'has_feat': False,
                'missing_requirements': missing_reqs
            }
            
            # Cache the result for 5 minutes
            cache.set(cache_key, validation_result, timeout=300)
            
            return Response(validation_result, status=status.HTTP_200_OK)
            
        except ValueError:
            return Response(
                {'error': 'Invalid feat ID'},
                status=status.HTTP_400_BAD_REQUEST
            )
        except Exception as e:
            return self._handle_character_error(character_pk, e, "validate_feat")