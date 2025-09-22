"""
API views for managing NWN2 path configuration
"""
from rest_framework.decorators import api_view
from rest_framework.response import Response
from rest_framework import status
from config.nwn2_settings import nwn2_paths
from parsers.resource_manager import ResourceManager
import logging

logger = logging.getLogger(__name__)


@api_view(['GET'])
def get_path_config(request):
    """Get current path configuration"""
    response_data = {
        'paths': nwn2_paths.get_all_paths_info(),
        'saves': str(nwn2_paths.saves),
        'user_folder': str(nwn2_paths.user_folder),
        'localvault': str(nwn2_paths.localvault),
    }
    return Response(response_data)


@api_view(['POST'])
def set_game_folder(request):
    """Set the main game installation folder"""
    path = request.data.get('path')
    if not path:
        return Response({'error': 'Path is required'}, status=status.HTTP_400_BAD_REQUEST)
    
    success = nwn2_paths.set_game_folder(path)
    if success:
        # Clear module cache when game folder changes
        if hasattr(request, 'resource_manager'):
            request.resource_manager.clear_module_cache()
        
        return Response({
            'success': True,
            'message': 'Game folder updated successfully',
            'paths': nwn2_paths.get_all_paths_info()
        })
    else:
        return Response({
            'error': 'Invalid path or directory does not exist'
        }, status=status.HTTP_400_BAD_REQUEST)


@api_view(['POST'])
def set_documents_folder(request):
    """Set the documents folder"""
    path = request.data.get('path')
    if not path:
        return Response({'error': 'Path is required'}, status=status.HTTP_400_BAD_REQUEST)
    
    success = nwn2_paths.set_documents_folder(path)
    if success:
        return Response({
            'success': True,
            'message': 'Documents folder updated successfully',
            'paths': nwn2_paths.get_all_paths_info()
        })
    else:
        return Response({
            'error': 'Invalid path or directory does not exist'
        }, status=status.HTTP_400_BAD_REQUEST)


@api_view(['POST'])
def set_steam_workshop_folder(request):
    """Set the Steam workshop folder"""
    path = request.data.get('path')
    if not path:
        return Response({'error': 'Path is required'}, status=status.HTTP_400_BAD_REQUEST)
    
    success = nwn2_paths.set_steam_workshop_folder(path)
    if success:
        # Clear workshop cache when workshop folder changes
        if hasattr(request, 'resource_manager'):
            request.resource_manager.clear_workshop_cache()
        
        return Response({
            'success': True,
            'message': 'Steam workshop folder updated successfully',
            'paths': nwn2_paths.get_all_paths_info()
        })
    else:
        return Response({
            'error': 'Invalid path or directory does not exist'
        }, status=status.HTTP_400_BAD_REQUEST)


@api_view(['POST'])
def add_custom_override_folder(request):
    """Add a custom override folder"""
    path = request.data.get('path')
    if not path:
        return Response({'error': 'Path is required'}, status=status.HTTP_400_BAD_REQUEST)
    
    success = nwn2_paths.add_custom_override_folder(path)
    if success:
        # Also add to resource manager if available
        if hasattr(request, 'resource_manager'):
            request.resource_manager.add_custom_override_directory(path)
        
        return Response({
            'success': True,
            'message': 'Custom override folder added successfully',
            'paths': nwn2_paths.get_all_paths_info()
        })
    else:
        return Response({
            'error': 'Invalid path or directory does not exist'
        }, status=status.HTTP_400_BAD_REQUEST)


@api_view(['POST'])
def remove_custom_override_folder(request):
    """Remove a custom override folder"""
    path = request.data.get('path')
    if not path:
        return Response({'error': 'Path is required'}, status=status.HTTP_400_BAD_REQUEST)
    
    success = nwn2_paths.remove_custom_override_folder(path)
    if success:
        # Also remove from resource manager if available
        if hasattr(request, 'resource_manager'):
            request.resource_manager.remove_custom_override_directory(path)
        
        return Response({
            'success': True,
            'message': 'Custom override folder removed successfully',
            'paths': nwn2_paths.get_all_paths_info()
        })
    else:
        return Response({
            'error': 'Path not found in custom override folders'
        }, status=status.HTTP_400_BAD_REQUEST)


@api_view(['POST'])
def add_custom_module_folder(request):
    """Add a custom module folder"""
    path = request.data.get('path')
    if not path:
        return Response({'error': 'Path is required'}, status=status.HTTP_400_BAD_REQUEST)
    
    success = nwn2_paths.add_custom_module_folder(path)
    if success:
        return Response({
            'success': True,
            'message': 'Custom module folder added successfully',
            'paths': nwn2_paths.get_all_paths_info()
        })
    else:
        return Response({
            'error': 'Invalid path or directory does not exist'
        }, status=status.HTTP_400_BAD_REQUEST)


@api_view(['POST'])
def remove_custom_module_folder(request):
    """Remove a custom module folder"""
    path = request.data.get('path')
    if not path:
        return Response({'error': 'Path is required'}, status=status.HTTP_400_BAD_REQUEST)
    
    success = nwn2_paths.remove_custom_module_folder(path)
    if success:
        return Response({
            'success': True,
            'message': 'Custom module folder removed successfully',
            'paths': nwn2_paths.get_all_paths_info()
        })
    else:
        return Response({
            'error': 'Path not found in custom module folders'
        }, status=status.HTTP_400_BAD_REQUEST)


@api_view(['POST'])
def add_custom_hak_folder(request):
    """Add a custom HAK folder"""
    path = request.data.get('path')
    if not path:
        return Response({'error': 'Path is required'}, status=status.HTTP_400_BAD_REQUEST)
    
    success = nwn2_paths.add_custom_hak_folder(path)
    if success:
        return Response({
            'success': True,
            'message': 'Custom HAK folder added successfully',
            'paths': nwn2_paths.get_all_paths_info()
        })
    else:
        return Response({
            'error': 'Invalid path or directory does not exist'
        }, status=status.HTTP_400_BAD_REQUEST)


@api_view(['POST'])
def remove_custom_hak_folder(request):
    """Remove a custom HAK folder"""
    path = request.data.get('path')
    if not path:
        return Response({'error': 'Path is required'}, status=status.HTTP_400_BAD_REQUEST)
    
    success = nwn2_paths.remove_custom_hak_folder(path)
    if success:
        return Response({
            'success': True,
            'message': 'Custom HAK folder removed successfully',
            'paths': nwn2_paths.get_all_paths_info()
        })
    else:
        return Response({
            'error': 'Path not found in custom HAK folders'
        }, status=status.HTTP_400_BAD_REQUEST)


@api_view(['GET'])
def auto_detect_paths(request):
    """Auto-detect NWN2 installation paths"""
    from config.nwn2_settings import NWN2PathFinder
    
    # Find game installations
    game_installations = NWN2PathFinder.auto_discover_nwn2_paths()
    
    # Find documents folder
    documents_folder = NWN2PathFinder.find_documents_folder()
    
    # Find Steam workshop
    steam_workshop = NWN2PathFinder.find_steam_workshop()
    
    return Response({
        'game_installations': [str(p) for p in game_installations],
        'documents_folder': str(documents_folder) if documents_folder else None,
        'steam_workshop': str(steam_workshop) if steam_workshop else None,
        'current_paths': nwn2_paths.get_all_paths_info()
    })