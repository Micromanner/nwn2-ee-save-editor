"""
Prerequisite Graph for optimized feat validation.

Pre-computes all feat prerequisite chains at startup,
enabling O(1) validation instead of recursive checking.
"""
import logging
import time
from typing import Dict, Set, Optional, Any, List, Tuple

logger = logging.getLogger(__name__)

# Import Rust implementation
from rust_extensions.python.nwn2_rust_extensions.nwn2_rust_wrapper import create_prerequisite_graph


class PrerequisiteGraph:
    """
    Pre-computed feat prerequisite dependency graph using Rust.
    
    Flattens all feat prerequisite chains at initialization,
    converting recursive prerequisite checking into simple set operations.
    """
    
    def __init__(self, game_data_loader):
        """
        Initialize and build the prerequisite graph using Rust.
        
        Args:
            game_data_loader: DynamicGameDataLoader instance with feat data
        """
        self.game_data_loader = game_data_loader
        self.using_rust = True  # Always true now
        self._init_rust_implementation()
    
    def _init_rust_implementation(self):
        """Initialize using Rust implementation"""
        logger.info("Initializing Rust PrerequisiteGraph...")
        
        # Create Rust graph
        self.rust_graph = create_prerequisite_graph()
        
        # Get feat table
        feat_table = self.game_data_loader.get_table('feat')
        if not feat_table:
            raise ValueError("No feat table found in game data!")
        
        # Convert feat table to format expected by Rust
        feat_data = []
        for feat in feat_table:
            feat_dict = {}
            # Extract fields - handle both dict and object access
            if isinstance(feat, dict):
                feat_dict = feat
            else:
                # Map from NWN2 field names to Rust field names
                field_mapping = {
                    '_PREREQFEAT1': 'prereqfeat1',
                    '_PREREQFEAT2': 'prereqfeat2', 
                    '_MINSTR': 'minstr',
                    '_MINDEX': 'mindex',
                    '_MINCON': 'mincon',
                    '_MININT': 'minint',
                    '_MINWIS': 'minwis',
                    '_MINCHA': 'mincha',
                    '_MinLevel': 'minlevel',
                    '_MINATTACKBONUS': 'minattackbonus',
                    '_MINSPELLLVL': 'minspelllvl'
                }
                
                for nwn2_field, rust_field in field_mapping.items():
                    if hasattr(feat, nwn2_field):
                        value = getattr(feat, nwn2_field)
                        # Handle empty strings and invalid values
                        if isinstance(value, str) and value.strip() == '':
                            value = -1
                        elif isinstance(value, str):
                            try:
                                value = int(value)
                            except:
                                value = -1
                        feat_dict[rust_field] = value
                    else:
                        feat_dict[rust_field] = -1
            feat_data.append(feat_dict)
        
        # Build the graph in Rust
        start_time = time.time()
        self.rust_graph.build_from_feat_table(feat_data)
        build_time = time.time() - start_time
        
        # Get statistics from Rust
        rust_stats = self.rust_graph.get_statistics()
        
        # Set attributes
        self.is_built = rust_stats['is_built']
        self.build_time = build_time
        self.using_rust = True
        
        self.stats = {
            'total_feats': rust_stats['total_feats'],
            'feats_with_prereqs': rust_stats['feats_with_prerequisites'],
            'max_chain_depth': rust_stats['max_chain_depth'],
            'circular_dependencies': []
        }
        
        logger.info(
            f"Rust PrerequisiteGraph built in {build_time:.2f}s "
            f"(internal: {rust_stats['build_time_ms']:.1f}ms): "
            f"{self.stats['total_feats']} feats, "
            f"{self.stats['feats_with_prereqs']} with prerequisites"
        )
    
    def get_all_feat_requirements(self, feat_id: int) -> Set[int]:
        """
        Get all flattened feat requirements for a given feat.
        
        Args:
            feat_id: The feat to get requirements for
            
        Returns:
            Set of all required feat IDs (empty if no requirements)
        """
        if not self.is_built:
            logger.warning("Prerequisite graph not built yet!")
            return set()
        
        return set(self.rust_graph.get_all_feat_requirements(feat_id))
    
    def get_direct_prerequisites(self, feat_id: int) -> Dict[str, Any]:
        """
        Get only the direct prerequisites for a feat.
        
        Args:
            feat_id: The feat to get direct prerequisites for
            
        Returns:
            Dict with direct prerequisites (abilities, feats, class, level, etc.)
        """
        if not self.is_built:
            logger.warning("Prerequisite graph not built yet!")
            return {'abilities': {}, 'feats': [], 'class': None, 'level': 0, 'bab': 0, 'spell_level': 0}
        
        return self.rust_graph.get_direct_prerequisites(feat_id)
    
    def validate_feat_prerequisites_fast(
        self,
        feat_id: int,
        character_feats: Set[int],
        character_data: Optional[Dict[str, Any]] = None
    ) -> Tuple[bool, List[str]]:
        """
        Fast validation using pre-computed graph.
        
        Args:
            feat_id: The feat to validate
            character_feats: Set of feat IDs the character has
            character_data: Optional dict with character abilities, level, etc.
            
        Returns:
            Tuple of (is_valid, list_of_missing_requirements)
        """
        if not self.is_built:
            logger.warning("Prerequisite graph not built")
            return True, []  # Allow by default if graph not ready
        
        return self.rust_graph.validate_feat_prerequisites_fast(
            feat_id, character_feats, character_data
        )
    
    def validate_batch_fast(
        self,
        feat_ids: List[int],
        character_feats: Set[int],
        character_data: Optional[Dict[str, Any]] = None
    ) -> Dict[int, Tuple[bool, List[str]]]:
        """
        Validate multiple feats at once using the pre-computed graph.
        
        Args:
            feat_ids: List of feat IDs to validate
            character_feats: Set of feat IDs the character has
            character_data: Optional dict with character abilities, level, etc.
            
        Returns:
            Dict mapping feat_id to (is_valid, list_of_errors)
        """
        return self.rust_graph.validate_batch_fast(
            feat_ids, character_feats, character_data
        )
    
    def get_statistics(self) -> Dict[str, Any]:
        """
        Get statistics about the prerequisite graph.
        
        Returns:
            Dict with graph statistics
        """
        return {
            'is_built': self.is_built,
            'build_time': self.build_time,
            'total_feats': self.stats['total_feats'],
            'feats_with_prerequisites': self.stats['feats_with_prereqs'],
            'max_chain_depth': self.stats['max_chain_depth'],
            'circular_dependencies_count': len(self.stats['circular_dependencies']),
            'memory_estimate_mb': 0.0  # Handled by Rust
        }


# Singleton storage
_prerequisite_graph_instance: Optional[PrerequisiteGraph] = None


def get_prerequisite_graph(game_data_loader=None, force_rebuild: bool = False) -> Optional[PrerequisiteGraph]:
    """
    Get the singleton PrerequisiteGraph instance.
    
    Args:
        game_data_loader: DynamicGameDataLoader to use (required on first call)
        force_rebuild: If True, rebuild the graph even if it exists
        
    Returns:
        PrerequisiteGraph instance or None if not available
    """
    global _prerequisite_graph_instance
    
    if _prerequisite_graph_instance and not force_rebuild:
        return _prerequisite_graph_instance
    
    if game_data_loader is None:
        logger.warning("Cannot create PrerequisiteGraph without game_data_loader")
        return None
    
    logger.info("Creating PrerequisiteGraph singleton...")
    _prerequisite_graph_instance = PrerequisiteGraph(game_data_loader)
    
    return _prerequisite_graph_instance


def clear_prerequisite_graph():
    """Clear the cached prerequisite graph."""
    global _prerequisite_graph_instance
    _prerequisite_graph_instance = None
    logger.info("PrerequisiteGraph singleton cleared")