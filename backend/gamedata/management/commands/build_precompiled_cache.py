"""
Django management command to build pre-compiled 2DA cache.
Usage: python manage.py build_precompiled_cache
"""
from django.core.management.base import BaseCommand
from django.conf import settings
import logging
import time

logger = logging.getLogger(__name__)


class Command(BaseCommand):
    help = 'Build pre-compiled 2DA cache for 60-70% startup speedup'

    def add_arguments(self, parser):
        parser.add_argument(
            '--force',
            action='store_true',
            help='Force rebuild even if cache exists',
        )
        parser.add_argument(
            '--stats',
            action='store_true',
            help='Show cache statistics after building',
        )

    def handle(self, *args, **options):
        self.stdout.write(self.style.SUCCESS('=' * 80))
        self.stdout.write(self.style.SUCCESS('BUILDING PRE-COMPILED 2DA CACHE'))
        self.stdout.write(self.style.SUCCESS('=' * 80))
        
        # Ensure cache is enabled
        if not getattr(settings, 'ENABLE_PRECOMPILED_CACHE', True):
            self.stdout.write(self.style.ERROR('Pre-compiled cache is disabled in settings'))
            self.stdout.write('Set ENABLE_PRECOMPILED_CACHE=True to enable')
            return
        
        # Get ResourceManager instance
        from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader
        
        self.stdout.write('Initializing game data loader...')
        loader = get_dynamic_game_data_loader()
        
        # Wait for loader to be ready
        if not loader.is_ready():
            self.stdout.write('Waiting for game data loader to be ready...')
            loader.wait_for_ready(timeout=60)
        
        # Get resource manager from loader (it's stored as 'rm')
        resource_manager = loader.rm
        
        # Check if cache already exists
        cache_stats = resource_manager._precompiled_cache.get_cache_stats()
        if cache_stats.get('valid') and not options['force']:
            self.stdout.write(self.style.WARNING('Cache already exists and is valid'))
            self.stdout.write('Use --force to rebuild')
            
            if options['stats']:
                self._show_stats(cache_stats)
            return
        
        # Build the cache
        self.stdout.write('Building cache for ~440 character-related 2DA tables...')
        start_time = time.time()
        
        success = resource_manager._precompiled_cache.build_cache()
        
        if success:
            elapsed = time.time() - start_time
            self.stdout.write(self.style.SUCCESS(f'✓ Cache built successfully in {elapsed:.2f} seconds'))
            
            # Show statistics
            if options['stats']:
                cache_stats = resource_manager._precompiled_cache.get_cache_stats()
                self._show_stats(cache_stats)
            
            # Test cache performance
            self._test_cache_performance(resource_manager)
        else:
            self.stdout.write(self.style.ERROR('✗ Failed to build cache'))
    
    def _show_stats(self, stats):
        """Display cache statistics."""
        self.stdout.write('\n' + '=' * 80)
        self.stdout.write('CACHE STATISTICS')
        self.stdout.write('=' * 80)
        
        self.stdout.write(f"Enabled: {stats.get('enabled', False)}")
        self.stdout.write(f"Valid: {stats.get('valid', False)}")
        self.stdout.write(f"Cache Size: {stats.get('cache_size_mb', 0):.2f} MB")
        self.stdout.write(f"Tables Loaded: {stats.get('total_tables_loaded', 0)}")
        
        if 'cache_key' in stats:
            self.stdout.write(f"Cache Key: {stats.get('cache_key')}")
        
        if 'version' in stats:
            self.stdout.write(f"Version: {stats.get('version')}")
    
    def _test_cache_performance(self, resource_manager):
        """Test cache performance with a few tables."""
        self.stdout.write('\n' + '=' * 80)
        self.stdout.write('TESTING CACHE PERFORMANCE')
        self.stdout.write('=' * 80)
        
        test_tables = ['classes.2da', 'feat.2da', 'spells.2da', 'baseitems.2da']
        
        for table in test_tables:
            start = time.time()
            parser = resource_manager.get_2da(table)
            elapsed_ms = (time.time() - start) * 1000
            
            if parser:
                self.stdout.write(f"✓ {table}: {elapsed_ms:.2f}ms")
            else:
                self.stdout.write(self.style.ERROR(f"✗ {table}: Failed to load"))
        
        # Show overall stats
        cache_hits = getattr(resource_manager, '_cache_hits', 0)
        cache_misses = getattr(resource_manager, '_cache_misses', 0)
        self.stdout.write(f"\nCache hits: {cache_hits}")
        self.stdout.write(f"Cache misses: {cache_misses}")