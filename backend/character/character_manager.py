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
        
        # Cache ability score mapping - hardcoded standard mapping
        self._ability_mapping = {
            'strength': 'Str',
            'dexterity': 'Dex', 
            'constitution': 'Con',
            'intelligence': 'Int',
            'wisdom': 'Wis',
            'charisma': 'Cha'
        }
        
        # Register ContentManager to handle custom content detection
        from .managers.content_manager import ContentManager
        self.register_manager('content', ContentManager)
        
        # Register CharacterStateManager to handle state manipulation
        from .managers.character_state_manager import CharacterStateManager
        self.register_manager('state', CharacterStateManager)
        
        # Initialize custom content detection through ContentManager
        content_manager = self.get_manager('content')
        if content_manager:
            content_manager._detect_custom_content_dynamic()
            self.custom_content = content_manager.custom_content
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
    
    
    def commit_transaction(self) -> Dict[str, Any]:
        """Commit the current transaction"""
        if not self._current_transaction:
            raise RuntimeError("No transaction in progress")
        
        # Validate before committing
        is_valid, errors = self.validate_changes()
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
    
    def get_character_summary(self) -> Dict[str, Any]:
        """Get a summary of the character's current state aggregated from managers"""
        # Get managers
        attr_manager = self.get_manager('attribute')
        class_manager = self.get_manager('class')
        race_manager = self.get_manager('race')
        
        summary = {
            'name': attr_manager._get_character_name() if attr_manager else '',
            'level': class_manager._get_total_level() if class_manager else 1,
            'race': race_manager._get_race_name(self.gff.get('Race', 0)) if race_manager else f"Race_{self.gff.get('Race', 0)}",
            'alignment': {
                'law_chaos': self.gff.get('LawfulChaotic', 50),
                'good_evil': self.gff.get('GoodEvil', 50)
            },
            'custom_content_count': len(self.custom_content)
        }
        
        # Aggregate class information from ClassManager
        if class_manager and hasattr(class_manager, 'get_class_summary'):
            summary['classes'] = class_manager.get_class_summary()
        else:
            # Fallback to direct access
            summary['classes'] = [
                {
                    'class_id': c.get('Class', 0),
                    'level': c.get('ClassLevel', 0),
                    'name': class_manager._get_class_name(c.get('Class', 0)) if class_manager else f"Class_{c.get('Class', 0)}"
                }
                for c in self.gff.get('ClassList', [])
                if isinstance(c, dict)
            ]
        
        # Aggregate ability scores from AttributeManager
        attribute_manager = self.get_manager('attribute')
        if attribute_manager and hasattr(attribute_manager, 'get_attributes'):
            # Convert attribute manager format to expected format
            attributes = attribute_manager.get_attributes()
            summary['abilities'] = {
                'strength': attributes.get('Str', 10),
                'dexterity': attributes.get('Dex', 10),
                'constitution': attributes.get('Con', 10),
                'intelligence': attributes.get('Int', 10),
                'wisdom': attributes.get('Wis', 10),
                'charisma': attributes.get('Cha', 10)
            }
        else:
            # Fallback to direct access via attribute manager
            attribute_manager = self.get_manager('attribute')
            if attribute_manager:
                summary['abilities'] = attribute_manager.get_ability_scores()
            else:
                # Last resort - direct GFF access
                summary['abilities'] = {
                    'strength': self.gff.get('Str', 10),
                    'dexterity': self.gff.get('Dex', 10),
                    'constitution': self.gff.get('Con', 10),
                    'intelligence': self.gff.get('Int', 10),
                    'wisdom': self.gff.get('Wis', 10),
                    'charisma': self.gff.get('Cha', 10)
                }
        
        return summary
    
    def validate_changes(self, preview: bool = False) -> tuple[bool, List[str]]:
        """
        Validate all pending changes - corruption prevention only
        
        Args:
            preview: If True, validate without applying
            
        Returns:
            (is_valid, list_of_errors)
        """
        errors = []
        
        # Only corruption prevention - no game rule validation
        # Let each manager validate its state (they should only check corruption too)
        for name, manager in self._managers.items():
            if hasattr(manager, 'validate'):
                is_valid, manager_errors = manager.validate()
                if not is_valid:
                    errors.extend([f"{name}: {e}" for e in manager_errors])
        
        return len(errors) == 0, errors
    
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
                    attribute_manager = self.get_manager('attribute')
                    if attribute_manager:
                        attribute_manager.set_ability_score(update['attribute'], update['value'])
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
    
    
    
    
    
    
    
    
    
    
    
    
