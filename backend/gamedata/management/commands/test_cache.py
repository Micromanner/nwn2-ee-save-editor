from django.core.management.base import BaseCommand
from parsers.resource_manager import ResourceManager


class Command(BaseCommand):
    help = 'Test 2DA caching functionality'

    def handle(self, *args, **options):
        # Create resource manager
        rm = ResourceManager(suppress_warnings=False)
        
        # Request a 2DA that's not cached yet
        self.stdout.write("Requesting appearance.2da...")
        result = rm.get_2da('appearance')
        if result:
            self.stdout.write(f"Got appearance.2da with {result.get_resource_count()} rows")
        else:
            self.stdout.write("Failed to get appearance.2da")
        
        # Check cache directory
        cache_dir = rm.cache_dir
        self.stdout.write(f"\nCache directory: {cache_dir}")
        cache_files = list(cache_dir.glob('*.msgpack'))
        self.stdout.write(f"Cache files: {len(cache_files)} msgpack files")
        
        # List first 10 cache files
        self.stdout.write("\nFirst 10 cache files:")
        for i, f in enumerate(sorted(cache_files)):
            if i >= 10:
                break
            self.stdout.write(f"  {f.name}")
            
        # Check if appearance.2da.msgpack was created
        appearance_cache = cache_dir / 'appearance.2da.msgpack'
        if appearance_cache.exists():
            self.stdout.write(self.style.SUCCESS("\nSuccess! appearance.2da.msgpack was created"))
        else:
            self.stdout.write(self.style.ERROR("\nError: appearance.2da.msgpack was NOT created"))