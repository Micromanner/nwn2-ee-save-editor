"""
Django management command to pre-generate all Python classes for distribution.
This avoids the 24-second startup time for end users.

Usage:
    python manage.py pregenerate_classes
    python manage.py pregenerate_classes --output-dir /path/to/dist
"""
from django.core.management.base import BaseCommand
from django.conf import settings
from pathlib import Path
import time
import shutil

from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader
from gamedata.dynamic_loader.code_cache import SecureCodeCache
from parsers.resource_manager import ResourceManager


class Command(BaseCommand):
    help = 'Pre-generate Python classes for all character-related tables'

    def add_arguments(self, parser):
        parser.add_argument(
            '--output-dir',
            type=str,
            help='Output directory for generated classes (default: gamedata/cache/generated_code_dist)'
        )
        parser.add_argument(
            '--clear-cache',
            action='store_true',
            help='Clear existing cache before generating'
        )

    def handle(self, *args, **options):
        self.stdout.write("=== Pre-generating Python Classes for Distribution ===\n")
        
        # Setup paths
        backend_dir = Path(settings.BASE_DIR)
        cache_dir = backend_dir / "gamedata" / "cache" / "generated_code"
        
        # Output directory
        if options['output_dir']:
            dist_dir = Path(options['output_dir'])
        else:
            dist_dir = backend_dir / "gamedata" / "cache" / "generated_code_dist"
        
        # Clear cache if requested
        if options['clear_cache'] and cache_dir.exists():
            self.stdout.write("Clearing existing generated code...")
            cache = SecureCodeCache(cache_dir)
            cache.clear_cache()
            if cache.metadata_file.exists():
                cache.metadata_file.unlink()
        
        # Create resource manager
        self.stdout.write("\nGenerating classes for all character-related tables...")
        rm = ResourceManager(suppress_warnings=True)
        
        start_time = time.time()
        
        # Load all tables - this will generate the classes
        # For management commands, we don't use the singleton since we need
        # specific settings (no validation, specific resource manager)
        from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader
        loader = DynamicGameDataLoader(
            resource_manager=rm,
            use_async=False,
            validate_relationships=False  # Skip validation for speed
        )
        
        elapsed = time.time() - start_time
        
        # Get stats
        stats = loader.get_stats()
        self.stdout.write(self.style.SUCCESS(
            f"\nGenerated in {elapsed:.2f} seconds"
        ))
        self.stdout.write(f"Tables processed: {stats['tables_loaded']}")
        cache_stats = stats.get('cache_stats', {})
        self.stdout.write(f"Classes generated: {cache_stats.get('file_count', 0)}")
        
        # Create distribution directory
        dist_dir.mkdir(parents=True, exist_ok=True)
        
        # Copy generated files
        self.stdout.write(f"\nCopying generated classes to {dist_dir}...")
        copied = 0
        for py_file in cache_dir.glob("*.py"):
            shutil.copy2(py_file, dist_dir / py_file.name)
            copied += 1
        
        # Also copy metadata and relationships
        for json_file in cache_dir.glob("*.json"):
            shutil.copy2(json_file, dist_dir / json_file.name)
        
        self.stdout.write(self.style.SUCCESS(f"Copied {copied} Python files"))
        
        # Create a version file
        version_file = dist_dir / "generation_info.txt"
        with open(version_file, 'w') as f:
            f.write(f"Generated on: {time.strftime('%Y-%m-%d %H:%M:%S')}\n")
            f.write(f"Tables processed: {stats['tables_loaded']}\n")
            f.write(f"Total rows: {stats['total_rows']}\n")
            f.write(f"Generation time: {elapsed:.2f} seconds\n")
            f.write(f"NWN2 installation: {rm.nwn2_path}\n")
        
        self.stdout.write(self.style.SUCCESS(
            f"\nDone! Pre-generated classes saved to:\n  {dist_dir}"
        ))
        self.stdout.write(
            "\nThese files can be shipped with your application to avoid the "
            "24-second generation time on first startup."
        )
        self.stdout.write(
            "If users have mods that change table structures, the system will "
            "automatically regenerate affected classes."
        )