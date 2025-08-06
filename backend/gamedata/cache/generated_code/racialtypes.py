
class RacialtypesData:
    """
    Auto-generated data class for racialtypes.2da
    
    This class provides efficient attribute access to 2DA row data
    with automatic column name mapping for mod compatibility.
    """
    __slots__ = ('_resource_manager', '_Label', '_Abrev', '_Name', '_NamePlural', '_NameLower', '_NameLowerPlural', '_ConverName', '_ConverNameLower', '_Description', '_Appearance', '_StrAdjust', '_DexAdjust', '_IntAdjust', '_ChaAdjust', '_WisAdjust', '_ConAdjust', '_Endurance', '_Favored', '_FeatsTable', '_Biography', '_PlayerRace', '_Constant', '_AGE', '_CRModifier', '_IsHumanoid', '_DefaultSubRace', '_female_race_icon', '_male_race_icon', '_FEATFavoredEnemy', '_FEATImprovedFavoredEnemy', '_FEATFavoredPowerAttack', '_FEATIgnoreCritImmunity')
    _column_mapping = {'Label': 'Label', 'Abrev': 'Abrev', 'Name': 'Name', 'NamePlural': 'NamePlural', 'NameLower': 'NameLower', 'NameLowerPlural': 'NameLowerPlural', 'ConverName': 'ConverName', 'ConverNameLower': 'ConverNameLower', 'Description': 'Description', 'Appearance': 'Appearance', 'StrAdjust': 'StrAdjust', 'DexAdjust': 'DexAdjust', 'IntAdjust': 'IntAdjust', 'ChaAdjust': 'ChaAdjust', 'WisAdjust': 'WisAdjust', 'ConAdjust': 'ConAdjust', 'Endurance': 'Endurance', 'Favored': 'Favored', 'FeatsTable': 'FeatsTable', 'Biography': 'Biography', 'PlayerRace': 'PlayerRace', 'Constant': 'Constant', 'AGE': 'AGE', 'CRModifier': 'CRModifier', 'IsHumanoid': 'IsHumanoid', 'DefaultSubRace': 'DefaultSubRace', 'female_race_icon': 'female_race_icon', 'male_race_icon': 'male_race_icon', 'FEATFavoredEnemy': 'FEATFavoredEnemy', 'FEATImprovedFavoredEnemy': 'FEATImprovedFavoredEnemy', 'FEATFavoredPowerAttack': 'FEATFavoredPowerAttack', 'FEATIgnoreCritImmunity': 'FEATIgnoreCritImmunity'}
    _table_name = 'racialtypes'
    _safe_columns = ['Label', 'Abrev', 'Name', 'NamePlural', 'NameLower', 'NameLowerPlural', 'ConverName', 'ConverNameLower', 'Description', 'Appearance', 'StrAdjust', 'DexAdjust', 'IntAdjust', 'ChaAdjust', 'WisAdjust', 'ConAdjust', 'Endurance', 'Favored', 'FeatsTable', 'Biography', 'PlayerRace', 'Constant', 'AGE', 'CRModifier', 'IsHumanoid', 'DefaultSubRace', 'female_race_icon', 'male_race_icon', 'FEATFavoredEnemy', 'FEATImprovedFavoredEnemy', 'FEATFavoredPowerAttack', 'FEATIgnoreCritImmunity']
    
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
        pk_column = 'Label'
        if pk_column and hasattr(self, '_' + pk_column):
            pk_value = getattr(self, '_' + pk_column)
            return f"<RacialtypesData {pk_column}={pk_value!r}>>"
        else:
            return f"<RacialtypesData>"
    
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
