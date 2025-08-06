"""
Data-Driven CharacterManager using DynamicGameDataLoader
All character data access is driven by 2DA files, not hardcoded mappings
"""

from typing import Dict, List, Any, Optional, Type, TypeVar, Union, overload, Callable, Set, TYPE_CHECKING
import copy
import time
import logging
from dataclasses import dataclass
from collections import defaultdict

from .events import EventEmitter, EventData
from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader
from gamedata.services.game_rules_service import GameRulesService
from gamedata.services.rule_detector import RuleDetector

if TYPE_CHECKING:
    from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader

logger = logging.getLogger(__name__)


@dataclass
class Transaction:
    """Represents a set of character changes that can be committed or rolled back"""
    id: str
    manager: 'CharacterManager'
    original_state: Dict[str, Any]
    changes: List[Dict[str, Any]]
    timestamp: float
    
    def __init__(self, manager: 'CharacterManager'):
        self.id = f"txn_{int(time.time() * 1000)}"
        self.manager = manager
        self.original_state = copy.deepcopy(manager.character_data)
        self.changes = []
        self.timestamp = time.time()
    
    def add_change(self, change_type: str, details: Dict[str, Any]):
        """Record a change in this transaction"""
        self.changes.append({
            'type': change_type,
            'details': details,
            'timestamp': time.time()
        })
    
    def rollback(self):
        """Restore character to state before transaction"""
        logger.info(f"Rolling back transaction {self.id}")
        self.manager.character_data = self.original_state
        # Also update the GFF wrapper to point to the restored data
        self.manager.gff = GFFDataWrapper(self.manager.character_data)
        self.manager._notify_managers('transaction_rollback', {'transaction_id': self.id})
    
    def commit(self) -> Dict[str, Any]:
        """Finalize the transaction and return summary"""
        logger.info(f"Committing transaction {self.id} with {len(self.changes)} changes")
        return {
            'transaction_id': self.id,
            'changes': self.changes,
            'duration': time.time() - self.timestamp
        }


T = TypeVar('T')


class LazyManagerProxy:
    """
    Proxy for lazy-loaded managers.
    Defers actual manager instantiation until first access.
    """
    
    def __init__(self, name: str, manager_class: Type, character_manager: 'CharacterManager'):
        self._name = name
        self._manager_class = manager_class
        self._character_manager = character_manager
        self._initialized = False
        self._instance = None
    
    def _initialize(self):
        """Initialize the actual manager instance"""
        if not self._initialized:
            logger.debug(f"Lazy-initializing {self._name} manager")
            self._instance = self._manager_class(self._character_manager)
            self._initialized = True
            
            # Replace proxy with real instance in parent's registry
            self._character_manager._managers[self._name] = self._instance
            
            # Call any pending lifecycle hooks
            if hasattr(self._character_manager, '_manager_hooks'):
                hooks = self._character_manager._manager_hooks.get(self._name, {})
                on_register = hooks.get('on_register')
                if on_register:
                    try:
                        on_register(self._instance)
                    except Exception as e:
                        logger.error(f"Error in on_register hook for {self._name}: {e}")
    
    def __getattr__(self, name):
        """Delegate attribute access to the real manager, initializing if needed"""
        self._initialize()
        return getattr(self._instance, name)
    
    def __setattr__(self, name, value):
        """Handle attribute setting"""
        if name.startswith('_'):
            # Internal attributes go to the proxy
            object.__setattr__(self, name, value)
        else:
            # External attributes trigger initialization and go to the real manager
            self._initialize()
            setattr(self._instance, name, value)


class GFFDataWrapper:
    """Wrapper around GFF data for easier access and modification"""
    
    def __init__(self, gff_data: Dict[str, Any]):
        self._data = gff_data
    
    @overload
    def get(self, path: str) -> Any: ...
    
    @overload
    def get(self, path: str, default: T) -> Union[Any, T]: ...
    
    def get(self, path: str, default: Optional[T] = None) -> Union[Any, T]:
        """
        Get value at path (e.g., 'ClassList.0.Class')
        
        Args:
            path: Dot-separated path to value
            default: Default value if path doesn't exist
            
        Returns:
            Value at path or default
        """
        parts = path.split('.')
        current = self._data
        
        for part in parts:
            if current is None:
                return default
                
            # Handle list indices
            if isinstance(current, list) and part.isdigit():
                idx = int(part)
                if 0 <= idx < len(current):
                    current = current[idx]
                else:
                    return default
            # Handle dict keys
            elif isinstance(current, dict):
                current = current.get(part, default)
            else:
                return default
                
        return current
    
    def set(self, path: str, value: Any) -> None:
        """
        Set value at path
        
        Args:
            path: Dot-separated path to value
            value: Value to set
        """
        parts = path.split('.')
        current = self._data
        
        # Navigate to parent
        for i, part in enumerate(parts[:-1]):
            if isinstance(current, list) and part.isdigit():
                idx = int(part)
                if 0 <= idx < len(current):
                    current = current[idx]
                else:
                    raise IndexError(f"Index {idx} out of range at {'.'.join(parts[:i+1])}")
            elif isinstance(current, dict):
                if part not in current:
                    # Create missing intermediate dicts
                    next_part = parts[i+1]
                    if next_part.isdigit():
                        current[part] = []
                    else:
                        current[part] = {}
                current = current[part]
            else:
                raise ValueError(f"Cannot navigate path at {'.'.join(parts[:i+1])}")
        
        # Set final value
        final_key = parts[-1]
        if isinstance(current, list) and final_key.isdigit():
            idx = int(final_key)
            if 0 <= idx < len(current):
                current[idx] = value
            else:
                raise IndexError(f"Index {idx} out of range")
        elif isinstance(current, dict):
            current[final_key] = value
        else:
            raise ValueError(f"Cannot set value at {path}")
    
    def get_typed(self, path: str, expected_type: Type[T], default: Optional[T] = None) -> T:
        """
        Get value at path with type checking
        
        Args:
            path: Dot-separated path to value
            expected_type: Expected type of the value
            default: Default value if path doesn't exist or type mismatch
            
        Returns:
            Value at path if it matches expected type, otherwise default
        """
        value = self.get(path, default)
        if isinstance(value, expected_type):
            return value
        return default if default is not None else expected_type()
    
    def get_list(self, path: str, default: Optional[List] = None) -> List[Any]:
        """Get a list value at path, ensuring it's a list"""
        value = self.get(path)
        if isinstance(value, list):
            return value
        return default if default is not None else []
    
    def get_dict(self, path: str, default: Optional[Dict] = None) -> Dict[str, Any]:
        """Get a dict value at path, ensuring it's a dict"""
        value = self.get(path)
        if isinstance(value, dict):
            return value
        return default if default is not None else {}
    
    @property
    def raw_data(self) -> Dict[str, Any]:
        """Get the raw GFF data"""
        return self._data


