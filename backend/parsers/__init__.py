"""
NWN2 file parsers
"""

from .gff import (
    GFFParser, GFFWriter, GFFElement, GFFFieldType, 
    LocalizedString, LocalizedSubstring
)
from .gff_validator import (
    GFFValidator, GFFRecovery, ValidationLevel, ValidationSeverity,
    ValidationIssue, FieldSchema, StructSchema  
)
from .gff_streaming import (
    StreamingGFFParser, StreamingOptions, LazyGFFElement,
    extract_character_name, count_module_areas
)
# Import Rust parsers
from rust_tda_parser import TDAParser
from .rust_tlk_parser import TLKParser
from rust_erf_parser import ErfParser as ERFParser
from .resource_manager import ResourceManager

__all__ = [
    # GFF
    'GFFParser', 'GFFWriter', 'GFFElement', 'GFFFieldType',
    'LocalizedString', 'LocalizedSubstring',
    
    # GFF Validation
    'GFFValidator', 'GFFRecovery', 'ValidationLevel', 'ValidationSeverity',
    'ValidationIssue', 'FieldSchema', 'StructSchema',
    
    # GFF Streaming
    'StreamingGFFParser', 'StreamingOptions', 'LazyGFFElement',
    'extract_character_name', 'count_module_areas',
    
    # Other parsers
    'TDAParser', 'TLKParser', 'ERFParser', 'ResourceManager'
]