"""Central registry for all character managers and their registration order."""

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
    CharacterStateManager,
    GameStateManager,
    IdentityManager
)

# Define all managers and their registration order
# Order matters for proper event setup - managers that emit events should come before listeners
MANAGER_REGISTRY: List[Tuple[str, Type]] = [
    # Core managers that others depend on
    ('ability', AbilityManager),     # Emits ATTRIBUTE_CHANGED events
    ('race', RaceManager),           # Provides racial modifiers
    ('class', ClassManager),         # Emits CLASS_CHANGED, LEVEL_GAINED events
    ('identity', IdentityManager),   # Character identity: name, age, background, XP

    # Content detection and campaign data
    ('content', ContentManager),     # Detects custom content and extracts campaign data (read-only)
    ('game_state', GameStateManager), # Edits game state (quests, reputation, influence)

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
    """Get all manager specifications in proper registration order."""
    return MANAGER_REGISTRY.copy()


def get_manager_names() -> List[str]:
    """Get names of all registered managers."""
    return [name for name, _ in MANAGER_REGISTRY]


def get_manager_class(name: str) -> Type:
    """Get the manager class for a given name, or None if not found."""
    for mgr_name, mgr_class in MANAGER_REGISTRY:
        if mgr_name == name:
            return mgr_class
    return None