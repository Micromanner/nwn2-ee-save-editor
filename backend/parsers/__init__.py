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
# Import parsers - Rust parsers are optional for standalone mode
try:
    from rust_tda_parser import TDAParser
except ImportError:
    TDAParser = None

try:
    from rust_tlk_parser import TLKParser
except ImportError:
    TLKParser = None

try:
    from rust_erf_parser import ErfParser as ERFParser
except ImportError:
    ERFParser = None
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
    
    # Other parsers (may be None if Rust modules not available)
    'TDAParser', 'TLKParser', 'ERFParser', 'ResourceManager'
]