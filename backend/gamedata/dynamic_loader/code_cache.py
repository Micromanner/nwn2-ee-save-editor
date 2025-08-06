"""
Code Cache - Secure caching for generated Python code

This module provides secure caching of generated Python code strings (not compiled objects)
to speed up application startup. Uses file hashing for cache invalidation.
"""
import hashlib
import logging
from pathlib import Path
from typing import Optional, Dict, Callable, Any
import json
from datetime import datetime
import threading

logger = logging.getLogger(__name__)


class SecureCodeCache:
    """
    Cache generated code strings between sessions - NEVER cache compiled objects.
    
    Security principles:
    - Only cache source code strings, never compiled code or pickled objects
    - Use content + mtime hashing for cache invalidation
    - Clean up old cache entries automatically
    """
    
    def __init__(self, cache_dir: Path):
        """
        Initialize code cache.
        
        Args:
            cache_dir: Directory to store cached code files
        """
        self.cache_dir = Path(cache_dir)
        self.cache_dir.mkdir(parents=True, exist_ok=True)
        
        # Metadata file for tracking cache entries
        self.metadata_file = self.cache_dir / "cache_metadata.json"
        self._metadata_lock = threading.Lock()
        self.metadata = self._load_metadata()
    
    def _load_metadata(self) -> Dict[str, Any]:
        """Load cache metadata from disk."""
        if self.metadata_file.exists():
            try:
                with open(self.metadata_file, 'r') as f:
                    return json.load(f)
            except Exception as e:
                logger.warning(f"Failed to load cache metadata: {e}")
        return {}
    
    def _save_metadata(self):
        """Save cache metadata to disk."""
        try:
            # Create a copy of metadata while holding the lock to avoid iteration issues
            with self._metadata_lock:
                metadata_copy = self.metadata.copy()
            
            # Write the copy without holding the lock
            with open(self.metadata_file, 'w') as f:
                json.dump(metadata_copy, f, indent=2)
        except Exception as e:
            logger.error(f"Failed to save cache metadata: {e}")
    
    def get_file_hash(self, file_path: Path) -> str:
        """
        Generate hash of file content + modification time.
        
        Args:
            file_path: Path to the file to hash
            
        Returns:
            SHA256 hash hex string
        """
        try:
            content = file_path.read_bytes()
            mtime = str(file_path.stat().st_mtime)
            hash_input = content + mtime.encode()
            return hashlib.sha256(hash_input).hexdigest()
        except Exception as e:
            logger.error(f"Failed to hash file {file_path}: {e}")
            return ""
    
    def get_table_hash(self, table_name: str, table_data: Any) -> str:
        """
        Generate hash for a table's structure.
        
        Args:
            table_name: Name of the 2DA table
            table_data: Table data object (TDAParser or similar)
            
        Returns:
            SHA256 hash hex string
        """
        try:
            # Extract column headers
            if hasattr(table_data, 'get_column_headers'):
                columns = table_data.get_column_headers()
            elif hasattr(table_data, 'columns'):
                columns = table_data.columns
            else:
                columns = []
            
            # Include table name and column structure in hash
            hash_data = {
                'table_name': table_name,
                'columns': sorted(columns),  # Sort for consistency
                'column_count': len(columns)
            }
            
            hash_input = json.dumps(hash_data, sort_keys=True).encode()
            return hashlib.sha256(hash_input).hexdigest()
        except Exception as e:
            logger.error(f"Failed to hash table {table_name}: {e}")
            return ""
    
    def load_or_generate(self, table_name: str, file_path: Optional[Path], 
                        code_generator_func: Callable[[], str]) -> str:
        """
        Load code string from cache if valid, otherwise generate.
        
        Args:
            table_name: Name of the 2DA table
            file_path: Optional path to source file (for hash validation)
            code_generator_func: Function to generate code if not cached
            
        Returns:
            Generated Python code string
        """
        # Generate cache key
        if file_path and file_path.exists():
            file_hash = self.get_file_hash(file_path)
            cache_key = f"{table_name}_{file_hash}"
        else:
            # No file path, use table name only
            cache_key = table_name
        
        cache_file = self.cache_dir / f"{cache_key}.py"
        
        # Check if cached version exists and is valid
        if cache_file.exists():
            try:
                # Verify metadata (thread-safe access)
                with self._metadata_lock:
                    cached_info = self.metadata.get(cache_key)
                
                if cached_info:
                    # If we have a file path, verify it hasn't changed
                    if file_path and file_path.exists():
                        current_hash = self.get_file_hash(file_path)
                        if cached_info.get('file_hash') == current_hash:
                            # Cache is valid, load it
                            code = cache_file.read_text(encoding='utf-8')
                            logger.debug(f"Loaded cached code for {table_name}")
                            return code
                    elif not file_path:
                        # No file to validate against, trust the cache
                        code = cache_file.read_text(encoding='utf-8')
                        logger.debug(f"Loaded cached code for {table_name} (no file validation)")
                        return code
            except Exception as e:
                logger.warning(f"Failed to load cached code for {table_name}: {e}")
        
        # Generate new code
        logger.debug(f"Generating new code for {table_name}")
        code_string = code_generator_func()
        
        # Save to cache
        try:
            cache_file.write_text(code_string, encoding='utf-8')
            
            # Update metadata (thread-safe)
            with self._metadata_lock:
                self.metadata[cache_key] = {
                    'table_name': table_name,
                    'file_hash': self.get_file_hash(file_path) if file_path else None,
                    'generated_at': datetime.now().isoformat(),
                    'code_size': len(code_string)
                }
            self._save_metadata()
            
            # Clean old cache files for this table
            self._clean_old_cache(table_name, cache_key)
            
        except Exception as e:
            logger.error(f"Failed to cache code for {table_name}: {e}")
        
        return code_string
    
    def _clean_old_cache(self, table_name: str, current_key: str):
        """
        Remove outdated cache files for this table.
        
        Args:
            table_name: Name of the table
            current_key: Current cache key to keep
        """
        pattern = f"{table_name}_*.py"
        removed_count = 0
        keys_to_remove = []
        
        for old_file in self.cache_dir.glob(pattern):
            cache_key = old_file.stem
            if cache_key != current_key:
                try:
                    old_file.unlink()
                    keys_to_remove.append(cache_key)
                    removed_count += 1
                except Exception as e:
                    logger.warning(f"Failed to remove old cache file {old_file}: {e}")
        
        # Remove keys from metadata (thread-safe)
        if keys_to_remove:
            with self._metadata_lock:
                for cache_key in keys_to_remove:
                    self.metadata.pop(cache_key, None)
        
        if removed_count > 0:
            logger.debug(f"Removed {removed_count} old cache files for {table_name}")
            self._save_metadata()
    
    def clear_cache(self):
        """Clear all cached code files."""
        removed_count = 0
        
        for cache_file in self.cache_dir.glob("*.py"):
            try:
                cache_file.unlink()
                removed_count += 1
            except Exception as e:
                logger.error(f"Failed to remove cache file {cache_file}: {e}")
        
        # Clear metadata (thread-safe)
        with self._metadata_lock:
            self.metadata.clear()
        self._save_metadata()
        
        logger.info(f"Cleared code cache ({removed_count} files removed)")
    
    def get_cache_stats(self) -> Dict[str, Any]:
        """Get statistics about the code cache."""
        cache_files = list(self.cache_dir.glob("*.py"))
        total_size = sum(f.stat().st_size for f in cache_files)
        
        # Create snapshot of metadata for stats
        with self._metadata_lock:
            metadata_copy = self.metadata.copy()
        
        return {
            'cache_dir': str(self.cache_dir),
            'file_count': len(cache_files),
            'total_size_kb': total_size / 1024,
            'metadata_entries': len(metadata_copy),
            'oldest_entry': min(
                (entry.get('generated_at') for entry in metadata_copy.values()),
                default=None
            ),
            'newest_entry': max(
                (entry.get('generated_at') for entry in metadata_copy.values()),
                default=None
            )
        }
    
    def cleanup_orphaned_files(self):
        """Remove cache files that aren't in metadata."""
        cache_files = list(self.cache_dir.glob("*.py"))
        orphaned_count = 0
        
        # Get snapshot of metadata keys
        with self._metadata_lock:
            metadata_keys = set(self.metadata.keys())
        
        for cache_file in cache_files:
            cache_key = cache_file.stem
            if cache_key not in metadata_keys:
                try:
                    cache_file.unlink()
                    orphaned_count += 1
                    logger.debug(f"Removed orphaned cache file: {cache_file.name}")
                except Exception as e:
                    logger.error(f"Failed to remove orphaned file {cache_file}: {e}")
        
        if orphaned_count > 0:
            logger.info(f"Cleaned up {orphaned_count} orphaned cache files")
    
    def save_relationships(self, relationships: Any, validation_report: Any):
        """
        Save detected relationships and validation report to cache.
        
        Args:
            relationships: Set of RelationshipDefinition objects
            validation_report: ValidationReport object
        """
        relationships_file = self.cache_dir / "relationships.json"
        
        try:
            # Convert relationships to serializable format
            rel_data = []
            for rel in relationships:
                rel_data.append({
                    'source_table': rel.source_table,
                    'source_column': rel.source_column,
                    'target_table': rel.target_table,
                    'relationship_type': rel.relationship_type.value,
                    'is_nullable': rel.is_nullable
                })
            
            # Convert validation report to serializable format
            report_data = {
                'total_relationships': validation_report.total_relationships,
                'valid_relationships': validation_report.valid_relationships,
                'broken_references': validation_report.broken_references[:100],  # Limit size
                'missing_tables': list(validation_report.missing_tables),
                'dependency_order': validation_report.dependency_order
            }
            
            # Save to file
            cache_data = {
                'relationships': rel_data,
                'validation_report': report_data,
                'generated_at': datetime.now().isoformat()
            }
            
            with open(relationships_file, 'w') as f:
                json.dump(cache_data, f, indent=2)
            
            logger.debug(f"Saved {len(rel_data)} relationships to cache")
            
        except Exception as e:
            logger.error(f"Failed to save relationships to cache: {e}")
    
    def load_relationships(self) -> Optional[Dict[str, Any]]:
        """
        Load cached relationships if available.
        
        Returns:
            Dict with 'relationships' and 'validation_report' or None
        """
        relationships_file = self.cache_dir / "relationships.json"
        
        if not relationships_file.exists():
            return None
        
        try:
            with open(relationships_file, 'r') as f:
                data = json.load(f)
            
            logger.debug(f"Loaded {len(data.get('relationships', []))} relationships from cache")
            return data
            
        except Exception as e:
            logger.warning(f"Failed to load relationships from cache: {e}")
            return None
    
    def get_relationships_hash(self, table_data: Dict[str, Any]) -> str:
        """
        Generate hash for current table structure to validate cached relationships.
        
        Args:
            table_data: Dict of table_name -> table instances
            
        Returns:
            SHA256 hash of table structures
        """
        try:
            # Build hash data from table names and column counts
            hash_data = {}
            for table_name in sorted(table_data.keys()):
                instances = table_data[table_name]
                if instances:
                    # Get column count from first instance
                    first_instance = instances[0]
                    if hasattr(first_instance.__class__, 'get_safe_columns'):
                        column_count = len(first_instance.__class__.get_safe_columns())
                    elif hasattr(first_instance, '_safe_columns'):
                        column_count = len(first_instance._safe_columns)
                    else:
                        column_count = 0
                    hash_data[table_name] = column_count
            
            hash_input = json.dumps(hash_data, sort_keys=True).encode()
            return hashlib.sha256(hash_input).hexdigest()
            
        except Exception as e:
            logger.error(f"Failed to generate relationships hash: {e}")
            return ""