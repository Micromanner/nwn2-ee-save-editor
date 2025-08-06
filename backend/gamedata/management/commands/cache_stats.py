from django.core.management.base import BaseCommand
from parsers.resource_manager import ResourceManager
import json


class Command(BaseCommand):
    help = 'Display current memory cache statistics'

    def add_arguments(self, parser):
        parser.add_argument(
            '--json',
            action='store_true',
            help='Output in JSON format',
        )
        parser.add_argument(
            '--watch',
            action='store_true',
            help='Continuously update stats every 5 seconds',
        )

    def handle(self, *args, **options):
        rm = ResourceManager()
        
        if options['watch']:
            import time
            try:
                while True:
                    self._display_stats(rm, options['json'])
                    time.sleep(5)
                    # Clear screen
                    print('\033[2J\033[H')
            except KeyboardInterrupt:
                self.stdout.write('\nStopped watching.')
        else:
            self._display_stats(rm, options['json'])
    
    def _display_stats(self, rm, as_json=False):
        stats = rm.get_cache_stats()
        
        if as_json:
            self.stdout.write(json.dumps(stats, indent=2))
        else:
            self.stdout.write(self.style.SUCCESS('\n=== Memory Cache Statistics ===\n'))
            
            # Basic settings
            self.stdout.write(f"Cache Enabled: {stats['enabled']}")
            self.stdout.write(f"Compression: {stats['compression_enabled']}")
            self.stdout.write(f"Smart Preload: {stats['preload_enabled']}")
            self.stdout.write('')
            
            # Memory usage
            self.stdout.write(self.style.MIGRATE_HEADING('Memory Usage:'))
            self.stdout.write(f"  Current: {stats['current_size_mb']:.2f} MB")
            self.stdout.write(f"  Maximum: {stats['max_size_mb']} MB")
            self.stdout.write(f"  Usage: {(stats['current_size_mb'] / stats['max_size_mb'] * 100):.1f}%")
            self.stdout.write('')
            
            # Cache performance
            self.stdout.write(self.style.MIGRATE_HEADING('Performance:'))
            self.stdout.write(f"  Hit Rate: {stats['hit_rate']}")
            self.stdout.write(f"  Cache Hits: {stats['cache_hits']}")
            self.stdout.write(f"  Cache Misses: {stats['cache_misses']}")
            self.stdout.write('')
            
            # Compression stats
            self.stdout.write(self.style.MIGRATE_HEADING('Compression:'))
            self.stdout.write(f"  Compressed Items: {stats['compressed_items']}/{stats['cached_items']}")
            self.stdout.write(f"  Compression Ratio: {stats['compression_ratio']}")
            self.stdout.write('')
            
            # Top cached items
            self.stdout.write(self.style.MIGRATE_HEADING('Recently Cached Items:'))
            for item in stats['2da_cache_keys'][:10]:
                self.stdout.write(f"  - {item}")
            
            if stats['cached_items'] > 10:
                self.stdout.write(f"  ... and {stats['cached_items'] - 10} more")