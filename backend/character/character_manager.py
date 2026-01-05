"""Data-driven CharacterManager using DynamicGameDataLoader for game data."""

from typing import Dict, List, Any, Optional, Type, TypeVar, Union, overload, Callable, TYPE_CHECKING
import copy
import time
from dataclasses import dataclass
from loguru import logger

from .events import EventEmitter, EventData
from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader
from services.gamedata.game_rules_service import GameRulesService

if TYPE_CHECKING:
    from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader


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
        """Restore character to state before transaction."""
        logger.info(f"Rolling back transaction {self.id}")
        self.manager.character_data = self.original_state
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
        """Initialize the actual manager instance."""
        if not self._initialized:
            logger.debug(f"Lazy-initializing {self._name} manager")
            self._instance = self._manager_class(self._character_manager)
            self._initialized = True
            self._character_manager._managers[self._name] = self._instance

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
        """Handle attribute setting."""
        if name.startswith('_'):
            object.__setattr__(self, name, value)
        else:
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
        """Get value at dot-separated path (e.g., 'ClassList.0.Class')."""
        parts = path.split('.')
        current = self._data

        for part in parts:
            if current is None:
                return default

            if isinstance(current, list) and part.isdigit():
                idx = int(part)
                if 0 <= idx < len(current):
                    current = current[idx]
                else:
                    return default
            elif isinstance(current, dict):
                current = current.get(part, default)
            else:
                return default

        return current
    
    def set(self, path: str, value: Any) -> None:
        """Set value at dot-separated path."""
        parts = path.split('.')
        current = self._data

        for i, part in enumerate(parts[:-1]):
            if isinstance(current, list) and part.isdigit():
                idx = int(part)
                if 0 <= idx < len(current):
                    current = current[idx]
                else:
                    raise IndexError(f"Index {idx} out of range at {'.'.join(parts[:i+1])}")
            elif isinstance(current, dict):
                if part not in current:
                    next_part = parts[i+1]
                    if next_part.isdigit():
                        current[part] = []
                    else:
                        current[part] = {}
                current = current[part]
            else:
                raise ValueError(f"Cannot navigate path at {'.'.join(parts[:i+1])}")

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
        """Get value at path with type checking, returns default if type mismatch."""
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
    """Data-driven character manager using DynamicGameDataLoader for game data."""

    def __init__(self, character_data: Dict[str, Any], game_data_loader: Optional['DynamicGameDataLoader'] = None, rules_service: Optional[GameRulesService] = None, save_path: Optional[str] = None, **kwargs):
        """Initialize the character manager with GFF data and game services."""
        super().__init__()
        self.character_data = character_data
        self.save_path = save_path

        if not isinstance(character_data, dict):
            raise ValueError(f"character_data must be a dictionary, got {type(character_data)}")
        if not character_data:
            raise ValueError("character_data cannot be empty")

        self.gff = GFFDataWrapper(character_data)

        try:
            from gamedata.dynamic_loader.singleton import wait_for_loader_ready
            if game_data_loader is None:
                if not wait_for_loader_ready(timeout=30.0):
                    raise RuntimeError("DynamicGameDataLoader initialization timed out after 30 seconds")
                self.game_data_loader = get_dynamic_game_data_loader()
            else:
                self.game_data_loader = game_data_loader
                if not self.game_data_loader.is_ready():
                    if not self.game_data_loader.wait_for_ready(timeout=30.0):
                        raise RuntimeError("Provided DynamicGameDataLoader not ready after 30 seconds")
        except Exception as e:
            logger.error(f"Failed to get DynamicGameDataLoader: {e}")
            raise RuntimeError(f"Could not get game data loader: {e}")

        try:
            self.rules_service = rules_service or GameRulesService()
        except Exception as e:
            logger.error(f"Failed to initialize GameRulesService: {e}")
            raise RuntimeError(f"Could not initialize rules service: {e}")

        self._managers: Dict[str, Any] = {}
        self._manager_classes: Dict[str, Type] = {}
        self.custom_content: Dict[str, Dict[str, Any]] = {}
        self._current_transaction: Optional[Transaction] = None
        self._transaction_history: List[Transaction] = []

        logger.info("CharacterManager initialized")
    
    def register_manager(self, name: str, manager_class: Type,
                        on_register: Optional[Callable] = None,
                        on_unregister: Optional[Callable] = None,
                        lazy: bool = False):
        """Register a subsystem manager with optional lifecycle hooks."""
        if not callable(manager_class):
            raise ValueError(f"Manager class {name} is not callable")

        self._manager_classes[name] = manager_class

        if not hasattr(self, '_manager_hooks'):
            self._manager_hooks = {}
        self._manager_hooks[name] = {'on_register': on_register, 'on_unregister': on_unregister}

        if lazy:
            self._managers[name] = LazyManagerProxy(name, manager_class, self)
        else:
            try:
                manager_instance = manager_class(self)
                self._managers[name] = manager_instance
                if on_register:
                    try:
                        on_register(manager_instance)
                    except Exception as e:
                        logger.error(f"Error in on_register hook for {name}: {e}")
            except Exception as e:
                logger.error(f"Failed to create {name} manager: {e}")
                if name in self._manager_classes:
                    del self._manager_classes[name]
                if name in self._manager_hooks:
                    del self._manager_hooks[name]
                raise RuntimeError(f"Could not create {name} manager: {e}")
    
    def get_manager(self, name: str):
        """Get a registered manager by name, triggering lazy initialization if needed."""
        manager = self._managers.get(name)
        if isinstance(manager, LazyManagerProxy):
            _ = manager._initialized
            return self._managers.get(name)
        return manager
    
    def unregister_manager(self, name: str):
        """Unregister a subsystem manager."""
        if name not in self._managers:
            logger.warning(f"Attempted to unregister non-existent manager: {name}")
            return

        if hasattr(self, '_manager_hooks') and name in self._manager_hooks:
            on_unregister = self._manager_hooks[name].get('on_unregister')
            if on_unregister:
                try:
                    on_unregister(self._managers[name])
                except Exception as e:
                    logger.error(f"Error in on_unregister hook for {name}: {e}")

        del self._managers[name]
        del self._manager_classes[name]
        if hasattr(self, '_manager_hooks') and name in self._manager_hooks:
            del self._manager_hooks[name]
    
    def begin_transaction(self) -> Transaction:
        """Start a new transaction for atomic changes."""
        if self._current_transaction:
            raise RuntimeError("Transaction already in progress")
        self._current_transaction = Transaction(self)
        return self._current_transaction

    def commit_transaction(self) -> Dict[str, Any]:
        """Commit the current transaction."""
        if not self._current_transaction:
            raise RuntimeError("No transaction in progress")
        is_valid, errors = self.validate_changes()
        if not is_valid:
            raise ValueError(f"Transaction validation failed: {'; '.join(errors)}")
        result = self._current_transaction.commit()
        self._transaction_history.append(self._current_transaction)
        self._current_transaction = None
        return result

    def rollback_transaction(self):
        """Rollback the current transaction."""
        if not self._current_transaction:
            raise RuntimeError("No transaction in progress")
        self._current_transaction.rollback()
        self._current_transaction = None

    def _notify_managers(self, notification_type: str, data: Dict[str, Any]):
        """Send notification to all managers that implement the handler."""
        for name, manager in self._managers.items():
            if hasattr(manager, f'on_{notification_type}'):
                method = getattr(manager, f'on_{notification_type}')
                method(data)
            if notification_type == 'transaction_rollback':
                manager.gff = self.gff
    
    def get_character_summary(self) -> Dict[str, Any]:
        """Get a summary of the character's current state aggregated from managers."""
        identity_manager = self.get_manager('identity')
        class_manager = self.get_manager('class')
        race_manager = self.get_manager('race')
        ability_manager = self.get_manager('ability')
        content_manager = self.get_manager('content')
        feat_manager = self.get_manager('feat')

        if not identity_manager:
            raise RuntimeError("IdentityManager is required but not registered")
        if not class_manager:
            raise RuntimeError("ClassManager is required but not registered")
        if not race_manager:
            raise RuntimeError("RaceManager is required but not registered")
        if not ability_manager:
            raise RuntimeError("AbilityManager is required but not registered")

        summary = {
            'name': identity_manager.get_character_name(),
            'level': class_manager.get_total_level(),
            'race': race_manager.get_race_name(self.gff.get('Race', 0)),
            'alignment': {
                'law_chaos': self.gff.get('LawfulChaotic', 50),
                'good_evil': self.gff.get('GoodEvil', 50)
            },
            'gender': self.gff.get('Gender', 0),
            'gold': self.gff.get('Gold', 0),
            'age': self.gff.get('Age', 0),
            'subrace': race_manager._get_subrace_name(self.gff.get('Subrace', '')),
            'custom_content_count': len(self.custom_content),
            'background': {},
            'domains': [],
            'deity': content_manager.get_deity() if content_manager else '',
            'biography': content_manager.get_biography() if content_manager else '',
            'classes': class_manager.get_class_summary(),
            'abilities': ability_manager.get_ability_scores()
        }

        if feat_manager:
            feat_summary = feat_manager.get_feat_summary_fast()
            if feat_summary.get('background_feats'):
                summary['background'] = feat_summary['background_feats'][0]
            domain_feats = feat_summary.get('domain_feats', [])
            summary['domains'] = [f for f in domain_feats if feat_manager.is_domain_epithet_feat(f['id'])]

        if content_manager:
            summary['campaign_name'] = content_manager.get_campaign_name()
            summary['module_name'] = content_manager.get_module_name()
            summary['area_name'] = content_manager.get_area_name()
            summary['quest_details'] = content_manager.get_quest_summary()
            campaign_info = content_manager.get_campaign_info()
            summary.update(campaign_info)

        return summary
    
    def validate_changes(self, preview: bool = False) -> tuple[bool, List[str]]:
        """Validate all pending changes for corruption prevention only."""
        errors = []

        for name, manager in self._managers.items():
            if hasattr(manager, 'validate'):
                is_valid, manager_errors = manager.validate()
                if not is_valid:
                    errors.extend([f"{name}: {e}" for e in manager_errors])
        
        return len(errors) == 0, errors
    
    def batch_update(self, updates: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Apply multiple updates atomically within a transaction."""
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
        """Get all registered managers."""
        return self._managers.copy()
    
    def reload_managers(self) -> None:
        """Reload all managers after game data changes."""
        logger.info("Reloading all managers")
        manager_classes = self._manager_classes.copy()

        for name in list(self._managers.keys()):
            self.unregister_manager(name)

        for name, manager_class in manager_classes.items():
            self.register_manager(name, manager_class)

        logger.info(f"Reloaded {len(manager_classes)} managers")
    
    def get_manager_status(self) -> Dict[str, Dict[str, Any]]:
        """Get status/health of all managers."""
        status = {}

        for name, manager in self._managers.items():
            manager_status = {
                'registered': True,
                'class': manager.__class__.__name__,
                'has_validate': hasattr(manager, 'validate')
            }
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
        """Undo the last committed transaction."""
        if not self._transaction_history:
            logger.warning("No transaction history to undo")
            return False

        last_transaction = self._transaction_history.pop()
        self.character_data = copy.deepcopy(last_transaction.original_state)
        self.gff = GFFDataWrapper(self.character_data)

        self._notify_managers('transaction_undone', {
            'transaction_id': last_transaction.id,
            'changes': last_transaction.changes
        })
        
        logger.info(f"Undone transaction {last_transaction.id}")
        return True

    def get_transaction_history(self) -> List[Dict[str, Any]]:
        """Get full transaction history."""
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
        """Get comprehensive character state aggregated from all managers."""
        character_info = self.get_character_summary()

        state_data = {
            'info': {
                'id': getattr(self, '_character_id', 'unknown'),
                'file_path': getattr(self, '_file_path', ''),
                'file_name': '',
                'file_type': 'bic',
                'first_name': character_info.get('name', '').split()[0] if character_info.get('name') else '',
                'last_name': ' '.join(character_info.get('name', '').split()[1:]) if len(character_info.get('name', '').split()) > 1 else '',
                'full_name': character_info.get('name', ''),
                'level': character_info.get('level', 1),
                'experience': self.gff.get('Experience', 0),
                'race_name': character_info.get('race', ''),
                'alignment': character_info.get('alignment', {}),
                'alignment_string': character_info.get('alignment_string', ''),
                'gender': int(self.gff.get('Gender', 0)),
                'is_savegame': False,
                'is_companion': False
            },
            'summary': character_info
        }

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

        manager_data = {}

        ability_manager = self.get_manager('ability')
        if ability_manager:
            try:
                if hasattr(ability_manager, 'get_ability_scores'):
                    manager_data['abilities'] = ability_manager.get_ability_scores()
                elif hasattr(ability_manager, 'get_attributes'):
                    manager_data['abilities'] = ability_manager.get_attributes()
            except Exception as e:
                logger.error(f"Failed to get ability data: {e}")

        combat_manager = self.get_manager('combat')
        if combat_manager:
            try:
                if hasattr(combat_manager, 'get_combat_stats'):
                    manager_data['combat'] = combat_manager.get_combat_stats()
            except Exception as e:
                logger.error(f"Failed to get combat data: {e}")

        skill_manager = self.get_manager('skill')
        if skill_manager:
            try:
                if hasattr(skill_manager, 'get_all_skills'):
                    manager_data['skills'] = skill_manager.get_all_skills()
                elif hasattr(skill_manager, 'get_skills'):
                    manager_data['skills'] = skill_manager.get_skills()
            except Exception as e:
                logger.error(f"Failed to get skill data: {e}")

        feat_manager = self.get_manager('feat')
        if feat_manager:
            try:
                if hasattr(feat_manager, 'get_all_feats'):
                    manager_data['feats'] = feat_manager.get_all_feats()
                elif hasattr(feat_manager, 'get_feats'):
                    manager_data['feats'] = feat_manager.get_feats()
            except Exception as e:
                logger.error(f"Failed to get feat data: {e}")

        spell_manager = self.get_manager('spell')
        if spell_manager:
            try:
                if hasattr(spell_manager, 'get_all_spells'):
                    manager_data['spells'] = spell_manager.get_all_spells()
                elif hasattr(spell_manager, 'get_spells'):
                    manager_data['spells'] = spell_manager.get_spells()
            except Exception as e:
                logger.error(f"Failed to get spell data: {e}")

        inventory_manager = self.get_manager('inventory')
        if inventory_manager:
            try:
                if hasattr(inventory_manager, 'get_inventory_state'):
                    manager_data['inventory'] = inventory_manager.get_inventory_state()
                elif hasattr(inventory_manager, 'get_inventory'):
                    manager_data['inventory'] = inventory_manager.get_inventory()
            except Exception as e:
                logger.error(f"Failed to get inventory data: {e}")

        class_manager = self.get_manager('class')
        if class_manager:
            try:
                if hasattr(class_manager, 'get_all_classes'):
                    manager_data['classes'] = class_manager.get_all_classes()
                elif hasattr(class_manager, 'get_classes'):
                    manager_data['classes'] = class_manager.get_classes()
            except Exception as e:
                logger.error(f"Failed to get class data: {e}")

        race_manager = self.get_manager('race')
        if race_manager:
            try:
                if hasattr(race_manager, 'get_race_info'):
                    manager_data['race'] = race_manager.get_race_info()
                elif hasattr(race_manager, 'get_race'):
                    manager_data['race'] = race_manager.get_race()
            except Exception as e:
                logger.error(f"Failed to get race data: {e}")

        save_manager = self.get_manager('save')
        if save_manager:
            try:
                if hasattr(save_manager, 'get_saving_throws'):
                    manager_data['saves'] = save_manager.get_saving_throws()
                elif hasattr(save_manager, 'get_saves'):
                    manager_data['saves'] = save_manager.get_saves()
            except Exception as e:
                logger.error(f"Failed to get save data: {e}")

        content_manager = self.get_manager('content')
        if content_manager:
            try:
                if hasattr(content_manager, 'get_content_summary'):
                    manager_data['content'] = content_manager.get_content_summary()
            except Exception as e:
                logger.error(f"Failed to get content data: {e}")

        state_data.update(manager_data)
        state_data['custom_content'] = self.custom_content or {}
        state_data['manager_status'] = self.get_manager_status()
        state_data['has_unsaved_changes'] = len(self._transaction_history) > 0

        return state_data
    
    def update_deity(self, deity: str) -> None:
        """Update character deity."""
        self.gff.set('Deity', deity)
        logger.info(f"Updated deity to: {deity}")

    def update_biography(self, biography: str) -> None:
        """Update character description/biography."""
        desc_struct = self.gff.get('Description', {})
        if isinstance(desc_struct, dict) and 'substrings' in desc_struct:
            # Update existing structure
            if desc_struct['substrings']:
                desc_struct['substrings'][0]['string'] = biography
            else:
                desc_struct['substrings'] = [{'string': biography, 'language': 0, 'gender': 0}]
            self.gff.set('Description', desc_struct)
        else:
            # Create new structure
            self.gff.set('Description', {
                'string_ref': 4294967295,
                'substrings': [{'string': biography, 'language': 0, 'gender': 0}]
            })
        logger.info(f"Updated biography")

    def update_name(self, first_name: Optional[str] = None, last_name: Optional[str] = None) -> None:
        """Update character first and/or last name."""
        if first_name is not None:
            first_name_struct = self.gff.get('FirstName', {})
            if isinstance(first_name_struct, dict):
                first_name_struct['value'] = first_name
                self.gff.set('FirstName', first_name_struct)
            else:
                self.gff.set('FirstName', {'value': first_name})
            logger.info(f"Updated first_name to: {first_name}")
            
        if last_name is not None:
            last_name_struct = self.gff.get('LastName', {})
            if isinstance(last_name_struct, dict):
                last_name_struct['value'] = last_name
                self.gff.set('LastName', last_name_struct)
            else:
                self.gff.set('LastName', {'value': last_name})
            logger.info(f"Updated last_name to: {last_name}")

    def get_level_up_state(self) -> Dict[str, Any]:
        """Get updated character state after class/level changes for UI updates."""
        updated_state = {}

        try:
            class_manager = self.get_manager('class')
            if class_manager:
                updated_state['classes'] = class_manager.get_class_summary()
                updated_state['combat'] = class_manager.get_attack_bonuses()
                updated_state['saves'] = class_manager.calculate_total_saves()

            skill_manager = self.get_manager('skill')
            if skill_manager:
                updated_state['skills'] = {
                    'available_points': self.gff.get('SkillPoints', 0),
                    'total_available': skill_manager.calculate_total_skill_points(
                        self.gff.get('ClassList', [{}])[0].get('Class', 0),
                        sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', []))
                    ),
                    'spent_points': skill_manager._calculate_spent_skill_points()
                }

            feat_manager = self.get_manager('feat')
            if feat_manager:
                feat_list = self.gff.get('FeatList', [])
                updated_state['feats'] = {
                    'total_feats': len(feat_list),
                    'feat_count': len(feat_list)
                }

            spell_manager = self.get_manager('spell')
            if spell_manager and hasattr(spell_manager, 'get_spell_summary'):
                updated_state['spells'] = spell_manager.get_spell_summary()

            ability_manager = self.get_manager('ability')
            if ability_manager:
                class_list = self.gff.get('ClassList', [])
                total_level = sum(c.get('ClassLevel', 0) for c in class_list)
                ability_increases_available = total_level // 4
                level_up_bonuses = ability_manager.get_level_up_modifiers()
                bonuses_used = sum(level_up_bonuses.values())

                updated_state['abilities'] = {
                    'level_up_available': ability_increases_available - bonuses_used,
                    'total_increases': ability_increases_available,
                    'used_increases': bonuses_used
                }

        except Exception as e:
            logger.error(f"Error getting level up state: {e}")
            updated_state['error'] = str(e)

        return updated_state

    def get_abilities_summary(self) -> Dict[str, Any]:
        """Aggregate all ability-related state for the UI."""
        ability_manager = self.get_manager('ability')
        if not ability_manager:
            raise RuntimeError("Ability manager not available")

        gff_abilities = ability_manager.get_attributes(include_equipment=False, include_racial=False)
        level_up_mods = ability_manager.get_level_up_modifiers()
        base_abilities = {
            attr: gff_abilities[attr] - level_up_mods.get(attr, 0)
            for attr in gff_abilities
        }

        effective_abilities = ability_manager.get_effective_attributes()
        
        # Get derived stats
        try:
            hit_points = ability_manager.get_hit_points()
            derived_stats = {
                'hit_points': {
                    'current': hit_points['current'],
                    'maximum': hit_points['max']
                }
            }
        except Exception as e:
            logger.error(f"Failed to get hit points: {e}")
            derived_stats = {'hit_points': {'current': 0, 'maximum': 0}}

        combat_stats = {}
        combat_manager = self.get_manager('combat')
        if combat_manager:
            try:
                combat_stats = {
                    'armor_class': combat_manager.calculate_armor_class(),
                    'initiative': combat_manager.calculate_initiative()
                }
            except Exception as e:
                logger.error(f"Failed to get combat stats: {e}")

        saving_throws = {}
        save_manager = self.get_manager('save')
        if save_manager:
            try:
                saving_throws = save_manager.calculate_saving_throws()
            except Exception as e:
                logger.error(f"Failed to get saving throws: {e}")

        biography = ""
        identity_manager = self.get_manager('identity')
        if identity_manager:
            biography = identity_manager.get_biography()

        return {
            'base_attributes': base_abilities,
            'effective_attributes': effective_abilities,
            'attribute_modifiers': ability_manager.get_attribute_modifiers(),
            'detailed_modifiers': {
                'base_modifiers': ability_manager.get_attribute_modifiers(),
                'racial_modifiers': ability_manager.get_racial_modifiers(),
                'item_modifiers': ability_manager.get_item_modifiers(),
                'enhancement_modifiers': ability_manager.get_enhancement_modifiers(),
                'temporary_modifiers': ability_manager.get_temporary_modifiers(),
                'level_up_modifiers': ability_manager.get_level_up_modifiers(),
                'total_modifiers': ability_manager.get_total_modifiers()
            },
            'point_buy_cost': ability_manager.calculate_point_buy_total(),
            'derived_stats': derived_stats,
            'combat_stats': combat_stats,
            'saving_throws': saving_throws,
            'encumbrance_limits': ability_manager.get_encumbrance_limits(),
            'saving_throw_modifiers': ability_manager.get_saving_throw_modifiers(),
            'skill_modifiers': ability_manager.get_skill_modifiers(),
            'attribute_dependencies': ability_manager.get_attribute_dependencies(),
            'biography': biography,
            'point_summary': ability_manager.get_ability_points_summary()
        }

    def validate_character(self) -> Dict[str, Any]:
        """Validate character data across all managers for corruption prevention."""
        all_errors = []
        all_warnings = []
        manager_errors = {}
        corruption_risks = []

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

        try:
            required_fields = ['FirstName', 'Race', 'ClassList']
            for field in required_fields:
                if self.gff.get(field) is None:
                    all_errors.append(f"Missing critical field: {field}")
                    corruption_risks.append(f"Missing {field} could cause game crashes")

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

            for ability in ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']:
                value = self.gff.get(ability, None)
                if value is None:
                    all_errors.append(f"Missing ability score: {ability}")
                elif not isinstance(value, int) or value < 1 or value > 255:
                    all_warnings.append(f"Unusual {ability} value: {value}")

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
