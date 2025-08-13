"""
Save Game specific API views
Handles importing and editing characters from NWN2 save games
"""
from rest_framework import status
from rest_framework.decorators import api_view, permission_classes
from rest_framework.permissions import IsAuthenticated, AllowAny
from rest_framework.response import Response
from api.decorators import desktop_or_authenticated
from django.core.files.storage import default_storage
from django.core.files.base import ContentFile
from django.db import transaction
from django.conf import settings
from django.views.decorators.csrf import csrf_exempt
import os
import tempfile
import logging

from character.models import Character
from character.services import CharacterImportService
from character.managers import *
from parsers.resource_manager import ResourceManager
from parsers.gff import GFFParser
from parsers.savegame_handler import SaveGameHandler
from api.serializers import CharacterDetailSerializer

logger = logging.getLogger(__name__)


def error_response(message: str, code: str = None, details: dict = None, status_code: int = status.HTTP_400_BAD_REQUEST):
    """
    Create a standardized error response
    
    Args:
        message: Human-readable error message
        code: Machine-readable error code (e.g., 'PERMISSION_DENIED', 'NOT_FOUND')
        details: Additional error details
        status_code: HTTP status code
    
    Returns:
        Response object with standardized error format
    """
    error_data = {
        'error': {
            'message': message,
            'code': code or 'ERROR'
        }
    }
    
    if details:
        error_data['error']['details'] = details
    
    return Response(error_data, status=status_code)


@api_view(['POST'])
def import_savegame(request):
    """
    Import a character from a save game directory
    
    Expected request data:
    - save_path: Full path to save game directory (e.g., "C:\\Users\\...\\saves\\000048 - 23-07-2025-13-31")
    
    For web upload, you'd need a separate endpoint that handles directory uploads
    """
    # Check authentication only if not in desktop mode
    if not settings.DESKTOP_MODE and not request.user.is_authenticated:
        return error_response(
            'Authentication required',
            code='AUTHENTICATION_REQUIRED',
            status_code=status.HTTP_401_UNAUTHORIZED
        )
    
    # In desktop mode, get the desktop user directly since DRF resets it
    if settings.DESKTOP_MODE:
        from django.contrib.auth.models import User
        user = User.objects.get(username='desktop_user')
    else:
        user = request.user
    
    save_path = request.data.get('save_path')
    rm = None
    
    if not save_path:
        return error_response(
            'save_path is required',
            code='MISSING_PARAMETER',
            details={'parameter': 'save_path'}
        )
    
    # No path conversion needed for native Windows
    
    try:
        # Verify the save directory exists and has resgff.zip
        if not os.path.isdir(save_path):
            return error_response(
                f'Save directory not found: {save_path}',
                code='DIRECTORY_NOT_FOUND',
                details={'path': save_path},
                status_code=status.HTTP_404_NOT_FOUND
            )
        
        resgff_path = os.path.join(save_path, 'resgff.zip')
        if not os.path.exists(resgff_path):
            return error_response(
                'resgff.zip not found in save directory',
                code='FILE_NOT_FOUND',
                details={'file': 'resgff.zip', 'directory': save_path},
                status_code=status.HTTP_404_NOT_FOUND
            )
        
        # Import the character from save game using shared ResourceManager
        from gamedata.middleware import get_resource_manager
        rm = get_resource_manager()
        if rm is None:
            # Fallback if no shared ResourceManager available
            from gamedata.middleware import get_shared_resource_manager
            rm = get_shared_resource_manager()
        service = CharacterImportService(rm)
        
        with transaction.atomic():
            character = service.import_character(save_path, owner=user)
            logger.info(
                f"Imported savegame character: user={user.username}, "
                f"character_id={character.id}, name={character.first_name} {character.last_name}, "
                f"save_path={save_path}"
            )
            
        # Return character data
        serializer = CharacterDetailSerializer(character)
        return Response(serializer.data, status=status.HTTP_201_CREATED)
        
    except Exception as e:
        # Handle database constraint violations with more helpful messages
        if hasattr(e, '__class__') and 'IntegrityError' in str(e.__class__):
            error_msg = str(e)
            
            # Parse constraint violations to provide helpful messages
            if 'CHECK constraint failed' in error_msg:
                constraint_name = error_msg.split('CHECK constraint failed: ')[-1].strip()
                
                # Map constraint names to helpful error messages
                constraint_messages = {
                    # Most constraints have been removed to allow loading of any save file
                    # Only critical database constraints remain
                }
                
                helpful_message = constraint_messages.get(constraint_name, f'Database constraint violation: {constraint_name}')
                
                logger.error(f"Database constraint error during save import: {helpful_message}", exc_info=True)
                return error_response(
                    f'Invalid character data: {helpful_message}',
                    code='VALIDATION_ERROR',
                    details={
                        'constraint': constraint_name,
                        'message': helpful_message,
                        'suggestion': 'The save file may be corrupted or from an incompatible version of NWN2. Try using a different save file.'
                    },
                    status_code=status.HTTP_400_BAD_REQUEST
                )
            
            elif 'UNIQUE constraint failed' in error_msg:
                logger.error(f"Duplicate character data during save import: {error_msg}", exc_info=True)
                return error_response(
                    'This character has already been imported',
                    code='DUPLICATE_CHARACTER',
                    details={'error': 'A character with this data already exists in the database'},
                    status_code=status.HTTP_409_CONFLICT
                )
        
        # Log full error details for debugging
        logger.error(f"Error importing save game: {e}", exc_info=True)
        return error_response(
            'Failed to import save game',
            code='IMPORT_ERROR',
            details={
                'error': str(e),
                'type': str(type(e).__name__),
                'suggestion': 'Check that the save file is valid and not corrupted'
            },
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )
    finally:
        # Don't close the shared ResourceManager - it's managed by middleware
        pass


