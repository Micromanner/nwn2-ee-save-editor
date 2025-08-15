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
        
        # Custom content will be detected when ContentManager is registered by factory
        logger.info(f"CharacterManager initialized")
    
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
        ability_manager = self.get_manager('ability')  # AbilityManager handles ability scores (strength, dex, etc.)
        class_manager = self.get_manager('class')
        race_manager = self.get_manager('race')
        
        summary = {
            'name': ability_manager.get_character_name() if ability_manager else '',
            'level': class_manager.get_total_level() if class_manager else 1,
            'race': race_manager.get_race_name(self.gff.get('Race', 0)) if race_manager else f"Race_{self.gff.get('Race', 0)}",
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
                    'name': class_manager.get_class_name(c.get('Class', 0)) if class_manager else f"Class_{c.get('Class', 0)}"
                }
                for c in self.gff.get('ClassList', [])
                if isinstance(c, dict)
            ]
        
        # Aggregate ability scores from AbilityManager (using correct NWN2 terminology)
        ability_manager = self.get_manager('ability')
        if ability_manager and hasattr(ability_manager, 'get_ability_scores'):
            # Use the standardized ability scores method that returns proper format
            summary['abilities'] = ability_manager.get_ability_scores()
        elif ability_manager and hasattr(ability_manager, 'get_attributes'):
            # Fallback: convert attribute manager format to expected format
            attributes = ability_manager.get_attributes()
            summary['abilities'] = {
                'strength': attributes.get('Str', 10),
                'dexterity': attributes.get('Dex', 10),
                'constitution': attributes.get('Con', 10),
                'intelligence': attributes.get('Int', 10),
                'wisdom': attributes.get('Wis', 10),
                'charisma': attributes.get('Cha', 10)
            }
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
        
        # Add campaign/module/quest data if ContentManager is available
        content_manager = self.get_manager('content')
        if content_manager:
            summary['campaign_name'] = content_manager.get_campaign_name()
            summary['module_name'] = content_manager.get_module_name()
            summary['area_name'] = content_manager.get_area_name()
            summary['quest_details'] = content_manager.get_quest_summary()
            
            # Also add the full campaign info
            campaign_info = content_manager.get_campaign_info()
            summary.update(campaign_info)
        
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
                    attribute_manager = self.get_manager('ability')
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
    
    def get_character_state(self) -> Dict[str, Any]:
        """
        Get comprehensive character state from all managers
        
        Returns:
            Dict containing aggregated character state data from all subsystem managers
        """
        # Start with basic info
        character_info = self.get_character_summary()
        
        state_data = {
            'info': {
                'id': getattr(self, '_character_id', 'unknown'),
                'file_path': getattr(self, '_file_path', ''),
                'file_name': '',  # Will be populated from file_path if available
                'file_type': 'bic',  # Default, can be overridden
                'first_name': character_info.get('name', '').split()[0] if character_info.get('name') else '',
                'last_name': ' '.join(character_info.get('name', '').split()[1:]) if len(character_info.get('name', '').split()) > 1 else '',
                'full_name': character_info.get('name', ''),
                'level': character_info.get('level', 1),
                'experience': self.gff.get('Experience', 0),
                'race_name': character_info.get('race', ''),
                'alignment': character_info.get('alignment', {}),
                'alignment_string': character_info.get('alignment_string', ''),
                'is_savegame': False,
                'is_companion': False
            },
            'summary': character_info
        }
        
        # Populate file info if available
        if hasattr(self, '_file_path') and self._file_path:
            from pathlib import Path
            file_path = Path(self._file_path)
            state_data['info']['file_name'] = file_path.name
            if file_path.suffix.lower() in ['.ros']:
                state_data['info']['file_type'] = 'ros'
                state_data['info']['is_companion'] = True
            elif file_path.suffix.lower() in ['.ifo']:
                state_data['info']['file_type'] = 'ifo'
                state_data['info']['is_savegame'] = True
        
        # Get data from all managers
        manager_data = {}
        
        # Abilities
        ability_manager = self.get_manager('ability')
        if ability_manager:
            try:
                if hasattr(ability_manager, 'get_ability_scores'):
                    manager_data['abilities'] = ability_manager.get_ability_scores()
                elif hasattr(ability_manager, 'get_attributes'):
                    manager_data['abilities'] = ability_manager.get_attributes()
            except Exception as e:
                logger.warning(f"Failed to get ability data: {e}")
        
        # Combat stats
        combat_manager = self.get_manager('combat')
        if combat_manager:
            try:
                if hasattr(combat_manager, 'get_combat_stats'):
                    manager_data['combat'] = combat_manager.get_combat_stats()
            except Exception as e:
                logger.warning(f"Failed to get combat data: {e}")
        
        # Skills
        skill_manager = self.get_manager('skill')
        if skill_manager:
            try:
                if hasattr(skill_manager, 'get_all_skills'):
                    manager_data['skills'] = skill_manager.get_all_skills()
                elif hasattr(skill_manager, 'get_skills'):
                    manager_data['skills'] = skill_manager.get_skills()
            except Exception as e:
                logger.warning(f"Failed to get skill data: {e}")
        
        # Feats
        feat_manager = self.get_manager('feat')
        if feat_manager:
            try:
                if hasattr(feat_manager, 'get_all_feats'):
                    manager_data['feats'] = feat_manager.get_all_feats()
                elif hasattr(feat_manager, 'get_feats'):
                    manager_data['feats'] = feat_manager.get_feats()
            except Exception as e:
                logger.warning(f"Failed to get feat data: {e}")
        
        # Spells
        spell_manager = self.get_manager('spell')
        if spell_manager:
            try:
                if hasattr(spell_manager, 'get_all_spells'):
                    manager_data['spells'] = spell_manager.get_all_spells()
                elif hasattr(spell_manager, 'get_spells'):
                    manager_data['spells'] = spell_manager.get_spells()
            except Exception as e:
                logger.warning(f"Failed to get spell data: {e}")
        
        # Inventory
        inventory_manager = self.get_manager('inventory')
        if inventory_manager:
            try:
                if hasattr(inventory_manager, 'get_inventory_state'):
                    manager_data['inventory'] = inventory_manager.get_inventory_state()
                elif hasattr(inventory_manager, 'get_inventory'):
                    manager_data['inventory'] = inventory_manager.get_inventory()
            except Exception as e:
                logger.warning(f"Failed to get inventory data: {e}")
        
        # Classes
        class_manager = self.get_manager('class')
        if class_manager:
            try:
                if hasattr(class_manager, 'get_all_classes'):
                    manager_data['classes'] = class_manager.get_all_classes()
                elif hasattr(class_manager, 'get_classes'):
                    manager_data['classes'] = class_manager.get_classes()
            except Exception as e:
                logger.warning(f"Failed to get class data: {e}")
        
        # Race
        race_manager = self.get_manager('race')
        if race_manager:
            try:
                if hasattr(race_manager, 'get_race_info'):
                    manager_data['race'] = race_manager.get_race_info()
                elif hasattr(race_manager, 'get_race'):
                    manager_data['race'] = race_manager.get_race()
            except Exception as e:
                logger.warning(f"Failed to get race data: {e}")
        
        # Save/Saving throws
        save_manager = self.get_manager('save')
        if save_manager:
            try:
                if hasattr(save_manager, 'get_saving_throws'):
                    manager_data['saves'] = save_manager.get_saving_throws()
                elif hasattr(save_manager, 'get_saves'):
                    manager_data['saves'] = save_manager.get_saves()
            except Exception as e:
                logger.warning(f"Failed to get save data: {e}")
        
        # Content
        content_manager = self.get_manager('content')
        if content_manager:
            try:
                if hasattr(content_manager, 'get_content_summary'):
                    manager_data['content'] = content_manager.get_content_summary()
            except Exception as e:
                logger.warning(f"Failed to get content data: {e}")
        
        # Add manager data to state
        state_data.update(manager_data)
        
        # Add metadata
        state_data['custom_content'] = self.custom_content or {}
        state_data['manager_status'] = self.get_manager_status()
        state_data['has_unsaved_changes'] = len(self._transaction_history) > 0
        
        return state_data
    
    def validate_character(self) -> Dict[str, Any]:
        """
        Validate character data across all managers
        
        Returns:
            Dict containing validation results with errors and warnings
        """
        all_errors = []
        all_warnings = []
        manager_errors = {}
        corruption_risks = []
        
        # Validate each manager
        for name, manager in self._managers.items():
            try:
                if hasattr(manager, 'validate'):
                    is_valid, errors = manager.validate()
                    if not is_valid:
                        manager_errors[name] = errors
                        all_errors.extend([f"{name}: {error}" for error in errors])
                elif hasattr(manager, 'validate_data'):
                    result = manager.validate_data()
                    if isinstance(result, dict) and not result.get('valid', True):
                        errors = result.get('errors', [])
                        manager_errors[name] = errors
                        all_errors.extend([f"{name}: {error}" for error in errors])
            except Exception as e:
                error_msg = f"Validation failed for {name}: {str(e)}"
                manager_errors[name] = [error_msg]
                all_errors.append(error_msg)
                corruption_risks.append(f"{name} manager validation threw exception")
        
        # Run basic GFF validation
        try:
            # Check critical fields exist
            required_fields = ['FirstName', 'Race', 'ClassList']
            for field in required_fields:
                if not self.gff.has_field(field):
                    all_errors.append(f"Missing critical field: {field}")
                    corruption_risks.append(f"Missing {field} could cause game crashes")
            
            # Check class list integrity
            class_list = self.gff.get('ClassList', [])
            if not class_list:
                all_errors.append("Empty ClassList - character needs at least one class")
                corruption_risks.append("Empty ClassList will cause game crashes")
            else:
                for i, char_class in enumerate(class_list):
                    if not isinstance(char_class, dict):
                        all_errors.append(f"ClassList[{i}] is not a valid class structure")
                        corruption_risks.append(f"Invalid class structure at index {i}")
                    else:
                        if 'Class' not in char_class:
                            all_errors.append(f"ClassList[{i}] missing 'Class' field")
                        if 'ClassLevel' not in char_class:
                            all_errors.append(f"ClassList[{i}] missing 'ClassLevel' field")
            
            # Check ability scores
            for ability in ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']:
                value = self.gff.get(ability, None)
                if value is None:
                    all_errors.append(f"Missing ability score: {ability}")
                elif not isinstance(value, int) or value < 1 or value > 255:
                    all_warnings.append(f"Unusual {ability} value: {value}")
            
            # Check alignment values
            law_chaos = self.gff.get('LawfulChaotic', None)
            good_evil = self.gff.get('GoodEvil', None)
            if law_chaos is None:
                all_errors.append("Missing LawfulChaotic alignment value")
            elif not (0 <= law_chaos <= 100):
                all_errors.append(f"Invalid LawfulChaotic value: {law_chaos} (must be 0-100)")
            
            if good_evil is None:
                all_errors.append("Missing GoodEvil alignment value")
            elif not (0 <= good_evil <= 100):
                all_errors.append(f"Invalid GoodEvil value: {good_evil} (must be 0-100)")
                
        except Exception as e:
            error_msg = f"GFF validation failed: {str(e)}"
            all_errors.append(error_msg)
            corruption_risks.append("GFF structure validation failed - data may be corrupted")
        
        # Check custom content integrity
        if self.custom_content:
            try:
                content_manager = self.get_manager('content')
                if content_manager and hasattr(content_manager, 'validate_custom_content'):
                    content_result = content_manager.validate_custom_content()
                    if not content_result.get('valid', True):
                        content_errors = content_result.get('errors', [])
                        all_warnings.extend(content_errors)
            except Exception as e:
                all_warnings.append(f"Custom content validation failed: {str(e)}")
        
        return {
            'valid': len(all_errors) == 0,
            'errors': all_errors,
            'warnings': all_warnings,
            'manager_errors': manager_errors,
            'corruption_risks': corruption_risks
        }
    
    
    
    
    
    
    
    
    
    
    
    
