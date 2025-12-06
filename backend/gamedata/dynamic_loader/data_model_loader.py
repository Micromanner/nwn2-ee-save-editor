"""
Data Model Loader - Async loading system with progress tracking

This module handles the loading of game data at application startup,
generating runtime classes and providing progress updates to the UI.
"""
import asyncio
import logging
import json
from typing import Dict, List, Any, Optional, Callable, Tuple, Type
from pathlib import Path
import time

from gamedata.dynamic_loader.runtime_class_generator import RuntimeDataClassGenerator
from gamedata.dynamic_loader.code_cache import SecureCodeCache
from gamedata.dynamic_loader.relationship_validator import RelationshipValidator, ValidationReport
from parsers.resource_manager import ResourceManager
from utils.performance_profiler import get_profiler

logger = logging.getLogger(__name__)


class LoadingProgress:
    """Simple progress tracking for UI updates."""
    
    def __init__(self, callback: Optional[Callable[[str, int], None]] = None):
        self.callback = callback
        self.current = 0
        self.total = 100
        self.message = ""
    
    def update(self, message: str, percent: int):
        """Update progress with message and percentage."""
        self.message = message
        self.current = percent
        if self.callback:
            self.callback(message, percent)
        else:
            logger.debug(f"Loading: {message} ({percent}%)")