@api_view(['GET'])
@permission_classes([AllowAny])
def list_savegame_companions(request, character_id):
    """
    List all companions available in a save game
    
    Returns list of companion names that can be edited
    """
    # In desktop mode, get the desktop user directly since DRF resets it
    if settings.DESKTOP_MODE:
        from django.contrib.auth.models import User
        user = User.objects.get(username='desktop_user')
    else:
        user = request.user
        # Check authentication only if not in desktop mode
        if not user.is_authenticated:
            return error_response(
                'Authentication required',
                code='AUTHENTICATION_REQUIRED',
                status_code=status.HTTP_401_UNAUTHORIZED
            )
    
    try:
        character = Character.objects.get(id=character_id)
        
        # Check user owns this character
        if character.owner != user:
            return error_response(
                'You do not have permission to view this character',
                code='PERMISSION_DENIED',
                details={'character_id': character_id},
                status_code=status.HTTP_403_FORBIDDEN
            )
        
        if not character.is_savegame:
            return error_response(
                'This character is not from a save game',
                code='INVALID_CHARACTER_TYPE',
                details={'character_id': character_id}
            )
        
        # Validate that the save game directory still exists
        if not os.path.exists(character.file_path):
            return error_response(
                f'Save game directory not found: {character.file_path}. It may have been moved or deleted.',
                code='SAVEGAME_NOT_FOUND',
                details={'path': character.file_path},
                status_code=status.HTTP_404_NOT_FOUND
            )
        
        handler = SaveGameHandler(character.file_path)
        companions = handler.list_companions()
        
        logger.info(
            f"Listed savegame companions: user={request.user.username}, "
            f"character_id={character_id}, companions_count={len(companions)}"
        )
        
        return Response({
            'companions': companions,
            'count': len(companions)
        }, status=status.HTTP_200_OK)
        
    except Character.DoesNotExist:
        return error_response(
            'Character not found',
            code='CHARACTER_NOT_FOUND',
            details={'character_id': character_id},
            status_code=status.HTTP_404_NOT_FOUND
        )
    except Exception as e:
        return error_response(
            'Failed to list companions',
            code='COMPANION_LIST_ERROR',
            details={'error': str(e)},
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


@api_view(['POST'])
@permission_classes([AllowAny])
def update_savegame_character(request, character_id):
    """
    Update character data in a save game
    
    This updates both playerlist.ifo and player.bic files in the save game zip
    to ensure NWN2 recognizes the changes
    """
    # In desktop mode, get the desktop user directly since DRF resets it
    if settings.DESKTOP_MODE:
        from django.contrib.auth.models import User
        user = User.objects.get(username='desktop_user')
    else:
        user = request.user
        # Check authentication only if not in desktop mode
        if not user.is_authenticated:
            return error_response(
                'Authentication required',
                code='AUTHENTICATION_REQUIRED',
                status_code=status.HTTP_401_UNAUTHORIZED
            )
    
    try:
        character = Character.objects.get(id=character_id)
        
        # Check user owns this character
        if character.owner != user:
            return error_response(
                'You do not have permission to update this character',
                code='PERMISSION_DENIED',
                details={'character_id': character_id},
                status_code=status.HTTP_403_FORBIDDEN
            )
        
        if not character.is_savegame:
            return error_response(
                'This character is not from a save game',
                code='INVALID_CHARACTER_TYPE',
                details={'character_id': character_id}
            )
        
        # Validate that the save game directory still exists
        if not os.path.exists(character.file_path):
            return error_response(
                f'Save game directory not found: {character.file_path}. It may have been moved or deleted.',
                code='SAVEGAME_NOT_FOUND',
                details={'path': character.file_path},
                status_code=status.HTTP_404_NOT_FOUND
            )
        
        # Get the CharacterManager updates
        updates = request.data.get('updates', {})
        if not updates:
            return error_response(
                'No updates provided',
                code='MISSING_UPDATES',
                details={'character_id': character_id}
            )
        
        # Get or create character session from registry  
        # This will load the save files into memory if not already loaded
        from character.session_registry import get_character_session, save_character_session
        try:
            session = get_character_session(character_id)
            manager = session.character_manager
            
            if not manager:
                return error_response(
                    'Failed to load character data',
                    code='CHARACTER_LOAD_ERROR',
                    details={'character_id': character_id},
                    status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
                )
        except ValueError as e:
            return error_response(
                str(e),
                code='SESSION_ERROR',
                details={'character_id': character_id},
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
            )
        
        # Register managers based on what's being updated
        if 'attributes' in updates:
            manager.register_manager('attribute', AttributeManager)
            manager.register_manager('combat', CombatManager)
            manager.register_manager('save', SaveManager)
        
        if 'class' in updates or 'classes' in updates:
            manager.register_manager('class', ClassManager)
            manager.register_manager('feat', FeatManager)
            manager.register_manager('spell', SpellManager)
            manager.register_manager('skill', SkillManager)
        
        if 'feats' in updates:
            manager.register_manager('feat', FeatManager)
        
        if 'spells' in updates:
            manager.register_manager('spell', SpellManager)
        
        if 'skills' in updates:
            manager.register_manager('skill', SkillManager)
        
        # Apply updates
        changes = {}
        
        # Example: Update attributes
        if 'attributes' in updates:
            attr_manager = manager.get_manager('attribute')
            for attr, value in updates['attributes'].items():
                if attr in ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']:
                    # Validate attribute value range (D&D 3.5 rules)
                    if not isinstance(value, int):
                        return error_response(
                            f'Attribute {attr} value must be an integer',
                            code='INVALID_ATTRIBUTE_TYPE',
                            details={'attribute': attr, 'value': value}
                        )
                    if value < 3 or value > 100:
                        return error_response(
                            f'Attribute {attr} value must be between 3 and 100, got {value}',
                            code='INVALID_ATTRIBUTE_RANGE',
                            details={'attribute': attr, 'value': value, 'min': 3, 'max': 100}
                        )
                    
                    result = attr_manager.set_attribute(attr, value)
                    changes[attr] = result
        
        # Update classes
        if 'classes' in updates:
            class_manager = manager.get_manager('class')
            if 'add_class' in updates['classes']:
                # Add a new class
                class_data = updates['classes']['add_class']
                result = class_manager.add_class(
                    class_data.get('class_id'),
                    class_data.get('level', 1)
                )
                changes['class_added'] = result
            elif 'level_up' in updates['classes']:
                # Level up existing class
                class_id = updates['classes']['level_up']
                result = class_manager.level_up_class(class_id)
                changes['class_leveled'] = result
        
        # Update feats
        if 'feats' in updates:
            feat_manager = manager.get_manager('feat')
            if 'add' in updates['feats']:
                for feat_id in updates['feats']['add']:
                    try:
                        result = feat_manager.add_feat(feat_id)
                        if 'feats_added' not in changes:
                            changes['feats_added'] = []
                        changes['feats_added'].append(result)
                    except ValueError as e:
                        logger.warning(f"Failed to add feat {feat_id}: {e}")
            if 'remove' in updates['feats']:
                for feat_id in updates['feats']['remove']:
                    try:
                        result = feat_manager.remove_feat(feat_id)
                        if 'feats_removed' not in changes:
                            changes['feats_removed'] = []
                        changes['feats_removed'].append(result)
                    except ValueError as e:
                        logger.warning(f"Failed to remove feat {feat_id}: {e}")
        
        # Update skills
        if 'skills' in updates:
            skill_manager = manager.get_manager('skill')
            for skill_id, ranks in updates['skills'].items():
                try:
                    # Validate skill ranks
                    if not isinstance(ranks, (int, float)) or ranks < 0:
                        continue
                    result = skill_manager.set_skill_rank(int(skill_id), ranks)
                    if 'skills_updated' not in changes:
                        changes['skills_updated'] = []
                    changes['skills_updated'].append(result)
                except (ValueError, KeyError) as e:
                    logger.warning(f"Failed to update skill {skill_id}: {e}")
        
        # Update spells
        if 'spells' in updates:
            spell_manager = manager.get_manager('spell')
            if 'known' in updates['spells']:
                # Update known spells
                for class_id, spell_data in updates['spells']['known'].items():
                    for level, spells in spell_data.items():
                        try:
                            # Add each spell individually
                            for spell_id in spells:
                                result = spell_manager.add_known_spell(
                                    int(class_id), 
                                    int(level), 
                                    int(spell_id)
                                )
                                if 'spells_updated' not in changes:
                                    changes['spells_updated'] = []
                                changes['spells_updated'].append({
                                    'class_id': int(class_id),
                                    'level': int(level),
                                    'spell_id': int(spell_id),
                                    'added': result
                                })
                        except (ValueError, KeyError) as e:
                            logger.warning(f"Failed to update spells: {e}")
        
        # Save the session to disk (this handles both player.bic and playerlist.ifo sync)
        from django.utils import timezone
        
        try:
            with transaction.atomic():
                # Save the session - this creates backup and updates both files
                success = save_character_session(character_id, create_backup=True)
                
                if not success:
                    return error_response(
                        'Failed to save character changes',
                        code='SAVE_FAILED',
                        details={'character_id': character_id},
                        status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
                    )
                
                # Update Django model
                character.last_modified = timezone.now()
                character.save()
                
                # Log the successful update with details
                logger.info(
                    f"Updated savegame character: user={request.user.username}, "
                    f"character_id={character_id}, name={character.first_name} {character.last_name}, "
                    f"changes={changes}, backup_created=True"
                )
                
            return Response({
                'success': True,
                'changes': changes,
                'backup_created': True
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            # If backup or file update failed, the transaction will rollback
            logger.error(f"Failed to update savegame (backup/write error): {e}", exc_info=True)
            return error_response(
                f'Failed to update save game: {str(e)}. No changes were made.',
                code='UPDATE_FAILED',
                details={'error': str(e)},
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
            )
        
    except Character.DoesNotExist:
        return error_response(
            'Character not found',
            code='CHARACTER_NOT_FOUND',
            details={'character_id': character_id},
            status_code=status.HTTP_404_NOT_FOUND
        )
    except Exception as e:
        logger.error(f"Error updating save game: {e}", exc_info=True)
        return error_response(
            'Failed to update save game',
            code='UPDATE_ERROR',
            details={'error': str(e)},
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )
    finally:
        # Don't close the shared ResourceManager - it's managed by middleware
        pass


@api_view(['GET'])
@permission_classes([AllowAny])
def get_savegame_info(request, character_id):
    """
    Get information about the save game, including backup status
    """
    # In desktop mode, get the desktop user directly since DRF resets it
    if settings.DESKTOP_MODE:
        from django.contrib.auth.models import User
        user = User.objects.get(username='desktop_user')
    else:
        user = request.user
        # Check authentication only if not in desktop mode
        if not user.is_authenticated:
            return error_response(
                'Authentication required',
                code='AUTHENTICATION_REQUIRED',
                status_code=status.HTTP_401_UNAUTHORIZED
            )
    
    try:
        character = Character.objects.get(id=character_id)
        
        # Check user owns this character
        if character.owner != user:
            return error_response(
                'You do not have permission to view this character',
                code='PERMISSION_DENIED',
                details={'character_id': character_id},
                status_code=status.HTTP_403_FORBIDDEN
            )
        
        if not character.is_savegame:
            return error_response(
                'This character is not from a save game',
                code='INVALID_CHARACTER_TYPE',
                details={'character_id': character_id}
            )
        
        save_dir = character.file_path
        
        # Validate that the save game directory still exists
        if not os.path.exists(save_dir):
            return error_response(
                f'Save game directory not found: {save_dir}. It may have been moved or deleted.',
                code='SAVEGAME_NOT_FOUND',
                details={'path': save_dir},
                status_code=status.HTTP_404_NOT_FOUND
            )
        
        # Check for backups
        import glob
        backup_pattern = f"{save_dir}_backup_*"
        backups = sorted(glob.glob(backup_pattern))
        
        # Get save game info
        handler = SaveGameHandler(save_dir)
        
        logger.info(
            f"Retrieved savegame info: user={request.user.username}, "
            f"character_id={character_id}, backups_count={len(backups)}"
        )
        
        return Response({
            'save_directory': save_dir,
            'original_save_exists': os.path.exists(save_dir),
            'backups': [
                {
                    'path': backup,
                    'name': os.path.basename(backup),
                    'created': os.path.getctime(backup)
                }
                for backup in backups
            ],
            'companions': handler.list_companions(),
            'files_in_save': handler.list_files()
        }, status=status.HTTP_200_OK)
        
    except Character.DoesNotExist:
        return error_response(
            'Character not found',
            code='CHARACTER_NOT_FOUND',
            details={'character_id': character_id},
            status_code=status.HTTP_404_NOT_FOUND
        )
    except Exception as e:
        return error_response(
            'Failed to get save game information',
            code='INFO_ERROR',
            details={'error': str(e)},
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


@api_view(['POST'])
@permission_classes([AllowAny])
def restore_savegame_backup(request, character_id):
    """
    Restore a save game from a backup
    
    Expected request data:
    - backup_path: Full path to the backup directory
    """
    # In desktop mode, get the desktop user directly since DRF resets it
    if settings.DESKTOP_MODE:
        from django.contrib.auth.models import User
        user = User.objects.get(username='desktop_user')
    else:
        user = request.user
        # Check authentication only if not in desktop mode
        if not user.is_authenticated:
            return error_response(
                'Authentication required',
                code='AUTHENTICATION_REQUIRED',
                status_code=status.HTTP_401_UNAUTHORIZED
            )
    
    try:
        character = Character.objects.get(id=character_id)
        
        # Check user owns this character
        if character.owner != user:
            return error_response(
                'You do not have permission to restore this character',
                code='PERMISSION_DENIED',
                details={'character_id': character_id},
                status_code=status.HTTP_403_FORBIDDEN
            )
        
        if not character.is_savegame:
            return error_response(
                'This character is not from a save game',
                code='INVALID_CHARACTER_TYPE',
                details={'character_id': character_id}
            )
        
        backup_path = request.data.get('backup_path')
        if not backup_path:
            return error_response(
                'backup_path is required',
                code='MISSING_PARAMETER',
                details={'parameter': 'backup_path'}
            )
        
        # Security check: ensure backup path is related to the character's save
        save_dir = character.file_path
        expected_prefix = f"{save_dir}_backup_"
        if not backup_path.startswith(expected_prefix):
            return error_response(
                'Invalid backup path',
                code='INVALID_BACKUP_PATH',
                details={'backup_path': backup_path}
            )
        
        # Validate backup exists
        if not os.path.exists(backup_path):
            return error_response(
                f'Backup not found: {backup_path}',
                code='BACKUP_NOT_FOUND',
                details={'backup_path': backup_path},
                status_code=status.HTTP_404_NOT_FOUND
            )
        
        # Validate backup has required files
        backup_resgff = os.path.join(backup_path, 'resgff.zip')
        if not os.path.exists(backup_resgff):
            return error_response(
                'Invalid backup: missing resgff.zip',
                code='INVALID_BACKUP',
                details={'missing_file': 'resgff.zip', 'backup_path': backup_path}
            )
        
        # Create a backup of current state before restoring
        import datetime
        import shutil
        timestamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
        pre_restore_backup = f"{save_dir}_pre_restore_{timestamp}"
        
        try:
            # Backup current state
            shutil.copytree(save_dir, pre_restore_backup)
            
            # Remove current save directory
            shutil.rmtree(save_dir)
            
            # Restore from backup
            shutil.copytree(backup_path, save_dir)
            
            logger.info(
                f"Restored savegame from backup: user={request.user.username}, "
                f"character_id={character_id}, backup_path={backup_path}, "
                f"save_dir={save_dir}, pre_restore_backup={pre_restore_backup}"
            )
            
            return Response({
                'success': True,
                'restored_from': backup_path,
                'pre_restore_backup': pre_restore_backup,
                'message': 'Save game restored successfully'
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            # Try to restore from pre-restore backup if something went wrong
            if os.path.exists(pre_restore_backup) and not os.path.exists(save_dir):
                try:
                    shutil.copytree(pre_restore_backup, save_dir)
                except:
                    pass
            
            logger.error(f"Failed to restore backup: {e}", exc_info=True)
            return error_response(
                f'Failed to restore backup: {str(e)}',
                code='RESTORE_FAILED',
                details={'error': str(e)},
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
            )
            
    except Character.DoesNotExist:
        return error_response(
            'Character not found',
            code='CHARACTER_NOT_FOUND',
            details={'character_id': character_id},
            status_code=status.HTTP_404_NOT_FOUND
        )
    except Exception as e:
        logger.error(f"Error restoring backup: {e}", exc_info=True)
        return error_response(
            'Failed to restore backup',
            code='RESTORE_ERROR',
            details={'error': str(e)},
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )
