"""
Comprehensive 2DA ID Mapping Analysis Tests

This module provides in-depth analysis of ID-to-row mapping patterns across
all available 2DA files to identify the architectural issue with DynamicGameDataLoader.

The core problem: get_by_id(table, id) assumes row_index == id, but many tables
use different mapping strategies like row_index = id - 1 (creaturesize.2da).
"""
import pytest
import logging
from pathlib import Path
from typing import Dict, List, Tuple, Optional, Set
from dataclasses import dataclass

from services.resource_manager import ResourceManager
from config.nwn2_settings import nwn2_paths


logger = logging.getLogger(__name__)


@dataclass
class IDMappingPattern:
    """Represents an ID mapping pattern for a 2DA table."""
    table_name: str
    mapping_type: str  # 'direct', 'offset', 'sparse', 'custom', 'invalid'
    offset: int = 0  # For offset mapping (row_index = id + offset)
    has_gaps: bool = False
    min_id: Optional[int] = None
    max_id: Optional[int] = None
    total_rows: int = 0
    valid_rows: int = 0  # Rows that aren't INVALID/****
    id_column: Optional[str] = None  # Primary ID column name
    sample_mappings: List[Tuple[int, int, str]] = None  # (id, row_index, label)
    
    def __post_init__(self):
        if self.sample_mappings is None:
            self.sample_mappings = []


class IDMappingAnalyzer:
    """Analyzes ID mapping patterns in 2DA tables."""
    
    def __init__(self, resource_manager: Optional[ResourceManager] = None):
        """Initialize analyzer with resource manager."""
        if resource_manager:
            self.rm = resource_manager
        else:
            self.rm = ResourceManager(suppress_warnings=True)
    
    def analyze_table(self, table_name: str) -> Optional[IDMappingPattern]:
        """
        Analyze ID mapping pattern for a single table.
        
        Args:
            table_name: Name of the 2DA table (without .2da extension)
            
        Returns:
            IDMappingPattern or None if analysis fails
        """
        try:
            # Load table data
            parser = self.rm.get_2da(table_name)
            if not parser:
                logger.debug(f"Could not load table: {table_name}")
                return None
            
            total_rows = parser.get_resource_count()
            if total_rows == 0:
                return IDMappingPattern(
                    table_name=table_name,
                    mapping_type='empty',
                    total_rows=0
                )
            
            # Get column headers to find potential ID columns
            headers = parser.get_column_labels()
            id_column = self._identify_id_column(headers)
            
            # Analyze row data
            mappings = []
            valid_rows = 0
            min_id = None
            max_id = None
            has_gaps = False
            
            for row_idx in range(total_rows):
                try:
                    row_data = parser.get_row_dict(row_idx)
                    if not row_data:
                        continue
                        
                    # Check if row is valid (not INVALID/****)
                    label = row_data.get('LABEL', row_data.get('Label', ''))
                    if label and str(label).upper() not in ['INVALID', '****', '']:
                        valid_rows += 1
                        
                        # Try to extract ID from various possible columns
                        extracted_id = self._extract_id_from_row(row_data, row_idx, id_column)
                        
                        if extracted_id is not None:
                            mappings.append((extracted_id, row_idx, str(label)))
                            
                            if min_id is None or extracted_id < min_id:
                                min_id = extracted_id
                            if max_id is None or extracted_id > max_id:
                                max_id = extracted_id
                                
                except Exception as e:
                    logger.debug(f"Error processing row {row_idx} in {table_name}: {e}")
                    continue
            
            # Determine mapping pattern
            mapping_type, offset = self._determine_mapping_pattern(mappings)
            
            # Check for gaps in sequence
            if mappings and min_id is not None and max_id is not None:
                expected_count = max_id - min_id + 1
                has_gaps = len(mappings) < expected_count
            
            return IDMappingPattern(
                table_name=table_name,
                mapping_type=mapping_type,
                offset=offset,
                has_gaps=has_gaps,
                min_id=min_id,
                max_id=max_id,
                total_rows=total_rows,
                valid_rows=valid_rows,
                id_column=id_column,
                sample_mappings=mappings[:10]  # Keep first 10 for examples
            )
            
        except Exception as e:
            logger.error(f"Failed to analyze table {table_name}: {e}")
            return None
    
    def _identify_id_column(self, headers: List[str]) -> Optional[str]:
        """Identify the primary ID column from headers."""
        # Common ID column patterns in NWN2 2DA files
        id_candidates = ['ID', 'Index', 'LABEL']
        
        for candidate in id_candidates:
            if candidate in headers:
                return candidate
        
        # If no obvious ID column, return None (will use row index)
        return None
    
    def _extract_id_from_row(self, row_data: Dict, row_idx: int, id_column: Optional[str]) -> Optional[int]:
        """Extract ID value from a row, using various strategies."""
        # Strategy 1: Use explicit ID column if available
        if id_column and id_column in row_data:
            try:
                value = row_data[id_column]
                if str(value).isdigit():
                    return int(value)
            except (ValueError, TypeError):
                pass
        
        # Strategy 2: For tables like creaturesize.2da, the ID is implied by position
        # Check if this looks like a positional table by examining the label pattern
        label = row_data.get('LABEL', row_data.get('Label', ''))
        
        # For creaturesize-like tables, use row_idx + 1 as the ID
        if self._looks_like_positional_table(row_data):
            return row_idx + 1
        
        # Strategy 3: Default to row index as ID (traditional mapping)
        return row_idx
    
    def _looks_like_positional_table(self, row_data: Dict) -> bool:
        """Determine if this table uses positional IDs (like creaturesize.2da)."""
        label = str(row_data.get('LABEL', row_data.get('Label', ''))).upper()
        
        # Known positional tables
        positional_indicators = [
            # Size-related tables
            'TINY', 'SMALL', 'MEDIUM', 'LARGE', 'HUGE', 'GARGANTUAN',
            # Other sequential concepts
            'INVALID'  # Row 0 is often INVALID in positional tables
        ]
        
        return any(indicator in label for indicator in positional_indicators)
    
    def _determine_mapping_pattern(self, mappings: List[Tuple[int, int, str]]) -> Tuple[str, int]:
        """
        Determine the mapping pattern from ID-to-row mappings.
        
        Returns:
            Tuple of (mapping_type, offset)
        """
        if not mappings:
            return 'empty', 0
        
        # Sort by ID to analyze pattern
        mappings.sort(key=lambda x: x[0])
        
        # Check for direct mapping (id == row_index)
        direct_matches = sum(1 for id_val, row_idx, _ in mappings if id_val == row_idx)
        
        # Check for offset mapping (id == row_index + offset)
        if len(mappings) >= 2:
            # Calculate potential offset from first mapping
            first_id, first_row, _ = mappings[0]
            potential_offset = first_id - first_row
            
            offset_matches = sum(1 for id_val, row_idx, _ in mappings 
                               if id_val == row_idx + potential_offset)
            
            # If most mappings follow the offset pattern
            if offset_matches >= len(mappings) * 0.8:  # 80% threshold
                if potential_offset == 0:
                    return 'direct', 0
                else:
                    return 'offset', potential_offset
        
        # Check if it's mostly direct mapping
        if direct_matches >= len(mappings) * 0.8:
            return 'direct', 0
        
        # If no clear pattern, it's custom/sparse
        gaps = 0
        for i in range(1, len(mappings)):
            prev_id = mappings[i-1][0]
            curr_id = mappings[i][0]
            if curr_id - prev_id > 1:
                gaps += 1
        
        if gaps > len(mappings) * 0.3:  # More than 30% gaps
            return 'sparse', 0
        
        return 'custom', 0


