from django.core.management.base import BaseCommand
from django.contrib.auth.models import User

from character.services import CharacterImportService
from parsers.resource_manager import ResourceManager


class Command(BaseCommand):
    help = 'Import a character from a .bic or .ros file'
    
    def add_arguments(self, parser):
        parser.add_argument('file_path', type=str, help='Path to the character file')
        parser.add_argument('--nwn2-path', type=str, default='nwn2_ee_data', help='Path to NWN2 data')
        parser.add_argument('--username', type=str, help='Username to assign character to')
        
    def handle(self, *args, **options):
        file_path = options['file_path']
        nwn2_path = options['nwn2_path']
        username = options.get('username')
        
        # Get owner if specified
        owner = None
        if username:
            try:
                owner = User.objects.get(username=username)
            except User.DoesNotExist:
                self.stdout.write(self.style.ERROR(f'User {username} not found'))
                return
                
        # Initialize resource manager
        self.stdout.write('Initializing resource manager...')
        rm = ResourceManager(nwn2_path)
        
        # Import character
        self.stdout.write(f'Importing character from {file_path}...')
        service = CharacterImportService(rm)
        
        try:
            character = service.import_character(file_path, owner)
            
            self.stdout.write(self.style.SUCCESS(f'Successfully imported: {character}'))
            self.stdout.write(f'  - Race: {character.race_name}')
            self.stdout.write(f'  - Alignment: {character.alignment}')
            self.stdout.write(f'  - Classes: {", ".join(str(c) for c in character.classes.all())}')
            self.stdout.write(f'  - Feats: {character.feats.count()}')
            self.stdout.write(f'  - Skills: {character.skills.count()}')
            self.stdout.write(f'  - Items: {character.items.count()}')
            
        except Exception as e:
            self.stdout.write(self.style.ERROR(f'Error importing character: {e}'))
            import traceback
            traceback.print_exc()
        finally:
            rm.close()