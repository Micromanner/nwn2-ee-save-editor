"""Column Name Sanitizer - Ensures column names from 2DA files are safe Python identifiers."""
import keyword
import re
from typing import Dict, List, Set


class ColumnNameSanitizer:
    """Ensures column names are safe Python identifiers."""
    
    # Use Python's official keyword list - complete and version-aware
    RESERVED_WORDS = set(keyword.kwlist + keyword.softkwlist)
    VALID_IDENTIFIER = re.compile(r'^[a-zA-Z_][a-zA-Z0-9_]*$')
    

    REPLACEMENTS = {
        '-': '_',
        ' ': '_',
        '.': '_',
        '/': '_',
        '\\': '_',
        ':': '_',
        ';': '_',
        ',': '_',
        '(': '_',
        ')': '_',
        '[': '_',
        ']': '_',
        '{': '_',
        '}': '_',
        '<': '_',
        '>': '_',
        '=': '_',
        '+': 'plus',
        '&': 'and',
        '|': 'or',
        '%': 'pct',
        '#': 'num',
        '@': 'at',
        '!': 'not',
        '?': 'q',
        '*': 'star',
        '$': 'dollar',
    }
    
    def sanitize(self, column_name: str) -> str:
        """Convert a column name to a safe Python identifier."""
        if not column_name:
            return "col_empty"
        
        safe_name = column_name
        for old, new in self.REPLACEMENTS.items():
            safe_name = safe_name.replace(old, new)
        
        safe_name = re.sub(r'[^a-zA-Z0-9_]+', '_', safe_name)
        
        safe_name = safe_name.strip('_')
        safe_name = re.sub(r'_{2,}', '_', safe_name)
        
        if not safe_name:
            return "col_invalid"
        
        if safe_name[0].isdigit():
            safe_name = f'col_{safe_name}'
        
        if safe_name.lower() in self.RESERVED_WORDS:
            safe_name = f'{safe_name}_'
        
        if not self.VALID_IDENTIFIER.match(safe_name):
            safe_name = f"col_{abs(hash(column_name)) % 1000000}"
        
        return safe_name
    
    def sanitize_unique_columns(self, columns: List[str]) -> Dict[str, str]:
        """Sanitize column names ensuring uniqueness."""
        original_to_safe = {}
        used_names = set()
        
        for col in columns:
            safe_name = self.sanitize(col)
            original_safe = safe_name
            counter = 2
            
            # Handle collisions (e.g., "MOD-COLUMN" and "MOD_COLUMN" both -> "MOD_COLUMN")
            while safe_name in used_names:
                safe_name = f"{original_safe}_{counter}"
                counter += 1
            
            used_names.add(safe_name)
            original_to_safe[col] = safe_name
        
        return original_to_safe
    
    def sanitize_table_name(self, table_name: str) -> str:
        """Sanitize a table name for use as a class name."""
        # Remove .2da extension if present
        if table_name.lower().endswith('.2da'):
            table_name = table_name[:-4]
        
        # Sanitize similar to column names
        safe_name = self.sanitize(table_name)
        
        # Capitalize first letter for class name convention
        if safe_name and safe_name[0].islower():
            safe_name = safe_name[0].upper() + safe_name[1:]
        
        return safe_name
    
    def validate_batch(self, names: List[str]) -> Dict[str, bool]:
        """Validate a batch of names to check if they're already safe."""
        return {
            name: bool(self.VALID_IDENTIFIER.match(name) and 
                      name.lower() not in self.RESERVED_WORDS)
            for name in names
        }