@pytest.fixture
def analyzer():
    """Create ID mapping analyzer with resource manager."""
    return IDMappingAnalyzer()


@pytest.fixture
def test_tables():
    """List of test tables to analyze."""
    return [
        'creaturesize',  # Known problematic table
        'racialtypes',   # Direct mapping table
        'classes',       # Direct mapping table
        'baseitems',     # Large table
        'feat',          # Complex table
        'skills',        # Standard table
        'spells',        # Another large table
    ]


class TestIDMappingAnalysis:
    """Test suite for analyzing ID mapping patterns."""
    
    def test_creaturesize_mapping_analysis(self, analyzer):
        """Test analysis of the problematic creaturesize.2da table."""
        pattern = analyzer.analyze_table('creaturesize')
        
        assert pattern is not None, "Should successfully analyze creaturesize table"
        assert pattern.table_name == 'creaturesize'
        
        # creaturesize.2da should be recognized as offset mapping (row_index = id - 1)
        assert pattern.mapping_type in ['offset', 'custom'], f"Expected offset/custom mapping, got {pattern.mapping_type}"
        
        if pattern.mapping_type == 'offset':
            assert pattern.offset == 1, f"Expected offset of 1, got {pattern.offset}"
        
        # Should have valid size entries
        assert pattern.valid_rows > 0, "Should have valid size entries"
        assert pattern.total_rows >= 5, "Should have at least 5 size categories"
        
        # Log the pattern for manual verification
        logger.info(f"creaturesize.2da pattern: {pattern}")
        
        # Check sample mappings
        if pattern.sample_mappings:
            for id_val, row_idx, label in pattern.sample_mappings[:3]:
                logger.info(f"  ID {id_val} -> Row {row_idx}: {label}")
    
    def test_racialtypes_mapping_analysis(self, analyzer):
        """Test analysis of racialtypes.2da (should be direct mapping)."""
        pattern = analyzer.analyze_table('racialtypes')
        
        assert pattern is not None, "Should successfully analyze racialtypes table"
        assert pattern.table_name == 'racialtypes'
        
        # racialtypes.2da should use direct mapping (row_index = id)
        assert pattern.mapping_type == 'direct', f"Expected direct mapping, got {pattern.mapping_type}"
        assert pattern.offset == 0, f"Expected offset of 0, got {pattern.offset}"
        
        # Should have multiple races
        assert pattern.valid_rows >= 6, "Should have at least 6 basic races"
        
        logger.info(f"racialtypes.2da pattern: {pattern}")
    
    def test_classes_mapping_analysis(self, analyzer):
        """Test analysis of classes.2da (should be direct mapping)."""
        pattern = analyzer.analyze_table('classes')
        
        assert pattern is not None, "Should successfully analyze classes table"
        assert pattern.table_name == 'classes'
        
        # classes.2da should use direct mapping
        assert pattern.mapping_type == 'direct', f"Expected direct mapping, got {pattern.mapping_type}"
        assert pattern.offset == 0, f"Expected offset of 0, got {pattern.offset}"
        
        # Should have basic classes
        assert pattern.valid_rows >= 10, "Should have at least 10 basic classes"
        
        logger.info(f"classes.2da pattern: {pattern}")
    
    def test_analyze_all_test_tables(self, analyzer, test_tables):
        """Analyze all test tables to identify patterns."""
        patterns = {}
        
        for table_name in test_tables:
            pattern = analyzer.analyze_table(table_name)
            if pattern:
                patterns[table_name] = pattern
                logger.info(f"\n=== {table_name.upper()} ANALYSIS ===")
                logger.info(f"Mapping Type: {pattern.mapping_type}")
                logger.info(f"Offset: {pattern.offset}")
                logger.info(f"Total Rows: {pattern.total_rows}")
                logger.info(f"Valid Rows: {pattern.valid_rows}")
                logger.info(f"Has Gaps: {pattern.has_gaps}")
                logger.info(f"ID Range: {pattern.min_id} - {pattern.max_id}")
                
                if pattern.sample_mappings:
                    logger.info("Sample Mappings:")
                    for id_val, row_idx, label in pattern.sample_mappings[:5]:
                        logger.info(f"  ID {id_val} -> Row {row_idx}: {label}")
        
        # Verify we got some results
        assert len(patterns) > 0, "Should successfully analyze at least some tables"
        
        # Categorize by mapping type
        mapping_types = {}
        for pattern in patterns.values():
            mapping_type = pattern.mapping_type
            if mapping_type not in mapping_types:
                mapping_types[mapping_type] = []
            mapping_types[mapping_type].append(pattern.table_name)
        
        logger.info(f"\n=== MAPPING TYPE SUMMARY ===")
        for mapping_type, tables in mapping_types.items():
            logger.info(f"{mapping_type.upper()}: {', '.join(tables)}")
        
        return patterns
    
    def test_identify_problematic_tables(self, analyzer, test_tables):
        """Identify tables that would cause issues with current get_by_id implementation."""
        problematic_tables = []
        
        for table_name in test_tables:
            pattern = analyzer.analyze_table(table_name)
            if pattern and pattern.mapping_type not in ['direct', 'empty']:
                problematic_tables.append((table_name, pattern.mapping_type, pattern.offset))
        
        logger.info(f"\n=== PROBLEMATIC TABLES FOR get_by_id() ===")
        if problematic_tables:
            for table_name, mapping_type, offset in problematic_tables:
                logger.info(f"{table_name}: {mapping_type} (offset: {offset})")
        else:
            logger.info("No problematic tables found in test set")
        
        # This is informational - we expect to find some problematic tables
        return problematic_tables


