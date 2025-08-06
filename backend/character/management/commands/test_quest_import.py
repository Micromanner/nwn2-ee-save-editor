"""
Django management command to test quest data import
"""
from django.core.management.base import BaseCommand
from django.contrib.auth.models import User
from character.services import CharacterImportService
from parsers.resource_manager import ResourceManager
import os


class Command(BaseCommand):
    help = 'Test quest and campaign data import from save files'

    def add_arguments(self, parser):
        parser.add_argument(
            '--save-path',
            type=str,
            default="/home/michael/fun/nwn2_ee_editor/backend/sample_save/000000 - 23-07-2025-13-06",
            help='Path to save game directory'
        )

    def handle(self, *args, **options):
        save_path = options['save_path']
        
        if not os.path.exists(save_path):
            self.stdout.write(
                self.style.ERROR(f'Save path not found: {save_path}')
            )
            return

        self.stdout.write(f'Testing quest import from: {save_path}')
        
        try:
            # Get or create test user
            user, created = User.objects.get_or_create(username='test_quest_user')
            if created:
                self.stdout.write('Created test user')
            
            # Import character
            rm = ResourceManager('nwn2_ee_data')
            service = CharacterImportService(rm)
            
            self.stdout.write('Importing character...')
            character = service.import_character(save_path, owner=user)
            
            # Display results
            self.stdout.write(
                self.style.SUCCESS(f'✓ Character imported: {character.first_name} {character.last_name}')
            )
            
            self.stdout.write(f'Campaign: {character.campaign_name}')
            self.stdout.write(f'Module: {character.module_name}')
            self.stdout.write(f'Current area: {character.current_area}')
            self.stdout.write(f'Level: {character.character_level}')
            
            self.stdout.write(f'\nQuest Progress:')
            self.stdout.write(f'  Completed quests: {character.completed_quests_count}')
            self.stdout.write(f'  Active quests: {character.active_quests_count}')
            
            self.stdout.write(f'\nCompanion Influence:')
            for companion, influence in character.companion_influence.items():
                self.stdout.write(f'  {companion.capitalize()}: {influence}')
            
            self.stdout.write(f'\nLocation Progress:')
            self.stdout.write(f'  Unlocked locations: {len(character.unlocked_locations)}')
            if character.unlocked_locations:
                for location in character.unlocked_locations[:5]:
                    self.stdout.write(f'    - {location}')
                if len(character.unlocked_locations) > 5:
                    self.stdout.write(f'    ... and {len(character.unlocked_locations) - 5} more')
            
            rm.close()
            
            self.stdout.write(
                self.style.SUCCESS('\n✓ Quest data import test completed successfully!')
            )
            
        except Exception as e:
            self.stdout.write(
                self.style.ERROR(f'✗ Import failed: {e}')
            )
            import traceback
            traceback.print_exc()