class DataModelLoader:
    """
    Loads all game data with progress updates and optimized sequential processing.
    
    Features:
    - Async I/O for file operations
    - Sequential class loading with vanilla class optimization
    - Progress tracking for UI updates
    - Caching for faster subsequent loads
    """
    
    # Common 2DA files that should be loaded first (essential for character creation/editing)
    PRIORITY_TABLES = [
        'classes', 'racialtypes', 'feat', 'skills', 'spells',
        'baseitems', 'appearance', 'gender', 'alignment',
        'categories', 'cls_atk_1', 'cls_atk_2', 'cls_atk_3',
        'backgrounds', 'domains'  # Added commonly accessed tables
    ]
    
    def __init__(self, resource_manager: ResourceManager, 
                 cache_dir: Optional[Path] = None,
                 progress_callback: Optional[Callable[[str, int], None]] = None,
                 validate_relationships: bool = True,
                 priority_only: bool = False):
        """
        Initialize the data model loader.
        
        Args:
            resource_manager: ResourceManager instance for loading 2DA files
            cache_dir: Directory for code cache (defaults to backend/cache/generated_code)
            progress_callback: Optional callback for progress updates
            validate_relationships: Whether to validate table relationships
        """
        self.rm = resource_manager
        self.generator = RuntimeDataClassGenerator()
        self.progress = LoadingProgress(progress_callback)
        self.validate_relationships = validate_relationships
        self.priority_only = priority_only
        
        # Set up cache directory
        if cache_dir is None:
            # Default to backend/cache/generated_code
            backend_dir = Path(__file__).parent.parent
            cache_dir = backend_dir / "cache" / "generated_code"
        self.cache = SecureCodeCache(cache_dir)
        
        # Storage for generated classes and instances
        self.generated_classes: Dict[str, Type] = {}
        self.table_data: Dict[str, List[Any]] = {}
        
        # Relationship validator
        self.relationship_validator = RelationshipValidator()
        self.validation_report: Optional[ValidationReport] = None
        
        # Performance tracking for TLK batch lookups
        self._string_lookup_stats = {
            'total_lookups': 0,
            'batch_time_ms': 0.0,
            'cache_hits': 0
        }
    
    async def load_game_data(self) -> Dict[str, Any]:
        """
        Load all game data with progress updates.
        
        Returns:
            Dictionary of table_name -> list of data instances
        """
        profiler = get_profiler()
        start_time = time.time()
        
        with profiler.profile("DataModelLoader.load_game_data"):
            try:
                # Step 1: Scan for 2DA files
                self.progress.update("Scanning game files...", 10)
                with profiler.profile("Scan 2DA Files"):
                    tables = await self._scan_2da_files()
                    profiler.add_metadata("table_count", len(tables))
                
                # Step 2: Sort tables by dependency order (if possible) or priority
                self.progress.update("Analyzing dependencies...", 15)
                with profiler.profile("Sort by Dependencies"):
                    tables = await self._sort_by_dependency_order(tables)
                
                # Step 3: Load classes (with vanilla optimization)
                self.progress.update("Loading data models...", 30)
                with profiler.profile("Load Runtime Classes"):
                    await self._load_classes(tables)
                
                # Step 4: Load data into instances
                self.progress.update("Loading game data...", 60)
                with profiler.profile("Load Table Data"):
                    await self._load_table_data(tables)
                
                # Step 5: Post-processing and validation
                self.progress.update("Finalizing...", 90)
                with profiler.profile("Finalize Data"):
                    await self._finalize_data()
                
                # Done!
                self.progress.update("Complete!", 100)
                
                elapsed = time.time() - start_time
                logger.info(f"Loaded {len(self.table_data)} tables in {elapsed:.2f}s")
                
                return self.table_data
                
            except Exception as e:
                logger.error(f"Failed to load game data: {e}")
                raise
    
    def _get_base_character_files(self) -> List[str]:
        """Get the base list of character-related 2DA files."""
        # Load the filtered list of character-related 2DA files
        filter_file = Path(__file__).parent.parent.parent / 'config' / 'nw2_data_filtered.json'
    
        if filter_file.exists():
            import json
            with open(filter_file, 'r') as f:
                filter_data = json.load(f)
            
            # Get the character files list
            character_files = filter_data.get('character_files', [])
            logger.info(f"Using filtered list of {len(character_files)} character-related 2DA files")
            
            # Remove .2da extension for processing
            return [f.replace('.2da', '') for f in character_files]
        else:
            # Fallback to hardcoded list if filter file not found
            logger.warning(f"Filter file not found at {filter_file}, using fallback list")
            table_names = [
                'appearance', 'baseitems', 'categories', 'classes', 'cls_atk_1',
                'cls_atk_2', 'cls_atk_3', 'domains', 'feat', 'gender', 
                'iprp_damagetype', 'racialtypes', 'racialsubtypes', 'skills',
                'spells', 'spellschools'
            ]
            
            # Add common class-specific tables
            for class_abbr in ['barb', 'bard', 'cler', 'druid', 'fight', 'monk',
                              'pal', 'rang', 'rog', 'sorc', 'wiz', 'wlck']:
                table_names.extend([
                    f'cls_feat_{class_abbr}',
                    f'cls_savthr_{class_abbr}',
                    f'cls_skill_{class_abbr}',
                    f'cls_spgn_{class_abbr}'
                ])
            
            return table_names
    
    def _discover_custom_mod_files(self) -> List[str]:
        """
        Discover custom mod .2da files from workshop and override directories.
        
        Critical for save compatibility - we must load ALL available content
        to avoid corrupting saves that reference custom classes/feats/spells.
        """
        custom_files = set()
        
        # Load ignore prefixes to filter out irrelevant files
        ignore_prefixes = self._get_ignore_prefixes()
        character_prefixes = self._get_character_prefixes()
        
        # Check all override directories that ResourceManager knows about
        rm = self.rm
        
        # 1. Steam Workshop files  
        if hasattr(rm, '_workshop_file_paths'):
            for filename in rm._workshop_file_paths.keys():
                table_name = filename.replace('.2da', '')
                if self._is_character_related_file(table_name, character_prefixes, ignore_prefixes):
                    custom_files.add(table_name)
        
        # 2. User override directory files
        if hasattr(rm, '_override_file_paths'):
            for filename in rm._override_file_paths.keys():
                table_name = filename.replace('.2da', '')
                if self._is_character_related_file(table_name, character_prefixes, ignore_prefixes):
                    custom_files.add(table_name)
        
        # 3. Custom override directory files
        if hasattr(rm, '_custom_override_paths'):
            for filename in rm._custom_override_paths.keys():
                table_name = filename.replace('.2da', '')
                if self._is_character_related_file(table_name, character_prefixes, ignore_prefixes):
                    custom_files.add(table_name)
        
        # 4. Module-specific overrides (from HAKs or module files)
        if hasattr(rm, '_module_overrides'):
            for filename in rm._module_overrides.keys():
                table_name = filename.replace('.2da', '')
                if self._is_character_related_file(table_name, character_prefixes, ignore_prefixes):
                    custom_files.add(table_name)
        
        # 5. HAK overrides
        if hasattr(rm, '_hak_overrides'):
            for hak_dict in rm._hak_overrides:
                for filename in hak_dict.keys():
                    table_name = filename.replace('.2da', '')
                    if self._is_character_related_file(table_name, character_prefixes, ignore_prefixes):
                        custom_files.add(table_name)
        
        return sorted(list(custom_files))
    
    def _get_ignore_prefixes(self) -> List[str]:
        """Get the list of prefixes to ignore when discovering custom files."""
        filter_file = Path(__file__).parent.parent.parent / 'config' / 'nw2_data_filtered.json'
        
        if filter_file.exists():
            import json
            with open(filter_file, 'r') as f:
                filter_data = json.load(f)
            return filter_data.get('ignore_prefixes', [])
        
        # Fallback list of common non-character prefixes
        return [
            'ambientmusic', 'ambientsound', 'texture', 'tile', 'light', 'sound', 
            'vfx_', 'grass', 'water', 'sky', 'footstepsounds', 'inventorysnds'
        ]
    
    def _get_character_prefixes(self) -> List[str]:
        """Get the list of prefixes that indicate character-related files."""
        filter_file = Path(__file__).parent.parent.parent / 'config' / 'nw2_data_filtered.json'
        
        if filter_file.exists():
            import json
            with open(filter_file, 'r') as f:
                filter_data = json.load(f)
            return filter_data.get('character_prefixes', [])
        
        # Fallback list of character-related prefixes
        return [
            'classes', 'cls_', 'feat', 'spells', 'skills', 'race_', 'racial',
            'appearance', 'baseitems', 'armor', 'color_', 'backgrounds', 'domains'
        ]
    
    def _is_character_related_file(self, table_name: str, character_prefixes: List[str], ignore_prefixes: List[str]) -> bool:
        """
        Determine if a 2DA file is character-related and should be loaded.
        
        Args:
            table_name: Name of the table (without .2da extension)
            character_prefixes: List of prefixes that indicate character files
            ignore_prefixes: List of prefixes to ignore
            
        Returns:
            True if file should be loaded for character compatibility
        """
        table_lower = table_name.lower()
        
        # First check ignore list - if it matches, skip it
        for ignore_prefix in ignore_prefixes:
            if table_lower.startswith(ignore_prefix.lower()):
                return False
        
        # Check character prefixes - if it matches, include it
        for char_prefix in character_prefixes:
            if table_lower.startswith(char_prefix.lower()):
                return True
        
        # Special case: files that are clearly character-related but might not match prefixes
        character_keywords = [
            'class', 'feat', 'spell', 'skill', 'race', 'appearance', 'item', 
            'armor', 'weapon', 'background', 'domain', 'portrait', 'package'
        ]
        
        for keyword in character_keywords:
            if keyword in table_lower:
                return True
        
        # Default to exclude if we can't determine it's character-related
        return False
    
    async def _scan_2da_files(self) -> List[Dict[str, Any]]:
        """Scan for available 2DA files including custom mod content for save compatibility."""
        tables = []
        
        # Get data fetching rules instance to use scan mode
        from gamedata.services.data_fetching_rules import get_data_fetching_rules
        rules = get_data_fetching_rules()
        
        # Use scan mode to suppress recovery messages during table scanning
        with rules.scan_mode():
            # If priority_only mode, just return priority tables
            if self.priority_only:
                logger.info(f"Priority-only mode: loading {len(self.PRIORITY_TABLES)} essential tables")
                table_names = self.PRIORITY_TABLES
            else:
                # Load the base character-related files
                base_table_names = self._get_base_character_files()
                
                # Discover custom mod content to ensure save compatibility
                custom_table_names = self._discover_custom_mod_files()
                
                # Combine base + custom, removing duplicates while preserving order
                table_names = list(dict.fromkeys(base_table_names + custom_table_names))
                
                logger.info(f"Loading {len(base_table_names)} base + {len(custom_table_names)} custom = {len(table_names)} total 2DA files")
                if custom_table_names:
                    logger.info(f"Custom mod files detected: {', '.join(custom_table_names[:10])}")
                    if len(custom_table_names) > 10:
                        logger.info(f"... and {len(custom_table_names) - 10} more custom files")
            
            # Load tables from cache (msgpack) via ResourceManager
            for table_name in table_names:
                try:
                    # ResourceManager will load from msgpack cache first
                    table_data = self.rm.get_2da_with_overrides(table_name)
                    if table_data:
                        tables.append({
                            'name': table_name,
                            'data': table_data,
                            'row_count': table_data.get_resource_count() if hasattr(table_data, 'get_resource_count') else 0
                        })
                except Exception as e:
                    logger.debug(f"Skipping table {table_name}: {e}")
                    continue
        
        logger.info(f"Found {len(tables)} 2DA tables to process")
        return tables
    
    def _sort_tables_by_priority(self, tables: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Sort tables so priority tables are processed first."""
        priority_tables = []
        other_tables = []
        
        for table in tables:
            if table['name'] in self.PRIORITY_TABLES:
                priority_tables.append(table)
            else:
                other_tables.append(table)
        
        # Sort each group by size (smaller first for quicker initial progress)
        priority_tables.sort(key=lambda t: t['row_count'])
        other_tables.sort(key=lambda t: t['row_count'])
        
        return priority_tables + other_tables
    
    async def _sort_by_dependency_order(self, tables: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """
        Sort tables by dependency order to prevent retry loops during loading.
        
        This method analyzes table columns to detect relationships and uses
        topological sort to ensure referenced tables load before dependent ones.
        
        Falls back to priority sorting if dependency detection fails.
        """
        try:
            # Create a temporary data structure for relationship detection
            # We need column information to detect relationships
            temp_table_data = {}
            table_by_name = {}
            
            for table_info in tables:
                table_name = table_info['name']
                table_data = table_info['data']
                table_by_name[table_name] = table_info
                
                # Get column names from the table data
                columns = []
                if hasattr(table_data, 'get_columns'):
                    columns = table_data.get_columns()
                elif hasattr(table_data, 'columns'):
                    columns = table_data.columns
                else:
                    # Try to get first row to extract column names
                    if hasattr(table_data, 'get_resource'):
                        first_row = table_data.get_resource(0)
                        if first_row:
                            columns = list(first_row.keys())
                
                # Create a simple structure for relationship detection
                # RelationshipValidator expects objects with attributes
                class TableStub:
                    def __init__(self, name, cols):
                        self.table_name = name
                        self.columns = cols
                        # Add dummy instances to satisfy validator
                        self.instances = []
                
                temp_table_data[table_name] = [TableStub(table_name, columns)]
            
            # Use RelationshipValidator to detect dependencies
            from gamedata.dynamic_loader.relationship_validator import RelationshipValidator
            validator = RelationshipValidator()
            
            # Detect relationships based on column patterns
            validator.detect_relationships(temp_table_data)
            
            # Get the dependency order
            dependency_order = validator._calculate_load_order()
            
            if dependency_order:
                logger.info(f"Calculated dependency order for {len(dependency_order)} tables")
                logger.debug(f"Load order preview: {dependency_order[:10]}...")
                
                # Sort tables according to dependency order
                ordered_tables = []
                remaining_tables = []
                
                # First add tables in dependency order
                for table_name in dependency_order:
                    if table_name in table_by_name:
                        ordered_tables.append(table_by_name[table_name])
                
                # Then add any tables not in dependency order
                for table_info in tables:
                    if table_info['name'] not in dependency_order:
                        remaining_tables.append(table_info)
                
                # Combine ordered + remaining
                result = ordered_tables + remaining_tables
                
                logger.info(f"Tables will load in dependency order: {len(ordered_tables)} ordered, {len(remaining_tables)} unordered")
                return result
            else:
                logger.warning("No dependency order calculated, falling back to priority sorting")
                return self._sort_tables_by_priority(tables)
                
        except Exception as e:
            logger.warning(f"Failed to calculate dependency order: {e}, falling back to priority sorting")
            return self._sort_tables_by_priority(tables)
    
    async def _load_classes(self, tables: List[Dict[str, Any]]):
        """Load Python classes for all tables with vanilla optimization."""
        profiler = get_profiler()
        total_tables = len(tables)
        
        # Separate priority tables for faster processing
        priority_tables = [t for t in tables if t['name'] in self.PRIORITY_TABLES]
        other_tables = [t for t in tables if t['name'] not in self.PRIORITY_TABLES]
        
        # Process priority tables first
        with profiler.profile("Load Priority Classes", count=len(priority_tables)):
            for i, table_info in enumerate(priority_tables):
                with profiler.profile(f"Load Class: {table_info['name']}"):
                    self._load_class_for_table(table_info)
                progress = 30 + int((i / total_tables) * 15)
                self.progress.update(f"Loaded priority class: {table_info['name']}", progress)
        
        # Process remaining tables sequentially (no parallel overhead)
        with profiler.profile("Load Other Classes", count=len(other_tables)):
            for i, table_info in enumerate(other_tables):
                with profiler.profile(f"Load Class: {table_info['name']}"):
                    self._load_class_for_table(table_info)
                # Update progress every 10th table or at the end
                progress = 45 + int((i / len(other_tables)) * 15)
                if i % 10 == 0 or i == len(other_tables) - 1:
                    self.progress.update(f"Loaded class for {table_info['name']}", progress)
        
        # Log summary of vanilla vs dynamic classes
        vanilla_count = sum(1 for name in self.generated_classes.keys() 
                           if self._has_vanilla_class(name))
        total_count = len(self.generated_classes)
        logger.info(f"Class loading complete: {vanilla_count} vanilla classes, {total_count - vanilla_count} dynamic classes")
    
    def _load_class_for_table(self, table_info: Dict[str, Any]):
        """Load a class for a single table (synchronous for speed)."""
        table_name, generated_class = self._load_class_sync(table_info)
        self.generated_classes[table_name] = generated_class
    
    def _load_class_sync(self, table_info: Dict[str, Any]) -> Tuple[str, Type]:
        """Load a class for a single table (optimized with vanilla class loading)."""
        table_name = table_info['name']
        table_data = table_info['data']
        
        # OPTIMIZATION: Try to load vanilla class first (works for 99.9% of cases)
        vanilla_class = self._try_load_vanilla_class(table_name)
        if vanilla_class:
            logger.debug(f"Using pre-generated vanilla class for {table_name}")
            return table_name, vanilla_class
        
        # Fallback: Generate class dynamically (for new mod tables or missing vanilla classes)
        def generate_code():
            return self.generator.generate_code_for_table(table_name, table_data)
        
        # Get cached or generate new code
        code_string = self.cache.load_or_generate(
            table_name,
            None,  # No specific file path for base game tables
            generate_code
        )
        
        # Compile the code to get the class
        namespace = {}  # Use default builtins which includes __build_class__
        exec(code_string, namespace)
        
        # Find the generated class in namespace
        class_name = self.generator._generate_class_name(table_name)
        generated_class = namespace[class_name]
        
        return table_name, generated_class
    
    def _try_load_vanilla_class(self, table_name: str) -> Optional[Type]:
        """
        Try to load a pre-generated vanilla class for any table.
        
        Simplified approach: Always try vanilla first, regardless of mod override status.
        Vanilla classes can handle modded data perfectly when mods only add rows (99.9% of cases).
        
        Args:
            table_name: Name of the table (without .2da extension)
            
        Returns:
            Pre-generated class if available, None otherwise
        """
        try:
            # Try to import vanilla class from vanilla_classes directory
            from pathlib import Path
            
            # Get backend directory (FastAPI app, not Django)
            backend_dir = Path(__file__).parent.parent.parent
            vanilla_dir = backend_dir / "gamedata" / "cache" / "vanilla_classes"
            vanilla_file = vanilla_dir / f"{table_name}.py"
            
            if not vanilla_file.exists():
                # No vanilla class available
                return None
            
            # Import the vanilla class dynamically
            import importlib.util
            
            # Create module spec
            spec = importlib.util.spec_from_file_location(f"vanilla_{table_name}", vanilla_file)
            if spec is None or spec.loader is None:
                return None
            
            # Load the module
            vanilla_module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(vanilla_module)
            
            # Find the class in the module (should match the generator's naming pattern)
            class_name = self.generator._generate_class_name(table_name)
            
            if hasattr(vanilla_module, class_name):
                vanilla_class = getattr(vanilla_module, class_name)
                logger.debug(f"Loaded vanilla class for {table_name}")
                return vanilla_class
            else:
                logger.debug(f"Vanilla class not found for {table_name}, using dynamic generation")
                return None
                
        except Exception as e:
            logger.debug(f"Could not load vanilla class for {table_name}: {e}")
            return None
    
    def _has_vanilla_class(self, table_name: str) -> bool:
        """Check if a table has a vanilla class available."""
        backend_dir = Path(__file__).parent.parent.parent
        vanilla_dir = backend_dir / "gamedata" / "cache" / "vanilla_classes"
        vanilla_file = vanilla_dir / f"{table_name}.py"
        return vanilla_file.exists()
    
    async def _load_table_data(self, tables: List[Dict[str, Any]]):
        """Load actual data into class instances with universal batch creation for maximum performance."""
        total_tables = len(tables)
        
        
        for i, table_info in enumerate(tables):
            table_name = table_info['name']
            table_data = table_info['data']
            
            if table_name not in self.generated_classes:
                logger.warning(f"No generated class for table {table_name}")
                continue
            
            # Get the generated class
            data_class = self.generated_classes[table_name]

            # Collect row data
            row_count = table_data.get_resource_count() if hasattr(table_data, 'get_resource_count') else 0
            row_data_list = []
            for row_id in range(row_count):
                try:
                    if hasattr(table_data, 'get_row_dict'):
                        row_dict = table_data.get_row_dict(row_id)
                        if row_dict:
                            row_data_list.append(row_dict)
                except Exception as e:
                    logger.warning(f"Failed to load row {row_id} from {table_name}: {e}")
            
            # Pre-populate string cache using batch TLK lookups
            string_cache = self._create_string_cache(row_data_list, table_name)

            # Batch creation for all tables
            try:
                instances = data_class.create_batch(row_data_list, self.rm, string_cache)
            except Exception as e:
                logger.error(f"Batch creation failed for {table_name}: {e}")
                raise RuntimeError(f"Failed to create instances for table {table_name} using batch method: {e}") from e

            # Store instances
            self.table_data[table_name] = instances
            
            # Update progress - only log every 25th table, priority tables, or final table
            progress = 60 + int((i / total_tables) * 30)
            if i % 25 == 0 or table_name in self.PRIORITY_TABLES or i == total_tables - 1:
                self.progress.update(
                    f"Loaded {table_name} ({len(instances)} rows)", 
                    progress
                )
            else:
                # Still update internal progress but don't log
                self.progress.current = progress
            
            # Yield control occasionally without sleep overhead
            if i % 50 == 0:
                await asyncio.sleep(0)
    
    
    async def _finalize_data(self):
        """Perform any final validation or post-processing."""
        # Log summary statistics
        total_rows = sum(len(instances) for instances in self.table_data.values())
        logger.info(f"Loaded {len(self.table_data)} tables with {total_rows} total rows")
        
        # Validate relationships if enabled
        if self.validate_relationships:
            try:
                # Try to load cached relationships first
                cached_data = self.cache.load_relationships()
                if cached_data:
                    # Verify cache is still valid
                    current_hash = self.cache.get_relationships_hash(self.table_data)
                    cached_hash = cached_data.get('table_structure_hash')
                    
                    if current_hash == cached_hash:
                        # Use cached relationships
                        logger.info("Using cached relationship data")
                        # TODO: Reconstruct RelationshipValidator state from cache
                        self.validation_report = ValidationReport(**cached_data['validation_report'])
                    else:
                        # Cache is outdated, regenerate
                        logger.info("Table structure changed, regenerating relationships")
                        self._detect_and_validate_relationships()
                else:
                    # No cache, generate new
                    self._detect_and_validate_relationships()
                    
            except Exception as e:
                logger.error(f"Failed to validate relationships: {e}")
                # Don't fail the entire load due to validation errors
        
        # Clean up cache if needed
        self.cache.cleanup_orphaned_files()
    
    def _detect_and_validate_relationships(self):
        """Detect and validate relationships, then cache the results."""
        # Detect relationships
        self.relationship_validator.detect_relationships(self.table_data)
        
        # Validate them
        self.validation_report = self.relationship_validator.validate_relationships()
        
        # Log validation results
        logger.info(self.validation_report.get_summary())
        
        # If we have a suggested load order, log it
        if self.validation_report.dependency_order:
            logger.debug(f"Suggested table load order: {self.validation_report.dependency_order[:10]}...")
        
        # Warn about broken references
        if self.validation_report.broken_references:
            logger.warning(f"Found {len(self.validation_report.broken_references)} broken references")
        
        # Cache the results
        try:
            self.cache.save_relationships(
                self.relationship_validator.relationships,
                self.validation_report
            )
            # Also save the current table structure hash
            relationships_file = self.cache.cache_dir / "relationships.json"
            if relationships_file.exists():
                with open(relationships_file, 'r+') as f:
                    data = json.load(f)
                    data['table_structure_hash'] = self.cache.get_relationships_hash(self.table_data)
                    f.seek(0)
                    json.dump(data, f, indent=2)
                    f.truncate()
        except Exception as e:
            logger.error(f"Failed to cache relationships: {e}")
    
    def get_table(self, table_name: str) -> List[Any]:
        """
        Get all instances for a table.
        
        Args:
            table_name: Name of the table (without .2da extension)
            
        Returns:
            List of data instances
        """
        return self.table_data.get(table_name, [])
    
    def get_by_id(self, table_name: str, row_id: int) -> Optional[Any]:
        """
        Get a specific row by ID.
        
        Args:
            table_name: Name of the table
            row_id: Row ID/index
            
        Returns:
            Data instance or None
        """
        instances = self.get_table(table_name)
        if 0 <= row_id < len(instances):
            return instances[row_id]
        return None
    
    def get_stats(self) -> Dict[str, Any]:
        """Get statistics about loaded data."""
        stats = {
            'tables_loaded': len(self.table_data),
            'total_rows': sum(len(instances) for instances in self.table_data.values()),
            'cache_stats': self.cache.get_cache_stats(),
            'tables': {
                name: len(instances) 
                for name, instances in self.table_data.items()
            }
        }
        
        # Add relationship validation stats if available
        if self.validation_report:
            stats['relationships'] = {
                'total': self.validation_report.total_relationships,
                'valid': self.validation_report.valid_relationships,
                'broken_references': len(self.validation_report.broken_references),
                'missing_tables': len(self.validation_report.missing_tables)
            }
        
        return stats
    
    def get_validation_report(self) -> Optional[ValidationReport]:
        """Get the relationship validation report if available."""
        return self.validation_report
    
    def get_table_relationships(self, table_name: str) -> Dict[str, Any]:
        """
        Get relationship information for a specific table.
        
        Args:
            table_name: Name of the table
            
        Returns:
            Dict with 'dependencies' and 'dependents' lists
        """
        if not self.relationship_validator:
            return {'dependencies': [], 'dependents': []}
        
        return {
            'dependencies': list(self.relationship_validator.get_table_dependencies(table_name)),
            'dependents': list(self.relationship_validator.get_table_dependents(table_name))
        }
    
    def _create_string_cache(self, row_data_list: List[Dict[str, Any]], table_name: str) -> Dict[int, str]:
        """
        Create optimized string cache using batch TLK lookups for performance.
        
        This method analyzes all row data to identify string references, then uses
        the Rust TLK parser's batch functionality to resolve them all at once.
        This replaces thousands of individual TLK lookups with a single batch operation.
        
        Args:
            row_data_list: List of row dictionaries from 2DA data
            table_name: Name of the table being processed (for logging)
            
        Returns:
            Dictionary mapping str_ref -> resolved string for caching
        """
        if not row_data_list:
            return {}
        
        # Common string reference field patterns (from RuntimeDataClassGenerator)
        string_ref_fields = {
            'name', 'label', 'description', 'desc', 'displayname', 'tooltip',
            'strref', 'namestrref', 'descriptionstrref', 'tooltipstrref',
            'displaynamestrref', 'prereqfeat1', 'prereqfeat2', 'masterfeat',
            'successorfeat', 'spellid', 'category', 'featcategory',
            'allclassescanuse', 'icon', 'iconresref'
        }
        
        # Collect all potential string references
        str_refs_to_resolve = set()
        start_time = time.time()
        
        for row_dict in row_data_list:
            for column_name, value in row_dict.items():
                # Check if this looks like a string reference field
                if (column_name.lower() in string_ref_fields and 
                    isinstance(value, (str, int))):
                    try:
                        int_val = int(value)
                        # Only resolve reasonable string reference IDs (avoid huge numbers)
                        if 1 <= int_val <= 65535:
                            str_refs_to_resolve.add(int_val)
                    except (ValueError, TypeError):
                        pass
        
        # Early exit if no string references found
        if not str_refs_to_resolve:
            return {}
        
        # Convert to list for batch processing
        str_refs_list = list(str_refs_to_resolve)
        
        # Use ResourceManager's batch string lookup if available
        string_cache = {}
        if hasattr(self.rm, 'get_strings_batch'):
            try:
                batch_start = time.time()
                string_cache = self.rm.get_strings_batch(str_refs_list)
                batch_time = (time.time() - batch_start) * 1000
                
                # Update performance stats
                self._string_lookup_stats['total_lookups'] += len(str_refs_list)
                self._string_lookup_stats['batch_time_ms'] += batch_time
                
                # Calculate strings/sec safely (avoid division by zero)
                strings_per_sec = (len(str_refs_list) / batch_time * 1000) if batch_time > 0 else float('inf')
                logger.debug(f"TLK batch lookup for {table_name}: "
                           f"{len(str_refs_list)} strings in {batch_time:.2f}ms "
                           f"({strings_per_sec:.0f} strings/sec)" if strings_per_sec != float('inf') else
                           f"{len(str_refs_list)} strings in {batch_time:.2f}ms (instant)")
                
            except Exception as e:
                logger.warning(f"Batch TLK lookup failed for {table_name}, falling back to individual lookups: {e}")
                # Fallback to individual lookups
                for str_ref in str_refs_list:
                    try:
                        resolved = self.rm.get_string(str_ref)
                        if resolved and resolved != f"{{StrRef:{str_ref}}}":
                            string_cache[str_ref] = resolved
                    except Exception:
                        pass
        else:
            # Fallback for resource managers without batch support
            logger.debug(f"ResourceManager doesn't support batch lookups, using individual lookups for {table_name}")
            for str_ref in str_refs_list:
                try:
                    resolved = self.rm.get_string(str_ref)
                    if resolved and resolved != f"{{StrRef:{str_ref}}}":
                        string_cache[str_ref] = resolved
                except Exception:
                    pass
        
        total_time = (time.time() - start_time) * 1000
        
        if string_cache:
            logger.debug(f"Created string cache for {table_name}: "
                       f"{len(string_cache)} resolved strings in {total_time:.2f}ms")
        
        return string_cache