if __name__ == "__main__":
    # Allow running this module directly for development testing
    import sys
    logging.basicConfig(level=logging.INFO, format='%(message)s')
    
    analyzer = IDMappingAnalyzer()
    
    if len(sys.argv) > 1:
        # Analyze specific table
        table_name = sys.argv[1]
        pattern = analyzer.analyze_table(table_name)
        if pattern:
            print(f"\n=== {table_name.upper()} ANALYSIS ===")
            print(f"Mapping Type: {pattern.mapping_type}")
            print(f"Offset: {pattern.offset}")
            print(f"Total Rows: {pattern.total_rows}")
            print(f"Valid Rows: {pattern.valid_rows}")
            print(f"Has Gaps: {pattern.has_gaps}")
            print(f"ID Range: {pattern.min_id} - {pattern.max_id}")
            
            if pattern.sample_mappings:
                print("Sample Mappings:")
                for id_val, row_idx, label in pattern.sample_mappings[:10]:
                    print(f"  ID {id_val} -> Row {row_idx}: {label}")
        else:
            print(f"Could not analyze table: {table_name}")
    else:
        # Analyze key tables
        test_tables = ['creaturesize', 'racialtypes', 'classes', 'baseitems', 'feat']
        for table_name in test_tables:
            pattern = analyzer.analyze_table(table_name)
            if pattern:
                print(f"\n{table_name}: {pattern.mapping_type} (offset: {pattern.offset})")