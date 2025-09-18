
class Cls_bfeat_invisibladeData:
    """
    Auto-generated data class for cls_bfeat_invisiblade.2da
    
    This class provides efficient attribute access to 2DA row data
    with automatic column name mapping for mod compatibility.
    """
    __slots__ = ('_resource_manager', '_Bonus')
    _column_mapping = {'Bonus': 'Bonus'}
    _table_name = 'cls_bfeat_invisiblade'
    _safe_columns = ['Bonus']
    
    def __init__(self, _resource_manager=None, _string_cache=None, **row_data):
        """Initialize from 2DA row data with optimized string resolution."""
        # Store resource manager for string resolution
        self._resource_manager = _resource_manager
        
        # Optimized: Only initialize slots that will be used
        if row_data:
            # Map from original to safe column names with cached string resolution
            column_mapping = self._column_mapping
            for orig_col, value in row_data.items():
                if orig_col in column_mapping:
                    safe_col = column_mapping[orig_col]
                    
                    # Use cached string resolution if available
                    resolved_value = self._resolve_string_reference_cached(orig_col, value, _string_cache)
                    
                    # Direct assignment (faster than setattr)
                    object.__setattr__(self, '_' + safe_col, resolved_value)
        
        # Initialize remaining slots to None only if needed
        for slot in self.__slots__:
            if not hasattr(self, slot):
                object.__setattr__(self, slot, None)
    
    def _resolve_string_reference_cached(self, column_name, value, string_cache=None):
        """Resolve string references using cache when available."""
        # Common string reference field patterns
        string_ref_fields = {
            'name', 'description', 'plural', 'lower', 'label',
            'displaynametext', 'desc', 'tooltip', 'help'
        }
        
        # Check if this field should be resolved as a string reference
        if (column_name.lower() in string_ref_fields and 
            isinstance(value, (str, int))):
            try:
                int_val = int(value)
                # Only resolve if it's a reasonable string reference ID (NWN2 uses larger ranges)
                # Special case: 0 is usually an invalid/null string reference
                if int_val == 0:
                    return value  # Don't try to resolve 0, return original value
                
                if 1 <= int_val <= 16777215:  # Expanded range for NWN2 string references
                    # Use cache first if available
                    if string_cache and int_val in string_cache:
                        return string_cache[int_val]
                    
                    # Fallback to resource manager
                    if self._resource_manager:
                        resolved = self._resource_manager.get_string(int_val)
                        if resolved and resolved != str(int_val):
                            return resolved
            except (ValueError, TypeError):
                pass
        
        return value
    
    def _resolve_string_reference(self, column_name, value):
        """Resolve string references for known string fields (legacy method)."""
        return self._resolve_string_reference_cached(column_name, value, None)
    
    def __getattr__(self, name):
        """Provide access without underscore prefix, with case-insensitive fallback."""
        if name.startswith('_'):
            raise AttributeError(f"'{self.__class__.__name__}' object has no attribute '{name}'")
        
        # First try exact match
        slot_name = '_' + name
        if slot_name in self.__slots__:
            return getattr(self, slot_name)
        
        # Try case-insensitive match
        lower_name = name.lower()
        for slot in self.__slots__:
            if slot == '_resource_manager':
                continue
            if slot[1:].lower() == lower_name:  # Skip underscore prefix
                return getattr(self, slot)
        
        # Check original column names too (case-insensitive)
        for orig_col, safe_col in self._column_mapping.items():
            if orig_col.lower() == lower_name:
                slot_name = '_' + safe_col
                if slot_name in self.__slots__:
                    return getattr(self, slot_name)
        
        # Helpful error message
        available_attrs = []
        # Add safe column names
        available_attrs.extend(col[1:] for col in self.__slots__ if col != '_resource_manager')
        # Add original column names
        available_attrs.extend(self._column_mapping.keys())
        
        raise AttributeError(
            f"'{self.__class__.__name__}' object has no attribute '{name}' (case-insensitive). "
            f"Available: {', '.join(sorted(set(available_attrs)))}"
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
                raise AttributeError(f"Cannot set attribute '{name}' on {self.__class__.__name__}")
    
    def __repr__(self):
        """Readable representation using primary key."""
        pk_column = 'Bonus'
        if pk_column and hasattr(self, '_' + pk_column):
            pk_value = getattr(self, '_' + pk_column)
            return f"<Cls_bfeat_invisibladeData {pk_column}={pk_value!r}>>"
        else:
            return f"<Cls_bfeat_invisibladeData>"
    
    def to_dict(self, use_original_names=False):
        """Convert to dictionary."""
        if use_original_names:
            # Reverse mapping
            safe_to_orig = {v: k for k, v in self._column_mapping.items()}
            result = {}
            for slot in self.__slots__:
                if slot == '_resource_manager':
                    continue  # Skip resource manager
                safe_col = slot[1:]  # Remove underscore
                if safe_col in safe_to_orig:
                    orig_col = safe_to_orig[safe_col]
                    value = getattr(self, slot)
                    if value is not None:
                        result[orig_col] = value
            return result
        else:
            # Use safe names
            return {
                slot[1:]: getattr(self, slot)
                for slot in self.__slots__
                if slot != '_resource_manager' and getattr(self, slot) is not None
            }
    
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
        """
        Optimized batch creation using __new__ to bypass __init__ overhead.
        
        This method creates multiple instances 25-40% faster by:
        1. Using __new__ to allocate objects without calling __init__
        2. Directly setting attributes via object.__setattr__
        3. Reusing column mapping and string cache
        
        Args:
            row_data_list: List of dictionaries containing row data
            resource_manager: ResourceManager for string resolution
            string_cache: Pre-populated string cache for batch lookups
            
        Returns:
            List of initialized instances
        """
        instances = []
        column_mapping = cls._column_mapping
        
        # Pre-compute slot names to avoid repeated string concatenation
        slot_names = {orig: '_' + safe for orig, safe in column_mapping.items()}
        
        # Common string reference fields for optimization
        string_ref_fields = {
            'name', 'description', 'plural', 'lower', 'label',
            'displaynametext', 'desc', 'tooltip', 'help'
        }
        
        for row_data in row_data_list:
            # Allocate object without calling __init__
            instance = object.__new__(cls)
            
            # Directly set resource manager
            object.__setattr__(instance, '_resource_manager', resource_manager)
            
            # Fast path: set attributes directly
            for orig_col, value in row_data.items():
                if orig_col in slot_names:
                    slot_name = slot_names[orig_col]
                    
                    # Inline string resolution for performance
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
                    
                    # Direct assignment
                    object.__setattr__(instance, slot_name, value)
            
            # Initialize remaining slots to None (optimized)
            for slot in cls.__slots__:
                if not hasattr(instance, slot):
                    object.__setattr__(instance, slot, None)
            
            instances.append(instance)
        
        return instances
