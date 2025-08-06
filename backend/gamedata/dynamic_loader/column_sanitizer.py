"""
Column Name Sanitizer - Ensures column names from 2DA files are safe Python identifiers

This module provides security for runtime code generation by sanitizing column names
from potentially untrusted mod files, preventing code injection and ensuring
valid Python identifiers.
"""
import keyword
import re
from typing import Dict, List, Set


class ColumnNameSanitizer:
    """
    Ensures column names are safe Python identifiers.
    
    Critical for security when generating code at runtime from mod data.
    Handles edge cases like reserved words, special characters, and collisions.
    """
    
    # Use Python's official keyword list - complete and version-aware
    RESERVED_WORDS = set(keyword.kwlist + keyword.softkwlist)
    
    # Valid Python identifier pattern
    VALID_IDENTIFIER = re.compile(r'^[a-zA-Z_][a-zA-Z0-9_]*$')
    
    # Common replacements for better readability
    REPLACEMENTS = {
        # Common separators
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
        """
        Convert a column name to a safe Python identifier.
        
        Args:
            column_name: Raw column name from 2DA file
            
        Returns:
            Safe Python identifier
            
        Raises:
            ValueError: If column name cannot be sanitized
        """
        if not column_name:
            return "col_empty"
        
        # First pass: replace known problematic characters
        safe_name = column_name
        for old, new in self.REPLACEMENTS.items():
            safe_name = safe_name.replace(old, new)
        
        # Second pass: replace any remaining non-alphanumeric with underscore
        safe_name = re.sub(r'[^a-zA-Z0-9_]+', '_', safe_name)
        
        # Clean up underscores
        safe_name = safe_name.strip('_')
        safe_name = re.sub(r'_{2,}', '_', safe_name)
        
        # Handle empty result
        if not safe_name:
            return "col_invalid"
        
        # Ensure it starts with letter/underscore
        if safe_name[0].isdigit():
            safe_name = f'col_{safe_name}'
        
        # Handle reserved words
        if safe_name.lower() in self.RESERVED_WORDS:
            safe_name = f'{safe_name}_'
        
        # Final validation
        if not self.VALID_IDENTIFIER.match(safe_name):
            # Last resort: create generic safe name
            safe_name = f"col_{abs(hash(column_name)) % 1000000}"
        
        return safe_name
    
    def sanitize_unique_columns(self, columns: List[str]) -> Dict[str, str]:
        """
        Sanitize column names ensuring uniqueness.
        
        Args:
            columns: List of column names from 2DA
            
        Returns:
            Dict mapping original names to safe names
        """
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
        """
        Sanitize a table name for use as a class name.
        
        Args:
            table_name: Name of the 2DA table
            
        Returns:
            Safe class name
        """
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
        """
        Validate a batch of names to check if they're already safe.
        
        Args:
            names: List of names to validate
            
        Returns:
            Dict mapping names to whether they're already safe
        """
        return {
            name: bool(self.VALID_IDENTIFIER.match(name) and 
                      name.lower() not in self.RESERVED_WORDS)
            for name in names
        }