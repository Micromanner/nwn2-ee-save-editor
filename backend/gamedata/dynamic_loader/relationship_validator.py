"""
Relationship Validator - Validates foreign key relationships between 2DA tables

This module provides automatic detection and validation of relationships between
2DA tables, enabling referential integrity checks and load order optimization.
"""
import logging
from typing import Dict, List, Set, Tuple, Optional, Any, Type
from dataclasses import dataclass, field
from collections import defaultdict
from enum import Enum

logger = logging.getLogger(__name__)


class RelationshipType(Enum):
    """Types of relationships between tables."""
    ONE_TO_ONE = "one_to_one"
    ONE_TO_MANY = "one_to_many"
    LOOKUP = "lookup"  # Simple foreign key reference
    TABLE_REFERENCE = "table_reference"  # Column contains table name


@dataclass
class RelationshipDefinition:
    """
    Defines a relationship between two tables.
    
    Attributes:
        source_table: Name of the source table
        source_column: Column in source table containing the reference
        target_table: Name of the target table
        relationship_type: Type of relationship
        is_nullable: Whether the relationship allows null/invalid references
        validation_errors: List of validation errors found
    """
    source_table: str
    source_column: str
    target_table: str
    relationship_type: RelationshipType
    is_nullable: bool = True
    validation_errors: List[str] = field(default_factory=list)
    
    def __hash__(self):
        """Make hashable for use in sets."""
        return hash((self.source_table, self.source_column, self.target_table))
    
    def __str__(self):
        """Human-readable representation."""
        return f"{self.source_table}.{self.source_column} -> {self.target_table}"


@dataclass
class ValidationReport:
    """
    Report of relationship validation results.
    
    Attributes:
        total_relationships: Total number of relationships found
        valid_relationships: Number of valid relationships
        broken_references: List of broken reference details
        missing_tables: Set of referenced tables that don't exist
        dependency_order: Suggested table loading order
    """
    total_relationships: int = 0
    valid_relationships: int = 0
    broken_references: List[Dict[str, Any]] = field(default_factory=list)
    missing_tables: Set[str] = field(default_factory=set)
    dependency_order: List[str] = field(default_factory=list)
    
    def add_broken_reference(self, source_table: str, source_column: str, 
                           source_row: int, target_table: str, target_id: Any):
        """Add a broken reference to the report."""
        self.broken_references.append({
            'source_table': source_table,
            'source_column': source_column,
            'source_row': source_row,
            'target_table': target_table,
            'target_id': target_id,
            'error': f"Row {source_row} in {source_table}.{source_column} references non-existent {target_table} ID {target_id}"
        })
    
    def get_summary(self) -> str:
        """Get a summary of the validation report."""
        lines = [
            f"Relationship Validation Report:",
            f"- Total relationships: {self.total_relationships}",
            f"- Valid relationships: {self.valid_relationships}",
            f"- Broken references: {len(self.broken_references)}",
            f"- Missing tables: {len(self.missing_tables)}"
        ]
        
        if self.missing_tables:
            lines.append(f"- Missing tables: {', '.join(sorted(self.missing_tables))}")
        
        if self.broken_references:
            lines.append("\nFirst 5 broken references:")
            for ref in self.broken_references[:5]:
                lines.append(f"  - {ref['error']}")
        
        return "\n".join(lines)


