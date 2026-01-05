"""Runtime Data Class Generator - Creates Python classes dynamically from 2DA structures."""
from loguru import logger
from typing import Dict, List, Any, Optional, Type
from gamedata.dynamic_loader.column_sanitizer import ColumnNameSanitizer


class RuntimeDataClassGenerator:
    """Generates Python classes at runtime from 2DA table structures."""
    
    def __init__(self):
        self.sanitizer = ColumnNameSanitizer()
        self._class_name_cache: Dict[str, str] = {}  # table_name -> class_name
        self._generated_classes: Dict[str, Type] = {}  # class_name -> generated class
    
    def generate_class_from_2da(self, table_name: str, columns: List[str], 
                               sample_data: Optional[List[Dict]] = None) -> Type:
        """Generate a Python class at runtime from 2DA structure."""
        class_name = self._generate_class_name(table_name)
        
        if class_name in self._generated_classes:
            return self._generated_classes[class_name]
        
        column_mapping = self.sanitizer.sanitize_unique_columns(columns)
        safe_columns = list(column_mapping.values())
        
        class_code = self._generate_class_code(
            class_name, safe_columns, column_mapping, table_name
        )
        
        namespace = {}
        exec(class_code, namespace)
        generated_class = namespace[class_name]
        
        self._generated_classes[class_name] = generated_class
        
        logger.debug(f"Generated class {class_name} for table {table_name} with {len(safe_columns)} columns")
        
        return generated_class
    
    def _generate_class_name(self, table_name: str) -> str:
        """Generate a unique class name for a table."""
        if table_name in self._class_name_cache:
            return self._class_name_cache[table_name]
        
        # Sanitize table name
        safe_table_name = self.sanitizer.sanitize_table_name(table_name)
        class_name = f"{safe_table_name}Data"
        
        # Handle collisions
        counter = 2
        original_name = class_name
        while class_name in self._generated_classes:
            class_name = f"{original_name}{counter}"
            counter += 1
        
        self._class_name_cache[table_name] = class_name
        return class_name
    
    def _generate_class_code(self, class_name: str, safe_columns: List[str],
                           column_mapping: Dict[str, str], table_name: str) -> str:
        """Generate the actual Python code for the class."""
        slots = ['_resource_manager'] + ['_' + col for col in safe_columns]
        slots_str = repr(tuple(slots))
        
        mapping_str = repr(column_mapping)
        
        pk_column = None
        if safe_columns:
            for col in safe_columns:
                if col.lower() in ('id', 'index', 'idx', 'row'):
                    pk_column = col
                    break
            if not pk_column:
                pk_column = safe_columns[0]
        
        code = f'''
class {class_name}:
    """Auto-generated data class for {table_name}.2da."""
    __slots__ = {slots_str}
    _column_mapping = {mapping_str}
    _table_name = {repr(table_name)}
    _safe_columns = {repr(safe_columns)}
    
    def __init__(self, _resource_manager=None, _string_cache=None, **row_data):
        """Initialize from 2DA row data with optimized string resolution."""
        self._resource_manager = _resource_manager
        
        if row_data:
            column_mapping = self._column_mapping
            for orig_col, value in row_data.items():
                if orig_col in column_mapping:
                    safe_col = column_mapping[orig_col]
                    resolved_value = self._resolve_string_reference_cached(orig_col, value, _string_cache)
                    object.__setattr__(self, '_' + safe_col, resolved_value)
        
        for slot in self.__slots__:
            if not hasattr(self, slot):
                object.__setattr__(self, slot, None)
    
    def _resolve_string_reference_cached(self, column_name, value, string_cache=None):
        """Resolve string references using cache when available."""
        string_ref_fields = {{
            'name', 'description', 'plural', 'lower', 'label',
            'displaynametext', 'desc', 'tooltip', 'help'
        }}
        
        if (column_name.lower() in string_ref_fields and 
            isinstance(value, (str, int))):
            try:
                int_val = int(value)
                if int_val == 0:
                    return value
                
                if 1 <= int_val <= 16777215:
                    if string_cache and int_val in string_cache:
                        return string_cache[int_val]
                    
                    if self._resource_manager:
                        resolved = self._resource_manager.get_string(int_val)
                        if resolved and resolved != str(int_val):
                            return resolved
            except (ValueError, TypeError):
                pass
        
        return value
    
    def _resolve_string_reference(self, column_name, value):
        """Resolve string references for known string fields."""
        return self._resolve_string_reference_cached(column_name, value, None)
    
    def __getattr__(self, name):
        """Provide access without underscore prefix, with case-insensitive fallback."""
        if name.startswith('_'):
            raise AttributeError(f"'{{self.__class__.__name__}}' object has no attribute '{{name}}'")
        
        slot_name = '_' + name
        if slot_name in self.__slots__:
            return getattr(self, slot_name)
        
        lower_name = name.lower()
        for slot in self.__slots__:
            if slot == '_resource_manager':
                continue
            if slot[1:].lower() == lower_name:
                return getattr(self, slot)
        
        for orig_col, safe_col in self._column_mapping.items():
            if orig_col.lower() == lower_name:
                slot_name = '_' + safe_col
                if slot_name in self.__slots__:
                    return getattr(self, slot_name)
        
        available_attrs = []
        available_attrs.extend(col[1:] for col in self.__slots__ if col != '_resource_manager')
        available_attrs.extend(self._column_mapping.keys())
        
        raise AttributeError(
            f"'{{self.__class__.__name__}}' object has no attribute '{{name}}' (case-insensitive). "
            f"Available: {{', '.join(sorted(set(available_attrs)))}}"
        )
    
    def __setattr__(self, name, value):
        """Set attribute with or without underscore prefix."""
        if name in ('_column_mapping', '_table_name', '_safe_columns', '_resource_manager') or name.startswith('_'):
            object.__setattr__(self, name, value)
        else:
            slot_name = '_' + name
            if slot_name in self.__slots__:
                object.__setattr__(self, slot_name, value)
            else:
                raise AttributeError(f"Cannot set attribute '{{name}}' on {{self.__class__.__name__}}")
    
    def __repr__(self):
        """Readable representation using primary key."""
        pk_column = {repr(pk_column)}
        if pk_column and hasattr(self, '_' + pk_column):
            pk_value = getattr(self, '_' + pk_column)
            return f"<{class_name} {{pk_column}}={{pk_value!r}}>>"
        else:
            return f"<{class_name}>"
    
    def to_dict(self, use_original_names=False):
        """Convert to dictionary."""
        if use_original_names:
            safe_to_orig = {{v: k for k, v in self._column_mapping.items()}}
            result = {{}}
            for slot in self.__slots__:
                if slot == '_resource_manager':
                    continue
                safe_col = slot[1:]
                if safe_col in safe_to_orig:
                    orig_col = safe_to_orig[safe_col]
                    value = getattr(self, slot)
                    if value is not None:
                        result[orig_col] = value
            return result
        else:
            return {{
                slot[1:]: getattr(self, slot)
                for slot in self.__slots__
                if slot != '_resource_manager' and getattr(self, slot) is not None
            }}
    
    @classmethod
    def from_dict(cls, data):
        """Create instance from dictionary."""
        return cls(**data)
    
    @classmethod
    def get_column_mapping(cls):
        """Get the column name mapping."""
        return cls._column_mapping.copy()
    
    @classmethod
    def get_safe_columns(cls):
        """Get list of safe column names."""
        return cls._safe_columns.copy()
    
    @classmethod
    def create_batch(cls, row_data_list, resource_manager=None, string_cache=None):
        """Optimized batch creation using __new__ to bypass __init__ overhead."""
        instances = []
        column_mapping = cls._column_mapping
        
        slot_names = {{orig: '_' + safe for orig, safe in column_mapping.items()}}
        
        string_ref_fields = {{
            'name', 'description', 'plural', 'lower', 'label',
            'displaynametext', 'desc', 'tooltip', 'help'
        }}
        
        for row_data in row_data_list:
            instance = object.__new__(cls)
            
            object.__setattr__(instance, '_resource_manager', resource_manager)
            
            for orig_col, value in row_data.items():
                if orig_col in slot_names:
                    slot_name = slot_names[orig_col]
                    
                    if string_cache and orig_col.lower() in string_ref_fields and isinstance(value, (str, int)):
                        try:
                            int_val = int(value)
                            if int_val > 0 and int_val in string_cache:
                                value = string_cache[int_val]
                            elif int_val > 0 and int_val <= 16777215 and resource_manager:
                                resolved = resource_manager.get_string(int_val)
                                if resolved and resolved != str(int_val):
                                    value = resolved
                        except (ValueError, TypeError):
                            pass
                    
                    object.__setattr__(instance, slot_name, value)
            
            for slot in cls.__slots__:
                if not hasattr(instance, slot):
                    object.__setattr__(instance, slot, None)
            
            instances.append(instance)
        
        return instances
'''
        return code
    
    def generate_code_for_table(self, table_name: str, table_data: Any) -> str:
        """Generate class code for a table using actual 2DA data."""
        # Extract columns from table data
        if hasattr(table_data, 'get_column_headers'):
            columns = table_data.get_column_headers()
        elif hasattr(table_data, 'columns'):
            columns = table_data.columns
        else:
            raise ValueError(f"Cannot extract columns from table data of type {type(table_data)}")
        
        # Generate class name
        class_name = self._generate_class_name(table_name)
        
        # Sanitize columns
        column_mapping = self.sanitizer.sanitize_unique_columns(columns)
        safe_columns = list(column_mapping.values())
        
        # Generate and return code
        return self._generate_class_code(class_name, safe_columns, column_mapping, table_name)
    
    def clear_cache(self):
        """Clear all cached class names and generated classes."""
        self._class_name_cache.clear()
        self._generated_classes.clear()
        logger.info("Cleared runtime class generator cache")