"""Data Model Loader - Async loading system with progress tracking."""
import asyncio
from loguru import logger
import json
from typing import Dict, List, Any, Optional, Callable, Tuple, Type
from pathlib import Path
import time

from gamedata.dynamic_loader.runtime_class_generator import RuntimeDataClassGenerator
from gamedata.dynamic_loader.code_cache import SecureCodeCache
from gamedata.dynamic_loader.relationship_validator import RelationshipValidator, ValidationReport
from services.core.resource_manager import ResourceManager
from utils.performance_profiler import get_profiler
from utils.paths import get_writable_dir


class LoadingProgress:
    """Tracks loading progress and reports via callback."""
    
    def __init__(self, callback: Optional[Callable[[str, int], None]] = None):
        self.callback = callback
        self.current = 0
        self.total = 100
        self.message = ""

    def update(self, message: str, percent: int):
        """Update progress state and invoke callback."""

        self.message = message
        self.current = percent
        if self.callback:
            self.callback(message, percent)
        else:
            logger.debug(f"Loading: {message} ({percent}%)")


class DataModelLoader:
    """Async loading system for 2DA data with progress tracking and caching."""

    
    PRIORITY_TABLES = [
        'classes', 'racialtypes', 'feat', 'skills', 'spells',
        'baseitems', 'appearance', 'gender', 'alignment',
        'categories', 'cls_atk_1', 'cls_atk_2', 'cls_atk_3',
        'backgrounds', 'domains'
    ]
    
    def __init__(self, resource_manager: ResourceManager, 
                 cache_dir: Optional[Path] = None,
                 progress_callback: Optional[Callable[[str, int], None]] = None,
                 validate_relationships: bool = True,
                 priority_only: bool = False):
        self.rm = resource_manager
        self.generator = RuntimeDataClassGenerator()
        self.progress = LoadingProgress(progress_callback)
        self.validate_relationships = validate_relationships
        self.priority_only = priority_only

        if cache_dir is None:
            cache_dir = get_writable_dir("cache/generated_code")
        self.cache = SecureCodeCache(cache_dir)

        self.generated_classes: Dict[str, Type] = {}
        self.table_data: Dict[str, List[Any]] = {}

        self.relationship_validator = RelationshipValidator()
        self.validation_report: Optional[ValidationReport] = None
        self._string_lookup_stats = {
            'total_lookups': 0,
            'batch_time_ms': 0.0,
            'cache_hits': 0
        }

    
    async def load_game_data(self) -> Dict[str, Any]:
        """Load all 2DA data tables asynchronously with progress tracking."""

        profiler = get_profiler()
        start_time = time.time()
        
        with profiler.profile("DataModelLoader.load_game_data"):
            try:
                self.progress.update("Scanning game files...", 10)
                with profiler.profile("Scan 2DA Files"):
                    tables = await self._scan_2da_files()
                    profiler.add_metadata("table_count", len(tables))

                self.progress.update("Analyzing dependencies...", 15)
                with profiler.profile("Sort by Dependencies"):
                    tables = await self._sort_by_dependency_order(tables)


                self.progress.update("Loading data models...", 30)
                with profiler.profile("Load Runtime Classes"):
                    await self._load_classes(tables)

                self.progress.update("Loading game data...", 60)
                with profiler.profile("Load Table Data"):
                    await self._load_table_data(tables)

                self.progress.update("Finalizing...", 90)
                with profiler.profile("Finalize Data"):
                    await self._finalize_data()

                self.progress.update("Complete!", 100)
                
                elapsed = time.time() - start_time
                logger.info(f"Loaded {len(self.table_data)} tables in {elapsed:.2f}s")
                
                return self.table_data
                
            except Exception as e:
                logger.error(f"Failed to load game data: {e}")
                raise

    def _get_base_character_files(self) -> List[str]:
        """Get list of base character-related 2DA files from filter config."""
        filter_file = Path(__file__).parent.parent.parent / 'config' / 'nw2_data_filtered.json'

        if filter_file.exists():
            with open(filter_file, 'r') as f:
                filter_data = json.load(f)
            
            character_files = filter_data.get('character_files', [])
            logger.info(f"Using filtered list of {len(character_files)} character-related 2DA files")
            
            return [f.replace('.2da', '') for f in character_files]
        else:
            raise FileNotFoundError(f"Required filter file not found: {filter_file}")

    def _discover_custom_mod_files(self) -> List[str]:
        """Discover custom 2DA files from mods and overrides."""

        custom_files = set()
        
        ignore_prefixes = self._get_ignore_prefixes()
        character_prefixes = self._get_character_prefixes()
        
        rm = self.rm
        
        if hasattr(rm, '_workshop_file_paths'):
            for filename in rm._workshop_file_paths.keys():
                table_name = filename.replace('.2da', '')
                if self._is_character_related_file(table_name, character_prefixes, ignore_prefixes):
                    custom_files.add(table_name)
        
        if hasattr(rm, '_override_file_paths'):
            for filename in rm._override_file_paths.keys():
                table_name = filename.replace('.2da', '')
                if self._is_character_related_file(table_name, character_prefixes, ignore_prefixes):
                    custom_files.add(table_name)
        
        if hasattr(rm, '_custom_override_paths'):
            for filename in rm._custom_override_paths.keys():
                table_name = filename.replace('.2da', '')
                if self._is_character_related_file(table_name, character_prefixes, ignore_prefixes):
                    custom_files.add(table_name)
        
        if hasattr(rm, '_module_overrides'):
            for filename in rm._module_overrides.keys():
                table_name = filename.replace('.2da', '')
                if self._is_character_related_file(table_name, character_prefixes, ignore_prefixes):
                    custom_files.add(table_name)

        if hasattr(rm, '_hak_overrides'):
            for hak_dict in rm._hak_overrides:
                for filename in hak_dict.keys():
                    table_name = filename.replace('.2da', '')
                    if self._is_character_related_file(table_name, character_prefixes, ignore_prefixes):
                        custom_files.add(table_name)
        
        return sorted(list(custom_files))
    
    def _get_ignore_prefixes(self) -> List[str]:
        """Get list of 2DA prefixes to ignore during discovery."""

        filter_file = Path(__file__).parent.parent.parent / 'config' / 'nw2_data_filtered.json'
        
        if filter_file.exists():
            with open(filter_file, 'r') as f:
                filter_data = json.load(f)
            return filter_data.get('ignore_prefixes', [])

        return [
            'ambientmusic', 'ambientsound', 'texture', 'tile', 'light', 'sound', 
            'vfx_', 'grass', 'water', 'sky', 'footstepsounds', 'inventorysnds'
        ]
    
    def _get_character_prefixes(self) -> List[str]:
        """Get list of character-related 2DA prefixes."""

        filter_file = Path(__file__).parent.parent.parent / 'config' / 'nw2_data_filtered.json'
        
        if filter_file.exists():
            with open(filter_file, 'r') as f:
                filter_data = json.load(f)
            return filter_data.get('character_prefixes', [])

        return [
            'classes', 'cls_', 'feat', 'spells', 'skills', 'race_', 'racial',
            'appearance', 'baseitems', 'armor', 'color_', 'backgrounds', 'domains'
        ]
    
    def _is_character_related_file(self, table_name: str, character_prefixes: List[str], ignore_prefixes: List[str]) -> bool:
        """Check if a table file is character-related based on prefix patterns."""
        table_lower = table_name.lower()

        for ignore_prefix in ignore_prefixes:
            if table_lower.startswith(ignore_prefix.lower()):
                return False

        for char_prefix in character_prefixes:
            if table_lower.startswith(char_prefix.lower()):
                return True

        character_keywords = [
            'class', 'feat', 'spell', 'skill', 'race', 'appearance', 'item', 
            'armor', 'weapon', 'background', 'domain', 'portrait', 'package'
        ]
        
        for keyword in character_keywords:
            if keyword in table_lower:
                return True

        return False

    async def _scan_2da_files(self) -> List[Dict[str, Any]]:
        """Scan and load all 2DA files from game data and mods."""

        tables = []

        from services.gamedata.data_fetching_rules import get_data_fetching_rules
        rules = get_data_fetching_rules()

        with rules.scan_mode():
            if self.priority_only:
                logger.info(f"Priority-only mode: loading {len(self.PRIORITY_TABLES)} essential tables")
                table_names = self.PRIORITY_TABLES
            else:
                base_table_names = self._get_base_character_files()
                custom_table_names = self._discover_custom_mod_files()
                table_names = list(dict.fromkeys(base_table_names + custom_table_names))
                
                logger.info(f"Loading {len(base_table_names)} base + {len(custom_table_names)} custom = {len(table_names)} total 2DA files")
                if custom_table_names:
                    logger.info(f"Custom mod files detected: {', '.join(custom_table_names[:10])}")
                    if len(custom_table_names) > 10:
                        logger.info(f"... and {len(custom_table_names) - 10} more custom files")

            for table_name in table_names:
                try:
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
        """Sort tables with priority tables first, then by size."""

        priority_tables = []
        other_tables = []
        
        for table in tables:
            if table['name'] in self.PRIORITY_TABLES:
                priority_tables.append(table)
            else:
                other_tables.append(table)

        priority_tables.sort(key=lambda t: t['row_count'])
        other_tables.sort(key=lambda t: t['row_count'])
        
        return priority_tables + other_tables
    
    async def _sort_by_dependency_order(self, tables: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Sort tables by dependency order using relationship detection."""
        try:
            temp_table_data = {}
            table_by_name = {}

            for table_info in tables:
                table_name = table_info['name']
                table_data = table_info['data']
                table_by_name[table_name] = table_info

                columns = []
                if hasattr(table_data, 'get_columns'):
                    columns = table_data.get_columns()
                elif hasattr(table_data, 'columns'):
                    columns = table_data.columns
                elif hasattr(table_data, 'get_resource'):
                    first_row = table_data.get_resource(0)
                    if first_row:
                        columns = list(first_row.keys())

                class TableStub:
                    def __init__(self, name, cols):
                        self.table_name = name
                        self.columns = cols
                        self.instances = []
                
                temp_table_data[table_name] = [TableStub(table_name, columns)]

            from gamedata.dynamic_loader.relationship_validator import RelationshipValidator
            validator = RelationshipValidator()

            validator.detect_relationships(temp_table_data)

            dependency_order = validator._calculate_load_order()
            
            if dependency_order:
                logger.debug(f"Calculated dependency order for {len(dependency_order)} tables")

                ordered_tables = []
                remaining_tables = []

                for table_name in dependency_order:
                    if table_name in table_by_name:
                        ordered_tables.append(table_by_name[table_name])

                for table_info in tables:
                    if table_info['name'] not in dependency_order:
                        remaining_tables.append(table_info)

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
        """Load or generate runtime data classes for all tables."""

        profiler = get_profiler()
        total_tables = len(tables)
        priority_tables = [t for t in tables if t['name'] in self.PRIORITY_TABLES]
        other_tables = [t for t in tables if t['name'] not in self.PRIORITY_TABLES]

        with profiler.profile("Load Priority Classes", count=len(priority_tables)):
            for i, table_info in enumerate(priority_tables):
                with profiler.profile(f"Load Class: {table_info['name']}"):
                    self._load_class_for_table(table_info)
                progress = 30 + int((i / total_tables) * 15)
                self.progress.update(f"Loaded priority class: {table_info['name']}", progress)

        with profiler.profile("Load Other Classes", count=len(other_tables)):
            for i, table_info in enumerate(other_tables):
                with profiler.profile(f"Load Class: {table_info['name']}"):
                    self._load_class_for_table(table_info)
                progress = 45 + int((i / len(other_tables)) * 15)
                if i % 10 == 0 or i == len(other_tables) - 1:
                    self.progress.update(f"Loaded class for {table_info['name']}", progress)

        vanilla_count = sum(1 for name in self.generated_classes.keys() 
                           if self._has_vanilla_class(name))
        total_count = len(self.generated_classes)
        logger.info(f"Class loading complete: {vanilla_count} vanilla classes, {total_count - vanilla_count} dynamic classes")
    
    def _load_class_for_table(self, table_info: Dict[str, Any]):
        """Load or generate a runtime class for a single table."""

        table_name, generated_class = self._load_class_sync(table_info)
        self.generated_classes[table_name] = generated_class
    
    def _load_class_sync(self, table_info: Dict[str, Any]) -> Tuple[str, Type]:
        """Synchronously load a class from cache or generate dynamically."""

        table_name = table_info['name']
        table_data = table_info['data']
        
        vanilla_class = self._try_load_vanilla_class(table_name)
        if vanilla_class:
        # logger.debug(f"Using pre-generated vanilla class for {table_name}")
            return table_name, vanilla_class
        
        def generate_code():
            return self.generator.generate_code_for_table(table_name, table_data)

        code_string = self.cache.load_or_generate(
            table_name,
            None,  # No specific file path for base game tables
            generate_code
        )

        namespace = {}
        exec(code_string, namespace)

        class_name = self.generator._generate_class_name(table_name)
        generated_class = namespace[class_name]
        
        return table_name, generated_class

    def _try_load_vanilla_class(self, table_name: str) -> Optional[Type]:
        """Attempt to load a pre-generated vanilla class from cache."""

        try:
            from pathlib import Path

            backend_dir = Path(__file__).parent.parent.parent
            vanilla_dir = backend_dir / "gamedata" / "cache" / "vanilla_classes"
            vanilla_file = vanilla_dir / f"{table_name}.py"

            if not vanilla_file.exists():
                return None

            import importlib.util

            spec = importlib.util.spec_from_file_location(f"vanilla_{table_name}", vanilla_file)
            if spec is None or spec.loader is None:
                return None

            vanilla_module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(vanilla_module)

            class_name = self.generator._generate_class_name(table_name)
            
            if hasattr(vanilla_module, class_name):
                vanilla_class = getattr(vanilla_module, class_name)
                # logger.debug(f"Loaded vanilla class for {table_name}")
                return vanilla_class
            else:
                # logger.debug(f"Vanilla class not found for {table_name}, using dynamic generation")
                return None
                
        except Exception as e:
            # logger.debug(f"Could not load vanilla class for {table_name}: {e}")
            return None

    def _has_vanilla_class(self, table_name: str) -> bool:
        """Check if a pre-generated vanilla class exists for a table."""

        backend_dir = Path(__file__).parent.parent.parent
        vanilla_dir = backend_dir / "gamedata" / "cache" / "vanilla_classes"
        vanilla_file = vanilla_dir / f"{table_name}.py"
        return vanilla_file.exists()
    
    async def _load_table_data(self, tables: List[Dict[str, Any]]):
        """Load data into generated class instances for all tables."""
        total_tables = len(tables)

        for i, table_info in enumerate(tables):
            table_name = table_info['name']
            table_data = table_info['data']
            
            if table_name not in self.generated_classes:
                logger.warning(f"No generated class for table {table_name}")
                continue

            data_class = self.generated_classes[table_name]
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

            string_cache = self._create_string_cache(row_data_list, table_name)

            try:
                instances = data_class.create_batch(row_data_list, self.rm, string_cache)
            except Exception as e:
                logger.error(f"Batch creation failed for {table_name}: {e}")
                raise RuntimeError(f"Failed to create instances for table {table_name} using batch method: {e}") from e

            self.table_data[table_name] = instances

            progress = 60 + int((i / total_tables) * 30)
            if i % 25 == 0 or table_name in self.PRIORITY_TABLES or i == total_tables - 1:
                self.progress.update(
                    f"Loaded {table_name} ({len(instances)} rows)", 
                    progress
                )
            else:
                self.progress.current = progress

            if i % 50 == 0:
                await asyncio.sleep(0)

    async def _finalize_data(self):
        """Finalize data loading and validate relationships."""
        total_rows = sum(len(instances) for instances in self.table_data.values())
        logger.info(f"Loaded {len(self.table_data)} tables with {total_rows} total rows")

        if self.validate_relationships:
            try:
                cached_data = self.cache.load_relationships()
                if cached_data:
                    current_hash = self.cache.get_relationships_hash(self.table_data)
                    cached_hash = cached_data.get('table_structure_hash')

                    if current_hash == cached_hash:
                        logger.info("Using cached relationship data")
                        self.validation_report = ValidationReport(**cached_data['validation_report'])
                    else:
                        logger.info("Table structure changed, regenerating relationships")
                        self._detect_and_validate_relationships()
                else:
                    self._detect_and_validate_relationships()
                    
            except Exception as e:
                logger.error(f"Failed to validate relationships: {e}")

        self.cache.cleanup_orphaned_files()

    def _detect_and_validate_relationships(self):
        """Detect and validate relationships between loaded tables."""
        self.relationship_validator.detect_relationships(self.table_data)

        self.validation_report = self.relationship_validator.validate_relationships()

        logger.info(self.validation_report.get_summary())

        if self.validation_report.dependency_order:
            logger.debug(f"Suggested table load order: {self.validation_report.dependency_order[:10]}...")

        if self.validation_report.broken_references:
            logger.warning(f"Found {len(self.validation_report.broken_references)} broken references")

        try:
            self.cache.save_relationships(
                self.relationship_validator.relationships,
                self.validation_report
            )
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
        """Get all instances for a given table."""
        return self.table_data.get(table_name, [])

    def get_by_id(self, table_name: str, row_id: int) -> Optional[Any]:
        """Get a specific row instance by table name and row ID."""
        instances = self.get_table(table_name)
        if 0 <= row_id < len(instances):
            return instances[row_id]
        return None

    def get_stats(self) -> Dict[str, Any]:
        """Get statistics about loaded tables and relationships."""

        stats = {
            'tables_loaded': len(self.table_data),
            'total_rows': sum(len(instances) for instances in self.table_data.values()),
            'cache_stats': self.cache.get_cache_stats(),
            'tables': {
                name: len(instances) 
                for name, instances in self.table_data.items()
            }
        }

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
        """Get dependency relationships for a specific table."""
        if not self.relationship_validator:
            return {'dependencies': [], 'dependents': []}
        
        return {
            'dependencies': list(self.relationship_validator.get_table_dependencies(table_name)),
            'dependents': list(self.relationship_validator.get_table_dependents(table_name))
        }

    def _create_string_cache(self, row_data_list: List[Dict[str, Any]], table_name: str) -> Dict[int, str]:
        """Create a pre-populated string cache for TLK batch lookups."""
        if not row_data_list:
            return {}

        string_ref_fields = {
            'name', 'label', 'description', 'desc', 'displayname', 'tooltip',
            'strref', 'namestrref', 'descriptionstrref', 'tooltipstrref',
            'displaynamestrref', 'prereqfeat1', 'prereqfeat2', 'masterfeat',
            'successorfeat', 'spellid', 'category', 'featcategory',
            'allclassescanuse', 'icon', 'iconresref'
        }

        str_refs_to_resolve = set()
        start_time = time.time()
        
        for row_dict in row_data_list:
            for column_name, value in row_dict.items():
                if (column_name.lower() in string_ref_fields and 
                    isinstance(value, (str, int))):
                    try:
                        int_val = int(value)
                        if 1 <= int_val <= 65535:
                            str_refs_to_resolve.add(int_val)
                    except (ValueError, TypeError):
                        pass

        if not str_refs_to_resolve:
            return {}

        str_refs_list = list(str_refs_to_resolve)
        string_cache = {}
        if hasattr(self.rm, 'get_strings_batch'):
            try:
                batch_start = time.time()
                string_cache = self.rm.get_strings_batch(str_refs_list)
                batch_time = (time.time() - batch_start) * 1000

                self._string_lookup_stats['total_lookups'] += len(str_refs_list)
                self._string_lookup_stats['batch_time_ms'] += batch_time

                strings_per_sec = (len(str_refs_list) / batch_time * 1000) if batch_time > 0 else float('inf')
                # logger.debug(f"TLK batch lookup for {table_name}: "
                #            f"{len(str_refs_list)} strings in {batch_time:.2f}ms "
                #            f"({strings_per_sec:.0f} strings/sec)" if strings_per_sec != float('inf') else
                #            f"{len(str_refs_list)} strings in {batch_time:.2f}ms (instant)")
                pass
                
            except Exception as e:
                logger.warning(f"Batch TLK lookup failed for {table_name}, falling back to individual lookups: {e}")
                for str_ref in str_refs_list:
                    try:
                        resolved = self.rm.get_string(str_ref)
                        if resolved and resolved != f"{{StrRef:{str_ref}}}":
                            string_cache[str_ref] = resolved
                    except Exception:
                        pass
        else:
            logger.debug(f"ResourceManager doesn't support batch lookups, using individual lookups for {table_name}")
            for str_ref in str_refs_list:
                try:
                    resolved = self.rm.get_string(str_ref)
                    if resolved and resolved != f"{{StrRef:{str_ref}}}":
                        string_cache[str_ref] = resolved
                except Exception:
                    pass
        
        total_time = (time.time() - start_time) * 1000
        
        # if string_cache:
        #     logger.debug(f"Created string cache for {table_name}: "
        #                f"{len(string_cache)} resolved strings in {total_time:.2f}ms")
        pass
        
        return string_cache