class RelationshipValidator:
    """
    Validates relationships between 2DA tables.
    
    This validator:
    - Detects foreign key relationships automatically
    - Validates referential integrity
    - Builds dependency graphs for load optimization
    - Generates validation reports
    """
    
    NULL_VALUE = '****'
    
    def __init__(self, rule_detector: Optional[Any] = None):
        """
        Initialize the relationship validator.
        
        Args:
            rule_detector: Optional RuleDetector instance for pattern matching
        """
        self.rule_detector = rule_detector
        self.relationships: Set[RelationshipDefinition] = set()
        self.table_data: Dict[str, List[Any]] = {}
        self._dependency_graph: Dict[str, Set[str]] = defaultdict(set)
    
    def detect_relationships(self, table_data: Dict[str, List[Any]]) -> Set[RelationshipDefinition]:
        """
        Automatically detect relationships between tables.
        
        Args:
            table_data: Dictionary of table_name -> list of data instances
            
        Returns:
            Set of detected relationships
        """
        self.table_data = table_data
        self.relationships.clear()
        
        for table_name, instances in table_data.items():
            if not instances:
                continue
            
            # Get first instance to check available attributes
            first_instance = instances[0]
            
            # Check each attribute for potential relationships
            # get_safe_columns is a class method
            if hasattr(first_instance.__class__, 'get_safe_columns'):
                for attr_name in first_instance.__class__.get_safe_columns():
                    self._check_attribute_for_relationship(table_name, attr_name)
            elif hasattr(first_instance, '_safe_columns'):
                for attr_name in first_instance._safe_columns:
                    self._check_attribute_for_relationship(table_name, attr_name)
        
        logger.info(f"Detected {len(self.relationships)} relationships across {len(table_data)} tables")
        return self.relationships
    
    def _check_attribute_for_relationship(self, table_name: str, column_name: str):
        """Check if a column represents a relationship to another table."""
        # Use rule detector if available
        if self.rule_detector:
            # Check if column matches reference patterns
            column_purpose = self._get_column_purpose(table_name, column_name)
            
            if column_purpose == 'feats_table':
                # Column contains a table name
                self._add_table_reference(table_name, column_name, 'feat')
            elif column_purpose == 'skills_table':
                self._add_table_reference(table_name, column_name, 'skill')
            elif column_purpose == 'saving_throw_table':
                self._add_table_reference(table_name, column_name, 'savthr')
            elif column_purpose in ['spell_id', 'feat_index', 'class_id', 'skill_index']:
                # Direct ID reference
                target_table = column_purpose.replace('_id', '').replace('_index', '')
                # Map to actual table names
                table_map = {
                    'spell': 'spells',
                    'feat': 'feat',
                    'class': 'classes',
                    'skill': 'skills'
                }
                target_table = table_map.get(target_table, target_table)
                self._add_lookup_reference(table_name, column_name, target_table)
        
        # Pattern-based detection (fallback or additional)
        column_lower = column_name.lower()
        
        # Check for ID references (e.g., ClassID, FavoredClass)
        if column_lower.endswith('id') or column_lower.endswith('_id'):
            potential_table = column_lower.replace('_id', '').replace('id', '')
            if potential_table in self.table_data:
                self._add_lookup_reference(table_name, column_name, potential_table)
        
        # Check for table references (e.g., FeatsTable, SkillsTable)
        elif column_lower.endswith('table'):
            # This column likely contains a table name
            self._add_table_reference(table_name, column_name, None)
        
        # Special NWN2 patterns
        elif column_lower == 'favoredclass':
            self._add_lookup_reference(table_name, column_name, 'classes')
        elif column_lower == 'weapontype':
            self._add_lookup_reference(table_name, column_name, 'weapontypes')
        elif column_lower.startswith('prereqfeat') or column_lower.startswith('orfeat'):
            self._add_lookup_reference(table_name, column_name, 'feat')
        elif column_lower == 'reqskill' or column_lower.startswith('reqskill'):
            self._add_lookup_reference(table_name, column_name, 'skills')
    
    def _get_column_purpose(self, table_name: str, column_name: str) -> Optional[str]:
        """Get the purpose of a column from rule detector."""
        if not self.rule_detector:
            return None
        
        # Use the rule detector's get_column_purpose method
        if hasattr(self.rule_detector, 'get_column_purpose'):
            return self.rule_detector.get_column_purpose(table_name, column_name)
        
        return None
    
    def _add_lookup_reference(self, source_table: str, source_column: str, 
                             target_table: str):
        """Add a simple lookup reference."""
        rel = RelationshipDefinition(
            source_table=source_table,
            source_column=source_column,
            target_table=target_table,
            relationship_type=RelationshipType.LOOKUP
        )
        self.relationships.add(rel)
        self._dependency_graph[source_table].add(target_table)
    
    def _add_table_reference(self, source_table: str, source_column: str, 
                           hint_table: Optional[str]):
        """Add a table reference (column contains table name)."""
        # For table references, we need to check actual values
        instances = self.table_data.get(source_table, [])
        if not instances:
            return
        
        # Check first few instances to find referenced tables
        referenced_tables = set()
        for instance in instances[:10]:  # Sample first 10
            value = getattr(instance, source_column, None)
            if value and str(value) != self.NULL_VALUE:
                # Clean table name (remove .2da extension if present)
                table_ref = str(value).lower().replace('.2da', '')
                if table_ref in self.table_data:
                    referenced_tables.add(table_ref)
        
        # Add relationships for each referenced table
        for target_table in referenced_tables:
            rel = RelationshipDefinition(
                source_table=source_table,
                source_column=source_column,
                target_table=target_table,
                relationship_type=RelationshipType.TABLE_REFERENCE
            )
            self.relationships.add(rel)
            self._dependency_graph[source_table].add(target_table)
    
    def validate_relationships(self, strict: bool = False) -> ValidationReport:
        """
        Validate all detected relationships.
        
        Args:
            strict: If True, treat invalid references as errors
            
        Returns:
            Validation report with results
        """
        report = ValidationReport()
        report.total_relationships = len(self.relationships)
        
        for relationship in self.relationships:
            if self._validate_single_relationship(relationship, report, strict):
                report.valid_relationships += 1
        
        # Calculate dependency order
        report.dependency_order = self._calculate_load_order()
        
        return report
    
    def _validate_single_relationship(self, rel: RelationshipDefinition, 
                                    report: ValidationReport, strict: bool) -> bool:
        """Validate a single relationship."""
        # Check if target table exists
        if rel.target_table not in self.table_data:
            report.missing_tables.add(rel.target_table)
            return False
        
        source_instances = self.table_data.get(rel.source_table, [])
        target_instances = self.table_data.get(rel.target_table, [])
        
        if not source_instances or not target_instances:
            return True  # Can't validate empty tables
        
        # Build target ID set for efficient lookup
        target_ids = set()
        for instance in target_instances:
            # Try common ID attributes
            for id_attr in ['id', 'ID', 'index', 'Index', '_id']:
                if hasattr(instance, id_attr):
                    target_ids.add(getattr(instance, id_attr))
                    break
        
        # Validate each source instance
        is_valid = True
        for i, source_instance in enumerate(source_instances):
            value = getattr(source_instance, rel.source_column, None)
            
            # Skip null values
            if value is None or str(value) == self.NULL_VALUE:
                continue
            
            # For table references, check if table exists
            if rel.relationship_type == RelationshipType.TABLE_REFERENCE:
                table_ref = str(value).lower().replace('.2da', '')
                if table_ref not in self.table_data:
                    report.add_broken_reference(
                        rel.source_table, rel.source_column, i,
                        'table', table_ref
                    )
                    is_valid = False
            else:
                # For ID references, check if ID exists
                try:
                    ref_id = int(value)
                    if ref_id not in target_ids and ref_id != -1:  # -1 often means "none"
                        report.add_broken_reference(
                            rel.source_table, rel.source_column, i,
                            rel.target_table, ref_id
                        )
                        is_valid = False
                except (ValueError, TypeError):
                    # Non-numeric reference
                    if strict:
                        is_valid = False
        
        return is_valid
    
    def _calculate_load_order(self) -> List[str]:
        """
        Calculate optimal table loading order based on dependencies.
        Uses topological sort to ensure referenced tables load first.
        """
        # Find tables with no dependencies
        all_tables = set(self.table_data.keys())
        tables_with_deps = set(self._dependency_graph.keys())
        no_deps = all_tables - tables_with_deps
        
        # Topological sort
        load_order = []
        visited = set()
        
        def visit(table: str):
            if table in visited:
                return
            visited.add(table)
            
            # Visit dependencies first
            for dep in self._dependency_graph.get(table, []):
                if dep in all_tables:  # Only visit existing tables
                    visit(dep)
            
            load_order.append(table)
        
        # Start with tables that have no dependencies
        for table in sorted(no_deps):
            visit(table)
        
        # Then visit remaining tables
        for table in sorted(all_tables - visited):
            visit(table)
        
        return load_order
    
    def get_table_dependencies(self, table_name: str) -> Set[str]:
        """Get all tables that a given table depends on."""
        deps = set()
        
        for rel in self.relationships:
            if rel.source_table == table_name:
                deps.add(rel.target_table)
        
        return deps
    
    def get_table_dependents(self, table_name: str) -> Set[str]:
        """Get all tables that depend on a given table."""
        dependents = set()
        
        for rel in self.relationships:
            if rel.target_table == table_name:
                dependents.add(rel.source_table)
        
        return dependents
    
    def generate_dot_graph(self) -> str:
        """
        Generate a Graphviz DOT representation of table relationships.
        Useful for visualization.
        """
        lines = ["digraph TableRelationships {"]
        lines.append('  rankdir=LR;')
        lines.append('  node [shape=box];')
        
        # Add nodes
        for table in sorted(self.table_data.keys()):
            lines.append(f'  "{table}";')
        
        # Add edges
        for rel in sorted(self.relationships, key=str):
            style = 'dashed' if rel.relationship_type == RelationshipType.TABLE_REFERENCE else 'solid'
            lines.append(f'  "{rel.source_table}" -> "{rel.target_table}" '
                        f'[label="{rel.source_column}", style={style}];')
        
        lines.append("}")
        return "\n".join(lines)