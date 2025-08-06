"""
Django management command to rebuild 2DA cache with mod support
"""
from django.core.management.base import BaseCommand
from django.conf import settings
from pathlib import Path
import json
from scripts.cache_with_mods import ModAwareCacher
from config.nwn2_settings import nwn2_paths


class Command(BaseCommand):
    help = 'Rebuild 2DA cache including mod overrides'

    def add_arguments(self, parser):
        parser.add_argument(
            '--docs-path',
            type=str,
            help='Path to Documents/Neverwinter Nights 2 folder',
        )
        parser.add_argument(
            '--workshop-path',
            type=str,
            help='Path to Steam Workshop mods folder',
        )

    def handle(self, *args, **options):
        self.stdout.write('Starting cache rebuild...')
        
        # Get paths from nwn2_settings or arguments
        nwn2_path = str(nwn2_paths.game_folder)
        docs_path = options.get('docs_path') or str(nwn2_paths.user_folder)
        workshop_path = options.get('workshop_path') or getattr(settings, 'NWN2_WORKSHOP_PATH', None)
        
        # Auto-detect if not provided
        if not docs_path:
            common_docs = [
                Path.home() / "Documents" / "Neverwinter Nights 2",
                Path.home() / "My Documents" / "Neverwinter Nights 2",
            ]
            for path in common_docs:
                if path.exists():
                    docs_path = str(path)
                    self.stdout.write(f'Auto-detected documents path: {docs_path}')
                    break
        
        cacher = ModAwareCacher(
            nwn2_path=nwn2_path,
            user_docs_path=docs_path,
            workshop_path=workshop_path
        )
        
        output_dir = Path(settings.BASE_DIR) / 'cache'
        cacher.cache_all_files(output_dir)
        
        self.stdout.write(self.style.SUCCESS('Cache rebuild complete!'))