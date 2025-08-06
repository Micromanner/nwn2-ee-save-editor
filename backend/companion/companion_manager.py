"""
Central CompanionManager orchestrating all companion modifications
Based on CharacterManager pattern but adapted for companions

OUTDATED!!!!!!!!!!!!!!!!!!
"""

from typing import Dict, List, Any, Optional, Type, TypeVar, Union, overload, Callable
import copy
import time
import logging
from dataclasses import dataclass

from character.events import EventEmitter, EventData
from character.character_manager import GFFDataWrapper, Transaction
from gamedata.services.game_rules_service import GameRulesService
from gamedata.middleware import get_game_rules_service

logger = logging.getLogger(__name__)


class CompanionManager(EventEmitter):
    """
    Central manager for all companion modifications
    Coordinates between subsystem managers and ensures consistency
    """
    
    def __init__(self, companion_data: Dict[str, Any], vanilla_rules=None, gff_element=None):
        """
        Initialize the companion manager
        
        Args:
            companion_data: Raw GFF companion data (.ros format)
            vanilla_rules: Game rules service (defaults to global)
            gff_element: Optional GFFElement for direct updates
        """
        super().__init__()
        self.companion_data = companion_data
        
        # Use DirectGFFWrapper if gff_element is provided
        if gff_element:
            from character.gff_direct_wrapper import DirectGFFWrapper
            self.gff = DirectGFFWrapper(gff_element)
            self.gff_element = gff_element
        else:
            self.gff = GFFDataWrapper(companion_data)
            self.gff_element = None
            
        self.vanilla_rules = vanilla_rules or get_game_rules_service() or GameRulesService()
        
        # Manager registry - companions can reuse many character managers
        self._managers: Dict[str, Any] = {}
        self._manager_classes: Dict[str, Type] = {}
        
        # Custom content tracking
        self.custom_content: Dict[str, Dict[str, Any]] = {}
        
        # Transaction support
        self._current_transaction: Optional[Transaction] = None
        self._transaction_history: List[Transaction] = []
        
        # NPC-specific data tracking
        self.npc_data = {
            'scripts': self._extract_scripts(),
            'ai_data': self._extract_ai_data(),
            'companion_specific': self._extract_companion_specific()
        }
        
        # Initialize
        self._detect_custom_content()
        logger.info(f"CompanionManager initialized for {self._get_companion_name()}")
    
    def register_manager(self, name: str, manager_class: Type, 
                        on_register: Optional[Callable] = None,
                        on_unregister: Optional[Callable] = None):
        """
        Register a subsystem manager - can reuse character managers
        
        Args:
            name: Manager name (e.g., 'class', 'feat', 'inventory')
            manager_class: Manager class to instantiate
            on_register: Optional callback after registration
            on_unregister: Optional callback before unregistration
        """
        self._manager_classes[name] = manager_class
        # Instantiate manager with reference to this CompanionManager
        manager_instance = manager_class(self)
        self._managers[name] = manager_instance
        
        # Store lifecycle hooks
        if not hasattr(self, '_manager_hooks'):
            self._manager_hooks = {}
        self._manager_hooks[name] = {
            'on_register': on_register,
            'on_unregister': on_unregister
        }
        
        # Call registration hook if provided
        if on_register:
            try:
                on_register(manager_instance)
            except Exception as e:
                logger.error(f"Error in on_register hook for {name}: {e}")
        
        logger.info(f"Registered {name} manager for companion")
    
    def get_manager(self, name: str):
        """Get a registered manager by name"""
        return self._managers.get(name)
    
    def _extract_scripts(self) -> Dict[str, str]:
        """Extract all script references from companion"""
        script_fields = [
            'ScriptAttacked', 'ScriptDamaged', 'ScriptDeath', 'ScriptDialogue',
            'ScriptDisturbed', 'ScriptEndRound', 'ScriptHeartbeat', 'ScriptOnBlocked',
            'ScriptOnNotice', 'ScriptRested', 'ScriptSpawn', 'ScriptSpellAt',
            'ScriptUserDefine'
        ]
        
        scripts = {}
        for field in script_fields:
            value = self.gff.get(field, '')
            if value:
                scripts[field] = value
        return scripts
    
    def _extract_ai_data(self) -> Dict[str, Any]:
        """Extract AI-related data"""
        return {
            'action_list': self.gff.get('ActionList', []),
            'combat_info': self.gff.get('CombatInfo', {}),
            'combat_mode': self.gff.get('CombatMode', 0),
            'combat_round_data': self.gff.get('CombatRoundData', {}),
            'perception_list': self.gff.get('PerceptionList', []),
            'perception_range': self.gff.get('PerceptionRange', 10.0),
            'is_commandable': self.gff.get('IsCommandable', 1),
            'block_combat': self.gff.get('BlockCombat', 0),
            'ignore_target': self.gff.get('IgnoreTarget', 0)
        }
    
    def _extract_companion_specific(self) -> Dict[str, Any]:
        """Extract companion-specific fields"""
        return {
            'roster_tag': self.gff.get('RosterTag', ''),
            'roster_member': self.gff.get('RosterMember', 1),
            'conversation': self.gff.get('Conversation', ''),
            'faction_id': self.gff.get('FactionID', 2),  # Usually 2 for party members
            'personal_rep_list': self.gff.get('PersonalRepList', []),
            'expression_list': self.gff.get('ExpressionList', []),
            'hotbar_list': self.gff.get('HotbarList', [])
        }
    
    def _get_companion_name(self) -> str:
        """Extract companion name from localized string structure"""
        first_name = self.gff.get('FirstName', {})
        last_name = self.gff.get('LastName', {})
        roster_tag = self.gff.get('RosterTag', '')
        
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
        
        # Include roster tag for clarity
        if roster_tag and roster_tag != full_name:
            return f"{full_name} ({roster_tag})"
        return full_name if full_name and full_name != " " else roster_tag
    
    def get_companion_summary(self) -> Dict[str, Any]:
        """Get a summary of the companion's current state"""
        return {
            'name': self._get_companion_name(),
            'roster_tag': self.gff.get('RosterTag', ''),
            'level': sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', []) if isinstance(c, dict)),
            'classes': [
                {
                    'class_id': c.get('Class', 0),
                    'level': c.get('ClassLevel', 0),
                    'name': getattr(self.vanilla_rules.classes.get(c.get('Class', 0), {}), 'label', 'Unknown')
                }
                for c in self.gff.get('ClassList', [])
                if isinstance(c, dict)
            ],
            'race': self.vanilla_rules.races.get(self.gff.get('Race', 0), {}).get('name', 'Unknown'),
            'alignment': {
                'law_chaos': self.gff.get('LawfulChaotic', 50),
                'good_evil': self.gff.get('GoodEvil', 50)
            },
            'abilities': {
                'STR': self.gff.get('Str', 10),
                'DEX': self.gff.get('Dex', 10),
                'CON': self.gff.get('Con', 10),
                'INT': self.gff.get('Int', 10),
                'WIS': self.gff.get('Wis', 10),
                'CHA': self.gff.get('Cha', 10)
            },
            'npc_specific': {
                'has_scripts': len(self.npc_data['scripts']) > 0,
                'script_count': len(self.npc_data['scripts']),
                'has_conversation': bool(self.npc_data['companion_specific']['conversation']),
                'faction': self.npc_data['companion_specific']['faction_id']
            },
            'custom_content_count': len(self.custom_content)
        }
    
    def update_script(self, script_name: str, script_resref: str):
        """
        Update a companion script
        
        Args:
            script_name: Script field name (e.g., 'ScriptHeartbeat')
            script_resref: New script resource reference
        """
        if script_name not in self.npc_data['scripts']:
            logger.warning(f"Unknown script field: {script_name}")
            return
        
        old_value = self.gff.get(script_name, '')
        self.gff.set(script_name, script_resref)
        self.npc_data['scripts'][script_name] = script_resref
        
        # Record change if in transaction
        if self._current_transaction:
            self._current_transaction.add_change('script_update', {
                'field': script_name,
                'old_value': old_value,
                'new_value': script_resref
            })
        
        # Emit event
        self.emit_event('script_changed', EventData(
            event_type='script_changed',
            source_manager='companion',
            data={
                'script_name': script_name,
                'old_value': old_value,
                'new_value': script_resref
            }
        ))
    
    def update_conversation(self, conversation_resref: str):
        """Update companion's conversation file"""
        old_value = self.gff.get('Conversation', '')
        self.gff.set('Conversation', conversation_resref)
        self.npc_data['companion_specific']['conversation'] = conversation_resref
        
        if self._current_transaction:
            self._current_transaction.add_change('conversation_update', {
                'old_value': old_value,
                'new_value': conversation_resref
            })
    
    def preserve_npc_data(self) -> Dict[str, Any]:
        """
        Get all NPC-specific data that should be preserved during export
        This ensures AI behavior and companion functionality remains intact
        """
        return {
            'scripts': self.npc_data['scripts'],
            'ai': self.npc_data['ai_data'],
            'companion': self.npc_data['companion_specific'],
            'effect_list': self.gff.get('EffectList', []),
            'var_table': self.gff.get('VarTable', [])
        }
    
    def _detect_custom_content(self):
        """Detect non-vanilla content in the companion"""
        self.custom_content = {}
        
        # Check feats - same as character
        feat_list = self.gff.get('FeatList', [])
        for i, feat in enumerate(feat_list):
            feat_id = feat.get('Feat', 0)
            if feat_id > 10000 or feat_id not in self.vanilla_rules.feats:
                self.custom_content[f'feat_{feat_id}'] = {
                    'type': 'feat',
                    'id': feat_id,
                    'index': i,
                    'protected': True,
                    'companion': self._get_companion_name()
                }
        
        # Check scripts for custom content
        for script_name, script_ref in self.npc_data['scripts'].items():
            if script_ref and not script_ref.startswith('nw_'):  # Non-standard scripts
                self.custom_content[f'script_{script_ref}'] = {
                    'type': 'script',
                    'name': script_ref,
                    'field': script_name,
                    'protected': True,
                    'companion': self._get_companion_name()
                }
    
    # Transaction methods - can reuse from CharacterManager
    def begin_transaction(self) -> Transaction:
        """Start a new transaction for atomic changes"""
        if self._current_transaction:
            raise RuntimeError("Transaction already in progress")
            
        self._current_transaction = Transaction(self)
        logger.info(f"Started transaction {self._current_transaction.id} for companion")
        return self._current_transaction
    
    def commit_transaction(self) -> Dict[str, Any]:
        """Commit the current transaction"""
        if not self._current_transaction:
            raise RuntimeError("No transaction in progress")
            
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
    
    def validate_changes(self, preview: bool = False) -> tuple[bool, List[str]]:
        """
        Validate all pending changes
        
        Args:
            preview: If True, validate without applying
            
        Returns:
            (is_valid, list_of_errors)
        """
        errors = []
        
        # Basic companion validation
        roster_tag = self.gff.get('RosterTag', '')
        if not roster_tag:
            errors.append("Companion must have a RosterTag")
        
        # Ensure critical NPC fields are preserved
        if not self.gff.get('IsCommandable', 1):
            errors.append("Warning: Companion is not commandable")
        
        # Let each manager validate its state
        for name, manager in self._managers.items():
            if hasattr(manager, 'validate'):
                is_valid, manager_errors = manager.validate()
                if not is_valid:
                    errors.extend([f"{name}: {e}" for e in manager_errors])
        
        return len(errors) == 0, errors