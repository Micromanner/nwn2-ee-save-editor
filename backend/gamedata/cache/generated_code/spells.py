
class SpellsData:
    """
    Auto-generated data class for spells.2da
    
    This class provides efficient attribute access to 2DA row data
    with automatic column name mapping for mod compatibility.
    """
    __slots__ = ('_resource_manager', '_Label', '_Name', '_IconResRef', '_School', '_Range', '_VS', '_MetaMagic', '_TargetType', '_ImpactScript', '_Bard', '_Cleric', '_Druid', '_Paladin', '_Ranger', '_Wiz_Sorc', '_Warlock', '_Innate', '_ConjTime', '_ConjAnim', '_ConjVisual0', '_LowConjVisual0', '_ConjVisual1', '_ConjVisual2', '_ConjSoundVFX', '_ConjSoundMale', '_ConjSoundFemale', '_ConjSoundOverride', '_CastAnim', '_CastTime', '_CastVisual0', '_LowCastVisual0', '_CastVisual1', '_CastVisual2', '_CastSound', '_Proj', '_ProjModel', '_ProjSEF', '_LowProjSEF', '_ProjType', '_ProjSpwnPoint', '_ProjSound', '_ProjOrientation', '_ImpactSEF', '_LowImpactSEF', '_ImmunityType', '_ItemImmunity', '_SubRadSpell1', '_SubRadSpell2', '_SubRadSpell3', '_SubRadSpell4', '_SubRadSpell5', '_Category', '_Master', '_UserType', '_SpellDesc', '_UseConcentration', '_SpontaneouslyCast', '_SpontCastClassReq', '_AltMessage', '_HostileSetting', '_FeatID', '_Counter1', '_Counter2', '_HasProjectile', '_AsMetaMagic', '_TargetingUI', '_CastableOnDead', '_REMOVED')
    _column_mapping = {'Label': 'Label', 'Name': 'Name', 'IconResRef': 'IconResRef', 'School': 'School', 'Range': 'Range', 'VS': 'VS', 'MetaMagic': 'MetaMagic', 'TargetType': 'TargetType', 'ImpactScript': 'ImpactScript', 'Bard': 'Bard', 'Cleric': 'Cleric', 'Druid': 'Druid', 'Paladin': 'Paladin', 'Ranger': 'Ranger', 'Wiz_Sorc': 'Wiz_Sorc', 'Warlock': 'Warlock', 'Innate': 'Innate', 'ConjTime': 'ConjTime', 'ConjAnim': 'ConjAnim', 'ConjVisual0': 'ConjVisual0', 'LowConjVisual0': 'LowConjVisual0', 'ConjVisual1': 'ConjVisual1', 'ConjVisual2': 'ConjVisual2', 'ConjSoundVFX': 'ConjSoundVFX', 'ConjSoundMale': 'ConjSoundMale', 'ConjSoundFemale': 'ConjSoundFemale', 'ConjSoundOverride': 'ConjSoundOverride', 'CastAnim': 'CastAnim', 'CastTime': 'CastTime', 'CastVisual0': 'CastVisual0', 'LowCastVisual0': 'LowCastVisual0', 'CastVisual1': 'CastVisual1', 'CastVisual2': 'CastVisual2', 'CastSound': 'CastSound', 'Proj': 'Proj', 'ProjModel': 'ProjModel', 'ProjSEF': 'ProjSEF', 'LowProjSEF': 'LowProjSEF', 'ProjType': 'ProjType', 'ProjSpwnPoint': 'ProjSpwnPoint', 'ProjSound': 'ProjSound', 'ProjOrientation': 'ProjOrientation', 'ImpactSEF': 'ImpactSEF', 'LowImpactSEF': 'LowImpactSEF', 'ImmunityType': 'ImmunityType', 'ItemImmunity': 'ItemImmunity', 'SubRadSpell1': 'SubRadSpell1', 'SubRadSpell2': 'SubRadSpell2', 'SubRadSpell3': 'SubRadSpell3', 'SubRadSpell4': 'SubRadSpell4', 'SubRadSpell5': 'SubRadSpell5', 'Category': 'Category', 'Master': 'Master', 'UserType': 'UserType', 'SpellDesc': 'SpellDesc', 'UseConcentration': 'UseConcentration', 'SpontaneouslyCast': 'SpontaneouslyCast', 'SpontCastClassReq': 'SpontCastClassReq', 'AltMessage': 'AltMessage', 'HostileSetting': 'HostileSetting', 'FeatID': 'FeatID', 'Counter1': 'Counter1', 'Counter2': 'Counter2', 'HasProjectile': 'HasProjectile', 'AsMetaMagic': 'AsMetaMagic', 'TargetingUI': 'TargetingUI', 'CastableOnDead': 'CastableOnDead', 'REMOVED': 'REMOVED'}
    _table_name = 'spells'
    _safe_columns = ['Label', 'Name', 'IconResRef', 'School', 'Range', 'VS', 'MetaMagic', 'TargetType', 'ImpactScript', 'Bard', 'Cleric', 'Druid', 'Paladin', 'Ranger', 'Wiz_Sorc', 'Warlock', 'Innate', 'ConjTime', 'ConjAnim', 'ConjVisual0', 'LowConjVisual0', 'ConjVisual1', 'ConjVisual2', 'ConjSoundVFX', 'ConjSoundMale', 'ConjSoundFemale', 'ConjSoundOverride', 'CastAnim', 'CastTime', 'CastVisual0', 'LowCastVisual0', 'CastVisual1', 'CastVisual2', 'CastSound', 'Proj', 'ProjModel', 'ProjSEF', 'LowProjSEF', 'ProjType', 'ProjSpwnPoint', 'ProjSound', 'ProjOrientation', 'ImpactSEF', 'LowImpactSEF', 'ImmunityType', 'ItemImmunity', 'SubRadSpell1', 'SubRadSpell2', 'SubRadSpell3', 'SubRadSpell4', 'SubRadSpell5', 'Category', 'Master', 'UserType', 'SpellDesc', 'UseConcentration', 'SpontaneouslyCast', 'SpontCastClassReq', 'AltMessage', 'HostileSetting', 'FeatID', 'Counter1', 'Counter2', 'HasProjectile', 'AsMetaMagic', 'TargetingUI', 'CastableOnDead', 'REMOVED']
    
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
        pk_column = 'Label'
        if pk_column and hasattr(self, '_' + pk_column):
            pk_value = getattr(self, '_' + pk_column)
            return f"<SpellsData {pk_column}={pk_value!r}>>"
        else:
            return f"<SpellsData>"
    
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
