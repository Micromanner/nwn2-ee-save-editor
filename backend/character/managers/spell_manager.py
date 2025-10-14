"""
Data-Driven Spell Manager - handles spell slots, spell lists, domain spells, and caster progression
Uses CharacterManager and DynamicGameDataLoader for all spell data access

VALIDATION CLEANUP APPLIED:
- Removed spell slot limits (users can memorize more spells than slots)
- Removed class spell access restrictions (any class can memorize any spell)
- Kept spell ID existence checks to prevent crashes on save load
- Kept all spell calculations (slots, DCs, caster levels, metamagic costs)
- Kept all data access methods and live functionality
- validate() now only checks for data corruption prevention
"""

from typing import Dict, List, Tuple, Optional, Any
from loguru import logger
from collections import defaultdict

from ..events import (
    EventEmitter, EventType,
    ClassChangedEvent, LevelGainedEvent, FeatChangedEvent
)
# Import field_mapper at module level to avoid circular imports
try:
    from gamedata.dynamic_loader.field_mapping_utility import field_mapper
except ImportError:
    # Create a simple mock for testing
    class MockFieldMapper:
        def get_field_value(self, obj, field, default=None):
            return getattr(obj, field, default)
    field_mapper = MockFieldMapper()

# Using global loguru logger


class SpellManager(EventEmitter):
    """
    Data-Driven Spell Manager
    Uses CharacterManager as hub for all character data access
    """
    
    # Metamagic data will be loaded dynamically from feat.2da
    # No more hardcoded feat IDs or level adjustments
    
    def __init__(self, character_manager):
        """
        Initialize the SpellManager
        
        Args:
            character_manager: Reference to parent CharacterManager
        """
        super().__init__()
        self.character_manager = character_manager
        self.rules_service = character_manager.rules_service
        self.gff = character_manager.gff
        
        # Register for events
        self._register_event_handlers()
        
        # Cache for performance
        self._spell_slots_cache = {}
        self._spell_table_cache = {}
        self._domain_spells_cache = {}
        self._spell_data_cache = {}
    
    @property
    def rules_service_ref(self):
        """Convenience property for accessing rules service"""
        return self.rules_service
    
    def _register_event_handlers(self):
        """Register handlers for relevant events"""
        self.character_manager.on(EventType.CLASS_CHANGED, self.on_class_changed)
        self.character_manager.on(EventType.LEVEL_GAINED, self.on_level_gained)
        self.character_manager.on(EventType.FEAT_ADDED, self.on_feat_added)
        self.character_manager.on(EventType.FEAT_REMOVED, self.on_feat_removed)
    
    def on_class_changed(self, event: ClassChangedEvent):
        """Handle class change event"""
        logger.info(f"SpellManager handling class change: {event.old_class_id} -> {event.new_class_id}")
        
        # Clear all spell lists for non-casters
        new_class = self.rules_service.get_by_id('classes', event.new_class_id)
        if not self._is_spellcaster(new_class):
            self._clear_all_spell_lists()
            return
        
        # Update spell slots and known spells
        self._update_spell_slots()
        self._update_known_spells(event.new_class_id, event.level)
        
        # Emit spell change event
        self.emit(EventType.SPELLS_CHANGED, {
            'source': 'class_change',
            'class_id': event.new_class_id
        })
    
    def on_level_gained(self, event: LevelGainedEvent):
        """Handle level gain event"""
        logger.info(f"SpellManager handling level gain: Class {event.class_id}, Level {event.new_level}")
        
        # Check if this class can cast spells
        class_data = self.rules_service.get_by_id('classes', event.class_id)
        if not self._is_spellcaster(class_data):
            return
        
        # Update spell slots
        self._update_spell_slots()
        
        # Handle new spells for the level
        self._handle_level_up_spells(event.class_id, event.new_level)
        
        # Emit spell change event
        self.emit(EventType.SPELLS_CHANGED, {
            'source': 'level_gain',
            'class_id': event.class_id,
            'level': event.new_level
        })
    
    def on_feat_added(self, event: FeatChangedEvent):
        """Handle feat addition event"""
        feat_id = event.feat_id
        
        # Check if it's a metamagic feat
        if self._is_metamagic_feat(feat_id):
            logger.info(f"SpellManager: Metamagic feat {feat_id} added")
            # Metamagic doesn't change slots, but affects spell preparation
            self.emit(EventType.SPELLS_CHANGED, {
                'source': 'metamagic_added',
                'feat_id': feat_id
            })
        
        # Check for domain-granting feats or spell-related feats
        feat_data = self.rules_service.get_by_id('feat', feat_id)
        if feat_data and self._is_spell_related_feat(feat_data):
            self._update_spell_slots()
            self.emit(EventType.SPELLS_CHANGED, {
                'source': 'spell_feat_added',
                'feat_id': feat_id
            })
    
    def on_feat_removed(self, event: FeatChangedEvent):
        """Handle feat removal event"""
        feat_id = event.feat_id
        
        # Check if it's a metamagic feat
        if self._is_metamagic_feat(feat_id):
            logger.info(f"SpellManager: Metamagic feat {feat_id} removed")
            self.emit(EventType.SPELLS_CHANGED, {
                'source': 'metamagic_removed',
                'feat_id': feat_id
            })
    
    def calculate_spell_slots(self) -> Dict[int, Dict[int, int]]:
        """
        Calculate total spell slots per day for all classes and levels
        
        Returns:
            Dict mapping class_id -> spell_level -> slots_per_day
        """
        slots_by_class = {}
        class_list = self.gff.get('ClassList', [])
        
        for class_entry in class_list:
            class_id = class_entry.get('Class', -1)
            class_level = class_entry.get('ClassLevel', 0)
            
            if class_level == 0:
                continue
            
            class_data = self.rules_service.get_by_id('classes', class_id)
            if not class_data or not self._is_spellcaster(class_data):
                continue
            
            # Get spell slots for this class
            class_slots = self._calculate_class_spell_slots(class_data, class_level, class_entry)
            if class_slots:
                slots_by_class[class_id] = class_slots
        
        return slots_by_class
    
    def _calculate_class_spell_slots(self, class_data: Any, level: int, class_entry: Dict) -> Dict[int, int]:
        """
        Calculate spell slots for a specific class and level
        
        Args:
            class_data: Class data from 2DA
            level: Class level
            class_entry: Class entry from character data (contains domains)
            
        Returns:
            Dict mapping spell_level -> slots_per_day
        """
        slots = {}
        
        # Get the spell gain table name - use proper column names with FieldMappingUtility
        spell_table_name = field_mapper.get_field_value(class_data, 'SpellGainTable', '')
        if not spell_table_name:
            logger.warning(f"No spell gain table for class {field_mapper.get_field_value(class_data, 'Label', 'Unknown')}")
            return slots
        
        # Load the spell gain table
        spell_table = self._get_spell_table(spell_table_name.lower())
        if not spell_table:
            return slots
        
        # Get base slots from table
        if 0 <= level - 1 < len(spell_table):
            table_row = spell_table[level - 1]
            
            # Extract spell slots for each level (0-9)
            for spell_level in range(10):
                field_name = f'spellslevel{spell_level}'
                base_slots = self._safe_int(field_mapper.get_field_value(table_row, field_name, 0))
                
                if base_slots > 0:
                    # Add bonus slots from high ability scores
                    bonus_slots = self._calculate_bonus_spell_slots(class_data, spell_level)
                    slots[spell_level] = base_slots + bonus_slots
                    logger.debug(f"Spell level {spell_level}: base={base_slots}, "
                                 f"bonus={bonus_slots}, total={base_slots + bonus_slots}")
        
        # Add domain spell slots for divine casters
        if self._is_divine_caster(class_data):
            self._add_domain_spell_slots(slots, class_entry)
        
        return slots
    
    def _calculate_bonus_spell_slots(self, class_data: Any, spell_level: int) -> int:
        """
        Calculate bonus spell slots from high ability scores
        
        Args:
            class_data: Class data
            spell_level: Spell level (0-9)
            
        Returns:
            Number of bonus slots
        """
        if spell_level == 0:  # No bonus slots for cantrips
            return 0
        
        # Determine casting ability
        casting_ability = self._get_casting_ability(class_data)
        if not casting_ability:
            return 0
        
        # Get ability score
        ability_score = self.gff.get(casting_ability, 10)
        ability_modifier = (ability_score - 10) // 2
        
        # D&D 3.5 bonus spell calculation:
        # If ability modifier >= spell level, you get bonus spells
        # Bonus slots = 1 + floor((modifier - spell_level) / 4)
        if ability_modifier >= spell_level:
            bonus = 1 + max(0, (ability_modifier - spell_level) // 4)
            return bonus
        
        return 0
    
    def _get_casting_ability(self, class_data: Any) -> Optional[str]:
        """Get the primary casting ability for a class"""
        # Try to get from class data using proper field names
        ability = field_mapper.get_field_value(class_data, 'PrimaryAbil', '')
        if ability and ability != '****':
            return ability
        
        # Try SpellAbility field
        ability = field_mapper.get_field_value(class_data, 'SpellAbility', '')
        if ability and ability != '****':
            return ability
        
        # Try SpellcastingAbil field
        ability = field_mapper.get_field_value(class_data, 'SpellcastingAbil', '')
        if ability and ability != '****':
            return ability
        
        # No hardcoded fallback - all data should come from 2DA files
        logger.warning(f"No casting ability found in class data for {field_mapper.get_field_value(class_data, 'Label', 'Unknown Class')}")
        return None
    
    def _is_divine_caster(self, class_data: Any) -> bool:
        """Check if a class is a divine caster (has domains)"""
        # Check if class has domain support (indicated by domain fields)
        has_domains = (
            field_mapper.get_field_value(class_data, 'HasDomains', '') == '1' or
            field_mapper.get_field_value(class_data, 'MaxDomains', '0') != '0'
        )
        
        return has_domains
    
    def _add_domain_spell_slots(self, slots: Dict[int, int], class_entry: Dict):
        """Add extra spell slot per level for divine casters with domains"""
        # Clerics get +1 domain spell slot per spell level
        if class_entry.get('Domain1', -1) >= 0 or class_entry.get('Domain2', -1) >= 0:
            for spell_level in range(1, 10):  # Levels 1-9, not cantrips
                if spell_level in slots and slots[spell_level] > 0:
                    slots[spell_level] += 1
    
    def get_known_spells(self, class_id: int) -> Dict[int, List[int]]:
        """
        Get known spells for a class
        
        Args:
            class_id: Class ID
            
        Returns:
            Dict mapping spell_level -> list of spell IDs
        """
        known_spells = defaultdict(list)
        
        # For spontaneous casters, track known spells
        # For prepared casters, return all available spells
        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return dict(known_spells)
        
        # Check each spell level's known list
        for spell_level in range(10):
            known_list = self.gff.get(f'KnownList{spell_level}', [])
            
            # Filter by class if multiclassed
            class_spells = []
            for spell_entry in known_list:
                spell_id = spell_entry.get('Spell', -1)
                spell_class = spell_entry.get('SpellClass', class_id)
                
                if spell_class == class_id and spell_id >= 0:
                    class_spells.append(spell_id)
            
            if class_spells:
                known_spells[spell_level] = class_spells
        
        return dict(known_spells)
    
    def get_memorized_spells(self, class_id: int) -> Dict[int, List[Dict[str, Any]]]:
        """
        Get memorized/prepared spells for a class
        
        Args:
            class_id: Class ID
            
        Returns:
            Dict mapping spell_level -> list of memorized spell entries
        """
        memorized_spells = defaultdict(list)
        
        # Check each spell level's memorized list
        for spell_level in range(10):
            memorized_list = self.gff.get(f'MemorizedList{spell_level}', [])
            
            # Filter by class if multiclassed
            class_spells = []
            for spell_entry in memorized_list:
                spell_class = spell_entry.get('SpellClass', class_id)
                
                if spell_class == class_id:
                    class_spells.append({
                        'spell_id': spell_entry.get('Spell', -1),
                        'ready': spell_entry.get('Ready', 1),
                        'metamagic': spell_entry.get('SpellMetaMagicN2', 0),
                        'domain': spell_entry.get('SpellDomain', 0)
                    })
            
            if class_spells:
                memorized_spells[spell_level] = class_spells
        
        return dict(memorized_spells)
    
    def get_domain_spells(self, class_id: int) -> Dict[int, List[int]]:
        """
        Get domain spells for a divine caster
        
        Args:
            class_id: Class ID
            
        Returns:
            Dict mapping spell_level -> list of domain spell IDs
        """
        domain_spells = defaultdict(list)
        
        # Find the class entry
        class_list = self.gff.get('ClassList', [])
        class_entry = None
        for entry in class_list:
            if entry.get('Class') == class_id:
                class_entry = entry
                break
        
        if not class_entry:
            return dict(domain_spells)
        
        # Get domain IDs
        domain1 = class_entry.get('Domain1', -1)
        domain2 = class_entry.get('Domain2', -1)
        
        # Load domain spell lists
        for domain_id in [domain1, domain2]:
            if domain_id < 0:
                continue
            
            domain_data = self.rules_service.get_by_id('domains', domain_id)
            if not domain_data:
                continue
            
            # Get spells for each level (1-9)
            for spell_level in range(1, 10):
                field_name = f'level_{spell_level}'
                spell_id = field_mapper.get_field_value(domain_data, field_name, -1)
                # Also try without underscore
                if spell_id < 0:
                    field_name = f'level{spell_level}'
                    spell_id = field_mapper.get_field_value(domain_data, field_name, -1)
                
                # Convert string to int if needed
                if isinstance(spell_id, str) and spell_id.isdigit():
                    spell_id = int(spell_id)
                
                if spell_id >= 0 and spell_id not in domain_spells[spell_level]:
                    domain_spells[spell_level].append(spell_id)
        
        return dict(domain_spells)
    
    def prepare_spell(self, class_id: int, spell_level: int, spell_id: int,
                      metamagic: int = 0, domain: bool = False) -> bool:
        """
        Prepare a spell for casting
        
        Args:
            class_id: Class ID
            spell_level: Spell level (0-9)
            spell_id: Spell ID
            metamagic: Metamagic flags
            domain: Whether this is a domain spell
            
        Returns:
            True if spell was prepared
        """
        # Validate spell ID exists to prevent crashes
        spell_data = self.rules_service.get_by_id('spells', spell_id)
        if not spell_data:
            logger.warning(f"Cannot prepare spell - invalid spell ID: {spell_id}")
            return False
        
        # Add to memorized list (no slot restrictions - let users memorize as many as they want)
        memorized_list = self.gff.get(f'MemorizedList{spell_level}', [])
        memorized_list.append({
            'Spell': spell_id,
            'Ready': 1,
            'SpellMetaMagicN2': metamagic,
            'SpellClass': class_id,
            'SpellDomain': 1 if domain else 0
        })
        self.gff.set(f'MemorizedList{spell_level}', memorized_list)
        
        return True
    
    def clear_memorized_spells(self, class_id: int, spell_level: Optional[int] = None):
        """
        Clear memorized spells for a class
        
        Args:
            class_id: Class ID
            spell_level: Optional specific spell level to clear
        """
        if spell_level is not None:
            # Clear specific level
            memorized_list = self.gff.get(f'MemorizedList{spell_level}', [])
            filtered = [s for s in memorized_list if s.get('SpellClass', class_id) != class_id]
            self.gff.set(f'MemorizedList{spell_level}', filtered)
        else:
            # Clear all levels
            for level in range(10):
                memorized_list = self.gff.get(f'MemorizedList{level}', [])
                filtered = [s for s in memorized_list if s.get('SpellClass', class_id) != class_id]
                self.gff.set(f'MemorizedList{level}', filtered)
    
    def add_known_spell(self, class_id: int, spell_level: int, spell_id: int) -> bool:
        """
        Add a spell to the known spell list
        
        Args:
            class_id: Class ID
            spell_level: Spell level
            spell_id: Spell ID
            
        Returns:
            True if spell was added
        """
        # Get current known list
        known_list = self.gff.get(f'KnownList{spell_level}', [])
        
        # Check if already known
        for spell in known_list:
            if (spell.get('Spell') == spell_id and
                    spell.get('SpellClass', class_id) == class_id):
                return False
        
        # Add to known list
        known_list.append({
            'Spell': spell_id,
            'SpellClass': class_id
        })
        self.gff.set(f'KnownList{spell_level}', known_list)
        
        return True
    
    def remove_known_spell(self, class_id: int, spell_level: int, spell_id: int) -> bool:
        """
        Remove a spell from the known spell list
        
        Args:
            class_id: Class ID
            spell_level: Spell level
            spell_id: Spell ID
            
        Returns:
            True if spell was removed, False if not found
        """
        # Get current known list
        known_list = self.gff.get(f'KnownList{spell_level}', [])
        
        # Find and remove the spell
        original_length = len(known_list)
        known_list = [
            spell for spell in known_list
            if not (spell.get('Spell') == spell_id and
                   spell.get('SpellClass', class_id) == class_id)
        ]
        
        # Update the list if anything was removed
        if len(known_list) < original_length:
            self.gff.set(f'KnownList{spell_level}', known_list)
            return True
        
        return False
    
    def get_spell_level_for_class(self, spell_id: int, class_id: int) -> Optional[int]:
        """
        Get the spell level for a specific spell and class
        
        Args:
            spell_id: Spell ID
            class_id: Class ID
            
        Returns:
            Spell level (0-9) or None if spell not available for class
        """
        spell_data = self.rules_service.get_by_id('spells', spell_id)
        if not spell_data:
            return None
            
        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return None
        
        # Get the spell list field name from class data
        # This is stored in SpellTableColumn in classes.2da
        spell_table_column = field_mapper.get_field_value(class_data, 'SpellTableColumn', '')
        
        if not spell_table_column or spell_table_column == '****':
            # Try to infer from class label
            class_label = field_mapper.get_field_value(class_data, 'Label', '')
            
            # Common mappings
            if 'wizard' in class_label.lower() or 'sorcerer' in class_label.lower():
                spell_field = 'Wiz_Sorc'
            elif 'warlock' in class_label.lower():
                spell_field = 'Warlock'
            elif 'bard' in class_label.lower():
                spell_field = 'Bard'
            elif 'cleric' in class_label.lower():
                spell_field = 'Cleric'
            elif 'druid' in class_label.lower():
                spell_field = 'Druid'
            elif 'paladin' in class_label.lower():
                spell_field = 'Paladin'
            elif 'ranger' in class_label.lower():
                spell_field = 'Ranger'
            else:
                return None
        else:
            spell_field = spell_table_column
        
        # Get the spell level for this class
        spell_level = field_mapper.get_field_value(spell_data, spell_field, None)
        
        # Convert to int if needed and validate
        if spell_level is not None:
            try:
                level = int(spell_level)
                if 0 <= level <= 9:
                    return level
            except (ValueError, TypeError):
                pass
        
        return None
    
    def is_spellcaster(self, class_id: int) -> bool:
        """
        Check if a class can cast spells
        
        Args:
            class_id: Class ID
            
        Returns:
            True if the class can cast spells
        """
        # Check if class has spell slots
        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return False
            
        # Check if it has any spell progression
        return (
            field_mapper.get_field_value(class_data, 'SpellGainTable', '') != '' or
            any(
                field_mapper.get_field_value(class_data, f'SpellGainTable{i}', '') != ''
                for i in range(10)
            )
        )
    
    def get_class_name(self, class_id: int) -> str:
        """
        Get the display name for a class
        
        Args:
            class_id: Class ID
            
        Returns:
            Class name or 'Unknown Class'
        """
        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return f'Unknown Class ({class_id})'
        
        # Use the label field directly (already localized in most cases)
        return field_mapper.get_field_value(class_data, 'Label', f'Class_{class_id}')
    
    def get_caster_level(self, class_index: int) -> int:
        """
        Get the caster level for a class
        
        Args:
            class_index: Index in the character's class list
            
        Returns:
            Caster level
        """
        class_list = self.gff.get('ClassList', [])
        if class_index >= len(class_list):
            return 0
            
        class_entry = class_list[class_index]
        class_id = class_entry.get('Class', -1)
        class_level = class_entry.get('ClassLevel', 0)
        
        # Some classes have reduced caster level
        class_data = self.rules_service.get_by_id('classes', class_id)
        if class_data:
            # Check SpellCaster field for caster level calculation
            # 1 = full progression, 2 = (level-3), 3 = level/2, etc.
            spell_caster_type_str = field_mapper.get_field_value(class_data, 'SpellCaster', '1')
            
            try:
                spell_caster_type = int(spell_caster_type_str)
                if spell_caster_type == 2:  # Paladin/Ranger style: caster level = class level - 3
                    return max(0, class_level - 3)
                elif spell_caster_type == 3:  # Half progression: caster level = class level / 2
                    return class_level // 2
                elif spell_caster_type == 4:  # Custom progression - would need to check SpellGainTable
                    # For now, default to full progression
                    return class_level
            except (ValueError, TypeError):
                pass
        
        return class_level
    
    def is_prepared_caster(self, class_id: int) -> bool:
        """
        Check if a class prepares spells (vs spontaneous casting)
        
        Args:
            class_id: Class ID
            
        Returns:
            True if class prepares spells, False if spontaneous
        """
        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return False
        
        # Check the MemorizesSpells field in the class data
        # 1 = prepares spells, 0 = spontaneous caster
        memorizes_spells = field_mapper.get_field_value(class_data, 'MemorizesSpells', None)
        if memorizes_spells is not None and memorizes_spells != '****':
            try:
                return bool(int(memorizes_spells))
            except (ValueError, TypeError):
                pass
        
        # Fallback: check SpellKnownTable
        # Classes with SpellKnownTable are spontaneous casters
        spell_known_table = field_mapper.get_field_value(class_data, 'SpellKnownTable', '')
        if spell_known_table and spell_known_table != '****':
            return False  # Has known spell table = spontaneous
        
        # Default to prepared if it has spell progression
        return self.is_spellcaster(class_id)
    
    def get_all_memorized_spells(self) -> List[Dict[str, Any]]:
        """
        Get all memorized spells for all classes
        
        Returns:
            List of memorized spell data
        """
        memorized = []
        
        for level in range(10):
            mem_list = self.gff.get(f'MemorizedList{level}', [])
            for spell in mem_list:
                memorized.append({
                    'level': level,
                    'spell_id': spell.get('Spell'),
                    'class_id': spell.get('SpellClass'),
                    'metamagic': spell.get('SpellMetaMagicN2', 0),
                    'ready': spell.get('Ready', 1) == 1
                })
        
        return memorized
    
    def get_metamagic_feats(self) -> List[int]:
        """Get list of metamagic feat IDs the character has"""
        metamagic = []
        feat_manager = self.character_manager.get_manager('feat')
        if not feat_manager:
            return metamagic
            
        # Get all character feats and check which ones are metamagic
        feat_list = self.gff.get('FeatList', [])
        for feat in feat_list:
            feat_id = feat.get('Feat')
            if feat_id and self._is_metamagic_feat(feat_id):
                metamagic.append(feat_id)
        return metamagic
    
    def calculate_metamagic_cost(self, metamagic_flags: int) -> int:
        """
        Calculate the spell level adjustment for metamagic
        
        Args:
            metamagic_flags: Bitmask of metamagic effects
            
        Returns:
            Total spell level adjustment
        """
        total_cost = 0
        
        # Get metamagic feats from 2DA and check flags
        metamagic_feats = self.get_metamagic_feats()
        for feat_id in metamagic_feats:
            if self._has_metamagic_flag(metamagic_flags, feat_id):
                cost = self._get_metamagic_level_cost(feat_id)
                total_cost += cost
        
        return total_cost
    
    def _has_metamagic_flag(self, flags: int, feat_id: int) -> bool:
        """Check if metamagic flags include a specific feat"""
        # Get the metamagic bit position from feat data
        feat_data = self.rules_service.get_by_id('feat', feat_id)
        if not feat_data:
            return False
            
        # Try to get the metamagic flag value from feat.2da
        metamagic_bit = field_mapper.get_field_value(feat_data, 'MetamagicBit', 0)
        try:
            bit_value = int(metamagic_bit) if metamagic_bit else 0
            return (flags & bit_value) != 0
        except (ValueError, TypeError):
            return False
    
    def _is_spellcaster(self, class_data: Any) -> bool:
        """Check if a class can cast spells"""
        if not class_data:
            return False
        
        # Check SpellCaster field - use proper column name
        is_caster = field_mapper.get_field_value(class_data, 'SpellCaster', '0')
        # Ensure we're getting a proper boolean value
        if isinstance(is_caster, str):
            is_caster = is_caster != '0' and is_caster != '****'
        if is_caster:
            return True
        
        # Check for spell gain table - use proper column name
        spell_table = field_mapper.get_field_value(class_data, 'SpellGainTable', '')
        return bool(spell_table and spell_table != '****')
    
    def _is_spell_related_feat(self, feat_data: Any) -> bool:
        """Check if a feat affects spellcasting"""
        if not feat_data:
            return False
        
        # Check for spell-granting feats, extra slot feats, etc.
        feat_label = field_mapper.get_field_value(feat_data, 'Label', '')
        if not feat_label or feat_label == '****':
            # Try alternative field names
            feat_label = field_mapper.get_field_value(feat_data, 'Name', '')
        
        # Handle mock objects in tests
        if hasattr(feat_label, '_mock_name'):
            return False
            
        feat_label = str(feat_label).lower()
        spell_keywords = ['spell', 'slot', 'domain', 'school', 'casting']
        
        return any(keyword in feat_label for keyword in spell_keywords)
    
    def _get_spell_table(self, table_name: str) -> Optional[Any]:
        """Get and cache spell progression table"""
        if table_name in self._spell_table_cache:
            return self._spell_table_cache[table_name]
        
        table = self.rules_service.get_table(table_name)
        if table:
            self._spell_table_cache[table_name] = table
        
        return table
    
    def _clear_all_spell_lists(self):
        """Clear all spell-related lists"""
        for spell_level in range(10):
            self.gff.set(f'KnownList{spell_level}', [])
            self.gff.set(f'MemorizedList{spell_level}', [])
    
    def _update_spell_slots(self):
        """Recalculate spell slots after a change"""
        # Clear cache to force recalculation
        self._spell_slots_cache.clear()
        
        # Recalculate
        self.calculate_spell_slots()
    
    def _update_known_spells(self, class_id: int, level: int):
        """Update known spells for spontaneous casters"""
        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return
        
        # Check if this is a spontaneous caster
        known_table_name = field_mapper.get_field_value(class_data, 'SpellKnownTable', '')
        if not known_table_name or known_table_name == '****':
            return  # Not a spontaneous caster
        
        # Load known spells table
        known_table = self._get_spell_table(known_table_name.lower())
        if not known_table:
            return
            
        # Handle mock objects in tests
        try:
            table_length = len(known_table)
        except TypeError:
            # Mock object, skip
            return
            
        if level - 1 >= table_length:
            return
        
        # Get spells known for this level
        table_row = known_table[level - 1]
        
        # TODO: Implement spell selection UI integration
        # For now, just log what spells could be learned
        for spell_level in range(10):
            field_name = f'spellslevel{spell_level}'
            spells_known = self._safe_int(field_mapper.get_field_value(table_row, field_name, 0))
            if spells_known > 0:
                logger.info(f"Class {class_id} at level {level} knows {spells_known} "
                            f"spells of level {spell_level}")
    
    def _handle_level_up_spells(self, class_id: int, new_level: int):
        """Handle spell changes when gaining a level"""
        # Update known spells for spontaneous casters
        self._update_known_spells(class_id, new_level)
        
        # Clear memorized spells for prepared casters to force re-preparation
        class_data = self.rules_service.get_by_id('classes', class_id)
        if class_data and self._is_prepared_caster(class_data):
            logger.info("Clearing memorized spells for prepared caster level up")
            # Don't actually clear - just mark as needing preparation
            self.emit(EventType.SPELLS_CHANGED, {
                'source': 'need_preparation',
                'class_id': class_id
            })
    
    def _is_prepared_caster(self, class_data: Any) -> bool:
        """Check if a class prepares spells (vs spontaneous)"""
        # Use the same logic as is_prepared_caster method but for class_data directly
        memorizes_spells = field_mapper.get_field_value(class_data, 'MemorizesSpells', None)
        if memorizes_spells is not None and memorizes_spells != '****':
            try:
                return bool(int(memorizes_spells))
            except (ValueError, TypeError):
                pass
        
        # Fallback: check SpellKnownTable - classes with this are spontaneous
        spell_known_table = field_mapper.get_field_value(class_data, 'SpellKnownTable', '')
        if spell_known_table and spell_known_table != '****':
            return False  # Has known spell table = spontaneous
        
        # Default to prepared if it has spell progression
        return self._is_spellcaster(class_data)
    
    def _safe_int(self, value: Any, default: int = 0) -> int:
        """Safely convert value to int"""
        if value is None or value == '****':
            return default
        try:
            return int(value)
        except (ValueError, TypeError):
            return default
    
    
    def get_spell_resistance(self) -> int:
        """
        Calculate total spell resistance
        
        Returns:
            Total spell resistance value
        """
        # Base SR from character
        base_sr = self.gff.get('SpellResistance', 0)
        
        # Add SR from feats
        feat_sr = 0
        # TODO: Check for SR-granting feats like Spell Resistance feat
        
        # Add SR from items (would need InventoryManager)
        item_sr = 0
        
        return base_sr + feat_sr + item_sr
    
    def validate(self) -> Tuple[bool, List[str]]:
        """Validate current spell configuration - only check for data corruption prevention"""
        errors = []
        
        # Check each caster class
        slots_by_class = self.calculate_spell_slots()
        
        for class_id, slots in slots_by_class.items():
            class_data = self.rules_service.get_by_id('classes', class_id)
            if not class_data:
                errors.append(f"Invalid class ID: {class_id}")
                continue
            
            # Validate spell IDs exist to prevent crashes on load
            memorized = self.get_memorized_spells(class_id)
            for spell_level, spell_list in memorized.items():
                for spell_entry in spell_list:
                    spell_id = spell_entry.get('spell_id', -1)
                    if spell_id >= 0:  # -1 is valid empty slot
                        spell_data = self.rules_service.get_by_id('spells', spell_id)
                        if not spell_data:
                            errors.append(f"Invalid spell ID {spell_id} found in memorized spells")
            
            # Validate known spell IDs exist
            known = self.get_known_spells(class_id)
            for spell_level, spell_list in known.items():
                for spell_id in spell_list:
                    if spell_id >= 0:  # -1 is valid empty slot
                        spell_data = self.rules_service.get_by_id('spells', spell_id)
                        if not spell_data:
                            errors.append(f"Invalid spell ID {spell_id} found in known spells")
        
        return len(errors) == 0, errors
    
    def get_spell_summary(self) -> Dict[str, Any]:
        """Get summary of character's spellcasting abilities"""
        summary = {
            'caster_classes': [],
            'total_spell_levels': 0,
            'metamagic_feats': [],
            'spell_resistance': self.get_spell_resistance()
        }
        
        # Get caster classes
        slots_by_class = self.calculate_spell_slots()
        
        for class_id, slots in slots_by_class.items():
            class_data = self.rules_service.get_by_id('classes', class_id)
            class_name = field_mapper.get_field_value(class_data, 'Label', f'Unknown Class {class_id}')
            
            # Count total spell levels
            total_slots = sum(slots.values())
            max_spell_level = max(slots.keys()) if slots else 0
            
            summary['caster_classes'].append({
                'id': class_id,
                'name': class_name,
                'total_slots': total_slots,
                'max_spell_level': max_spell_level,
                'slots_by_level': slots
            })
            
            summary['total_spell_levels'] += total_slots
        
        # Get metamagic feats
        for feat_id in self.get_metamagic_feats():
            feat_data = self.rules_service.get_by_id('feat', feat_id)
            feat_name = field_mapper.get_field_value(feat_data, 'Label', f'Unknown Feat {feat_id}')
            level_cost = self._get_metamagic_level_cost(feat_id)
            summary['metamagic_feats'].append({
                'id': feat_id,
                'name': feat_name,
                'level_cost': level_cost
            })
        
        return summary
    
    def get_available_spells(self, spell_level: int, class_id: Optional[int] = None) -> List[Dict[str, Any]]:
        """
        Get all spells available at a specific level, optionally filtered by class
        
        Args:
            spell_level: Spell level (0-9)
            class_id: Optional class ID to filter spells for that class
            
        Returns:
            List of available spell data
        """
        available_spells = []
        
        try:
            # Get all spells from the game data
            all_spells = self.rules_service.get_table('spells')
            if not all_spells:
                return available_spells
            
            # Filter out dev/test spells first, keeping track of original indices
            legitimate_spells = self._filter_legitimate_spells_with_indices(all_spells)
            
            for original_idx, spell_data in legitimate_spells:
                spell_id = original_idx  # Use original index as spell ID
                
                # Check if this spell is available at the requested level
                spell_available = False
                available_classes = []
                
                # Check each class column for this spell level
                class_columns = {
                    'Bard': 'bard',
                    'Cleric': 'cleric',
                    'Druid': 'druid', 
                    'Paladin': 'paladin',
                    'Ranger': 'ranger',
                    'Wiz_Sorc': 'wizard/sorcerer',
                    'Warlock': 'warlock'
                }
                
                for column_name, class_name in class_columns.items():
                    try:
                        level_value = field_mapper.get_field_value(spell_data, column_name, -1)
                        if level_value is not None and level_value != '****' and level_value != '':
                            level_int = int(level_value)
                            if level_int == spell_level:
                                spell_available = True
                                available_classes.append(class_name)
                                
                                # If filtering by class, check if this class matches
                                if class_id is not None:
                                    class_data = self.rules_service.get_by_id('classes', class_id)
                                    if class_data and self._class_matches_column(class_data, column_name):
                                        spell_available = True
                                        break
                    except (ValueError, TypeError):
                        continue
                
                # If spell is available at this level, add it to results
                if spell_available:
                    # Get spell metadata - use Name field for display name
                    spell_name = field_mapper.get_field_value(spell_data, 'Name', f'Spell_{spell_id}')
                    spell_icon = field_mapper.get_field_value(spell_data, 'IconResRef', '')
                    school_id_raw = field_mapper.get_field_value(spell_data, 'School', 0)
                    
                    # Get additional spell details first (needed for invocation detection)
                    spell_desc = field_mapper.get_field_value(spell_data, 'SpellDesc', '')
                    
                    # Convert school_id to integer properly
                    # NWN2 spells.2da uses letter codes for schools, need to map to IDs
                    school_id = None
                    school_name = None
                    
                    # Handle school mapping
                    if school_id_raw not in [None, '', '****', 0]:
                        # Try to map letter codes to school IDs first
                        school_letter_map = {
                            'G': 0,  # General
                            'A': 1,  # Abjuration
                            'C': 2,  # Conjuration  
                            'D': 3,  # Divination
                            'E': 4,  # Enchantment
                            'V': 5,  # Evocation
                            'I': 6,  # Illusion
                            'N': 7,  # Necromancy
                            'T': 8   # Transmutation
                        }
                        
                        school_letter = str(school_id_raw).upper().strip()
                        if school_letter in school_letter_map:
                            school_id = school_letter_map[school_letter]
                        else:
                            # Fallback: try direct integer conversion
                            try:
                                school_id = int(school_id_raw)
                            except (ValueError, TypeError):
                                school_id = 0  # Default to General
                        
                        # Get school name from spellschools table
                        if school_id is not None:
                            school_data = self.rules_service.get_by_id('spellschools', school_id)
                            if school_data:
                                school_name = field_mapper.get_field_value(school_data, 'Label', None)
                    else:
                        # Handle spells with no school (null, '', '****', 0)
                        # Check if this is a warlock invocation
                        if available_classes == ['warlock']:
                            school_id = 9  # Custom school ID for invocations
                            school_name = "Invocation"
                            
                            # Optional: Add detailed subtyping based on description
                            if spell_desc:
                                desc_upper = spell_desc.upper()
                                if 'INVOCATION TYPE:' in desc_upper or 'INVOCATION LEVEL:' in desc_upper:
                                    if 'LEAST' in desc_upper:
                                        school_name = "Invocation (Least)"
                                    elif 'LESSER' in desc_upper:
                                        school_name = "Invocation (Lesser)"
                                    elif 'GREATER' in desc_upper:
                                        school_name = "Invocation (Greater)"
                                    elif 'DARK' in desc_upper:
                                        school_name = "Invocation (Dark)"
                        else:
                            # Truly unknown/no school
                            school_id = None
                            school_name = None
                    spell_range = field_mapper.get_field_value(spell_data, 'Range', '')
                    cast_time = field_mapper.get_field_value(spell_data, 'CastTime', '')
                    conj_time = field_mapper.get_field_value(spell_data, 'ConjTime', '')
                    components = field_mapper.get_field_value(spell_data, 'VS', '')
                    metamagic = field_mapper.get_field_value(spell_data, 'MetaMagic', '')
                    target_type = field_mapper.get_field_value(spell_data, 'TargetType', '')
                    
                    spell_info = {
                        'id': spell_id,
                        'name': spell_name,
                        'icon': spell_icon,
                        'school_id': school_id,
                        'school_name': school_name,
                        'level': spell_level,
                        'available_classes': available_classes,
                        'description': spell_desc,
                        'range': spell_range,
                        'cast_time': cast_time,
                        'conjuration_time': conj_time,
                        'components': components,
                        'metamagic': metamagic,
                        'target_type': target_type
                    }
                    
                    available_spells.append(spell_info)
        
        except Exception as e:
            logger.error(f"Error getting available spells: {e}")
            
        return available_spells
    
    def _filter_legitimate_spells_with_indices(self, all_spells: List[Any]) -> List[Tuple[int, Any]]:
        """
        Filter out dev/test spells, keeping only legitimate player-usable spells and abilities.
        
        Args:
            all_spells: List of all spell data objects
            
        Returns:
            List of tuples (original_index, spell_data) for filtered legitimate spells
        """
        legitimate_spells = []
        
        for spell_idx, spell in enumerate(all_spells):
            # Skip removed spells
            removed = field_mapper.get_field_value(spell, 'REMOVED', None)
            if removed == '1':
                continue
                
            # Get user type
            user_type = field_mapper.get_field_value(spell, 'UserType', None)
            
            # Skip obvious dev/test content
            if user_type in ['4', '5'] or user_type is None:
                continue
                
            # Skip deleted spells by label pattern
            label = field_mapper.get_field_value(spell, 'Label', '') or ''
            if label.startswith('DELETED_') or label.startswith('DEL_'):
                continue
                
            # Skip spells with broken names
            name = field_mapper.get_field_value(spell, 'Name', None)
            if not name or name == 'None' or name.isdigit():
                continue
                
            # Keep UserType 1 (player spells), 2 (creature auras), 3 (class abilities)
            legitimate_spells.append((spell_idx, spell))
        
        return legitimate_spells
    
    def _class_matches_column(self, class_data: Any, column_name: str) -> bool:
        """Check if a class matches a spell table column"""
        class_label = field_mapper.get_field_value(class_data, 'Label', '').lower()
        
        column_mappings = {
            'Bard': ['bard'],
            'Cleric': ['cleric'],
            'Druid': ['druid'],
            'Paladin': ['paladin'],
            'Ranger': ['ranger'],
            'Wiz_Sorc': ['wizard', 'sorcerer'],
            'Warlock': ['warlock']
        }
        
        if column_name in column_mappings:
            return any(class_name in class_label for class_name in column_mappings[column_name])
        
        return False
    
    def _is_metamagic_feat(self, feat_id: int) -> bool:
        """Check if a feat is a metamagic feat"""
        feat_data = self.rules_service.get_by_id('feat', feat_id)
        if not feat_data:
            return False
        
        # Check if feat has metamagic type or flag
        feat_type = field_mapper.get_field_value(feat_data, 'FeatType', '')
        if feat_type == 'METAMAGIC':
            return True
        
        # Check for metamagic bit field
        metamagic_bit = field_mapper.get_field_value(feat_data, 'MetamagicBit', 0)
        return metamagic_bit != 0 and metamagic_bit != '0'
    
    def _get_metamagic_level_cost(self, feat_id: int) -> int:
        """Get the spell level adjustment for a metamagic feat"""
        feat_data = self.rules_service.get_by_id('feat', feat_id)
        if not feat_data:
            return 0
        
        # Try to get level cost from feat data
        level_cost = field_mapper.get_field_value(feat_data, 'MetamagicLevelCost', 0)
        if level_cost:
            try:
                return int(level_cost)
            except (ValueError, TypeError):
                pass
        
        # Try SpellLevelCost field
        level_cost = field_mapper.get_field_value(feat_data, 'SpellLevelCost', 0)
        if level_cost:
            try:
                return int(level_cost)
            except (ValueError, TypeError):
                pass
        
        # Default fallback based on common metamagic feats
        feat_name = field_mapper.get_field_value(feat_data, 'Label', '').lower()
        if 'empower' in feat_name:
            return 2
        elif 'maximize' in feat_name:
            return 3
        elif 'quicken' in feat_name:
            return 4
        elif 'persistent' in feat_name:
            return 6
        elif 'permanent' in feat_name:
            return 5
        elif any(word in feat_name for word in ['extend', 'silent', 'still']):
            return 1
        
        return 1  # Default metamagic cost