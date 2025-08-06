"""
Feat ViewSet - All feat-related endpoints
Handles feat selection, prerequisites, and feat management
"""

from rest_framework import status
from rest_framework.decorators import action
from rest_framework.response import Response
import logging

from .base_character_view import BaseCharacterViewSet

logger = logging.getLogger(__name__)


class FeatViewSet(BaseCharacterViewSet):
    """
    ViewSet for feat-related operations
    All endpoints are nested under /api/characters/{id}/feats/
    """
    @action(detail=False, methods=['get'], url_path='state')
    def feats_state(self, request, character_pk=None):
        """Get current feats and available feats for the feats editor"""
        feat_type = request.query_params.get('type')
        
        try:
            character, manager = self._get_character_manager(character_pk)
            feat_manager = manager.get_manager('feat')
            
            state = {
                'current_feats': feat_manager.get_feat_summary(),
                'available_feats': feat_manager.get_available_feats(
                    feat_type=int(feat_type) if feat_type else None
                ),
                'feat_slots': {
                    'available': feat_manager.get_bonus_feats_available(),
                    'used': len(feat_manager.get_all_feats())
                },
                'categories': {
                    'general': 0,
                    'combat': 1,
                    'metamagic': 2,
                    'item_creation': 3
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
    def feats_by_category(self, request, character_pk=None):
        """Get feats organized by category"""
        try:
            character, manager = self._get_character_manager(character_pk)
            feat_manager = manager.get_manager('feat')
            
            categorized_feats = feat_manager.get_feat_categories()
            
            return Response(categorized_feats, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "feats_by_category")
    
    @action(detail=False, methods=['get'], url_path='legitimate')
    def legitimate_feats(self, request, character_pk=None):
        """Get legitimate feats with pagination (filtered to exclude dev/broken feats)"""
        feat_type = request.query_params.get('type', None)
        page = int(request.query_params.get('page', 1))
        limit = int(request.query_params.get('limit', 50))
        search = request.query_params.get('search', '').strip()
        
        try:
            character, manager = self._get_character_manager(character_pk)
            feat_manager = manager.get_manager('feat')
            
            # Get all legitimate feats first
            all_legitimate = feat_manager.get_legitimate_feats(
                feat_type=int(feat_type) if feat_type else None
            )
            
            # Apply search filter if provided
            if search:
                search_lower = search.lower()
                all_legitimate = [
                    feat for feat in all_legitimate 
                    if search_lower in feat.get('label', '').lower() or 
                       search_lower in feat.get('name', '').lower()
                ]
            
            # Calculate pagination
            total = len(all_legitimate)
            start_idx = (page - 1) * limit
            end_idx = start_idx + limit
            paginated_feats = all_legitimate[start_idx:end_idx]
            
            has_next = end_idx < total
            has_prev = page > 1
            total_pages = (total + limit - 1) // limit  # Ceiling division
            
            return Response({
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
                'filtered_from_total': 'Applied filtering to remove dev/broken feats'
            }, status=status.HTTP_200_OK)
            
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