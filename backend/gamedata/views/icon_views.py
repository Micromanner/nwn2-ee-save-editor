"""
API views for serving game icons from enhanced memory cache with override support.
"""
from django.http import HttpResponse, Http404, JsonResponse
from django.views import View
from rest_framework.views import APIView
from rest_framework.response import Response
from gamedata.cache.icon_cache import icon_cache
import logging

logger = logging.getLogger(__name__)


class IconView(View):
    """Serve individual icons from Rust memory cache with full override support."""
    
    def get(self, request, icon_path=None):
        """
        Get an icon by path.
        
        Args:
            icon_path: Icon path like "spell_fireball" or "spells/spell_fireball"
        """
        if not icon_path:
            return JsonResponse({'error': 'Icon path required'}, status=400)
        
        logger.debug(f"Icon request: {icon_path}")
        
        # Check if cache exists
        if not icon_cache:
            logger.error("Icon cache not available")
            return HttpResponse(
                b"Icon cache not initialized",
                status=503,
                content_type="text/plain"
            )
        
        # Initialize cache if needed (shouldn't happen with startup initialization)
        if not icon_cache._initialized:
            logger.warning("Icon cache not initialized at request time, initializing now...")
            icon_cache.initialize()
        
        # Try direct lookup first
        icon_data, mimetype = icon_cache.get_icon(icon_path)
        
        # If not found, try with path lookup
        if not icon_data:
            icon_data, mimetype = icon_cache.get_icon_by_path(icon_path)
        
        if icon_data:
            logger.debug(f"Icon found: {icon_path} ({len(icon_data)} bytes, {mimetype})")
            response = HttpResponse(icon_data, content_type=mimetype)
            # Cache headers for browser
            response['Cache-Control'] = 'public, max-age=31536000'  # 1 year
            response['X-Icon-Source'] = 'rust-cache'
            return response
        
        logger.info(f"Icon not found: {icon_path}")
        raise Http404(f"Icon not found: {icon_path}")


class IconStatsView(APIView):
    """Get statistics about the icon cache."""
    
    def get(self, request):
        """Get cache statistics."""
        if not icon_cache:
            return Response({
                'error': 'Icon cache not initialized',
                'initialized': False
            }, status=503)
        
        stats = icon_cache.get_statistics()
        
        return Response({
            'initialized': icon_cache._initialized,
            'initializing': icon_cache._initializing,
            'statistics': stats,
            'format': icon_cache.icon_format,
            'mimetype': icon_cache.icon_mimetype
        })


class IconListView(APIView):
    """List all available icons in the cache."""
    
    def get(self, request):
        """Get list of all available icon names."""
        if not icon_cache:
            return Response({'error': 'Icon cache not initialized'}, status=503)
        
        if not icon_cache._initialized:
            return Response({'error': 'Icon cache not ready'}, status=503)
        
        try:
            # Get all icon names from the Rust cache
            icon_names = icon_cache.rust_cache.get_all_icon_names()
            
            # Filter by search term if provided
            search = request.GET.get('search', '').lower()
            if search:
                icon_names = [name for name in icon_names if search in name.lower()]
            
            # Limit results for performance
            limit = int(request.GET.get('limit', 100))
            icon_names = icon_names[:limit]
            
            return Response({
                'icons': icon_names,
                'total_count': len(icon_names),
                'search': search,
                'limit': limit
            })
        except AttributeError:
            # Fallback if get_all_icon_names doesn't exist
            return Response({
                'error': 'Icon listing not available - method not implemented in Rust cache',
                'suggestion': 'Try searching for specific icon names like "align"'
            })


class ModuleIconView(APIView):
    """Update icon cache based on module HAK files."""
    
    def post(self, request):
        """
        Update cache with module HAK files.
        
        Expected JSON body:
        {
            "hak_list": ["hak1.hak", "hak2.hak", ...]
        }
        """
        hak_list = request.data.get('hak_list', [])
        
        if not isinstance(hak_list, list):
            return Response({'error': 'hak_list must be an array'}, status=400)
        
        # Check if cache exists
        if not icon_cache:
            return Response({'error': 'Icon cache not initialized'}, status=503)
        
        # Set module HAKs
        icon_cache.set_module_haks(hak_list)
        
        # Get updated statistics
        stats = icon_cache.get_statistics()
        
        return Response({
            'success': True,
            'haks_loaded': len(icon_cache._loaded_haks),
            'statistics': stats
        })


class LegacyIconView(View):
    """
    Legacy compatibility view that maps old category/icon_name URLs to new system.
    """
    
    def get(self, request, category, icon_name):
        """Get an icon by category and name (legacy format)."""
        # Map to new path format
        if category in ['spells', 'items', 'feats', 'skills', 'classes', 'races']:
            icon_path = f"{category}/{icon_name}"
        else:
            icon_path = icon_name
        
        # Redirect to icon view
        icon_view = IconView()
        return icon_view.get(request, icon_path=icon_path)