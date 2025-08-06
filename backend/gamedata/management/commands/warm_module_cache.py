"""
Django management command to warm module cache for specified modules
"""
from django.core.management.base import BaseCommand
from gamedata.middleware import warm_common_modules
from pathlib import Path
import logging


logger = logging.getLogger(__name__)


class Command(BaseCommand):
    help = 'Pre-load commonly used modules into cache'

    def add_arguments(self, parser):
        parser.add_argument(
            '--modules',
            nargs='+',
            type=str,
            help='Specific module paths to warm (optional)'
        )
        parser.add_argument(
            '--clear',
            action='store_true',
            help='Clear the existing cache before warming'
        )

    def handle(self, *args, **options):
        module_paths = options.get('modules')
        clear_cache = options.get('clear', False)
        
        if clear_cache:
            # Import and clear the global cache
            from gamedata.middleware import _common_module_cache
            _common_module_cache.clear()
            self.stdout.write(self.style.SUCCESS('Cleared existing module cache'))
        
        if module_paths:
            # Validate paths
            valid_paths = []
            for path in module_paths:
                if Path(path).exists():
                    valid_paths.append(path)
                else:
                    self.stdout.write(
                        self.style.WARNING(f'Module not found: {path}')
                    )
            
            if valid_paths:
                self.stdout.write(f'Warming cache for {len(valid_paths)} modules...')
                warm_common_modules(valid_paths)
        else:
            self.stdout.write('Warming cache for default common modules...')
            warm_common_modules()
        
        # Report results
        from gamedata.middleware import _common_module_cache
        self.stdout.write(
            self.style.SUCCESS(
                f'Cache warming complete. {len(_common_module_cache)} modules loaded.'
            )
        )
        
        # List cached modules
        if _common_module_cache:
            self.stdout.write('\nCached modules:')
            for module_path in _common_module_cache.keys():
                self.stdout.write(f'  - {Path(module_path).name}')