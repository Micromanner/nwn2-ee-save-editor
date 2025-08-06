"""
API views for exporting characters to NWN2
"""
import os
import shutil
from datetime import datetime
from rest_framework import status
from rest_framework.decorators import api_view
from rest_framework.response import Response
from django.conf import settings

from character.character_creation_service import CharacterCreationService
from parsers.resource_manager import ResourceManager
from config.nwn2_settings import nwn2_paths


@api_view(['POST'])
def export_to_localvault(request):
    """
    Export a created character directly to NWN2's localvault as player.bic
    
    Expected data:
    {
        "source_path": "/path/to/created/player.bic",
        "backup_existing": true  // Optional, backs up existing player.bic
    }
    """
    try:
        source_path = request.data.get('source_path')
        if not source_path or not os.path.exists(source_path):
            return Response(
                {'error': 'Invalid source path'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        # Get NWN2 localvault path
        localvault_path = os.path.join(nwn2_paths.nwn2_docs, 'localvault')
        if not os.path.exists(localvault_path):
            return Response(
                {'error': 'NWN2 localvault not found. Is NWN2 installed?'},
                status=status.HTTP_404_NOT_FOUND
            )
        
        target_path = os.path.join(localvault_path, 'player.bic')
        
        # Backup existing file if requested
        if request.data.get('backup_existing', True) and os.path.exists(target_path):
            backup_dir = os.path.join(localvault_path, 'backups')
            os.makedirs(backup_dir, exist_ok=True)
            
            timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
            backup_path = os.path.join(backup_dir, f'player_{timestamp}.bic')
            shutil.copy2(target_path, backup_path)
        
        # Copy the new file
        shutil.copy2(source_path, target_path)
        
        return Response({
            'success': True,
            'message': 'Character exported to NWN2 localvault',
            'localvault_path': target_path
        })
        
    except Exception as e:
        return Response(
            {'error': f'Failed to export character: {str(e)}'},
            status=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


@api_view(['POST'])
def export_for_module(request):
    """
    Export a character for a specific module or campaign
    
    Expected data:
    {
        "character_data": {...},  // Character creation data
        "module_name": "Module Name",  // Optional
        "export_path": "/custom/path/"  // Optional custom export path
    }
    """
    try:
        character_data = request.data.get('character_data')
        if not character_data:
            return Response(
                {'error': 'Character data required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        # Determine export path
        export_path = request.data.get('export_path')
        if not export_path:
            # Default to localvault
            export_path = os.path.join(nwn2_paths.nwn2_docs, 'localvault')
        
        os.makedirs(export_path, exist_ok=True)
        
        # Always export as player.bic
        output_path = os.path.join(export_path, 'player.bic')
        
        # Create the character
        rm = ResourceManager()
        service = CharacterCreationService(rm)
        
        # Get template
        templates = service.get_template_paths()
        if not templates:
            return Response(
                {'error': 'No character templates found'},
                status=status.HTTP_500_INTERNAL_SERVER_ERROR
            )
        
        template_path = next(iter(templates.values()))
        
        # Create and export
        character = service.create_character(character_data, template_path, output_path)
        
        return Response({
            'success': True,
            'character_id': character.id,
            'export_path': output_path,
            'ready_to_play': True
        })
        
    except Exception as e:
        return Response(
            {'error': f'Failed to export character: {str(e)}'},
            status=status.HTTP_500_INTERNAL_SERVER_ERROR
        )