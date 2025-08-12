"""
Django management command to generate vanilla Python classes for BASE game tables only.

This command generates classes ONLY from the base game 2DA files (2da.zip, 2da_x1.zip, 2da_x2.zip),
ignoring all mod content. These classes can be shipped with the application to avoid
the 24-second generation time for most users.

Usage:
    python manage.py generate_vanilla_classes
    python manage.py generate_vanilla_classes --output-dir /path/to/vanilla
"""
from django.core.management.base import BaseCommand
from django.conf import settings
from pathlib import Path
import time
import shutil
import json

from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader
from gamedata.dynamic_loader.code_cache import SecureCodeCache
from gamedata.dynamic_loader.runtime_class_generator import RuntimeDataClassGenerator
from parsers.resource_manager import ResourceManager


class VanillaResourceManager(ResourceManager):
    """
    Special ResourceManager that ONLY loads base game files.
    Ignores all override directories, workshop, HAK files, etc.
    """
    
    def __init__(self, *args, **kwargs):
        # Initialize normally but then clear all override sources
        super().__init__(*args, **kwargs)
        
        # Clear all mod/override sources - we only want base game
        self._override_file_paths = {}
        self._workshop_file_paths = {}  
        self._hak_overrides = []
        self._module_overrides = {}
        
        # Force re-scan to only get base game files
        self._scan_zip_files()
        
    def _scan_override_directories(self):
        """Override to do nothing - we don't want any overrides."""
        pass
        
    def _build_module_hak_index(self):
        """Override to do nothing - no module/HAK content."""
        pass


class Command(BaseCommand):
    help = 'Generate vanilla Python classes for base game 2DA tables only'

    def add_arguments(self, parser):
        parser.add_argument(
            '--output-dir',
            type=str,
            help='Output directory for vanilla classes (default: gamedata/cache/vanilla_classes)'
        )
        parser.add_argument(
            '--clear-existing',
            action='store_true',
            help='Clear existing vanilla classes before generating'
        )

    def handle(self, *args, **options):
        self.stdout.write("=== Generating Vanilla Python Classes for Base Game ===\n")
        
        # Setup paths
        backend_dir = Path(settings.BASE_DIR)
        
        # Output directory
        if options['output_dir']:
            vanilla_dir = Path(options['output_dir'])
        else:
            vanilla_dir = backend_dir / "gamedata" / "cache" / "vanilla_classes"
        
        # Clear existing if requested
        if options['clear_existing'] and vanilla_dir.exists():
            self.stdout.write("Clearing existing vanilla classes...")
            shutil.rmtree(vanilla_dir)
        
        vanilla_dir.mkdir(parents=True, exist_ok=True)
        
        # Create special ResourceManager that only sees base game files
        self.stdout.write("\nCreating vanilla-only ResourceManager...")
        vanilla_rm = VanillaResourceManager(suppress_warnings=True)
        
        # Create code cache for vanilla classes
        vanilla_cache = SecureCodeCache(vanilla_dir)
        generator = RuntimeDataClassGenerator()
        
        start_time = time.time()
        
        # Get list of ALL tables available in base game
        self.stdout.write("\nDiscovering base game tables...")
        base_game_tables = self._discover_base_game_tables(vanilla_rm)
        
        self.stdout.write(f"Found {len(base_game_tables)} base game tables")
        
        # Generate classes for each table
        self.stdout.write(f"\nGenerating vanilla classes...")
        generated_count = 0
        failed_count = 0
        
        for i, table_name in enumerate(base_game_tables, 1):
            try:
                # Show progress
                if i % 50 == 0 or i == len(base_game_tables):
                    self.stdout.write(f"Processing {i}/{len(base_game_tables)}: {table_name}")
                
                # Load table from base game only
                table_data = vanilla_rm.get_2da_with_overrides(table_name)
                if not table_data:
                    failed_count += 1
                    continue
                
                # Generate class code
                def generate_code():
                    return generator.generate_code_for_table(table_name, table_data)
                
                # Generate and save code
                code_string = vanilla_cache.load_or_generate(
                    table_name,
                    None,  # No file path needed
                    generate_code
                )
                
                if code_string:
                    generated_count += 1
                else:
                    failed_count += 1
                    
            except Exception as e:
                self.stdout.write(f"Failed to generate class for {table_name}: {e}")
                failed_count += 1
        
        elapsed = time.time() - start_time
        
        # Create __init__.py to make it a package
        init_file = vanilla_dir / "__init__.py"
        init_file.write_text('"""Pre-generated vanilla classes for base game 2DA tables."""\n')
        
        # Create metadata file
        metadata_file = vanilla_dir / "vanilla_metadata.json"
        metadata_content = {
            "description": "Pre-generated Python classes for base game 2DA tables only",
            "generated_at": time.strftime('%Y-%m-%d %H:%M:%S'),
            "tables_processed": len(base_game_tables),
            "classes_generated": generated_count,
            "generation_failed": failed_count,
            "generation_time_seconds": elapsed,
            "nwn2_installation": str(vanilla_rm.nwn2_path),
            "note": "These classes are for unmodified base game tables. Mods that add new tables will require dynamic generation."
        }
        
        with open(metadata_file, 'w') as f:
            json.dump(metadata_content, f, indent=2)
        
        # Get cache stats
        cache_stats = vanilla_cache.get_cache_stats()
        
        self.stdout.write(self.style.SUCCESS(
            f"\n✓ Vanilla class generation complete!"
        ))
        self.stdout.write(f"Generated in: {elapsed:.2f} seconds")
        self.stdout.write(f"Tables processed: {len(base_game_tables)}")
        self.stdout.write(f"Classes generated: {generated_count}")
        self.stdout.write(f"Generation failed: {failed_count}")
        self.stdout.write(f"Cache files: {cache_stats.get('file_count', 0)}")
        self.stdout.write(f"Total size: {cache_stats.get('total_size_kb', 0):.1f} KB")
        
        self.stdout.write(self.style.SUCCESS(
            f"\nVanilla classes saved to:\n  {vanilla_dir}"
        ))
        self.stdout.write(
            "\nThese classes can be shipped with your application to provide "
            "near-instant startup for users without mods, while still supporting "
            "full dynamic generation for modded installations."
        )

    def _discover_base_game_tables(self, vanilla_rm: VanillaResourceManager) -> list:
        """Discover all 2DA tables available in base game files."""
        tables = set()
        
        # Get all tables from the ResourceManager's index
        # This only contains base game tables since we cleared all overrides
        if hasattr(vanilla_rm, '_2da_locations'):
            for table_name in vanilla_rm._2da_locations.keys():
                # Remove .2da extension for consistency
                clean_name = table_name.replace('.2da', '')
                tables.add(clean_name)
        
        return sorted(list(tables))