"""
Central registry for all character managers.
Defines the standard set of managers and their registration order.
"""

from typing import Type, List, Tuple
from .managers import (
    AbilityManager,
    RaceManager,
    ClassManager,
    FeatManager,
    SkillManager,
    SpellManager,
    CombatManager,
    SaveManager,
    InventoryManager,
    ContentManager,
    CharacterStateManager
)

# Define all managers and their registration order
# Order matters for proper event setup - managers that emit events should come before listeners
MANAGER_REGISTRY: List[Tuple[str, Type]] = [
    # Core managers that others depend on
    ('ability', AbilityManager),  # Emits ATTRIBUTE_CHANGED events
    ('race', RaceManager),           # Provides racial modifiers
    ('class', ClassManager),         # Emits CLASS_CHANGED, LEVEL_GAINED events
    
    # Content detection and campaign data
    ('content', ContentManager),     # Detects custom content and extracts campaign data
    
    # Feature managers
    ('feat', FeatManager),           # Emits FEAT_ADDED/REMOVED events
    ('skill', SkillManager),         # Depends on attributes and class
    ('spell', SpellManager),         # Depends on class and attributes
    
    # Derived stat managers that listen to other events
    ('combat', CombatManager),       # Listens to attribute/class changes
    ('save', SaveManager),           # Listens to attribute/class/feat changes
    ('inventory', InventoryManager),  # May emit ITEM_EQUIPPED/UNEQUIPPED events
    
    # State management
    ('state', CharacterStateManager), # Manages character state changes and validation
]


def get_all_manager_specs() -> List[Tuple[str, Type]]:
    """
    Get all manager specifications for registration.
    
    Returns:
        List of (name, class) tuples in proper registration order
    """
    return MANAGER_REGISTRY.copy()


def get_manager_names() -> List[str]:
    """
    Get just the names of all registered managers.
    
    Returns:
        List of manager names
    """
    return [name for name, _ in MANAGER_REGISTRY]


def get_manager_class(name: str) -> Type:
    """
    Get the manager class for a given name.
    
    Args:
        name: Manager name
        
    Returns:
        Manager class or None if not found
    """
    for mgr_name, mgr_class in MANAGER_REGISTRY:
        if mgr_name == name:
            return mgr_class
    return None