class CharacterManager(EventEmitter):
    """
    Data-Driven Character Manager
    Uses DynamicGameDataLoader for all character data structure understanding
    """
    
    def __init__(self, character_data: Dict[str, Any], game_data_loader: Optional['DynamicGameDataLoader'] = None, gff_element=None, rules_service: Optional[GameRulesService] = None):
        """
        Initialize the data-driven character manager
        
        Args:
            character_data: Raw GFF character data
            game_data_loader: DynamicGameDataLoader instance (creates new one if not provided)
            gff_element: Optional GFFElement for direct updates
            rules_service: Optional GameRulesService instance
        """
        super().__init__()
        self.character_data = character_data
        
        # Validate character data
        if not isinstance(character_data, dict):
            raise ValueError(f"character_data must be a dictionary, got {type(character_data)}")
        
        if not character_data:
            raise ValueError("character_data cannot be empty")
        
        # Use DirectGFFWrapper if gff_element is provided
        if gff_element:
            try:
                from .gff_direct_wrapper import DirectGFFWrapper
                self.gff = DirectGFFWrapper(gff_element)
                self.gff_element = gff_element
                logger.info("Using DirectGFFWrapper for character data access")
            except ImportError as e:
                logger.error(f"Failed to import DirectGFFWrapper: {e}")
                self.gff = GFFDataWrapper(character_data)
                self.gff_element = None
        else:
            self.gff = GFFDataWrapper(character_data)
            self.gff_element = None
            logger.info("Using GFFDataWrapper for character data access")
            
        # Use provided loader or get singleton instance - this is our source of truth
        try:
            from gamedata.dynamic_loader.singleton import wait_for_loader_ready
            
            # If no loader provided, get/create singleton and wait for it
            if game_data_loader is None:
                logger.info("Waiting for DynamicGameDataLoader to be ready...")
                if not wait_for_loader_ready(timeout=30.0):
                    raise RuntimeError("DynamicGameDataLoader initialization timed out after 30 seconds")
                self.game_data_loader = get_dynamic_game_data_loader()
            else:
                # Use provided loader, but still check if ready
                self.game_data_loader = game_data_loader
                if not self.game_data_loader.is_ready():
                    logger.info("Waiting for provided DynamicGameDataLoader to be ready...")
                    if not self.game_data_loader.wait_for_ready(timeout=30.0):
                        raise RuntimeError("Provided DynamicGameDataLoader not ready after 30 seconds")
            
            logger.info("DynamicGameDataLoader obtained successfully and ready")
        except Exception as e:
            logger.error(f"Failed to get DynamicGameDataLoader: {e}")
            raise RuntimeError(f"Could not get game data loader: {e}")
        
        # Initialize rules service
        try:
            self.rules_service = rules_service or GameRulesService()
            logger.info("GameRulesService initialized successfully")
        except Exception as e:
            logger.error(f"Failed to initialize GameRulesService: {e}")
            raise RuntimeError(f"Could not initialize rules service: {e}")
        
        # Manager registry
        self._managers: Dict[str, Any] = {}
        self._manager_classes: Dict[str, Type] = {}
        
        # Custom content tracking - now using dynamic detection
        self.custom_content: Dict[str, Dict[str, Any]] = {}
        
        # Transaction support
        self._current_transaction: Optional[Transaction] = None
        self._transaction_history: List[Transaction] = []
        
        # Cache ability score mapping from game data
        self._ability_mapping = self._build_ability_mapping()
        
        # Initialize dynamic custom content detection
        self._detect_custom_content_dynamic()
        logger.info(f"CharacterManager initialized with {len(self.custom_content)} custom items")
    
    def register_manager(self, name: str, manager_class: Type, 
                        on_register: Optional[Callable] = None,
                        on_unregister: Optional[Callable] = None,
                        lazy: bool = False):
        """
        Register a subsystem manager with optional lifecycle hooks
        
        Args:
            name: Manager name (e.g., 'class', 'feat')
            manager_class: Manager class to instantiate
            on_register: Optional callback to call after registration
            on_unregister: Optional callback to call before unregistration
            lazy: If True, defer instantiation until first access
        """
        # Validate manager class
        if not callable(manager_class):
            raise ValueError(f"Manager class {name} is not callable")
        
        self._manager_classes[name] = manager_class
        
        # Store lifecycle hooks
        if not hasattr(self, '_manager_hooks'):
            self._manager_hooks = {}
        self._manager_hooks[name] = {
            'on_register': on_register,
            'on_unregister': on_unregister
        }
        
        if lazy:
            # Create a lazy proxy instead of instantiating immediately
            logger.info(f"Registering {name} manager for lazy initialization")
            self._managers[name] = LazyManagerProxy(name, manager_class, self)
        else:
            # Instantiate manager with reference to this CharacterManager
            try:
                logger.info(f"Instantiating {name} manager with class {manager_class.__name__}")
                manager_instance = manager_class(self)
                self._managers[name] = manager_instance
                logger.info(f"Successfully created {name} manager instance")
                
                # Call registration hook if provided
                if on_register:
                    try:
                        on_register(manager_instance)
                    except Exception as e:
                        logger.error(f"Error in on_register hook for {name}: {e}")
                        
            except Exception as e:
                logger.error(f"Failed to create {name} manager: {e}")
                # Clean up partial registration
                if name in self._manager_classes:
                    del self._manager_classes[name]
                if name in self._manager_hooks:
                    del self._manager_hooks[name]
                raise RuntimeError(f"Could not create {name} manager: {e}")
        
        logger.info(f"Registered {name} manager")
    
    def get_manager(self, name: str):
        """
        Get a registered manager by name.
        
        If the manager is a lazy proxy, this will trigger its initialization.
        
        Args:
            name: Manager name
            
        Returns:
            Manager instance or None if not registered
        """
        manager = self._managers.get(name)
        if isinstance(manager, LazyManagerProxy):
            # Access any attribute to trigger initialization
            # The proxy will replace itself in _managers
            _ = manager._initialized
            # Return the real manager from _managers
            return self._managers.get(name)
        return manager
    
    def unregister_manager(self, name: str):
        """
        Unregister a subsystem manager
        
        Args:
            name: Manager name to unregister
        """
        if name not in self._managers:
            logger.warning(f"Attempted to unregister non-existent manager: {name}")
            return
        
        # Call unregistration hook if provided
        if hasattr(self, '_manager_hooks') and name in self._manager_hooks:
            on_unregister = self._manager_hooks[name].get('on_unregister')
            if on_unregister:
                try:
                    on_unregister(self._managers[name])
                except Exception as e:
                    logger.error(f"Error in on_unregister hook for {name}: {e}")
        
        # Remove manager
        del self._managers[name]
        del self._manager_classes[name]
        if hasattr(self, '_manager_hooks') and name in self._manager_hooks:
            del self._manager_hooks[name]
        
        logger.info(f"Unregistered {name} manager")
    
    def begin_transaction(self) -> Transaction:
        """Start a new transaction for atomic changes"""
        if self._current_transaction:
            raise RuntimeError("Transaction already in progress")
            
        self._current_transaction = Transaction(self)
        logger.info(f"Started transaction {self._current_transaction.id}")
        return self._current_transaction
    
    def validate_transaction(self) -> tuple[bool, List[str]]:
        """Validate the current transaction using dynamic rules"""
        if not self._current_transaction:
            raise RuntimeError("No transaction in progress")
        
        errors = []
        
        # Validate ability scores using dynamic mapping
        for ability_name in self._ability_mapping.keys():
            value = self.get_ability_score(ability_name)
            min_val, max_val = self._get_ability_score_range(ability_name)
            if value < min_val or value > max_val:
                errors.append(f"{ability_name.title()} must be between {min_val} and {max_val} (got {value})")
        
        # Validate level using dynamic class data
        total_level = self._get_total_level()
        max_level = self._get_max_character_level()
        if total_level < 1 or total_level > max_level:
            errors.append(f"Total level must be between 1 and {max_level} (got {total_level})")
        
        # Validate alignment using dynamic ranges
        law_chaos = self.gff.get('LawfulChaotic', 50)
        good_evil = self.gff.get('GoodEvil', 50)
        if law_chaos < 0 or law_chaos > 100:
            errors.append(f"Law/Chaos alignment must be between 0 and 100 (got {law_chaos})")
        if good_evil < 0 or good_evil > 100:
            errors.append(f"Good/Evil alignment must be between 0 and 100 (got {good_evil})")
        
        # Let managers validate their own data
        is_valid, validation_errors = self.validate_changes(preview=True)
        errors.extend(validation_errors)
        
        return len(errors) == 0, errors
    
    def _get_ability_score_range(self, ability_name: str) -> tuple[int, int]:
        """
        Get valid ability score range from game data
        
        Args:
            ability_name: Ability name
            
        Returns:
            (min_value, max_value) tuple
        """
        # TODO: Read from game data - for now use standard D&D ranges
        return (3, 50)
    
    def _get_total_level(self) -> int:
        """Get total character level from all classes"""
        return sum(
            c.get('ClassLevel', 0) 
            for c in self.gff.get('ClassList', []) 
            if isinstance(c, dict)
        )
    
    def _get_max_character_level(self) -> int:
        """
        Get maximum character level from game data
        
        Returns:
            Maximum allowed character level
        """
        # TODO: Read from game data - for now use standard NWN2 cap
        return 30
    
    def commit_transaction(self) -> Dict[str, Any]:
        """Commit the current transaction"""
        if not self._current_transaction:
            raise RuntimeError("No transaction in progress")
        
        # Validate before committing
        is_valid, errors = self.validate_transaction()
        if not is_valid:
            raise ValueError(f"Transaction validation failed: {'; '.join(errors)}")
            
        result = self._current_transaction.commit()
        self._transaction_history.append(self._current_transaction)
        self._current_transaction = None
        return result
    
    def rollback_transaction(self):
        """Rollback the current transaction"""
        if not self._current_transaction:
            raise RuntimeError("No transaction in progress")
            
        self._current_transaction.rollback()
        self._current_transaction = None
    
    def _build_ability_mapping(self) -> Dict[str, str]:
        """
        Build dynamic ability score mapping from game data
        Maps standard names to GFF field names
        """
        # Try to get ability data from 2DA files
        # In NWN2, abilities are typically stored as Str, Dex, Con, Int, Wis, Cha
        default_mapping = {
            'strength': 'Str',
            'dexterity': 'Dex', 
            'constitution': 'Con',
            'intelligence': 'Int',
            'wisdom': 'Wis',
            'charisma': 'Cha'
        }
        
        # TODO: In future, this could read from iprp_abilities.2da or similar
        # for now, use the standard mapping but make it extensible
        return default_mapping
    
    def get_ability_score(self, ability_name: str, default: int = 10) -> int:
        """
        Get ability score using dynamic mapping
        
        Args:
            ability_name: Standard ability name (strength, dexterity, etc.)
            default: Default value if not found
            
        Returns:
            Ability score value
        """
        gff_field = self._ability_mapping.get(ability_name.lower())
        if gff_field:
            return self.gff.get(gff_field, default)
        return default
    
    def set_ability_score(self, ability_name: str, value: int):
        """
        Set ability score using dynamic mapping
        
        Args:
            ability_name: Standard ability name
            value: New ability score value
        """
        gff_field = self._ability_mapping.get(ability_name.lower())
        if gff_field:
            self.gff.set(gff_field, value)
            if self._current_transaction:
                self._current_transaction.add_change('ability_change', {
                    'ability': ability_name,
                    'old_value': self.gff.get(gff_field, 10),
                    'new_value': value
                })
    
    def get_ability_scores(self) -> Dict[str, int]:
        """Get all ability scores using dynamic mapping"""
        return {
            ability: self.get_ability_score(ability)
            for ability in self._ability_mapping.keys()
        }
    
    def is_custom_content(self, content_type: str, content_id: int) -> bool:
        """
        Check if a specific content ID is custom content
        
        Args:
            content_type: Type of content ('feat', 'spell', 'class', etc.)
            content_id: ID of the content
            
        Returns:
            True if the content is custom, False if vanilla
        """
        key = f"{content_type}_{content_id}"
        return key in self.custom_content
    
    def _detect_custom_content_dynamic(self):
        """
        Detect custom content using dynamic game data validation
        Uses DynamicGameDataLoader to determine what's vanilla vs custom
        """
        self.custom_content = {}
        
        # Check feats using dynamic data
        feat_list = self.gff.get('FeatList', [])
        for i, feat in enumerate(feat_list):
            if isinstance(feat, dict):
                feat_id = feat.get('Feat', 0)
                if not self._is_vanilla_content('feat', feat_id):
                    feat_name = self._get_content_name('feat', feat_id)
                    self.custom_content[f'feat_{feat_id}'] = {
                        'type': 'feat',
                        'id': feat_id,
                        'name': feat_name,
                        'index': i,
                        'protected': True,
                        'source': self._detect_content_source_dynamic('feat', feat_id)
                    }
        
        # Check spells using dynamic data
        for spell_level in range(10):  # Levels 0-9
            spell_list = self.gff.get(f'KnownList{spell_level}', [])
            for i, spell in enumerate(spell_list):
                if isinstance(spell, dict):
                    spell_id = spell.get('Spell', 0)
                    if not self._is_vanilla_content('spells', spell_id):
                        spell_name = self._get_content_name('spells', spell_id)
                        self.custom_content[f'spell_{spell_id}'] = {
                            'type': 'spell',
                            'id': spell_id,
                            'name': spell_name,
                            'level': spell_level,
                            'index': i,
                            'protected': True,
                            'source': self._detect_content_source_dynamic('spells', spell_id)
                        }
        
        # Check classes using dynamic data
        for class_entry in self.gff.get('ClassList', []):
            if isinstance(class_entry, dict):
                class_id = class_entry.get('Class', 0)
                if not self._is_vanilla_content('classes', class_id):
                    class_name = self._get_content_name('classes', class_id)
                    self.custom_content[f'class_{class_id}'] = {
                        'type': 'class',
                        'id': class_id,
                        'name': class_name,
                        'level': class_entry.get('ClassLevel', 0),
                        'protected': True,
                        'source': self._detect_content_source_dynamic('classes', class_id)
                    }
    
    def _is_vanilla_content(self, table_name: str, content_id: int) -> bool:
        """
        Check if content ID exists in vanilla game data using DynamicGameDataLoader
        
        Args:
            table_name: 2DA table name (feat, spells, classes, etc.)
            content_id: ID to check
            
        Returns:
            True if content exists in loaded game data
        """
        try:
            content_data = self.game_data_loader.get_by_id(table_name, content_id)
            return content_data is not None
        except Exception:
            return False
    
    def _get_content_name(self, table_name: str, content_id: int) -> str:
        """
        Get content name from game data or fallback to generic name
        
        Args:
            table_name: 2DA table name
            content_id: Content ID
            
        Returns:
            Content name or generic fallback
        """
        try:
            content_data = self.game_data_loader.get_by_id(table_name, content_id)
            if content_data:
                # Try different name fields depending on table
                for name_field in ['name', 'label', 'feat', 'spellname']:
                    if hasattr(content_data, name_field):
                        name = getattr(content_data, name_field)
                        if name and name.strip():
                            return name
        except Exception:
            pass
        
        # Fallback to generic name
        return f"Custom {table_name.title()[:-1]} {content_id}"
    
    def _detect_content_source_dynamic(self, table_name: str, content_id: int) -> str:
        """
        Detect content source using dynamic validation against loaded data
        
        Args:
            table_name: 2DA table name
            content_id: Content ID
            
        Returns:
            Source description
        """
        # If it's not in vanilla data, it's custom
        if not self._is_vanilla_content(table_name, content_id):
            return "custom-mod"
        return "vanilla"
    
    
    def _notify_managers(self, notification_type: str, data: Dict[str, Any]):
        """
        Internal notification system for managers
        
        Args:
            notification_type: Type of notification
            data: Notification data
        """
        for name, manager in self._managers.items():
            if hasattr(manager, f'on_{notification_type}'):
                method = getattr(manager, f'on_{notification_type}')
                method(data)
            
            # Special handling for transaction rollback - update gff references
            if notification_type == 'transaction_rollback':
                manager.gff = self.gff
    
    def _get_class_name(self, class_id: int) -> str:
        """Get class name from dynamic data"""
        return self._get_content_name('classes', class_id)
    
    def _get_race_name(self, race_id: int) -> str:
        """Get race name from dynamic data"""
        return self._get_content_name('racialtypes', race_id)
    
    def get_class_skills(self, class_id: int) -> Set[int]:
        """
        Get set of class skills for a class using dynamic data
        
        Args:
            class_id: The class ID
            
        Returns:
            Set of skill IDs that are class skills for this class
        """
        class_skills = set()
        class_data = self.game_data_loader.get_by_id('classes', class_id)
        
        if class_data:
            # Try to use skills table name if available
            skills_table_name = getattr(class_data, 'skills_table', None)
            if skills_table_name:
                try:
                    class_skills_table = self.game_data_loader.get_table(skills_table_name)
                    if class_skills_table:
                        for skill_entry in class_skills_table:
                            # Different 2DA files might use different column names
                            for skill_col in ['skill_index', 'skillindex', 'skill', 'id']:
                                skill_id = getattr(skill_entry, skill_col, None)
                                if skill_id is not None:
                                    class_skills.add(skill_id)
                                    break
                except Exception as e:
                    logger.warning(f"Could not load class skills table {skills_table_name}: {e}")
        
        return class_skills
    
    def _get_character_name(self) -> str:
        """Extract character name from localized string structure"""
        first_name = self.gff.get('FirstName', {})
        last_name = self.gff.get('LastName', {})
        
        # Handle localized string structure
        if isinstance(first_name, dict) and 'substrings' in first_name:
            first = first_name.get('substrings', [{}])[0].get('string', '')
        else:
            first = str(first_name)
            
        if isinstance(last_name, dict) and 'substrings' in last_name:
            last = last_name.get('substrings', [{}])[0].get('string', '')
        else:
            last = str(last_name)
            
        full_name = f"{first} {last}".strip()
        return full_name if full_name and full_name != " " else ""
    
    def get_character_summary(self) -> Dict[str, Any]:
        """Get a summary of the character's current state using dynamic data"""
        return {
            'name': self._get_character_name(),
            'level': self._get_total_level(),
            'classes': [
                {
                    'class_id': c.get('Class', 0),
                    'level': c.get('ClassLevel', 0),
                    'name': self._get_class_name(c.get('Class', 0))
                }
                for c in self.gff.get('ClassList', [])
                if isinstance(c, dict)
            ],
            'race': self._get_race_name(self.gff.get('Race', 0)),
            'alignment': {
                'law_chaos': self.gff.get('LawfulChaotic', 50),
                'good_evil': self.gff.get('GoodEvil', 50)
            },
            'abilities': self.get_ability_scores(),
            'custom_content_count': len(self.custom_content)
        }
    
    def validate_changes(self, preview: bool = False) -> tuple[bool, List[str]]:
        """
        Validate all pending changes
        
        Args:
            preview: If True, validate without applying
            
        Returns:
            (is_valid, list_of_errors)
        """
        errors = []
        
        # Let each manager validate its state
        for name, manager in self._managers.items():
            if hasattr(manager, 'validate'):
                is_valid, manager_errors = manager.validate()
                if not is_valid:
                    errors.extend([f"{name}: {e}" for e in manager_errors])
        
        # Use rules service for additional validation
        char_summary = self._create_character_summary_for_rules()
        rules_errors = self.rules_service.validate_character(char_summary)
        errors.extend(rules_errors)
        
        return len(errors) == 0, errors
    
    def _create_character_summary_for_rules(self) -> Dict[str, Any]:
        """Create character summary dict for rules service validation using dynamic data"""
        # Convert our dynamic ability scores to the format expected by rules service
        abilities = self.get_ability_scores()
        return {
            'level': self._get_total_level(),
            'classes': [
                {
                    'id': c.get('Class', 0),
                    'level': c.get('ClassLevel', 0)
                }
                for c in self.gff.get('ClassList', [])
                if isinstance(c, dict)
            ],
            'race': self.gff.get('Race', 0),
            'abilities': {
                # Map our dynamic ability names to rules service format
                'str': abilities.get('strength', 10),
                'dex': abilities.get('dexterity', 10),
                'con': abilities.get('constitution', 10),
                'int': abilities.get('intelligence', 10),
                'wis': abilities.get('wisdom', 10),
                'cha': abilities.get('charisma', 10)
            },
            'alignment': {
                'law_chaos': self.gff.get('LawfulChaotic', 50),
                'good_evil': self.gff.get('GoodEvil', 50)
            },
            'feats': [f.get('Feat', 0) for f in self.gff.get('FeatList', []) if isinstance(f, dict)],
            'skills': self._extract_skills_summary(),
            'hit_points': self.gff.get('HitPoints', 0),
            'base_attack_bonus': self.gff.get('BaseAttackBonus', 0)
        }
    
    def _extract_skills_summary(self) -> Dict[int, int]:
        """Extract skills summary for rules validation"""
        skills = {}
        skill_list = self.gff.get('SkillList', [])
        for skill in skill_list:
            if isinstance(skill, dict):
                skill_id = skill.get('Skill', -1)
                rank = skill.get('Rank', 0)
                if skill_id >= 0:
                    skills[skill_id] = rank
        return skills
    
    def get_available_feats(self) -> List[Dict[str, Any]]:
        """Get list of feats available to the character using rules service"""
        char_summary = self._create_character_summary_for_rules()
        available = self.rules_service.get_available_feats(
            char_summary,
            include_reasons=True
        )
        return available
    
    def get_available_classes(self) -> List[Dict[str, Any]]:
        """Get list of classes available for next level"""
        char_summary = self._create_character_summary_for_rules()
        return self.rules_service.get_available_classes(char_summary)
    
    def check_prerequisites(self, item_type: str, item_id: int) -> tuple[bool, List[str]]:
        """Check if character meets prerequisites for an item/feat/class/etc"""
        char_summary = self._create_character_summary_for_rules()
        return self.rules_service.check_prerequisites(
            char_summary, 
            item_type, 
            item_id
        )
    
    def calculate_derived_stats(self) -> Dict[str, Any]:
        """Calculate all derived statistics using rules service"""
        char_summary = self._create_character_summary_for_rules()
        return self.rules_service.calculate_derived_stats(char_summary)
    
    def get_class_progressions(self) -> Dict[str, Any]:
        """Get class progression info for all character classes"""
        progressions = {}
        for class_entry in self.gff.get('ClassList', []):
            if isinstance(class_entry, dict):
                class_id = class_entry.get('Class', 0)
                class_level = class_entry.get('ClassLevel', 0)
                
                progression = self.rules_service.get_class_progression(
                    class_id, 
                    class_level
                )
                if progression:
                    class_name = self._get_class_name(class_id)
                    progressions[class_name] = progression
        
        return progressions
    
    def apply_template(self, template_name: str) -> Dict[str, Any]:
        """
        Apply a character template/build
        
        Args:
            template_name: Name of the template to apply
            
        Returns:
            Summary of changes applied
        """
        # TODO: Load template definitions from configuration
        templates = {
            'fighter': {
                'attributes': {'strength': 16, 'constitution': 14, 'dexterity': 13},
                'skills': [2, 8, 11],  # Discipline, Intimidate, Parry
                'feats': [2, 3, 4]  # Power Attack, Cleave, Weapon Focus
            },
            'wizard': {
                'attributes': {'intelligence': 16, 'constitution': 12, 'dexterity': 14},
                'skills': [5, 13, 21],  # Concentration, Lore, Spellcraft
                'feats': [12, 13]  # Combat Casting, Spell Focus
            },
            'rogue': {
                'attributes': {'dexterity': 16, 'intelligence': 14, 'charisma': 12},
                'skills': [0, 7, 15, 19],  # Disable Device, Hide, Move Silently, Open Lock
                'feats': [21, 22]  # Weapon Finesse, Improved Initiative
            }
        }
        
        if template_name not in templates:
            raise ValueError(f"Unknown template: {template_name}")
        
        template = templates[template_name]
        changes = []
        
        # Begin transaction
        txn = self.begin_transaction()
        
        try:
            # Apply attributes
            if 'attributes' in template:
                for attr, value in template['attributes'].items():
                    self.set_ability_score(attr, value)
                    changes.append(f"Set {attr} to {value}")
            
            # Apply skills if skill manager available
            skill_mgr = self.get_manager('skill')
            if skill_mgr and 'skills' in template:
                for skill_id in template['skills']:
                    # Max ranks for level
                    max_ranks = self._get_total_level() + 3
                    skill_mgr.set_skill_rank(skill_id, max_ranks)
                    changes.append(f"Maximized skill {skill_id}")
            
            # Apply feats if feat manager available
            feat_mgr = self.get_manager('feat')
            if feat_mgr and 'feats' in template:
                for feat_id in template['feats']:
                    try:
                        feat_mgr.add_feat(feat_id)
                        changes.append(f"Added feat {feat_id}")
                    except Exception:
                        pass  # Skip if can't add feat
            
            self.commit_transaction()
            
            return {
                'template': template_name,
                'changes': changes,
                'success': True
            }
            
        except Exception as e:
            self.rollback_transaction()
            logger.error(f"Failed to apply template {template_name}: {e}")
            raise
    
    def batch_update(self, updates: List[Dict[str, Any]]) -> Dict[str, Any]:
        """
        Apply multiple updates at once
        
        Args:
            updates: List of update operations
            
        Returns:
            Summary of applied updates
        """
        results = []
        txn = self.begin_transaction()
        
        try:
            for update in updates:
                update_type = update.get('type')
                
                if update_type == 'attribute':
                    self.set_ability_score(update['attribute'], update['value'])
                    results.append({'type': 'attribute', 'success': True})
                    
                elif update_type == 'skill':
                    skill_mgr = self.get_manager('skill')
                    if skill_mgr:
                        skill_mgr.set_skill_rank(update['skill_id'], update['rank'])
                        results.append({'type': 'skill', 'success': True})
                    
                elif update_type == 'feat':
                    feat_mgr = self.get_manager('feat')
                    if feat_mgr:
                        if update.get('action') == 'add':
                            feat_mgr.add_feat(update['feat_id'])
                        elif update.get('action') == 'remove':
                            feat_mgr.remove_feat(update['feat_id'])
                        results.append({'type': 'feat', 'success': True})
                    
                else:
                    results.append({'type': update_type, 'success': False, 'error': 'Unknown update type'})
            
            self.commit_transaction()
            
            return {
                'total': len(updates),
                'successful': sum(1 for r in results if r['success']),
                'results': results
            }
            
        except Exception as e:
            self.rollback_transaction()
            logger.error(f"Batch update failed: {e}")
            raise
    
    def preview_changes(self, changes: List[Dict[str, Any]]) -> Dict[str, Any]:
        """
        Preview effects of changes without applying them
        
        Args:
            changes: List of changes to preview
            
        Returns:
            Preview of what would change
        """
        # Create a clone to test changes
        preview_manager = self.clone_character()
        
        try:
            # Apply changes to clone
            result = preview_manager.batch_update(changes)
            
            # Compare states
            original_summary = self.get_character_summary()
            preview_summary = preview_manager.get_character_summary()
            
            # Calculate differences
            differences = {
                'abilities': {},
                'level': preview_summary['level'] - original_summary['level'],
                'custom_content': preview_summary['custom_content_count'] - original_summary['custom_content_count']
            }
            
            # Compare abilities
            for ability, value in preview_summary['abilities'].items():
                if value != original_summary['abilities'][ability]:
                    differences['abilities'][ability] = {
                        'old': original_summary['abilities'][ability],
                        'new': value,
                        'change': value - original_summary['abilities'][ability]
                    }
            
            return {
                'preview': preview_summary,
                'differences': differences,
                'validation': preview_manager.validate_changes()
            }
            
        finally:
            # Cleanup preview manager
            del preview_manager
    
    def get_all_violations(self) -> List[Dict[str, Any]]:
        """
        Get all current rule violations
        
        Returns:
            List of rule violations with details
        """
        violations = []
        
        # Check each manager for violations
        for name, manager in self._managers.items():
            if hasattr(manager, 'validate'):
                is_valid, errors = manager.validate()
                if not is_valid:
                    for error in errors:
                        violations.append({
                            'source': name,
                            'error': error,
                            'severity': 'error'
                        })
        
        # Check general character rules
        is_valid, errors = self.validate_changes()
        if not is_valid:
            for error in errors:
                if not any(v['error'] == error for v in violations):
                    violations.append({
                        'source': 'character',
                        'error': error,
                        'severity': 'error'
                    })
        
        return violations
    
    def fix_violations(self, auto_fix: bool = True) -> Dict[str, Any]:
        """
        Auto-fix rule violations where possible
        
        Args:
            auto_fix: If True, automatically fix violations
            
        Returns:
            Summary of fixes applied
        """
        fixes = []
        violations = self.get_all_violations()
        
        if not auto_fix:
            return {
                'violations': violations,
                'fixes': [],
                'auto_fix': False
            }
        
        txn = self.begin_transaction()
        
        try:
            for violation in violations:
                # Fix ability score violations
                if 'must be between' in violation['error'] and 'ability' in violation['source'].lower():
                    # Extract ability and limits from error message
                    import re
                    match = re.search(r'(\w+) must be between (\d+) and (\d+)', violation['error'])
                    if match:
                        ability, min_val, max_val = match.groups()
                        current = self.get_ability_score(ability.lower())
                        if current < int(min_val):
                            self.set_ability_score(ability.lower(), int(min_val))
                            fixes.append(f"Set {ability} to minimum {min_val}")
                        elif current > int(max_val):
                            self.set_ability_score(ability.lower(), int(max_val))
                            fixes.append(f"Set {ability} to maximum {max_val}")
                
                # Fix alignment violations
                elif 'alignment must be between' in violation['error']:
                    if 'Law/Chaos' in violation['error']:
                        self.gff.set('LawfulChaotic', 50)
                        fixes.append("Reset Law/Chaos alignment to neutral")
                    elif 'Good/Evil' in violation['error']:
                        self.gff.set('GoodEvil', 50)
                        fixes.append("Reset Good/Evil alignment to neutral")
            
            self.commit_transaction()
            
            # Re-check violations
            remaining_violations = self.get_all_violations()
            
            return {
                'original_violations': len(violations),
                'fixes_applied': fixes,
                'remaining_violations': len(remaining_violations),
                'success': len(remaining_violations) < len(violations)
            }
            
        except Exception as e:
            self.rollback_transaction()
            logger.error(f"Failed to fix violations: {e}")
            raise
    
    def get_build_suggestions(self) -> List[Dict[str, Any]]:
        """
        Get suggestions for character build improvement
        
        Returns:
            List of build suggestions
        """
        suggestions = []
        
        # Analyze current build
        summary = self.get_character_summary()
        abilities = summary['abilities']
        level = summary['level']
        classes = summary['classes']
        
        # Suggest attribute improvements
        primary_class = classes[0] if classes else None
        if primary_class:
            class_name = primary_class['name'].lower()
            
            # Suggest based on class
            if 'fighter' in class_name or 'barbarian' in class_name:
                if abilities['strength'] < 14:
                    suggestions.append({
                        'type': 'attribute',
                        'suggestion': 'Increase Strength for better melee combat',
                        'priority': 'high'
                    })
            elif 'wizard' in class_name:
                if abilities['intelligence'] < 14:
                    suggestions.append({
                        'type': 'attribute',
                        'suggestion': 'Increase Intelligence for more spell slots',
                        'priority': 'high'
                    })
            elif 'rogue' in class_name:
                if abilities['dexterity'] < 14:
                    suggestions.append({
                        'type': 'attribute',
                        'suggestion': 'Increase Dexterity for better AC and skills',
                        'priority': 'high'
                    })
        
        # Suggest feats
        available_feats = self.get_available_feats()
        if available_feats:
            suggestions.append({
                'type': 'feat',
                'suggestion': f"You have {len(available_feats)} feats available to choose",
                'priority': 'medium',
                'count': len(available_feats)
            })
        
        # Suggest skill points
        skill_mgr = self.get_manager('skill')
        if skill_mgr and hasattr(skill_mgr, 'get_unspent_points'):
            unspent = skill_mgr.get_unspent_points()
            if unspent > 0:
                suggestions.append({
                    'type': 'skill',
                    'suggestion': f"You have {unspent} unspent skill points",
                    'priority': 'medium'
                })
        
        return suggestions
    
    def validate_alignment_for_class(self, class_id: int) -> tuple[bool, Optional[str]]:
        """Check if current alignment is valid for a class"""
        alignment = {
            'law_chaos': self.gff.get('LawfulChaotic', 50),
            'good_evil': self.gff.get('GoodEvil', 50)
        }
        return self.rules_service.validate_alignment_for_class(class_id, alignment)
    
    def get_all_managers(self) -> Dict[str, Any]:
        """
        Get all registered managers
        
        Returns:
            Dictionary of manager name to manager instance
        """
        return self._managers.copy()
    
    def reload_managers(self) -> None:
        """Reload all managers (useful after game data changes)"""
        logger.info("Reloading all managers")
        
        # Store current manager classes
        manager_classes = self._manager_classes.copy()
        
        # Unregister all managers
        for name in list(self._managers.keys()):
            self.unregister_manager(name)
        
        # Re-register all managers
        for name, manager_class in manager_classes.items():
            self.register_manager(name, manager_class)
        
        logger.info(f"Reloaded {len(manager_classes)} managers")
    
    def get_manager_status(self) -> Dict[str, Dict[str, Any]]:
        """
        Get status/health of all managers
        
        Returns:
            Dictionary with status info for each manager
        """
        status = {}
        
        for name, manager in self._managers.items():
            manager_status = {
                'registered': True,
                'class': manager.__class__.__name__,
                'has_validate': hasattr(manager, 'validate')
            }
            
            # Check if manager is healthy by calling validate if available
            if hasattr(manager, 'validate'):
                try:
                    is_valid, errors = manager.validate()
                    manager_status['is_valid'] = is_valid
                    manager_status['validation_errors'] = errors
                except Exception as e:
                    manager_status['is_valid'] = False
                    manager_status['validation_errors'] = [f"Validation failed: {str(e)}"]
            
            status[name] = manager_status
        
        return status
    
    def undo_last_change(self) -> bool:
        """
        Undo the last committed transaction
        
        Returns:
            True if undo was successful, False if no transaction to undo
        """
        if not self._transaction_history:
            logger.warning("No transaction history to undo")
            return False
        
        # Get last transaction
        last_transaction = self._transaction_history.pop()
        
        # Restore character to state before transaction
        self.character_data = copy.deepcopy(last_transaction.original_state)
        self.gff = GFFDataWrapper(self.character_data)
        
        # Notify managers
        self._notify_managers('transaction_undone', {
            'transaction_id': last_transaction.id,
            'changes': last_transaction.changes
        })
        
        logger.info(f"Undone transaction {last_transaction.id}")
        return True
    
    def redo_change(self) -> bool:
        """
        Redo a previously undone change
        
        Returns:
            True if redo was successful, False if nothing to redo
        """
        # TODO: Implement redo stack
        logger.warning("Redo not yet implemented")
        return False
    
    def get_transaction_history(self) -> List[Dict[str, Any]]:
        """
        Get full transaction history
        
        Returns:
            List of transaction summaries
        """
        return [
            {
                'id': txn.id,
                'timestamp': txn.timestamp,
                'changes': txn.changes,
                'change_count': len(txn.changes)
            }
            for txn in self._transaction_history
        ]
    
    def clear_transaction_history(self) -> None:
        """Clear all transaction history"""
        self._transaction_history.clear()
        logger.info("Cleared transaction history")
    
    def export_changes(self) -> Dict[str, Any]:
        """Export all changes made in the current session"""
        return {
            'summary': self.get_character_summary(),
            'transactions': [
                {
                    'id': t.id,
                    'changes': t.changes,
                    'timestamp': t.timestamp
                }
                for t in self._transaction_history
            ],
            'custom_content': self.custom_content,
            'event_history': [
                {
                    'type': e.event_type.value,
                    'source': e.source_manager,
                    'timestamp': e.timestamp
                }
                for e in self.get_event_history()
            ]
        }
    
    def get_related_table(self, source_table: str, column_name: str, row_id: int) -> Optional[List[Any]]:
        """
        Navigate 2DA table relationships
        
        Args:
            source_table: Name of the source table (e.g., 'classes')
            column_name: Column containing the related table name (e.g., 'AttackBonusTable')
            row_id: ID of the row in the source table
            
        Returns:
            List of rows from the related table, or None if not found
        """
        # Get the source row
        source_row = self.game_data_loader.get_by_id(source_table, row_id)
        if not source_row:
            logger.warning(f"Row {row_id} not found in {source_table}")
            return None
        
        # Get the related table name from the column
        related_table_name = getattr(source_row, column_name.lower(), None)
        if not related_table_name:
            logger.warning(f"Column {column_name} not found or empty in {source_table} row {row_id}")
            return None
        
        # Load the related table
        related_table = self.game_data_loader.get_table(related_table_name.lower())
        if not related_table:
            logger.warning(f"Related table {related_table_name} not found")
            return None
        
        return related_table
    
    def get_class_feats_for_level(self, class_data: Any, level: int) -> List[Dict[str, Any]]:
        """
        Get feats granted by a class at a specific level
        
        Args:
            class_data: Class data object from dynamic loader
            level: Character level to check
            
        Returns:
            List of feat dictionaries with 'feat_id' and 'list_type' keys
        """
        feats_for_level = []
        
        # Get the feat table name from class data
        feat_table_name = getattr(class_data, 'feats_table', None)
        if not feat_table_name:
            logger.debug(f"Class {getattr(class_data, 'label', 'Unknown')} has no feat table")
            return feats_for_level
        
        # Load the feat table
        feat_table = self.game_data_loader.get_table(feat_table_name.lower())
        if not feat_table:
            logger.warning(f"Feat table {feat_table_name} not found")
            return feats_for_level
        
        # Look for feats at this level
        # Class feat tables have columns like FeatIndex, GrantedOnLevel, List
        for feat_entry in feat_table:
            granted_level = getattr(feat_entry, 'granted_on_level', -1)
            if granted_level == level:
                feat_id = getattr(feat_entry, 'feat_index', -1)
                list_type = getattr(feat_entry, 'list', 3)  # Default to general list
                
                if feat_id >= 0:
                    feats_for_level.append({
                        'feat_id': feat_id,
                        'list_type': list_type,
                        'granted_on_level': granted_level
                    })
        
        return feats_for_level
    
    def detect_epithet_feats(self) -> Set[int]:
        """
        Detect epithet feats (special story/custom feats that should be protected)
        
        Returns:
            Set of feat IDs that are epithet feats
        """
        epithet_feats = set()
        
        # Get all feats from character
        feat_list = self.gff.get('FeatList', [])
        
        # Get vanilla feat data
        feat_table = self.game_data_loader.get_table('feat')
        if not feat_table:
            logger.warning("Feat table not found, cannot detect epithet feats")
            return epithet_feats
        
        # Build set of all vanilla feat IDs (using row indices which correspond to feat IDs)
        vanilla_feat_ids = set(range(len(feat_table)))
        
        # Check each character feat
        for feat_entry in feat_list:
            if isinstance(feat_entry, dict):
                feat_id = feat_entry.get('Feat', -1)
                
                # Check if this is a non-vanilla feat or has special properties
                if feat_id not in vanilla_feat_ids:
                    # This is a custom/mod feat
                    epithet_feats.add(feat_id)
                else:
                    # Check if it's an epithet feat by looking at properties
                    feat_data = self.game_data_loader.get_by_id('feat', feat_id)
                    if feat_data:
                        # Epithet feats often have specific naming patterns or categories
                        label = (getattr(feat_data, 'label', '') or '').lower()
                        category = (getattr(feat_data, 'categories', '') or '').lower()
                        
                        # Common patterns for epithet/story feats
                        epithet_patterns = [
                            'epithet', 'story', 'history', 'background',
                            'blessing', 'curse', 'gift', 'legacy'
                        ]
                        
                        for pattern in epithet_patterns:
                            if pattern in label or pattern in category:
                                epithet_feats.add(feat_id)
                                break
        
        logger.info(f"Detected {len(epithet_feats)} epithet feats: {epithet_feats}")
        return epithet_feats
    
    def get_size_modifier(self, size_id: int) -> int:
        """
        Get AC/attack modifier for a creature size
        
        Args:
            size_id: The size ID from creaturesize.2da
            
        Returns:
            AC modifier for the size (positive = bonus, negative = penalty)
        """
        try:
            size_data = self.game_data_loader.get_by_id('creaturesize', size_id)
            if size_data:
                # ACATTACKMOD column has the AC modifier
                ac_mod = getattr(size_data, 'acattackmod', 0)
                try:
                    return int(ac_mod)
                except (ValueError, TypeError):
                    return 0
        except Exception as e:
            logger.warning(f"Could not get size modifier for size {size_id}: {e}")
        
        return 0
    
    def get_racial_saves(self, race_id: int) -> Dict[str, int]:
        """
        Get racial save bonuses from race data using field mapping utility
        
        Args:
            race_id: The race ID
            
        Returns:
            Dict with fortitude, reflex, will save bonuses
        """
        from gamedata.dynamic_loader.field_mapping_utility import field_mapper
        
        try:
            race_data = self.game_data_loader.get_by_id('racialtypes', race_id)
            if race_data:
                return field_mapper.get_racial_saves(race_data)
        except Exception as e:
            logger.warning(f"Could not get racial saves for race {race_id}: {e}")
        
        # Return default values if no race data or error
        return {'fortitude': 0, 'reflex': 0, 'will': 0}
    
    def get_class_abilities(self, class_id: int, level: int) -> List[Dict[str, Any]]:
        """
        Get special abilities granted by a class at a specific level
        
        Args:
            class_id: The class ID
            level: The level to check
            
        Returns:
            List of ability info dicts
        """
        abilities = []
        
        try:
            class_data = self.game_data_loader.get_by_id('classes', class_id)
            if not class_data:
                return abilities
            
            # Check for ability table (like cls_bfeat_* tables)
            ability_table_name = getattr(class_data, 'ability_table', None)
            if not ability_table_name:
                # Try alternate naming
                label = getattr(class_data, 'label', '').lower()
                ability_table_name = f'cls_bfeat_{label}'
            
            if ability_table_name:
                ability_table = self.game_data_loader.get_table(ability_table_name.lower())
                if ability_table:
                    for ability in ability_table:
                        granted_level = getattr(ability, 'granted_on_level', -1)
                        if granted_level == level:
                            ability_id = getattr(ability, 'feat_index', -1)
                            if ability_id >= 0:
                                abilities.append({
                                    'ability_id': ability_id,
                                    'type': 'feat',
                                    'level': level
                                })
        except Exception as e:
            logger.warning(f"Could not get class abilities for class {class_id} level {level}: {e}")
        
        return abilities
    
    def get_spell_casting_attribute(self, class_id: int) -> Optional[str]:
        """
        Get the primary spell casting attribute for a class
        
        Args:
            class_id: The class ID
            
        Returns:
            Attribute name ('Int', 'Wis', 'Cha') or None
        """
        try:
            class_data = self.game_data_loader.get_by_id('classes', class_id)
            if class_data:
                # Check for spell casting ability field
                spell_ability = getattr(class_data, 'primary_ability', None)
                if spell_ability:
                    # Convert from short form (INT, WIS, CHA) to our form
                    return spell_ability.capitalize()
                
                # Fallback: infer from class label
                label = getattr(class_data, 'label', '').lower()
                if label in ['wizard']:
                    return 'Int'
                elif label in ['cleric', 'druid', 'ranger']:
                    return 'Wis'
                elif label in ['sorcerer', 'bard', 'paladin', 'warlock']:
                    return 'Cha'
        except Exception as e:
            logger.warning(f"Could not get spell casting attribute for class {class_id}: {e}")
        
        return None
    
    def has_class_by_name(self, class_name: str) -> bool:
        """
        Check if character has levels in a class by name
        
        Args:
            class_name: The class name to check
            
        Returns:
            True if character has this class
        """
        class_list = self.gff.get('ClassList', [])
        
        for class_info in class_list:
            class_id = class_info.get('Class', -1)
            class_data = self.game_data_loader.get_by_id('classes', class_id)
            if class_data:
                label = getattr(class_data, 'label', '')
                if label.lower() == class_name.lower():
                    return True
        
        return False
    
    def get_class_level_by_name(self, class_name: str) -> int:
        """
        Get level in a specific class by name
        
        Args:
            class_name: The class name
            
        Returns:
            Class level or 0 if not found
        """
        class_list = self.gff.get('ClassList', [])
        
        for class_info in class_list:
            class_id = class_info.get('Class', -1)
            class_data = self.game_data_loader.get_by_id('classes', class_id)
            if class_data:
                label = getattr(class_data, 'label', '')
                if label.lower() == class_name.lower():
                    return class_info.get('ClassLevel', 0)
        
        return 0
    
    def has_feat_by_name(self, feat_label: str) -> bool:
        """
        Check if character has a feat by its label
        
        Args:
            feat_label: The feat label to check
            
        Returns:
            True if character has the feat
        """
        feat_list = self.gff.get('FeatList', [])
        
        for feat in feat_list:
            feat_id = feat.get('Feat', -1)
            feat_data = self.game_data_loader.get_by_id('feat', feat_id)
            if feat_data:
                label = getattr(feat_data, 'label', '')
                if label == feat_label:
                    return True
        
        return False
    
    def get_character_age(self) -> int:
        """
        Get character age
        
        Returns:
            Character age in years
        """
        return self.gff.get('Age', 18)  # Default adult age
    
    def get_character_background(self) -> str:
        """
        Get character background/biography
        
        Returns:
            Character background text
        """
        bio = self.gff.get('Description', {})
        
        # Handle localized string structure
        if isinstance(bio, dict) and 'substrings' in bio:
            substrings = bio.get('substrings', [])
            if substrings and isinstance(substrings[0], dict):
                return substrings[0].get('string', '')
        elif isinstance(bio, str):
            return bio
        
        return ''
    
    def get_experience_points(self) -> int:
        """
        Get current experience points
        
        Returns:
            Current XP value
        """
        return self.gff.get('Experience', 0)
    
    def get_next_level_xp(self) -> int:
        """
        Get XP needed for next level
        
        Returns:
            XP required for next level
        """
        current_level = self._get_total_level()
        
        # Standard D&D 3.5 XP progression
        xp_table = {
            1: 0,
            2: 1000,
            3: 3000,
            4: 6000,
            5: 10000,
            6: 15000,
            7: 21000,
            8: 28000,
            9: 36000,
            10: 45000,
            11: 55000,
            12: 66000,
            13: 78000,
            14: 91000,
            15: 105000,
            16: 120000,
            17: 136000,
            18: 153000,
            19: 171000,
            20: 190000,
            21: 210000,
            22: 231000,
            23: 253000,
            24: 276000,
            25: 300000,
            26: 325000,
            27: 351000,
            28: 378000,
            29: 406000,
            30: 435000
        }
        
        next_level = min(current_level + 1, 30)
        return xp_table.get(next_level, 435000)
    
    def get_armor_class(self) -> Dict[str, Any]:
        """
        Get total AC with all modifiers
        
        Returns:
            AC breakdown with all components
        """
        # Use combat manager if available
        combat_mgr = self.get_manager('combat')
        if combat_mgr:
            return combat_mgr.calculate_armor_class()
        
        # Fallback calculation
        base_ac = 10
        dex_bonus = (self.gff.get('Dex', 10) - 10) // 2
        
        return {
            'total': base_ac + dex_bonus,
            'breakdown': {
                'base': base_ac,
                'dexterity': dex_bonus,
                'armor': 0,
                'shield': 0,
                'natural': 0,
                'deflection': 0,
                'dodge': 0,
                'size': 0
            }
        }
    
    def get_initiative(self) -> int:
        """
        Get initiative modifier
        
        Returns:
            Total initiative modifier
        """
        combat_mgr = self.get_manager('combat')
        if combat_mgr and hasattr(combat_mgr, 'calculate_initiative'):
            return combat_mgr.calculate_initiative()['total']
        
        # Fallback: Dex modifier
        return (self.gff.get('Dex', 10) - 10) // 2
    
    def get_movement_speed(self) -> int:
        """
        Get movement speed in feet per round
        
        Returns:
            Movement speed
        """
        race_id = self.gff.get('Race', 0)
        base_speed = self._get_base_speed(race_id)
        
        # TODO: Apply modifiers from armor, feats, etc.
        return base_speed
    
    def get_carrying_capacity(self) -> Dict[str, float]:
        """
        Get encumbrance limits based on strength
        
        Returns:
            Dictionary with light, medium, heavy load limits in pounds
        """
        str_score = self.gff.get('Str', 10)
        
        # Base carrying capacity table (simplified)
        base_capacity = {
            1: 3, 2: 6, 3: 10, 4: 13, 5: 16,
            6: 20, 7: 23, 8: 26, 9: 30, 10: 33,
            11: 38, 12: 43, 13: 50, 14: 58, 15: 66,
            16: 76, 17: 86, 18: 100, 19: 116, 20: 133,
            21: 153, 22: 173, 23: 200, 24: 233, 25: 266,
            26: 306, 27: 346, 28: 400, 29: 466, 30: 533
        }
        
        max_load = base_capacity.get(min(str_score, 30), 33)
        
        return {
            'light': max_load / 3,
            'medium': (max_load * 2) / 3,
            'heavy': max_load,
            'lift_overhead': max_load,
            'lift_ground': max_load * 2,
            'drag_push': max_load * 5
        }
    
    def get_damage_reduction(self) -> List[Dict[str, Any]]:
        """
        Get damage reduction from all sources
        
        Returns:
            List of DR entries with amount and bypass type
        """
        combat_mgr = self.get_manager('combat')
        if combat_mgr and hasattr(combat_mgr, 'get_damage_reduction'):
            return combat_mgr.get_damage_reduction()
        
        # TODO: Calculate from feats, class features, items
        return []
    
    def get_spell_resistance(self) -> int:
        """
        Get spell resistance value
        
        Returns:
            Total spell resistance
        """
        # Base SR from race
        race_id = self.gff.get('Race', 0)
        race_data = self.game_data_loader.get_by_id('racialtypes', race_id)
        base_sr = 0
        
        if race_data:
            base_sr = getattr(race_data, 'spell_resistance', 0)
        
        # TODO: Add SR from feats, items, class features
        return base_sr
    
    def get_energy_resistances(self) -> Dict[str, int]:
        """
        Get energy resistances (fire, cold, etc.)
        
        Returns:
            Dictionary of damage type to resistance amount
        """
        resistances = {
            'fire': 0,
            'cold': 0,
            'acid': 0,
            'electricity': 0,
            'sonic': 0,
            'positive': 0,
            'negative': 0
        }
        
        # TODO: Calculate from race, feats, items, class features
        return resistances
    
    def reset_character(self) -> None:
        """Reset character to a default state (level 1, base attributes)"""
        logger.info("Resetting character to default state")
        
        # Begin transaction for atomic reset
        txn = self.begin_transaction()
        
        try:
            # Reset basic info
            self.gff.set('FirstName', {'substrings': [{'string': 'New Character'}]})
            self.gff.set('LastName', {'substrings': [{'string': ''}]})
            
            # Reset to level 1 with first class
            class_list = self.gff.get('ClassList', [])
            if class_list:
                # Keep only first class at level 1
                first_class = class_list[0]
                first_class['ClassLevel'] = 1
                self.gff.set('ClassList', [first_class])
            
            # Reset attributes to base 10
            for ability_field in ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']:
                self.gff.set(ability_field, 10)
            
            # Reset alignment to true neutral
            self.gff.set('LawfulChaotic', 50)
            self.gff.set('GoodEvil', 50)
            
            # Clear feats except racial
            racial_feats = self.detect_epithet_feats()  # Preserve special feats
            feat_list = self.gff.get('FeatList', [])
            preserved_feats = [f for f in feat_list if f.get('Feat', -1) in racial_feats]
            self.gff.set('FeatList', preserved_feats)
            
            # Clear skills
            self.gff.set('SkillList', [])
            
            # Reset HP to base
            self.gff.set('HitPoints', 6)  # Base HP for level 1
            self.gff.set('CurrentHitPoints', 6)
            
            # Clear spells
            for level in range(10):
                self.gff.set(f'KnownList{level}', [])
                self.gff.set(f'MemorizedList{level}', [])
            
            # Notify managers
            self._notify_managers('character_reset', {})
            
            # Commit transaction
            self.commit_transaction()
            
            logger.info("Character reset completed successfully")
            
        except Exception as e:
            self.rollback_transaction()
            logger.error(f"Failed to reset character: {e}")
            raise
    
    def clone_character(self) -> 'CharacterManager':
        """
        Create a copy of the current character
        
        Returns:
            New CharacterManager instance with cloned character data
        """
        cloned_data = copy.deepcopy(self.character_data)
        
        # Create new manager with cloned data
        cloned_manager = CharacterManager(
            cloned_data,
            game_data_loader=self.game_data_loader,
            rules_service=self.rules_service
        )
        
        # Register same managers
        for name, manager_class in self._manager_classes.items():
            cloned_manager.register_manager(name, manager_class)
        
        logger.info(f"Created clone of character {self._get_character_name()}")
        return cloned_manager
    
    def import_character(self, character_data: Dict[str, Any]) -> None:
        """
        Import character data from exported format
        
        Args:
            character_data: Character data to import
        """
        if 'summary' not in character_data:
            raise ValueError("Invalid character data format - missing summary")
        
        logger.info("Importing character data")
        
        # Begin transaction
        txn = self.begin_transaction()
        
        try:
            # Import core character data
            if 'gff_data' in character_data:
                self.character_data = character_data['gff_data']
                self.gff = GFFDataWrapper(self.character_data)
            
            # Re-detect custom content
            self._detect_custom_content_dynamic()
            
            # Notify managers of import
            self._notify_managers('character_imported', character_data)
            
            # Validate imported data
            is_valid, errors = self.validate_changes()
            if not is_valid:
                raise ValueError(f"Imported character has validation errors: {errors}")
            
            self.commit_transaction()
            logger.info("Character import completed successfully")
            
        except Exception as e:
            self.rollback_transaction()
            logger.error(f"Failed to import character: {e}")
            raise
    
    def save_to_file(self, filepath: str) -> None:
        """
        Save character to a file
        
        Args:
            filepath: Path to save the character file
        """
        from parsers import gff
        
        try:
            # If we have a gff_element, use it for direct write
            if self.gff_element:
                gff.write_gff(self.gff_element, filepath)
            else:
                # Convert dict data back to GFF format
                gff_element = gff.dict_to_gff(self.character_data)
                gff.write_gff(gff_element, filepath)
            
            logger.info(f"Character saved to {filepath}")
            
        except Exception as e:
            logger.error(f"Failed to save character to {filepath}: {e}")
            raise
    
    def load_from_file(self, filepath: str) -> None:
        """
        Load character from a file
        
        Args:
            filepath: Path to the character file
        """
        from parsers import gff
        
        try:
            # Parse the GFF file
            gff_element = gff.parse_gff(filepath)
            character_data = gff.gff_to_dict(gff_element)
            
            # Import the loaded data
            self.import_character({'gff_data': character_data, 'summary': {}})
            
            # Store the gff_element for direct updates
            self.gff_element = gff_element
            from .gff_direct_wrapper import DirectGFFWrapper
            self.gff = DirectGFFWrapper(gff_element)
            
            logger.info(f"Character loaded from {filepath}")
            
        except Exception as e:
            logger.error(f"Failed to load character from {filepath}: {e}")
            raise
    
    # Missing methods called by various managers
    def has_feat(self, feat_id: int) -> bool:
        """Check if character has a specific feat by ID"""
        feat_list = self.gff.get('FeatList', [])
        for feat in feat_list:
            if isinstance(feat, dict) and feat.get('Feat') == feat_id:
                return True
        return False
    
    def _calculate_ability_modifiers(self) -> Dict[str, int]:
        """Calculate ability modifiers from ability scores"""
        abilities = {
            'STR': self.gff.get('Str', 10),
            'DEX': self.gff.get('Dex', 10),
            'CON': self.gff.get('Con', 10),
            'INT': self.gff.get('Int', 10),
            'WIS': self.gff.get('Wis', 10),
            'CHA': self.gff.get('Cha', 10)
        }
        
        return {
            ability: (value - 10) // 2
            for ability, value in abilities.items()
        }