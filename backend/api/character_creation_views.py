"""
API views for character creation
"""
import os
import tempfile
from datetime import datetime
from rest_framework import status
from rest_framework.decorators import api_view
from rest_framework.response import Response
from django.conf import settings

from character.character_creation_service import CharacterCreationService
from parsers.resource_manager import ResourceManager


@api_view(['POST'])
def create_character(request):
    """
    Create a new character from character builder data
    
    Expected data format:
    {
        "firstName": "John",
        "lastName": "Doe",
        "age": 25,
        "gender": 0,
        "deity": "Tyr",
        "raceId": 6,
        "classes": [
            {"classId": 4, "level": 5},
            {"classId": 11, "level": 3}
        ],
        "strength": 16,
        "dexterity": 14,
        "constitution": 14,
        "intelligence": 10,
        "wisdom": 12,
        "charisma": 8,
        "lawChaos": 75,
        "goodEvil": 80,
        "skills": {
            "0": 8,  // Concentration
            "1": 5   // Craft Alchemy
        },
        "feats": [0, 2, 3],  // Feat IDs
        "appearanceType": 0,
        "hairStyle": 1,
        "headModel": 1,
        "portraitId": "po_hu_m_01_"
    }
    """
    try:
        character_data = request.data
        
        # Validate required fields
        required_fields = ['firstName', 'classes']
        for field in required_fields:
            if field not in character_data:
                return Response(
                    {'error': f'Missing required field: {field}'},
                    status=status.HTTP_400_BAD_REQUEST
                )
        
        # Validate classes
        if not character_data['classes'] or len(character_data['classes']) == 0:
            return Response(
                {'error': 'At least one class is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        total_level = sum(cls['level'] for cls in character_data['classes'])
        if total_level > 40:
            return Response(
                {'error': 'Total character level cannot exceed 40'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        # Create character
        rm = ResourceManager()
        service = CharacterCreationService(rm)
        
        # Get template path
        templates = service.get_template_paths()
        if not templates:
            return Response(
                {'error': 'No character templates found. Please ensure NWN2 is properly installed.'},
                status=status.HTTP_500_INTERNAL_SERVER_ERROR
            )
        
        # Use first available template
        template_path = next(iter(templates.values()))
        
        # Generate output path - always use player.bic for NWN2 compatibility
        output_dir = os.path.join(settings.MEDIA_ROOT, 'characters', 'created')
        os.makedirs(output_dir, exist_ok=True)
        
        # For NWN2, the file must be named player.bic
        output_filename = "player.bic"
        output_path = os.path.join(output_dir, output_filename)
        
        # If we want to keep multiple versions, create subdirectories
        if 'create_subfolder' in request.data and request.data['create_subfolder']:
            safe_name = ''.join(c for c in character_data['firstName'] if c.isalnum() or c in (' ', '-', '_'))
            timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
            subfolder = f"{safe_name}_{timestamp}"
            output_dir = os.path.join(output_dir, subfolder)
            os.makedirs(output_dir, exist_ok=True)
            output_path = os.path.join(output_dir, output_filename)
        
        # Create the character
        character = service.create_character(character_data, template_path, output_path)
        
        # Return character data
        from api.serializers import CharacterDetailSerializer
        serializer = CharacterDetailSerializer(character)
        
        return Response({
            'character': serializer.data,
            'file_path': output_path,
            'file_name': output_filename
        }, status=status.HTTP_201_CREATED)
        
    except Exception as e:
        return Response(
            {'error': f'Failed to create character: {str(e)}'},
            status=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


@api_view(['GET'])
def get_character_templates(request):
    """Get available character templates"""
    try:
        rm = ResourceManager()
        service = CharacterCreationService(rm)
        templates = service.get_template_paths()
        
        return Response({
            'templates': [{'name': name, 'path': path} for name, path in templates.items()]
        })
    except Exception as e:
        return Response(
            {'error': f'Failed to get templates: {str(e)}'},
            status=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


