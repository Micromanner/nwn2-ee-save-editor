"""Spell Manager - handles spell slots, spell lists, domain spells, and caster progression."""

from typing import Dict, List, Tuple, Optional, Any
from loguru import logger
from collections import defaultdict
import time

from ..events import (
    EventEmitter, EventType,
    ClassChangedEvent, LevelGainedEvent, FeatChangedEvent, DomainChangedEvent,
    SpellChangedEvent
)
try:
    from gamedata.dynamic_loader.field_mapping_utility import field_mapper
except ImportError:
    class MockFieldMapper:
        def get_field_value(self, obj, field, default=None):
            return getattr(obj, field, default)
    field_mapper = MockFieldMapper()


class SpellManager(EventEmitter):
    """Handles spell slots, spell lists, domain spells, and caster progression."""

    MAX_SPELL_LEVEL = 9
    SCHOOL_LETTER_MAP = {
        'G': 0, 'A': 1, 'C': 2, 'D': 3,
        'E': 4, 'V': 5, 'I': 6, 'N': 7, 'T': 8
    }
    KNOWN_NON_CLASS_FIELDS = {
        'Label', 'Name', 'IconResRef', 'School', 'Range', 'VS', 'MetaMagic', 'TargetType',
        'ImpactScript', 'ConjTime', 'ConjAnim', 'ConjVisual0', 'LowConjVisual0',
        'ConjSoundMale', 'ConjSoundFemale', 'CastAnim', 'CastTime', 'CastVisual0', 'LowCastVisual0',
        'Proj', 'ProjSEF', 'LowProjSEF', 'ProjType', 'ProjSpwnPoint', 'ProjOrientation',
        'ImpactSEF', 'LowImpactSEF', 'ImmunityType', 'ItemImmunity', 'Category', 'UserType',
        'SpellDesc', 'UseConcentration', 'SpontaneouslyCast', 'SpontCastClassReq', 'HostileSetting',
        'HasProjectile', 'TargetingUI', 'CastableOnDead', 'REMOVED', 'Innate', 'ConjSoundOverride',
        'Counter1', 'Counter2', 'Counter3', 'Counter4', 'Counter5', 'Master', 'ProjModel',
        'SubRadSpell1', 'SubRadSpell2', 'SubRadSpell3', 'SubRadSpell4', 'SubRadSpell5',
        'AltMessage', 'ConjHeadVisual', 'ConjHandVisual', 'ConjGrndVisual',
        'CastHeadVisual', 'CastHandVisual', 'CastGrndVisual', 'FeatID', 'AsMetaMagic',
        'ConjSoundVFX', 'ProjSound'
    }

    def __init__(self, character_manager):
        """Initialize SpellManager with parent CharacterManager."""
        super().__init__()
        self.character_manager = character_manager
        self.rules_service = character_manager.rules_service
        self.gff = character_manager.gff

        self._register_event_handlers()

        self._spell_slots_cache = {}
        self._spell_table_cache = {}
        self._domain_spells_cache = {}
        self._spell_data_cache = {}
        self._legitimate_spells_cache = None
        self._master_spell_cache = None
        self._column_to_class_id_cache = None
    
    def get_spell_details(self, spell_id: int) -> Dict[str, Any]:
        """Get details for a specific spell, including resolved name and school."""
        if spell_id in self._spell_data_cache:
            return self._spell_data_cache[spell_id]

        spell_data = self.rules_service.get_by_id('spells', spell_id)
        if not spell_data:
            raise ValueError(f"Spell ID {spell_id} not found in spells.2da")

        name_raw = field_mapper.get_field_value(spell_data, 'Name', f'Spell_{spell_id}')
        if isinstance(name_raw, int):
            name = self.rules_service._loader.get_string(name_raw)
        elif isinstance(name_raw, str) and name_raw.isdigit():
            name = self.rules_service._loader.get_string(int(name_raw))
        else:
            name = str(name_raw)

        if not name:
            name = f'Spell {spell_id}'

        icon = field_mapper.get_field_value(spell_data, 'IconResRef', 'io_unknown')

        school_id_raw = field_mapper.get_field_value(spell_data, 'School', 0)
        school_id = None
        school_name = None

        if school_id_raw not in [None, '', '****', 0, '0']:
            school_letter = str(school_id_raw).upper().strip()
            if school_letter in self.SCHOOL_LETTER_MAP:
                school_id = self.SCHOOL_LETTER_MAP[school_letter]
            else:
                try:
                    school_id = int(school_id_raw)
                except (ValueError, TypeError):
                    school_id = 0

            if school_id is not None:
                school_data = self.rules_service.get_by_id('spellschools', school_id)
                if school_data:
                    school_name = field_mapper.get_field_value(school_data, 'Label', None)

        # Get additional spell details (same as get_available_spells)
        spell_desc = field_mapper.get_field_value(spell_data, 'SpellDesc', '')
        spell_range = field_mapper.get_field_value(spell_data, 'Range', '')
        cast_time = field_mapper.get_field_value(spell_data, 'CastTime', '')
        conj_time = field_mapper.get_field_value(spell_data, 'ConjTime', '')
        components = field_mapper.get_field_value(spell_data, 'VS', '')
        available_metamagic = field_mapper.get_field_value(spell_data, 'MetaMagic', '')
        target_type = field_mapper.get_field_value(spell_data, 'TargetType', '')

        details = {
            'id': spell_id,
            'name': name,
            'icon': icon,
            'school_id': school_id,
            'school_name': school_name,
            'description': spell_desc,
            'range': spell_range,
            'cast_time': cast_time,
            'conjuration_time': conj_time,
            'components': components,
            'available_metamagic': available_metamagic,
            'target_type': target_type
        }

        self._spell_data_cache[spell_id] = details
        return details

    
    def _register_event_handlers(self):
        self.character_manager.on(EventType.CLASS_CHANGED, self.on_class_changed)
        self.character_manager.on(EventType.LEVEL_GAINED, self.on_level_gained)
        self.character_manager.on(EventType.FEAT_ADDED, self.on_feat_added)
        self.character_manager.on(EventType.FEAT_REMOVED, self.on_feat_removed)
        self.character_manager.on(EventType.DOMAIN_ADDED, self.on_domain_added)
        self.character_manager.on(EventType.DOMAIN_REMOVED, self.on_domain_removed)

    def on_class_changed(self, event: ClassChangedEvent):
        new_class = self.rules_service.get_by_id('classes', event.new_class_id)
        if not self.is_spellcaster(class_data=new_class):
            return

        self._update_spell_slots()
        self._update_known_spells(event.new_class_id, event.level)

        if event.level == 1:
            self._grant_initial_spellbook(event.new_class_id)

        self.emit(EventType.SPELLS_CHANGED, {'source': 'class_change', 'class_id': event.new_class_id})

    def on_level_gained(self, event: LevelGainedEvent):
        class_data = self.rules_service.get_by_id('classes', event.class_id)
        if not self.is_spellcaster(class_data=class_data):
            return

        self._update_spell_slots()
        self._handle_level_up_spells(event.class_id, event.class_level_gained)
        self.emit(EventType.SPELLS_CHANGED, {'source': 'level_gain', 'class_id': event.class_id, 'level': event.new_level})

    def on_feat_added(self, event: FeatChangedEvent):
        feat_id = event.feat_id

        if self._is_metamagic_feat(feat_id):
            self.emit(EventType.SPELLS_CHANGED, {'source': 'metamagic_added', 'feat_id': feat_id})

        feat_data = self.rules_service.get_by_id('feat', feat_id)
        if feat_data and self._is_spell_related_feat(feat_data):
            self._update_spell_slots()
            self.emit(EventType.SPELLS_CHANGED, {'source': 'spell_feat_added', 'feat_id': feat_id})

    def on_feat_removed(self, event: FeatChangedEvent):
        if self._is_metamagic_feat(event.feat_id):
            self.emit(EventType.SPELLS_CHANGED, {'source': 'metamagic_removed', 'feat_id': event.feat_id})

    def on_domain_added(self, event: DomainChangedEvent):
        self._domain_spells_cache.clear()
        self._spell_slots_cache.clear()
        self.emit(EventType.SPELLS_CHANGED, {'source': 'domain_added', 'domain_id': event.domain_id, 'domain_name': event.domain_name})

    def on_domain_removed(self, event: DomainChangedEvent):
        self._domain_spells_cache.clear()
        self._spell_slots_cache.clear()

        class_list = self.gff.get('ClassList', [])
        for class_entry in class_list:
            class_id = class_entry.get('Class')
            class_data = self.rules_service.get_by_id('classes', class_id)
            if self._is_divine_caster(class_data):
                domain_spells_by_level = self._get_domain_spells_for_domain(event.domain_id)

                for spell_level, spell_ids in domain_spells_by_level.items():
                    memorized_list = class_entry.get(f'MemorizedList{spell_level}', [])
                    memorized_filtered = [
                        spell_entry for spell_entry in memorized_list
                        if not (spell_entry.get('SpellDomain', 0) == 1 and
                               spell_entry.get('Spell', -1) in spell_ids)
                    ]

                    if len(memorized_filtered) < len(memorized_list):
                        class_entry[f'MemorizedList{spell_level}'] = memorized_filtered

        self.gff.set('ClassList', class_list)

        self.emit(EventType.SPELLS_CHANGED, {
            'source': 'domain_removed',
            'domain_id': event.domain_id,
            'domain_name': event.domain_name
        })

    def _get_domain_spells_for_domain(self, domain_id: int) -> Dict[int, List[int]]:
        domain_spells = defaultdict(list)

        domain_data = self.rules_service.get_by_id('domains', domain_id)
        if not domain_data:
            return dict(domain_spells)

        for spell_level in range(1, self.MAX_SPELL_LEVEL + 1):
            field_name = f'Level_{spell_level}'
            spell_id = field_mapper.get_field_value(domain_data, field_name, -1)

            if isinstance(spell_id, str) and spell_id.isdigit():
                spell_id = int(spell_id)

            if spell_id >= 0:
                domain_spells[spell_level].append(spell_id)

        return dict(domain_spells)

    def calculate_spell_slots(self) -> Dict[int, Dict[int, int]]:
        """Calculate total spell slots per day for all classes and levels."""
        slots_by_class = {}
        class_list = self.gff.get('ClassList', [])

        for class_entry in class_list:
            class_id = class_entry.get('Class', -1)
            class_level = class_entry.get('ClassLevel', 0)

            if class_level == 0:
                continue

            class_data = self.rules_service.get_by_id('classes', class_id)
            if not class_data:
                raise ValueError(f"Class ID {class_id} not found in classes.2da")

            if not self.is_spellcaster(class_data=class_data):
                continue

            class_slots = self._calculate_class_spell_slots(class_data, class_level, class_entry)

            if class_slots:
                slots_by_class[class_id] = class_slots

        return slots_by_class
    
    def _calculate_class_spell_slots(self, class_data: Any, level: int, class_entry: Dict) -> Dict[int, int]:
        """Calculate spell slots, respects SpellCasterLevel override for PrC progression."""
        slots = {}

        spell_table_name = field_mapper.get_field_value(class_data, 'SpellGainTable', '')
        if not spell_table_name or spell_table_name == '****':
            return slots

        spell_table = self._get_spell_table(spell_table_name.lower())
        if not spell_table:
            return slots

        eff_level = level
        if class_entry and isinstance(class_entry, dict):
            scl = class_entry.get('SpellCasterLevel')
            if scl is not None:
                try:
                    scl_int = int(scl)
                    if scl_int > 0:
                        eff_level = scl_int
                except (ValueError, TypeError):
                    pass

        if 0 <= eff_level - 1 < len(spell_table):
            table_row = spell_table[eff_level - 1]
            for spell_level in range(self.MAX_SPELL_LEVEL + 1):
                field_name = f'SpellLevel{spell_level}'
                base_slots = self._safe_int(field_mapper.get_field_value(table_row, field_name, 0))
                if base_slots > 0:
                    bonus_slots = self._calculate_bonus_spell_slots(class_data, spell_level)
                    slots[spell_level] = base_slots + bonus_slots

        if self._is_divine_caster(class_data):
            self._add_domain_spell_slots(slots, class_entry)
        
        return slots
    
    def _calculate_bonus_spell_slots(self, class_data: Any, spell_level: int) -> int:
        """Calculate bonus spell slots from high ability scores (D&D 3.5 formula)."""
        if spell_level == 0:
            return 0

        casting_ability = self._get_casting_ability(class_data)
        if not casting_ability:
            return 0

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
        """Get the primary casting ability for a class, normalized to GFF field name."""
        ability_to_gff = {
            'str': 'Str', 'strength': 'Str', 'dex': 'Dex', 'dexterity': 'Dex',
            'con': 'Con', 'constitution': 'Con', 'int': 'Int', 'intelligence': 'Int',
            'wis': 'Wis', 'wisdom': 'Wis', 'cha': 'Cha', 'charisma': 'Cha',
        }

        ability = field_mapper.get_field_value(class_data, 'PrimaryAbil', '')
        if not ability or ability == '****':
            ability = field_mapper.get_field_value(class_data, 'SpellAbility', '')
        if not ability or ability == '****':
            ability = field_mapper.get_field_value(class_data, 'SpellcastingAbil', '')
        if not ability or ability == '****':
            return None

        ability_lower = ability.lower().strip()
        gff_field = ability_to_gff.get(ability_lower)
        if gff_field:
            return gff_field

        if ability in ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']:
            return ability

        return None
    
    def _is_divine_caster(self, class_data: Any) -> bool:
        return (
            field_mapper.get_field_value(class_data, 'HasDomains', '') == '1' or
            field_mapper.get_field_value(class_data, 'MaxDomains', '0') != '0'
        )

    def _add_domain_spell_slots(self, slots: Dict[int, int], class_entry: Dict):
        if class_entry.get('Domain1', -1) >= 0 or class_entry.get('Domain2', -1) >= 0:
            for spell_level in range(1, self.MAX_SPELL_LEVEL + 1):
                if spell_level in slots and slots[spell_level] > 0:
                    slots[spell_level] += 1
    
    def get_known_spells(self, class_id: int) -> Dict[int, List[int]]:
        """Get known spells for a class."""
        known_spells = defaultdict(list)

        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            raise ValueError(f"Class ID {class_id} not found in classes.2da")

        class_list = self.gff.get('ClassList', [])
        class_entry = None
        for entry in class_list:
            if entry.get('Class') == class_id:
                class_entry = entry
                break

        if not class_entry:
            raise ValueError(f"Class ID {class_id} not found in character's ClassList")

        for spell_level in range(self.MAX_SPELL_LEVEL + 1):
            known_list = class_entry.get(f'KnownList{spell_level}', [])
            class_spells = []
            for spell_entry in known_list:
                spell_id = spell_entry.get('Spell', -1)
                if spell_id >= 0:
                    class_spells.append(spell_id)
            if class_spells:
                known_spells[spell_level] = class_spells

        return dict(known_spells)

    def uses_all_spells_known(self, class_id: int) -> bool:
        """Check if class gets all spells from spells.2da (AllSpellsKnown=1) vs KnownList."""
        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return False
        all_spells_known = field_mapper.get_field_value(class_data, 'AllSpellsKnown', '0')
        return str(all_spells_known) == '1'

    def get_max_castable_spell_level(self, class_id: int) -> int:
        """Get maximum spell level a character can cast for a class, -1 if not a caster."""
        slots = self.calculate_spell_slots()
        class_slots = slots.get(class_id, {})
        if not class_slots:
            return -1
        return max(class_slots.keys())

    def get_character_spells_for_class(self, class_id: int) -> List[Dict[str, Any]]:
        """Get all spells the character knows for a specific class."""
        max_level = self.get_max_castable_spell_level(class_id)
        if max_level < 0:
            return []

        spells = []
        known_spell_ids = set()

        if self.uses_all_spells_known(class_id):
            master_cache = self._get_master_spell_cache()
            for spell_id, spell_data in master_cache.items():
                class_levels = spell_data.get('class_levels', {})
                if class_id in class_levels:
                    spell_level = class_levels[class_id]
                    if spell_level <= max_level:
                        known_spell_ids.add(spell_id)
                        spells.append({
                            'spell_id': spell_id,
                            'level': spell_level,
                            'name': spell_data['name'],
                            'icon': spell_data.get('icon', 'io_unknown'),
                            'school_name': spell_data.get('school_name'),
                            'description': spell_data.get('description'),
                            'class_id': class_id,
                            'is_domain_spell': False,
                        })

            domain_spells = self.get_domain_spells(class_id)
            for level, spell_ids in domain_spells.items():
                if level <= max_level:
                    for spell_id in spell_ids:
                        if spell_id not in known_spell_ids:
                            spell_details = self.get_spell_details(spell_id)
                            if spell_details:
                                known_spell_ids.add(spell_id)
                                spells.append({
                                    'spell_id': spell_id,
                                    'level': level,
                                    'name': spell_details['name'],
                                    'icon': spell_details.get('icon', 'io_unknown'),
                                    'school_name': spell_details.get('school_name'),
                                    'description': spell_details.get('description'),
                                    'class_id': class_id,
                                    'is_domain_spell': True,
                                })
        else:
            known_by_level = self.get_known_spells(class_id)
            for level, spell_ids in known_by_level.items():
                for spell_id in spell_ids:
                    spell_details = self.get_spell_details(spell_id)
                    spells.append({
                        'spell_id': spell_id,
                        'level': level,
                        'name': spell_details['name'],
                        'icon': spell_details.get('icon', 'io_unknown'),
                        'school_name': spell_details.get('school_name'),
                        'description': spell_details.get('description'),
                        'class_id': class_id,
                        'is_domain_spell': False,
                    })

        return spells

    def get_all_character_spells(self) -> List[Dict[str, Any]]:
        """Get all spells the character knows across all spellcasting classes."""
        all_spells = []
        class_list = self.gff.get('ClassList', [])
        for class_entry in class_list:
            class_id = class_entry.get('Class', -1)
            if self.is_spellcaster(class_id):
                class_spells = self.get_character_spells_for_class(class_id)
                all_spells.extend(class_spells)
        return all_spells

    def get_memorized_spells(self, class_id: int) -> Dict[int, List[Dict[str, Any]]]:
        """Get memorized/prepared spells for a class."""
        memorized_spells = defaultdict(list)

        class_list = self.gff.get('ClassList', [])
        class_entry = None
        for entry in class_list:
            if entry.get('Class') == class_id:
                class_entry = entry
                break

        if not class_entry:
            raise ValueError(f"Class ID {class_id} not found in character's ClassList")

        for spell_level in range(self.MAX_SPELL_LEVEL + 1):
            memorized_list = class_entry.get(f'MemorizedList{spell_level}', [])
            class_spells = []
            for spell_entry in memorized_list:
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
        """Get domain spells for a divine caster."""
        domain_spells = defaultdict(list)

        class_list = self.gff.get('ClassList', [])
        class_entry = None
        for entry in class_list:
            if entry.get('Class') == class_id:
                class_entry = entry
                break

        if not class_entry:
            return dict(domain_spells)

        domain1 = class_entry.get('Domain1', -1)
        domain2 = class_entry.get('Domain2', -1)

        for domain_id in [domain1, domain2]:
            if domain_id < 0:
                continue
            domain_data = self.rules_service.get_by_id('domains', domain_id)
            if not domain_data:
                continue

            for spell_level in range(1, self.MAX_SPELL_LEVEL + 1):
                field_name = f'Level_{spell_level}'
                spell_id = field_mapper.get_field_value(domain_data, field_name, -1)
                if isinstance(spell_id, str) and spell_id.isdigit():
                    spell_id = int(spell_id)
                if spell_id >= 0 and spell_id not in domain_spells[spell_level]:
                    domain_spells[spell_level].append(spell_id)

        return dict(domain_spells)
    
    def prepare_spell(self, class_id: int, spell_level: int, spell_id: int,
                      metamagic: int = 0, domain: bool = False) -> bool:
        """Prepare a spell for casting."""
        spell_data = self.rules_service.get_by_id('spells', spell_id)
        if not spell_data:
            raise ValueError(f"Spell ID {spell_id} not found in spells.2da")

        class_list = self.gff.get('ClassList', [])
        class_entry = None
        for entry in class_list:
            if entry.get('Class') == class_id:
                class_entry = entry
                break

        if not class_entry:
            raise ValueError(f"Class ID {class_id} not found in character's ClassList")

        # Add to memorized list (no slot restrictions - let users memorize as many as they want)
        memorized_list = class_entry.get(f'MemorizedList{spell_level}', [])
        memorized_list.append({
            'Spell': spell_id,
            'Ready': 1,
            'SpellMetaMagicN2': metamagic,
            'SpellClass': class_id,
            'SpellDomain': 1 if domain else 0
        })

        class_entry[f'MemorizedList{spell_level}'] = memorized_list
        self.gff.set('ClassList', class_list)

        return True

    def clear_memorized_spells(self, class_id: int, spell_level: Optional[int] = None):
        """Clear memorized spells for a class, optionally for a specific level."""
        class_list = self.gff.get('ClassList', [])
        class_entry = None
        for entry in class_list:
            if entry.get('Class') == class_id:
                class_entry = entry
                break

        if not class_entry:
            raise ValueError(f"Class ID {class_id} not found in character's ClassList")

        if spell_level is not None:
            class_entry[f'MemorizedList{spell_level}'] = []
        else:
            for level in range(self.MAX_SPELL_LEVEL + 1):
                class_entry[f'MemorizedList{level}'] = []

        self.gff.set('ClassList', class_list)
    
    def add_known_spell(self, class_id: int, spell_level: int, spell_id: int) -> bool:
        """Add a spell to the known spell list, returns False if already known."""
        class_list = self.gff.get('ClassList', [])
        class_entry = None
        for entry in class_list:
            if entry.get('Class') == class_id:
                class_entry = entry
                break

        if not class_entry:
            raise ValueError(f"Class ID {class_id} not found in character's ClassList")

        known_list = class_entry.get(f'KnownList{spell_level}', [])

        for spell in known_list:
            if spell.get('Spell') == spell_id:
                return False

        known_list.append({
            'Spell': spell_id,
            'SpellClass': class_id
        })

        class_entry[f'KnownList{spell_level}'] = known_list
        self.gff.set('ClassList', class_list)

        self.character_manager.emit(SpellChangedEvent(
            event_type=EventType.SPELL_LEARNED,
            source_manager='spell',
            timestamp=time.time(),
            spell_id=spell_id,
            spell_level=spell_level,
            action='learned',
            source='manual'
        ))

        return True

    def remove_known_spell(self, class_id: int, spell_level: int, spell_id: int) -> bool:
        """Remove a spell from the known spell list, returns False if not found."""
        class_list = self.gff.get('ClassList', [])
        class_entry = None
        for entry in class_list:
            if entry.get('Class') == class_id:
                class_entry = entry
                break

        if not class_entry:
            raise ValueError(f"Class ID {class_id} not found in character's ClassList")

        known_list = class_entry.get(f'KnownList{spell_level}', [])

        original_length = len(known_list)
        known_list = [
            spell for spell in known_list
            if spell.get('Spell') != spell_id
        ]

        if len(known_list) < original_length:
            class_entry[f'KnownList{spell_level}'] = known_list
            self.gff.set('ClassList', class_list)

            self.character_manager.emit(SpellChangedEvent(
                event_type=EventType.SPELL_FORGOTTEN,
                source_manager='spell',
                timestamp=time.time(),
                spell_id=spell_id,
                spell_level=spell_level,
                action='forgotten',
                source='manual'
            ))

            return True

        return False
    
    def get_spell_level_for_class(self, spell_id: int, class_id: int) -> Optional[int]:
        """Get the spell level for a specific spell and class, None if not available for class."""
        master_cache = self._get_master_spell_cache()
        spell_data = master_cache.get(spell_id)
        if not spell_data:
            raise ValueError(f"Spell ID {spell_id} not found in spells.2da")

        class_levels = spell_data.get('class_levels', {})
        return class_levels.get(class_id)
    
    def is_spellcaster(self, class_id: Optional[int] = None, class_data: Any = None) -> bool:
        """Check if a class can cast spells. Pass either class_id or class_data."""
        if class_data is None:
            if class_id is None:
                return False
            class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return False

        is_caster = field_mapper.get_field_value(class_data, 'SpellCaster', '0')
        if isinstance(is_caster, str):
            is_caster = is_caster != '0' and is_caster != '****'
        if is_caster:
            return True

        spell_table = field_mapper.get_field_value(class_data, 'SpellGainTable', '')
        return bool(spell_table and spell_table != '****')
    
    def get_class_name(self, class_id: int) -> str:
        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return f'Unknown Class ({class_id})'
        return field_mapper.get_field_value(class_data, 'Label', f'Class_{class_id}')

    def get_caster_level(self, class_index: int) -> int:
        """Get caster level accounting for SpellCaster progression type (1=full, 2=level-3, 3=half)."""
        class_list = self.gff.get('ClassList', [])
        if class_index >= len(class_list):
            return 0

        class_entry = class_list[class_index]
        class_id = class_entry.get('Class', -1)
        class_level = class_entry.get('ClassLevel', 0)

        class_data = self.rules_service.get_by_id('classes', class_id)
        if class_data:
            spell_caster_type_str = field_mapper.get_field_value(class_data, 'SpellCaster', '1')
            try:
                spell_caster_type = int(spell_caster_type_str)
                if spell_caster_type == 2:
                    return max(0, class_level - 3)
                elif spell_caster_type == 3:
                    return class_level // 2
                elif spell_caster_type == 4:
                    return class_level
            except (ValueError, TypeError):
                pass

        return class_level
    
    def is_prepared_caster(self, class_id: Optional[int] = None, class_data: Any = None) -> bool:
        """Check if a class prepares spells (vs spontaneous). Pass either class_id or class_data."""
        if class_data is None:
            if class_id is None:
                return False
            class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return False

        memorizes_spells = field_mapper.get_field_value(class_data, 'MemorizesSpells', None)
        if memorizes_spells is not None and memorizes_spells != '****':
            try:
                return bool(int(memorizes_spells))
            except (ValueError, TypeError):
                pass

        spell_known_table = field_mapper.get_field_value(class_data, 'SpellKnownTable', '')
        if spell_known_table and spell_known_table != '****':
            return False

        return self.is_spellcaster(class_data=class_data)
    
    def get_all_memorized_spells(self) -> List[Dict[str, Any]]:
        """Get all memorized spells for all classes."""
        memorized = []
        class_list = self.gff.get('ClassList', [])
        for class_entry in class_list:
            class_id = class_entry.get('Class', -1)
            for spell_level in range(self.MAX_SPELL_LEVEL + 1):
                mem_list = class_entry.get(f'MemorizedList{spell_level}', [])
                for spell in mem_list:
                    if not isinstance(spell, dict):
                        continue
                    memorized.append({
                        'level': spell_level,
                        'spell_id': spell.get('Spell'),
                        'class_id': class_id,
                        'metamagic': spell.get('SpellMetaMagic', 0),
                        'ready': spell.get('Ready', 1) == 1
                    })
        return memorized

    def get_metamagic_feats(self) -> List[int]:
        metamagic = []
        feat_manager = self.character_manager.get_manager('feat')
        if not feat_manager:
            return metamagic
        feat_list = self.gff.get('FeatList', [])
        for feat in feat_list:
            feat_id = feat.get('Feat')
            if feat_id and self._is_metamagic_feat(feat_id):
                metamagic.append(feat_id)
        return metamagic

    def calculate_metamagic_cost(self, metamagic_flags: int) -> int:
        """Calculate total spell level adjustment for metamagic flags bitmask."""
        total_cost = 0
        metamagic_feats = self.get_metamagic_feats()
        for feat_id in metamagic_feats:
            if self._has_metamagic_flag(metamagic_flags, feat_id):
                cost = self._get_metamagic_level_cost(feat_id)
                total_cost += cost
        return total_cost

    def _has_metamagic_flag(self, flags: int, feat_id: int) -> bool:
        feat_data = self.rules_service.get_by_id('feat', feat_id)
        if not feat_data:
            return False
        metamagic_bit = field_mapper.get_field_value(feat_data, 'MetamagicBit', 0)
        try:
            bit_value = int(metamagic_bit) if metamagic_bit else 0
            return (flags & bit_value) != 0
        except (ValueError, TypeError):
            return False

    def _is_spell_related_feat(self, feat_data: Any) -> bool:
        if not feat_data:
            return False
        feat_label = field_mapper.get_field_value(feat_data, 'Label', '')
        if not feat_label or feat_label == '****':
            feat_label = field_mapper.get_field_value(feat_data, 'Name', '')
        if hasattr(feat_label, '_mock_name'):
            return False
        feat_label = str(feat_label).lower()
        return any(kw in feat_label for kw in ['spell', 'slot', 'domain', 'school', 'casting'])

    def _get_spell_table(self, table_name: str) -> Optional[Any]:
        if table_name in self._spell_table_cache:
            return self._spell_table_cache[table_name]
        table = self.rules_service.get_table(table_name)
        if table:
            self._spell_table_cache[table_name] = table
        return table

    def _update_spell_slots(self):
        self._spell_slots_cache.clear()
        self.calculate_spell_slots()
    
    def _update_known_spells(self, class_id: int, level: int):
        pass

    def _handle_level_up_spells(self, class_id: int, new_level: int):
        self._update_known_spells(class_id, new_level)
        if new_level == 1:
            self._grant_initial_spellbook(class_id)
        class_data = self.rules_service.get_by_id('classes', class_id)
        if class_data and self.is_prepared_caster(class_data=class_data):
            self.emit(EventType.SPELLS_CHANGED, {'source': 'need_preparation', 'class_id': class_id})

    def _grant_initial_spellbook(self, class_id: int):
        """Grant cantrips to book casters (Wizard) - not divine or spontaneous casters."""
        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return
        if self.uses_all_spells_known(class_id):
            return
        if not self.is_prepared_caster(class_data=class_data):
            return
        if not self.is_spellcaster(class_data=class_data):
            return

        cantrips = self.get_available_spells(spell_level=0, class_id=class_id)
        for spell in cantrips:
            self.add_known_spell(class_id, 0, spell['id'])

    def _safe_int(self, value: Any, default: int = 0) -> int:
        if value is None or value == '****':
            return default
        try:
            return int(value)
        except (ValueError, TypeError):
            return default

    def validate(self) -> Tuple[bool, List[str]]:
        """Validate spell configuration for data corruption prevention."""
        errors = []
        slots_by_class = self.calculate_spell_slots()

        for class_id, slots in slots_by_class.items():
            class_data = self.rules_service.get_by_id('classes', class_id)
            if not class_data:
                errors.append(f"Invalid class ID: {class_id}")
                continue

            memorized = self.get_memorized_spells(class_id)
            for spell_level, spell_list in memorized.items():
                for spell_entry in spell_list:
                    spell_id = spell_entry.get('spell_id', -1)
                    if spell_id >= 0:
                        spell_data = self.rules_service.get_by_id('spells', spell_id)
                        if not spell_data:
                            errors.append(f"Invalid spell ID {spell_id} found in memorized spells")

            known = self.get_known_spells(class_id)
            for spell_level, spell_list in known.items():
                for spell_id in spell_list:
                    if spell_id >= 0:
                        spell_data = self.rules_service.get_by_id('spells', spell_id)
                        if not spell_data:
                            errors.append(f"Invalid spell ID {spell_id} found in known spells")

        return len(errors) == 0, errors

    def get_spell_summary(self) -> Dict[str, Any]:
        """Get summary of character's spellcasting abilities."""
        summary = {'caster_classes': [], 'total_spell_levels': 0, 'metamagic_feats': []}
        slots_by_class = self.calculate_spell_slots()

        for class_id, slots in slots_by_class.items():
            class_data = self.rules_service.get_by_id('classes', class_id)
            class_name = field_mapper.get_field_value(class_data, 'Label', f'Unknown Class {class_id}')
            total_slots = sum(slots.values())
            max_spell_level = max(slots.keys()) if slots else 0
            summary['caster_classes'].append({
                'id': class_id, 'name': class_name, 'total_slots': total_slots,
                'max_spell_level': max_spell_level, 'slots_by_level': slots
            })
            summary['total_spell_levels'] += total_slots

        for feat_id in self.get_metamagic_feats():
            feat_data = self.rules_service.get_by_id('feat', feat_id)
            feat_name = field_mapper.get_field_value(feat_data, 'Label', f'Unknown Feat {feat_id}')
            level_cost = self._get_metamagic_level_cost(feat_id)
            summary['metamagic_feats'].append({'id': feat_id, 'name': feat_name, 'level_cost': level_cost})

        return summary

    def get_spells_state_summary(self, include_available: bool = False) -> Dict[str, Any]:
        """Aggregate spell state summary for the UI."""
        # Get spellcasting classes
        spellcasting_classes = []
        for idx, class_info in enumerate(self.gff.get('ClassList', [])):
            class_id = class_info.get('Class', -1)
            class_level = class_info.get('ClassLevel', 0)
            if self.is_spellcaster(class_id):
                can_edit = not self.uses_all_spells_known(class_id)
                spellcasting_classes.append({
                    'index': idx,
                    'class_id': class_id,
                    'class_name': self.get_class_name(class_id),
                    'class_level': class_level,
                    'caster_level': self.get_caster_level(idx),
                    'spell_type': 'prepared' if self.is_prepared_caster(class_id) else 'spontaneous',
                    'can_edit_spells': can_edit
                })
        
        # Get spell summary
        spell_summary = self.get_spell_summary()
        
        # Get memorized spells (for prepared casters - basic info)
        memorized_data = self.get_all_memorized_spells() if spellcasting_classes else []

        memorized_spells = []
        for spell in memorized_data:
            spell_details = self.get_spell_details(spell['spell_id'])
            memorized_spells.append({
                'level': spell['level'],
                'spell_id': spell['spell_id'],
                'name': spell_details['name'],
                'icon': spell_details['icon'],
                'school_name': spell_details.get('school_name'),
                'description': spell_details.get('description'),
                'class_id': spell['class_id'],
                'metamagic': spell.get('metamagic', 0),
                'ready': spell.get('ready', True)
            })

        # Get known spells
        known_spells = []
        if spellcasting_classes:
            all_character_spells = self.get_all_character_spells()
            for spell in all_character_spells:
                known_spells.append({
                    'level': spell['level'],
                    'spell_id': spell['spell_id'],
                    'name': spell['name'],
                    'icon': spell['icon'],
                    'school_name': spell.get('school_name'),
                    'description': spell.get('description'),
                    'class_id': spell['class_id'],
                    'is_domain_spell': spell.get('is_domain_spell', False)
                })
        
        # Get available spells by level if requested (expensive operation)
        available_by_level = None
        if include_available and spellcasting_classes:
            available_by_level = {}
            for level in range(10):
                spells_data = self.get_available_spells(level)
                available_by_level[level] = spells_data
        
        return {
            'spellcasting_classes': spellcasting_classes,
            'spell_summary': spell_summary,
            'memorized_spells': memorized_spells,
            'known_spells': known_spells,
            'available_by_level': available_by_level
        }

    def manage_spell(self, action: str, spell_id: int, class_index: int, spell_level: Optional[int] = None) -> Tuple[bool, str]:
        """Add or remove spell, handling validation and level lookup."""
        # Get class ID from index
        class_list = self.gff.get('ClassList', [])
        if class_index >= len(class_list):
             return False, f"Invalid class index: {class_index}"
             
        class_id = class_list[class_index].get('Class', -1)
        
        if not self.is_spellcaster(class_id):
            return False, "Selected class cannot cast spells"

        if self.uses_all_spells_known(class_id):
            class_name = self.get_class_name(class_id)
            return False, f"{class_name} spells cannot be modified - this class automatically knows all spells"
        
        if action == 'add':
            # Determine spell level if not provided
            if spell_level is None:
                spell_level = self.get_spell_level_for_class(spell_id, class_id)
                if spell_level is None:
                    return False, "Could not determine spell level for this class"
            
            added = self.add_known_spell(class_id, spell_level, spell_id)
            if not added:
                 return False, "Spell is already known"
            return True, "Spell added successfully"
            
        elif action == 'remove':
            # Determine spell level if not provided
            if spell_level is None:
                spell_level = self.get_spell_level_for_class(spell_id, class_id)
                if spell_level is None:
                    return False, "Could not determine spell level for this class"
            
            removed = self.remove_known_spell(class_id, spell_level, spell_id)
            if not removed:
                 return False, "Spell not found in known spell list"
            return True, "Spell removed successfully"
            
        else:
             return False, f"Unsupported action: {action}"
    
    def get_available_spells(self, spell_level: int, class_id: Optional[int] = None) -> List[Dict[str, Any]]:
        """Get all spells available at a specific level, optionally filtered by class."""
        master_cache = self._get_master_spell_cache()
        column_to_classes = self._get_column_to_class_id_map()
        class_id_to_label = {}
        for col, class_ids in column_to_classes.items():
            for cid in class_ids:
                class_id_to_label[cid] = col.lower()

        result = []
        for spell_id, spell_data in master_cache.items():
            class_levels = spell_data.get('class_levels', {})

            if class_id is not None:
                if class_id in class_levels and class_levels[class_id] == spell_level:
                    result.append({
                        **spell_data,
                        'level': spell_level,
                        'available_classes': [class_id_to_label.get(class_id, str(class_id))]
                    })
            else:
                matching_classes = list(set(
                    class_id_to_label.get(cid, str(cid))
                    for cid, lvl in class_levels.items()
                    if lvl == spell_level
                ))
                if matching_classes:
                    result.append({
                        **spell_data,
                        'level': spell_level,
                        'available_classes': matching_classes
                    })

        return result
    
    def get_legitimate_spells(
        self,
        levels: Optional[List[int]] = None,
        schools: Optional[List[str]] = None,
        search: Optional[str] = None,
        page: int = 1,
        limit: int = 50,
        class_id: Optional[int] = None
    ) -> Dict[str, Any]:
        """Get legitimate spells with pagination and filtering."""
        all_spells = self._get_all_legitimate_spells_cached()

        filtered = all_spells

        if levels:
            filtered = [s for s in filtered if s['level'] in levels]

        if schools:
            school_set = set(schools)
            filtered = [s for s in filtered if s.get('school_name') in school_set]

        if class_id is not None:
            filtered = [s for s in filtered if self._spell_available_to_class(s, class_id)]

        if search:
            search_lower = search.lower()
            filtered = [
                s for s in filtered
                if search_lower in s['name'].lower() or
                   (s.get('description') and search_lower in s['description'].lower())
            ]

        total = len(filtered)
        start_idx = (page - 1) * limit
        end_idx = start_idx + limit
        page_spells = filtered[start_idx:end_idx]

        return {
            'spells': page_spells,
            'pagination': {
                'page': page,
                'limit': limit,
                'total': total,
                'pages': (total + limit - 1) // limit if limit > 0 else 0,
                'has_next': end_idx < total,
                'has_previous': page > 1
            }
        }

    def _get_all_legitimate_spells_cached(self) -> List[Dict[str, Any]]:
        if self._legitimate_spells_cache is not None:
            return self._legitimate_spells_cache

        master_cache = self._get_master_spell_cache()
        column_to_classes = self._get_column_to_class_id_map()
        class_id_to_label = {}
        for col, class_ids in column_to_classes.items():
            for cid in class_ids:
                class_id_to_label[cid] = col.lower()

        all_spells = []
        for spell_id, spell_data in master_cache.items():
            class_levels = spell_data.get('class_levels', {})
            levels_to_classes: Dict[int, List[str]] = {}
            for class_id, level in class_levels.items():
                if level not in levels_to_classes:
                    levels_to_classes[level] = []
                label = class_id_to_label.get(class_id, str(class_id))
                if label not in levels_to_classes[level]:
                    levels_to_classes[level].append(label)

            for spell_level, available_classes in levels_to_classes.items():
                all_spells.append({
                    **spell_data,
                    'level': spell_level,
                    'available_classes': available_classes
                })

        self._legitimate_spells_cache = all_spells
        return all_spells

    def _spell_available_to_class(self, spell: Dict[str, Any], class_id: int) -> bool:
        return class_id in spell.get('class_levels', {})

    def _filter_legitimate_spells_with_indices(self, all_spells: List[Any]) -> List[Tuple[int, Any]]:
        """Filter out dev/test spells, keeping only legitimate player-usable spells."""
        legitimate_spells = []
        for spell_idx, spell in enumerate(all_spells):
            removed = field_mapper.get_field_value(spell, 'REMOVED', None)
            if removed == '1':
                continue
            user_type = field_mapper.get_field_value(spell, 'UserType', None)
            if user_type in ['4', '5'] or user_type is None:
                continue
            label = field_mapper.get_field_value(spell, 'Label', '') or ''
            if label.startswith('DELETED_') or label.startswith('DEL_'):
                continue
            name = field_mapper.get_field_value(spell, 'Name', None)
            if not name or name == 'None' or name.isdigit():
                continue
            legitimate_spells.append((spell_idx, spell))
        return legitimate_spells

    def _get_all_spell_columns(self) -> Dict[str, str]:
        if hasattr(self, '_spell_columns_cache') and self._spell_columns_cache:
            return self._spell_columns_cache
        spell_columns = {}
        all_classes = self.rules_service.get_table('classes')
        if all_classes:
            for cls in all_classes:
                spell_col = field_mapper.get_field_value(cls, 'SpellTableColumn', '')
                if spell_col and spell_col != '****' and spell_col not in spell_columns:
                    class_label = field_mapper.get_field_value(cls, 'Label', spell_col)
                    spell_columns[spell_col] = class_label.lower().replace('_', ' ')
        self._spell_columns_cache = spell_columns
        return spell_columns

    def _get_column_to_class_id_map(self) -> Dict[str, List[int]]:
        if self._column_to_class_id_cache is not None:
            return self._column_to_class_id_cache

        spell_columns = self._get_spell_columns_from_spells_table()
        all_classes = self.rules_service.get_table('classes')
        column_to_classes: Dict[str, List[int]] = {col: [] for col in spell_columns}

        if all_classes:
            for class_id, cls in enumerate(all_classes):
                if not self.is_spellcaster(class_data=cls):
                    continue
                label = field_mapper.get_field_value(cls, 'Label', '').lower()
                if not label:
                    continue

                matched = False
                if 'wizard' in label or 'sorcerer' in label:
                    if 'Wiz_Sorc' in column_to_classes:
                        column_to_classes['Wiz_Sorc'].append(class_id)
                        matched = True
                elif 'spirit' in label and 'shaman' in label:
                    if 'Druid' in column_to_classes:
                        column_to_classes['Druid'].append(class_id)
                        matched = True
                elif 'favored' in label and 'soul' in label:
                    if 'Cleric' in column_to_classes:
                        column_to_classes['Cleric'].append(class_id)
                        matched = True

                if not matched:
                    for col in spell_columns:
                        if col.lower() == label or col.lower() in label:
                            column_to_classes[col].append(class_id)
                            break

        self._column_to_class_id_cache = column_to_classes
        return column_to_classes

    def _get_spell_columns_from_spells_table(self) -> List[str]:
        all_spells = self.rules_service.get_table('spells')
        if not all_spells or len(all_spells) == 0:
            return []

        all_keys = set()
        for spell in all_spells[:100]:
            if isinstance(spell, dict):
                all_keys.update(spell.keys())
            elif hasattr(spell, 'get_column_mapping'):
                all_keys.update(spell.get_column_mapping().keys())
                break
            elif hasattr(spell, '__slots__'):
                all_keys.update(s.lstrip('_') for s in spell.__slots__
                               if s.startswith('_') and s not in ['_resource_manager'])
                break

        return [k for k in all_keys if k not in self.KNOWN_NON_CLASS_FIELDS]

    def _get_master_spell_cache(self) -> Dict[int, Dict[str, Any]]:
        if self._master_spell_cache is not None:
            return self._master_spell_cache

        all_spells_raw = self.rules_service.get_table('spells')
        if not all_spells_raw:
            self._master_spell_cache = {}
            return {}

        legitimate = self._filter_legitimate_spells_with_indices(all_spells_raw)
        column_to_classes = self._get_column_to_class_id_map()

        cache = {}
        for spell_id, spell_data in legitimate:
            class_levels = {}
            for column_name, class_ids in column_to_classes.items():
                level_value = field_mapper.get_field_value(spell_data, column_name, -1)
                if level_value not in [None, '', '****', -1]:
                    try:
                        level_int = int(level_value)
                        if 0 <= level_int <= 9:
                            for class_id in class_ids:
                                class_levels[class_id] = level_int
                    except (ValueError, TypeError):
                        pass

            if not class_levels:
                continue

            name_raw = field_mapper.get_field_value(spell_data, 'Name', f'Spell_{spell_id}')
            if isinstance(name_raw, int):
                spell_name = self.rules_service._loader.get_string(name_raw)
            elif isinstance(name_raw, str) and name_raw.isdigit():
                spell_name = self.rules_service._loader.get_string(int(name_raw))
            else:
                spell_name = str(name_raw)
            if not spell_name:
                spell_name = f'Spell {spell_id}'

            school_id_raw = field_mapper.get_field_value(spell_data, 'School', 0)
            school_id = None
            school_name = None
            if school_id_raw not in [None, '', '****', 0]:
                school_letter = str(school_id_raw).upper().strip()
                if school_letter in self.SCHOOL_LETTER_MAP:
                    school_id = self.SCHOOL_LETTER_MAP[school_letter]
                else:
                    try:
                        school_id = int(school_id_raw)
                    except (ValueError, TypeError):
                        school_id = 0
                if school_id is not None:
                    school_data = self.rules_service.get_by_id('spellschools', school_id)
                    if school_data:
                        school_name = field_mapper.get_field_value(school_data, 'Label', None)

            cache[spell_id] = {
                'id': spell_id,
                'name': spell_name,
                'icon': field_mapper.get_field_value(spell_data, 'IconResRef', ''),
                'school_id': school_id,
                'school_name': school_name,
                'description': field_mapper.get_field_value(spell_data, 'SpellDesc', ''),
                'range': field_mapper.get_field_value(spell_data, 'Range', ''),
                'cast_time': field_mapper.get_field_value(spell_data, 'CastTime', ''),
                'conjuration_time': field_mapper.get_field_value(spell_data, 'ConjTime', ''),
                'components': field_mapper.get_field_value(spell_data, 'VS', ''),
                'available_metamagic': field_mapper.get_field_value(spell_data, 'MetaMagic', ''),
                'target_type': field_mapper.get_field_value(spell_data, 'TargetType', ''),
                'class_levels': class_levels
            }

        self._master_spell_cache = cache
        return cache

    def _class_matches_column(self, class_data: Any, column_name: str) -> bool:
        return field_mapper.get_field_value(class_data, 'SpellTableColumn', '') == column_name

    def _is_metamagic_feat(self, feat_id: int) -> bool:
        feat_data = self.rules_service.get_by_id('feat', feat_id)
        if not feat_data:
            return False
        feat_type = field_mapper.get_field_value(feat_data, 'FeatType', '')
        if feat_type == 'METAMAGIC':
            return True
        metamagic_bit = field_mapper.get_field_value(feat_data, 'MetamagicBit', 0)
        return metamagic_bit != 0 and metamagic_bit != '0'
    
    def _get_metamagic_level_cost(self, feat_id: int) -> int:
        """Get the spell level adjustment for a metamagic feat from feat.2da."""
        feat_data = self.rules_service.get_by_id('feat', feat_id)
        if not feat_data:
            raise ValueError(f"Feat ID {feat_id} not found in feat.2da")

        level_cost = field_mapper.get_field_value(feat_data, 'MetamagicLevelCost', None)
        if level_cost is not None and level_cost not in ('', '****'):
            try:
                return int(level_cost)
            except (ValueError, TypeError):
                pass

        level_cost = field_mapper.get_field_value(feat_data, 'SpellLevelCost', None)
        if level_cost is not None and level_cost not in ('', '****'):
            try:
                return int(level_cost)
            except (ValueError, TypeError):
                pass

        feat_label = field_mapper.get_field_value(feat_data, 'Label', f'Feat_{feat_id}')
        raise ValueError(f"Metamagic feat '{feat_label}' (ID {feat_id}) missing MetamagicLevelCost/SpellLevelCost in feat.2da")