"""
Event system for character management
Provides pub/sub pattern for communication between managers
"""

from typing import Dict, List, Callable, Any, Optional
from dataclasses import dataclass
from enum import Enum
import logging

logger = logging.getLogger(__name__)


class EventType(Enum):
    """Standard event types for character management"""
    CLASS_CHANGED = 'class_changed'
    CLASS_ADDED = 'class_added'  # For multiclassing
    LEVEL_GAINED = 'level_gained'
    FEAT_ADDED = 'feat_added'
    FEAT_REMOVED = 'feat_removed'
    SPELL_LEARNED = 'spell_learned'
    SPELL_FORGOTTEN = 'spell_forgotten'
    SPELLS_CHANGED = 'spells_changed'  # General spell system change
    SKILL_UPDATED = 'skill_updated'
    ITEM_EQUIPPED = 'item_equipped'
    ITEM_UNEQUIPPED = 'item_unequipped'
    ABILITY_CHANGED = 'ability_changed'
    ATTRIBUTE_CHANGED = 'attribute_changed'
    ALIGNMENT_CHANGED = 'alignment_changed'
    STATE_CHANGED = 'state_changed'  # Generic state change event for cache invalidation


@dataclass
class EventData:
    """Base class for event data"""
    event_type: EventType
    source_manager: str
    timestamp: float
    
    def validate(self) -> bool:
        """Validate event data"""
        return True


@dataclass
class ClassChangedEvent(EventData):
    """Data for class change events"""
    old_class_id: Optional[int]
    new_class_id: int
    level: int
    preserve_feats: List[int] = None  # Feats to keep
    
    def __post_init__(self):
        self.event_type = EventType.CLASS_CHANGED
        if self.preserve_feats is None:
            self.preserve_feats = []


@dataclass
class FeatChangedEvent(EventData):
    """Data for feat add/remove events"""
    feat_id: int
    action: str  # 'added' or 'removed'
    source: str  # 'class', 'level', 'manual'
    
    def __post_init__(self):
        if self.action == 'added':
            self.event_type = EventType.FEAT_ADDED
        else:
            self.event_type = EventType.FEAT_REMOVED


@dataclass
class SpellChangedEvent(EventData):
    """Data for spell learning/forgetting events"""
    spell_id: int
    spell_level: int
    action: str  # 'learned' or 'forgotten'
    source: str  # 'class', 'feat', 'manual'
    
    def __post_init__(self):
        if self.action == 'learned':
            self.event_type = EventType.SPELL_LEARNED
        else:
            self.event_type = EventType.SPELL_FORGOTTEN


@dataclass
class LevelGainedEvent(EventData):
    """Data for level up events"""
    class_id: int
    new_level: int
    total_level: int
    
    def __post_init__(self):
        self.event_type = EventType.LEVEL_GAINED


class EventEmitter:
    """Base class for objects that can emit and listen to events"""
    
    def __init__(self):
        self._observers: Dict[EventType, List[Callable]] = {}
        self._event_history: List[EventData] = []
    
    def on(self, event_type: EventType, callback: Callable[[EventData], None]):
        """
        Register a callback for an event type
        
        Args:
            event_type: The type of event to listen for
            callback: Function to call when event is emitted
        """
        if event_type not in self._observers:
            self._observers[event_type] = []
        self._observers[event_type].append(callback)
        logger.debug(f"Registered callback for {event_type.value}")
    
    def off(self, event_type: EventType, callback: Callable[[EventData], None]):
        """
        Unregister a callback for an event type
        
        Args:
            event_type: The type of event
            callback: The callback to remove
        """
        if event_type in self._observers:
            try:
                self._observers[event_type].remove(callback)
                logger.debug(f"Unregistered callback for {event_type.value}")
            except ValueError:
                pass  # Callback not in list
    
    def emit(self, event_type, data=None):
        """
        Emit an event to all registered observers
        
        Args:
            event_type: EventType or EventData. If EventType, data should be provided
            data: Optional dict of event data if event_type is EventType
        """
        # Handle two different call styles for compatibility
        if isinstance(event_type, EventData):
            event_data = event_type
            if not event_data.validate():
                logger.error(f"Invalid event data for {event_data.event_type}")
                return
        else:
            # Create a simple event data object for non-dataclass events
            event_data = EventData(
                event_type=event_type,
                source_manager=getattr(self, '__class__', type(self)).__name__,
                timestamp=0
            )
            
        self._event_history.append(event_data)
        
        # For EventType enum, check in _observers
        event_key = event_data.event_type if isinstance(event_data.event_type, EventType) else event_type
        
        if event_key in self._observers:
            logger.info(f"Emitting {event_key.value if hasattr(event_key, 'value') else event_key} from {event_data.source_manager}")
            for callback in self._observers[event_key]:
                try:
                    # Call with event_data for new style, or data dict for old style
                    if isinstance(event_type, EventData):
                        callback(event_data)
                    else:
                        callback(data or {})
                except Exception as e:
                    logger.error(f"Error in event callback: {e}")
    
    def emit_batch(self, events: List[EventData]):
        """
        Emit multiple events in order
        
        Args:
            events: List of events to emit
        """
        for event in events:
            self.emit(event)
    
    def get_event_history(self, event_type: Optional[EventType] = None) -> List[EventData]:
        """
        Get history of emitted events
        
        Args:
            event_type: Optional filter by event type
            
        Returns:
            List of event data
        """
        if event_type:
            return [e for e in self._event_history if e.event_type == event_type]
        return self._event_history.copy()
    
    def clear_event_history(self):
        """Clear the event history"""
        self._event_history.clear()