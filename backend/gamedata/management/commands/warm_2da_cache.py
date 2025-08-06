"""
Django management command to warm the 2DA cache by pre-loading all character-related 2DAs
"""
from django.core.management.base import BaseCommand
from parsers.resource_manager import ResourceManager
from pathlib import Path
import json
import time
import logging


logger = logging.getLogger(__name__)


class Command(BaseCommand):
    help = 'Pre-load all character-related 2DAs into disk cache'

    def add_arguments(self, parser):
        parser.add_argument(
            '--all',
            action='store_true',
            help='Cache ALL 2DAs, not just character-related ones'
        )
        parser.add_argument(
            '--force',
            action='store_true',
            help='Force re-cache even if files already exist'
        )

    def handle(self, *args, **options):
        cache_all = options.get('all', False)
        force = options.get('force', False)
        
        # Create resource manager
        rm = ResourceManager(suppress_warnings=True)
        cache_dir = rm.cache_dir
        
        # Load the filter list
        filter_path = Path(__file__).parent / 'nw2_data_filtered.json'
        if not filter_path.exists():
            self.stdout.write(self.style.ERROR('nw2_data_filtered.json not found'))
            return
            
        with open(filter_path) as f:
            filter_data = json.load(f)
            
        character_files = filter_data.get('character_files', [])
        ignore_prefixes = filter_data.get('ignore_prefixes', [])
        
        self.stdout.write(f'Cache directory: {cache_dir}')
        self.stdout.write(f'Character-related 2DAs: {len(character_files)}')
        
        # Get list of 2DAs to cache
        if cache_all:
            # Get all 2DAs from resource manager
            to_cache = []
            for name in rm._2da_locations.keys():
                base_name = name.replace('.2da', '')
                # Skip ignored prefixes
                if not any(base_name.startswith(prefix) for prefix in ignore_prefixes):
                    to_cache.append(base_name)
            self.stdout.write(f'Caching ALL non-ignored 2DAs: {len(to_cache)} files')
        else:
            # Just character files
            to_cache = [f.replace('.2da', '') for f in character_files]
            self.stdout.write(f'Caching character-related 2DAs only: {len(to_cache)} files')
        
        # Check existing cache
        existing = set()
        for f in cache_dir.glob('*.msgpack'):
            existing.add(f.stem)
            
        if not force:
            to_cache = [f for f in to_cache if f not in existing]
            self.stdout.write(f'Files already cached: {len(existing)}')
            self.stdout.write(f'Files to cache: {len(to_cache)}')
        
        if not to_cache:
            self.stdout.write(self.style.SUCCESS('All files already cached!'))
            return
            
        # Cache the files
        start_time = time.time()
        cached = 0
        failed = 0
        
        for i, name in enumerate(to_cache):
            try:
                # Request the 2DA - this will trigger caching
                result = rm.get_2da(name)
                if result:
                    cached += 1
                    if (i + 1) % 50 == 0:
                        self.stdout.write(f'Progress: {i + 1}/{len(to_cache)} files...')
                else:
                    failed += 1
                    self.stdout.write(self.style.WARNING(f'Failed to load: {name}'))
            except Exception as e:
                failed += 1
                self.stdout.write(self.style.ERROR(f'Error loading {name}: {e}'))
                
        elapsed = time.time() - start_time
        
        # Report results
        self.stdout.write('')
        self.stdout.write(self.style.SUCCESS(
            f'Cache warming complete in {elapsed:.1f}s'
        ))
        self.stdout.write(f'Successfully cached: {cached} files')
        if failed:
            self.stdout.write(self.style.WARNING(f'Failed: {failed} files'))
            
        # Show cache statistics
        total_size = sum(f.stat().st_size for f in cache_dir.glob('*.msgpack'))
        total_files = len(list(cache_dir.glob('*.msgpack')))
        self.stdout.write(f'\nCache statistics:')
        self.stdout.write(f'  Total files: {total_files}')
        self.stdout.write(f'  Total size: {total_size / 1024 / 1024:.1f} MB')