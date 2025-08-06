
class ClassesData:
    """
    Auto-generated data class for classes.2da
    
    This class provides efficient attribute access to 2DA row data
    with automatic column name mapping for mod compatibility.
    """
    __slots__ = ('_resource_manager', '_Label', '_Name', '_Plural', '_Lower', '_Description', '_Icon', '_BorderedIcon', '_HitDie', '_AttackBonusTable', '_FeatsTable', '_SavingThrowTable', '_SkillsTable', '_BonusFeatsTable', '_SkillPointBase', '_SpellGainTable', '_SpellKnownTable', '_PlayerClass', '_SpellCaster', '_MetaMagicAllowed', '_MemorizesSpells', '_HasArcane', '_HasDivine', '_HasSpontaneousSpells', '_SpontaneousConversionTable', '_SpellSwapMinLvl', '_SpellSwapLvlInterval', '_SpellSwapLvlDiff', '_AllSpellsKnown', '_HasInfiniteSpells', '_HasDomains', '_HasSchool', '_HasFamiliar', '_HasAnimalCompanion', '_Str', '_Dex', '_Con', '_Wis', '_Int', '_Cha', '_PrimaryAbil', '_SpellAbil', '_AlignRestrict', '_AlignRstrctType', '_InvertRestrict', '_Constant', '_EffCRLvl01', '_EffCRLvl02', '_EffCRLvl03', '_EffCRLvl04', '_EffCRLvl05', '_EffCRLvl06', '_EffCRLvl07', '_EffCRLvl08', '_EffCRLvl09', '_EffCRLvl10', '_EffCRLvl11', '_EffCRLvl12', '_EffCRLvl13', '_EffCRLvl14', '_EffCRLvl15', '_EffCRLvl16', '_EffCRLvl17', '_EffCRLvl18', '_EffCRLvl19', '_EffCRLvl20', '_PreReqTable', '_MaxLevel', '_XPPenalty', '_BonusSpellcasterLevelTable', '_BonusCasterFeatByClassMap', '_ArcSpellLvlMod', '_DivSpellLvlMod', '_EpicLevel', '_Package', '_FEATPracticedSpellcaster', '_FEATExtraSlot', '_FEATArmoredCaster', '_FavoredWeaponProficiency', '_FavoredWeaponFocus', '_FavoredWeaponSpecialization', '_CharGen_Chest', '_CharGen_Feet', '_CharGen_Hands', '_CharGen_Cloak', '_CharGen_Head')
    _column_mapping = {'Label': 'Label', 'Name': 'Name', 'Plural': 'Plural', 'Lower': 'Lower', 'Description': 'Description', 'Icon': 'Icon', 'BorderedIcon': 'BorderedIcon', 'HitDie': 'HitDie', 'AttackBonusTable': 'AttackBonusTable', 'FeatsTable': 'FeatsTable', 'SavingThrowTable': 'SavingThrowTable', 'SkillsTable': 'SkillsTable', 'BonusFeatsTable': 'BonusFeatsTable', 'SkillPointBase': 'SkillPointBase', 'SpellGainTable': 'SpellGainTable', 'SpellKnownTable': 'SpellKnownTable', 'PlayerClass': 'PlayerClass', 'SpellCaster': 'SpellCaster', 'MetaMagicAllowed': 'MetaMagicAllowed', 'MemorizesSpells': 'MemorizesSpells', 'HasArcane': 'HasArcane', 'HasDivine': 'HasDivine', 'HasSpontaneousSpells': 'HasSpontaneousSpells', 'SpontaneousConversionTable': 'SpontaneousConversionTable', 'SpellSwapMinLvl': 'SpellSwapMinLvl', 'SpellSwapLvlInterval': 'SpellSwapLvlInterval', 'SpellSwapLvlDiff': 'SpellSwapLvlDiff', 'AllSpellsKnown': 'AllSpellsKnown', 'HasInfiniteSpells': 'HasInfiniteSpells', 'HasDomains': 'HasDomains', 'HasSchool': 'HasSchool', 'HasFamiliar': 'HasFamiliar', 'HasAnimalCompanion': 'HasAnimalCompanion', 'Str': 'Str', 'Dex': 'Dex', 'Con': 'Con', 'Wis': 'Wis', 'Int': 'Int', 'Cha': 'Cha', 'PrimaryAbil': 'PrimaryAbil', 'SpellAbil': 'SpellAbil', 'AlignRestrict': 'AlignRestrict', 'AlignRstrctType': 'AlignRstrctType', 'InvertRestrict': 'InvertRestrict', 'Constant': 'Constant', 'EffCRLvl01': 'EffCRLvl01', 'EffCRLvl02': 'EffCRLvl02', 'EffCRLvl03': 'EffCRLvl03', 'EffCRLvl04': 'EffCRLvl04', 'EffCRLvl05': 'EffCRLvl05', 'EffCRLvl06': 'EffCRLvl06', 'EffCRLvl07': 'EffCRLvl07', 'EffCRLvl08': 'EffCRLvl08', 'EffCRLvl09': 'EffCRLvl09', 'EffCRLvl10': 'EffCRLvl10', 'EffCRLvl11': 'EffCRLvl11', 'EffCRLvl12': 'EffCRLvl12', 'EffCRLvl13': 'EffCRLvl13', 'EffCRLvl14': 'EffCRLvl14', 'EffCRLvl15': 'EffCRLvl15', 'EffCRLvl16': 'EffCRLvl16', 'EffCRLvl17': 'EffCRLvl17', 'EffCRLvl18': 'EffCRLvl18', 'EffCRLvl19': 'EffCRLvl19', 'EffCRLvl20': 'EffCRLvl20', 'PreReqTable': 'PreReqTable', 'MaxLevel': 'MaxLevel', 'XPPenalty': 'XPPenalty', 'BonusSpellcasterLevelTable': 'BonusSpellcasterLevelTable', 'BonusCasterFeatByClassMap': 'BonusCasterFeatByClassMap', 'ArcSpellLvlMod': 'ArcSpellLvlMod', 'DivSpellLvlMod': 'DivSpellLvlMod', 'EpicLevel': 'EpicLevel', 'Package': 'Package', 'FEATPracticedSpellcaster': 'FEATPracticedSpellcaster', 'FEATExtraSlot': 'FEATExtraSlot', 'FEATArmoredCaster': 'FEATArmoredCaster', 'FavoredWeaponProficiency': 'FavoredWeaponProficiency', 'FavoredWeaponFocus': 'FavoredWeaponFocus', 'FavoredWeaponSpecialization': 'FavoredWeaponSpecialization', 'CharGen_Chest': 'CharGen_Chest', 'CharGen_Feet': 'CharGen_Feet', 'CharGen_Hands': 'CharGen_Hands', 'CharGen_Cloak': 'CharGen_Cloak', 'CharGen_Head': 'CharGen_Head'}
    _table_name = 'classes'
    _safe_columns = ['Label', 'Name', 'Plural', 'Lower', 'Description', 'Icon', 'BorderedIcon', 'HitDie', 'AttackBonusTable', 'FeatsTable', 'SavingThrowTable', 'SkillsTable', 'BonusFeatsTable', 'SkillPointBase', 'SpellGainTable', 'SpellKnownTable', 'PlayerClass', 'SpellCaster', 'MetaMagicAllowed', 'MemorizesSpells', 'HasArcane', 'HasDivine', 'HasSpontaneousSpells', 'SpontaneousConversionTable', 'SpellSwapMinLvl', 'SpellSwapLvlInterval', 'SpellSwapLvlDiff', 'AllSpellsKnown', 'HasInfiniteSpells', 'HasDomains', 'HasSchool', 'HasFamiliar', 'HasAnimalCompanion', 'Str', 'Dex', 'Con', 'Wis', 'Int', 'Cha', 'PrimaryAbil', 'SpellAbil', 'AlignRestrict', 'AlignRstrctType', 'InvertRestrict', 'Constant', 'EffCRLvl01', 'EffCRLvl02', 'EffCRLvl03', 'EffCRLvl04', 'EffCRLvl05', 'EffCRLvl06', 'EffCRLvl07', 'EffCRLvl08', 'EffCRLvl09', 'EffCRLvl10', 'EffCRLvl11', 'EffCRLvl12', 'EffCRLvl13', 'EffCRLvl14', 'EffCRLvl15', 'EffCRLvl16', 'EffCRLvl17', 'EffCRLvl18', 'EffCRLvl19', 'EffCRLvl20', 'PreReqTable', 'MaxLevel', 'XPPenalty', 'BonusSpellcasterLevelTable', 'BonusCasterFeatByClassMap', 'ArcSpellLvlMod', 'DivSpellLvlMod', 'EpicLevel', 'Package', 'FEATPracticedSpellcaster', 'FEATExtraSlot', 'FEATArmoredCaster', 'FavoredWeaponProficiency', 'FavoredWeaponFocus', 'FavoredWeaponSpecialization', 'CharGen_Chest', 'CharGen_Feet', 'CharGen_Hands', 'CharGen_Cloak', 'CharGen_Head']
    
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
            return f"<ClassesData {pk_column}={pk_value!r}>>"
        else:
            return f"<ClassesData>"
    
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
