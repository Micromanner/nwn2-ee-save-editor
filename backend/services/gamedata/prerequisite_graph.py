"""Pre-computed feat prerequisite dependency graph using Rust for fast validation."""

import time
from typing import Dict, Set, Optional, Any, List, Tuple
from loguru import logger
from nwn2_rust import PrerequisiteGraph as create_prerequisite_graph


class PrerequisiteGraph:
    """Pre-computed feat prerequisite dependency graph using Rust."""

    def __init__(self, rules_service):
        """Initialize the graph from feat table data."""
        self.rules_service = rules_service
        self.is_built = False
        self._init_rust_implementation()

    def _init_rust_implementation(self):
        """Build the Rust-based prerequisite graph from feat data."""
        logger.info("Initializing Rust PrerequisiteGraph...")
        
        self.rust_graph = create_prerequisite_graph()
        
        feat_table = self.rules_service.get_table('feat')
        if not feat_table:
            raise RuntimeError("Feat table not found in game data")
        
        feat_data = []
        for feat in feat_table:
            feat_dict = {}
            if isinstance(feat, dict):
                feat_dict = feat
            else:
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
                    value = -1
                    if hasattr(feat, nwn2_field):
                        value = getattr(feat, nwn2_field)
                    elif hasattr(feat, nwn2_field.lower()):
                        value = getattr(feat, nwn2_field.lower())
                    else:
                        clean_name = nwn2_field.lstrip('_')
                        snake_name = clean_name.lower()
                        if 'prereqfeat' in snake_name:
                            snake_name = snake_name.replace('prereqfeat', 'prereq_feat')
                        elif 'minstr' in snake_name: snake_name = 'min_str'
                        elif 'mindex' in snake_name: snake_name = 'min_dex'
                        elif 'mincon' in snake_name: snake_name = 'min_con'
                        elif 'minint' in snake_name: snake_name = 'min_int'
                        elif 'minwis' in snake_name: snake_name = 'min_wis'
                        elif 'mincha' in snake_name: snake_name = 'min_cha'
                        elif 'minlevel' in snake_name: snake_name = 'min_level'
                        elif 'minattackbonus' in snake_name: snake_name = 'prereq_bab'
                        elif 'minspelllvl' in snake_name: snake_name = 'prereq_spell_level'

                        if hasattr(feat, snake_name):
                            value = getattr(feat, snake_name)

                    if isinstance(value, str) and value.strip() == '':
                        value = -1
                    elif isinstance(value, str):
                        try:
                            value = int(value)
                        except (ValueError, TypeError):
                            value = -1
                    feat_dict[rust_field] = value
            feat_data.append(feat_dict)
        
        start_time = time.time()
        self.rust_graph.build_from_data(feat_data)
        build_time = time.time() - start_time
        
        rust_stats = self.rust_graph.get_statistics()
        
        self.is_built = rust_stats['is_built']
        self.build_time = build_time
        
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
        """Return all recursive feat prerequisites for a feat."""
        if not self.is_built:
            raise RuntimeError("Prerequisite graph not built")
        return set(self.rust_graph.get_all_feat_requirements(feat_id))

    def get_direct_prerequisites(self, feat_id: int) -> Dict[str, Any]:
        """Return immediate prerequisites without recursion."""
        if not self.is_built:
            raise RuntimeError("Prerequisite graph not built")
        return self.rust_graph.get_direct_prerequisites(feat_id)

    def validate_feat_prerequisites_fast(
        self,
        feat_id: int,
        character_feats: Set[int],
        character_data: Optional[Dict[str, Any]] = None
    ) -> Tuple[bool, List[str]]:
        """Check if a character meets all prerequisites for a feat."""
        if not self.is_built:
            raise RuntimeError("Prerequisite graph not built")
        return self.rust_graph.validate_feat_prerequisites_fast(
            feat_id, character_feats, character_data
        )

    def validate_batch_fast(
        self,
        feat_ids: List[int],
        character_feats: Set[int],
        character_data: Optional[Dict[str, Any]] = None
    ) -> Dict[int, Tuple[bool, List[str]]]:
        """Validate prerequisites for multiple feats in a single call."""
        if not self.is_built:
            raise RuntimeError("Prerequisite graph not built")
        return self.rust_graph.validate_batch_fast(
            feat_ids, character_feats, character_data
        )

    def get_statistics(self) -> Dict[str, Any]:
        """Return build statistics and graph metrics."""
        return {
            'is_built': self.is_built,
            'build_time': getattr(self, 'build_time', 0.0),
            'total_feats': self.stats['total_feats'],
            'feats_with_prerequisites': self.stats['feats_with_prereqs'],
            'max_chain_depth': self.stats['max_chain_depth'],
            'circular_dependencies_count': len(self.stats['circular_dependencies']),
            'memory_estimate_mb': 0.0
        }


_prerequisite_graph_instance: Optional[PrerequisiteGraph] = None


def get_prerequisite_graph(rules_service, force_rebuild: bool = False) -> PrerequisiteGraph:
    """Return or create the singleton PrerequisiteGraph instance."""
    global _prerequisite_graph_instance
    
    if _prerequisite_graph_instance and not force_rebuild:
        return _prerequisite_graph_instance
    
    if rules_service is None:
        raise ValueError("Cannot create PrerequisiteGraph without rules_service")
    
    logger.info("Creating PrerequisiteGraph singleton...")
    _prerequisite_graph_instance = PrerequisiteGraph(rules_service)
    
    return _prerequisite_graph_instance


def clear_prerequisite_graph():
    """Reset the singleton instance to None."""
    global _prerequisite_graph_instance
    _prerequisite_graph_instance = None
    logger.info("PrerequisiteGraph singleton cleared")
