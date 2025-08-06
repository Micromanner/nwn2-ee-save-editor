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


@api_view(['POST'])
def validate_character_build(request):
    """Validate a character build before creation"""
    try:
        character_data = request.data
        errors = []
        warnings = []
        
        # Check ability scores
        abilities = ['strength', 'dexterity', 'constitution', 'intelligence', 'wisdom', 'charisma']
        for ability in abilities:
            score = character_data.get(ability, 10)
            if score < 3:
                errors.append(f"{ability.capitalize()} cannot be below 3")
            elif score > 18 and not character_data.get('allowHighStats', False):
                warnings.append(f"{ability.capitalize()} above 18 requires special circumstances")
        
        # Check alignment restrictions for classes
        law_chaos = character_data.get('lawChaos', 50)
        good_evil = character_data.get('goodEvil', 50)
        
        for cls_data in character_data.get('classes', []):
            class_id = cls_data['classId']
            
            # Paladin must be Lawful Good
            if class_id == 6 and (law_chaos < 70 or good_evil < 70):
                errors.append("Paladins must be Lawful Good")
            
            # Monk must be Lawful
            elif class_id == 5 and law_chaos < 70:
                errors.append("Monks must be Lawful")
            
            # Barbarian cannot be Lawful
            elif class_id == 0 and law_chaos >= 70:
                errors.append("Barbarians cannot be Lawful")
        
        # Check skill points
        total_skill_ranks = sum(character_data.get('skills', {}).values())
        # This is a simple check - actual calculation is complex
        if total_skill_ranks > 200:
            warnings.append("Skill point allocation seems high")
        
        return Response({
            'valid': len(errors) == 0,
            'errors': errors,
            'warnings': warnings
        })
        
    except Exception as e:
        return Response(
            {'error': f'Validation failed: {str(e)}'},
            status=status.HTTP_500_INTERNAL_SERVER_ERROR
        )